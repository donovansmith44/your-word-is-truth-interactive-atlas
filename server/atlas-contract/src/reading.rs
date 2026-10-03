use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use atlas_core::data::{AtlasData, CanonBook};
use atlas_core::history::resolve_display_name;
use atlas_core::refs::VerseId;
use atlas_core::xrefs::aggregate_span_xrefs;
use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::id::Position;
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::VerseRef;

use crate::error::{ApiError, ReferenceRefusals};
use crate::reference::{ChapterReference, Reference, VerseSpan};
use crate::wire;

/// The books of the canon in order, each with the verse count of every one of its chapters.
#[utoipa::path(get, path = "/api/books", responses((status = 200, body = Vec<atlas_core::data::CanonBook>)), tag = "reading")]
pub async fn books(State(data): State<Arc<AtlasData>>) -> Json<Vec<CanonBook>> {
    Json(data.canon.books.clone())
}

/// One chapter of Scripture verse by verse: the text, the places and people named in it, its heading where it opens one, and how many cross references start at it.
///
/// `{cref}` is `BOOK.CHAPTER`, such as `EXO.14`; a book-only or verse-shaped
/// segment is `bad_ref`. A chapter number past the end of the book is not an
/// error -- the response carries an empty `verses` list.
#[utoipa::path(get, path = "/api/chapter/{cref}", params(("cref" = String, Path)), responses((status = 200, body = wire::Chapter), ReferenceRefusals), tag = "reading")]
pub async fn chapter(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Reference(ChapterReference { book, chapter }): Reference<ChapterReference>,
) -> Result<Json<wire::Chapter>, ApiError> {
    let code = book.code();

    let verse_count = data.canon.verses_in(book, chapter).unwrap_or(0);

    let snap = graph.snapshot();
    let graph_texts: HashMap<u16, String> = graph
        .chapter_span(book.0, chapter)
        .map(|(start, n)| {
            window::window(&snap, atlas_graph::kjv_adapter::BIBLE_CORPUS, start, n, WindowDir::Onward)
                .iter()
                .filter_map(|id| atlas_graph::kjv_adapter::decode_text_unit(id).map(|(_, _, v)| (v, window::render(&snap, id).unwrap_or_default())))
                .collect()
        })
        .unwrap_or_default();

    let scene_source = graph.scene_source(&data);

    let mut verses = Vec::new();
    for v in 1..=verse_count {
        let key = format!("{code}.{chapter}.{v}");
        if let Some(text) = graph_texts.get(&v) {
            let places = scene_source
                .places_for_verse(&key)
                .iter()
                .filter_map(|pid| scene_source.place(pid))
                .map(|p| place_ref(&p.id, resolve_display_name(&p.name, data.place_history_for(&p.id), None, data.place_name_alias_for(&p.id)), &snap))
                .collect::<Result<_, _>>()?;
            let persons = graph
                .persons_at_verse(book.0, chapter, v)
                .into_iter()
                .map(|(id, name)| wire::PersonRef { id, name })
                .collect();
            let heading = graph.heading_index.get(&key).cloned();
            // `cites` is always the Forward direction from a verse: the verses this
            // one cites, not the ones citing it.
            let verse_pos = Position::Node(atlas_graph::kjv_adapter::verse_node_id(book.0, chapter, v));
            let xref_count =
                snap.edge_summary(&verse_pos).get(&EdgeKind::Directed(RelationId::Cites, Direction::Forward)).copied().unwrap_or(0);
            let words_of_christ = graph.red_letter_spans.get(&key).map(|spans| spans.iter().map(|&(start, end)| wire::WordsOfChristSpan { start, end }).collect()).unwrap_or_default();
            verses.push(wire::Verse { verse: v, text: text.to_string(), places, persons, heading, xref_count, words_of_christ });
        }
    }

    Ok(Json(wire::Chapter { r#ref: format!("{code}.{chapter}"), book: book.name().to_string(), chapter, verses }))
}

/// Kretzmann's commentary for one chapter: the items on each verse that has any, in document order.
///
/// `{cref}` is `BOOK.CHAPTER`, such as `PSA.119`; a book-only or verse-shaped
/// segment is `bad_ref`. A chapter with no commentary answers an empty `verses`
/// list. Each item's `id` fetches its prose from `/api/node/{id}`.
#[utoipa::path(get, path = "/api/kretzmann/chapter/{cref}", params(("cref" = String, Path)), responses((status = 200, body = wire::KretzmannChapter), ReferenceRefusals), tag = "reading")]
pub async fn kretzmann_chapter(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Reference(ChapterReference { book, chapter }): Reference<ChapterReference>,
) -> Result<Json<wire::KretzmannChapter>, ApiError> {
    let verse_count = data.canon.verses_in(book, chapter).unwrap_or(0);

    let snap = graph.snapshot();
    let rows = atlas_graph::kretzmann_adapter::chapter_commentary(&snap, book.0, chapter, verse_count);

    let mut verses: Vec<wire::KretzmannChapterVerse> = Vec::new();
    for row in rows {
        let item = wire::KretzmannChapterItem { id: crate::graph_wire::encode_node_id(&row.item_id, &snap)?, heading: row.heading };
        match verses.last_mut() {
            Some(v) if v.verse == row.verse => v.items.push(item),
            _ => verses.push(wire::KretzmannChapterVerse { verse: row.verse, items: vec![item] }),
        }
    }

    Ok(Json(wire::KretzmannChapter { verses, version: atlas_graph::version_hex(graph.version()) }))
}

/// The cross references of a verse or a span, strongest first: each target, how strongly it is attested, and a preview of the text it points at.
///
/// `{sref}` is `BOOK.CHAPTER.VERSE` or a same-chapter span such as `GEN.1.1-5`;
/// a book-only or chapter-only reference is `bad_ref`. A reference with no
/// recorded cross references answers an empty list.
#[utoipa::path(get, path = "/api/xrefs/{sref}", params(("sref" = String, Path)), responses((status = 200, body = Vec<wire::CrossRef>), ReferenceRefusals), tag = "reading")]
pub async fn xrefs(State(data): State<Arc<AtlasData>>, State(graph): State<Arc<GraphService>>, Reference(VerseSpan(span)): Reference<VerseSpan>) -> Result<Json<Vec<wire::CrossRef>>, ApiError> {
    let by_from = graph.cross_refs_for_span(&span);
    let aggregated = aggregate_span_xrefs(&span, &by_from, |key| {
        let v = VerseId::parse_canonical(key).ok()?;
        graph.verse_text_of(&VerseRef { book: v.book.0, chapter: v.chapter, verse: v.verse })
    });
    let provenance = crate::provenance::all_titled(&graph.provenance.by_family(atlas_graph::provenance::family::CROSS_REFS), &data)?;
    let out = aggregated
        .into_iter()
        .map(|x| wire::CrossRef::attributed(x, &provenance))
        .collect();
    Ok(Json(out))
}

pub(crate) fn place_ref(id: &str, name: String, query: &dyn GraphQuery) -> Result<wire::PlaceRef, ApiError> {
    Ok(wire::PlaceRef { id: id.to_string(), name, node: crate::graph_wire::node_ref(&atlas_graph::event_world::place_stub_node_id(id), query)? })
}

pub(crate) fn drain_edges(
    snap: &impl atlas_graph_types::store::GraphQuery,
    p: &atlas_graph_types::id::Position,
    kind: atlas_graph_types::edge::EdgeKind,
) -> Vec<atlas_graph_types::adjacency::EdgeEntry> {
    let mut cursor = atlas_graph_types::adjacency::Cursor::FIRST;
    let mut out = Vec::new();
    loop {
        let page = snap.edges(p, &atlas_graph_types::adjacency::EdgeQuery { kind, cursor, limit: usize::MAX });
        out.extend(page.entries);
        match page.next {
            Some(c) => cursor = c,
            None => break,
        }
    }
    out
}

pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(books))
        .routes(routes!(chapter))
        .routes(routes!(kretzmann_chapter))
        .routes(routes!(xrefs))
}
