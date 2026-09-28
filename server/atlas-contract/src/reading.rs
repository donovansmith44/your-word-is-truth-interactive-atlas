use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use atlas_core::data::{AtlasData, CanonBook, Event};
use atlas_core::history::resolve_display_name;
use atlas_core::refs::{ScriptureRef, VerseId};
use atlas_core::scene::to_scene_event;
use atlas_core::xrefs::{aggregate_span_xrefs, AggregatedXref};
use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::id::Position;
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::VerseRef;

use crate::error::{ApiError, ReferenceRefusals};
use crate::reference::{ChapterReference, Reference, VerseReference, VerseSpan};
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

    let verse_count = data
        .canon
        .books
        .iter()
        .find(|b| b.code == code)
        .and_then(|b| b.chapters.get((chapter - 1) as usize))
        .copied()
        .unwrap_or(0);

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
                .map(|p| wire::PlaceRef {
                    id: p.id.clone(),
                    // The reader locates a mention by matching this name against the
                    // verse's own words, so it must carry the wording the translation
                    // uses rather than the default modern name.
                    name: resolve_display_name(&p.name, data.place_history_for(&p.id), None, data.place_name_alias_for(&p.id)),
                })
                .collect();
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

    Ok(Json(wire::Chapter { sref: format!("{code}.{chapter}"), book: book.name().to_string(), chapter, verses }))
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
    let code = book.code();

    let verse_count = data
        .canon
        .books
        .iter()
        .find(|b| b.code == code)
        .and_then(|b| b.chapters.get((chapter - 1) as usize))
        .copied()
        .unwrap_or(0);

    let snap = graph.snapshot();
    let rows = atlas_graph::kretzmann_adapter::chapter_commentary(&snap, book.0, chapter, verse_count);

    let mut verses: Vec<wire::KretzmannChapterVerse> = Vec::new();
    for row in rows {
        let item = wire::KretzmannChapterItem { id: crate::graph_wire::encode_node_id(&row.item_id), heading: row.heading };
        match verses.last_mut() {
            Some(v) if v.verse == row.verse => v.items.push(item),
            _ => verses.push(wire::KretzmannChapterVerse { verse: row.verse, items: vec![item] }),
        }
    }

    Ok(Json(wire::KretzmannChapter { verses, version: atlas_graph::version_hex(graph.version()) }))
}

/// One verse in full: its text, who wrote its book, the events it belongs to, its cross references with previews, and the catechism items citing it.
///
/// `{vref}` is `BOOK.CHAPTER.VERSE`, such as `JHN.3.16`; any other shape is
/// `bad_ref`, and a well-formed reference this atlas holds no text for is
/// `not_found`.
#[utoipa::path(get, path = "/api/verse/{vref}", params(("vref" = String, Path)), responses((status = 200, body = wire::VerseDetail), ReferenceRefusals), tag = "reading")]
pub async fn verse(State(data): State<Arc<AtlasData>>, State(graph): State<Arc<GraphService>>, Reference(VerseReference(vid)): Reference<VerseReference>) -> Result<Json<wire::VerseDetail>, ApiError> {
    let canonical = format!("{}.{}.{}", vid.book.code(), vid.chapter, vid.verse);

    let snap = graph.snapshot();
    let text_id = atlas_graph::kjv_adapter::verse_node_id(vid.book.0, vid.chapter, vid.verse);
    let text = window::render(&snap, &text_id).ok_or_else(|| ApiError::not_found("verse"))?;
    let provenance = snap
        .node(&text_id)
        .map(|n| n.provenance)
        .filter(|p| !p.trim().is_empty())
        .ok_or_else(|| ApiError::internal(&format!("verse {canonical} rendered text with no node to attribute it to")))?;

    let book_meta = data.books_meta.iter().find(|b| b.book == vid.book.code()).cloned().unwrap_or_else(|| atlas_core::data::BookMeta {
        book: vid.book.code().to_string(),
        author: String::new(),
        write_place: None,
        write_from: None,
        write_to: None,
    });

    // An event whose own verses and one of its witnesses both name this verse must
    // still surface once.
    let mut seen_events: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut attesting_events: Vec<Event> = drain_edges(&snap, &Position::Node(text_id.clone()), EdgeKind::Directed(RelationId::Attests, Direction::Inverse))
        .into_iter()
        .filter_map(|entry| match entry.node {
            Position::Node(eid) => Some(eid),
            Position::Edge(_) => None,
        })
        .filter(|eid| seen_events.insert(eid.raw.clone()))
        .filter_map(|eid| atlas_graph::legacy::event_from_node(&eid, &snap, &graph.chronology.chrono))
        .collect();
    attesting_events.sort_by_key(|e| e.when.from_year);
    let events: Vec<wire::VerseEvent> = attesting_events
        .into_iter()
        .map(|e| {
            let node_provenance = snap
                .node(&atlas_graph::event_world::event_node_id(&e.id))
                .map(|n| n.provenance)
                .filter(|p| !p.trim().is_empty())
                .ok_or_else(|| ApiError::internal(&format!("event {} has no provenance to attribute this membership row to", e.id)))?;
            let se = to_scene_event(&e);
            let when = if e.kind == atlas_core::data::EventKind::Event { Some(se.when) } else { None };
            Ok(wire::VerseEvent {
                id: se.id,
                label: se.label,
                when,
                verse_groups: se.verse_groups,
                places: e.places.clone(),
                kind: e.kind,
                provenance: node_provenance,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    let cross_refs_provenance = graph.provenance.by_family(atlas_graph::provenance::family::CROSS_REFS);
    let by_from = graph.cross_refs_for_span(&ScriptureRef::Verse(vid));
    let cross_refs: Vec<wire::CrossRef> = by_from
        .get(&canonical)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(|cr| {
            let first = first_verse_of_target(&cr.target)?;
            let preview = graph.verse_text_of(&VerseRef { book: first.book.0, chapter: first.chapter, verse: first.verse })?;
            Some(wire::CrossRef::attributed(AggregatedXref { target: cr.target.clone(), votes: cr.votes, preview }, &cross_refs_provenance))
        })
        .collect();

    let catechism_provenance = graph.provenance.by_family(atlas_graph::provenance::family::CATECHISM);
    let catechism: Vec<wire::CatechismRef> = data
        .catechism_items_for_span(&ScriptureRef::Verse(vid))
        .into_iter()
        .map(|c| wire::CatechismRef::attributed(c, &catechism_provenance))
        .collect();

    let words_of_christ: Vec<wire::WordsOfChristSpan> = graph.red_letter_spans.get(&canonical).map(|spans| spans.iter().map(|&(start, end)| wire::WordsOfChristSpan { start, end }).collect()).unwrap_or_default();

    Ok(Json(wire::VerseDetail {
        sref: canonical,
        text,
        words_of_christ,
        book_meta,
        events,
        cross_refs,
        catechism,
        provenance,
        cross_refs_provenance,
        catechism_provenance,
    }))
}

/// The cross references of a verse or a span, strongest first: each target, how strongly it is attested, and a preview of the text it points at.
///
/// `{sref}` is `BOOK.CHAPTER.VERSE` or a same-chapter span such as `GEN.1.1-5`;
/// a book-only or chapter-only reference is `bad_ref`. A reference with no
/// recorded cross references answers an empty list.
#[utoipa::path(get, path = "/api/xrefs/{sref}", params(("sref" = String, Path)), responses((status = 200, body = Vec<wire::CrossRef>), ReferenceRefusals), tag = "reading")]
pub async fn xrefs(State(graph): State<Arc<GraphService>>, Reference(VerseSpan(span)): Reference<VerseSpan>) -> Result<Json<Vec<wire::CrossRef>>, ApiError> {
    let by_from = graph.cross_refs_for_span(&span);
    let aggregated = aggregate_span_xrefs(&span, &by_from, |key| {
        let v = VerseId::parse_canonical(key).ok()?;
        graph.verse_text_of(&VerseRef { book: v.book.0, chapter: v.chapter, verse: v.verse })
    });
    let provenance = graph.provenance.by_family(atlas_graph::provenance::family::CROSS_REFS);
    let out = aggregated
        .into_iter()
        .map(|x| wire::CrossRef::attributed(x, &provenance))
        .collect();
    Ok(Json(out))
}

/// The first verse id a canonicalised cross-ref target names, whether that target
/// is one verse, a same-chapter span or a cross-chapter span. Duplicated from the
/// ETL binary rather than shared: nothing serving a request may depend on it.
fn first_verse_of_target(target: &str) -> Option<VerseId> {
    if let Ok(v) = VerseId::parse_canonical(target) {
        return Some(v);
    }
    if let Ok(ScriptureRef::Passage { book, chapter, from_verse, .. }) = ScriptureRef::parse(target) {
        return Some(VerseId { book, chapter, verse: from_verse });
    }
    let (left, _right) = target.split_once('-')?;
    VerseId::parse_canonical(left).ok()
}

pub(crate) fn drain_edges(
    snap: &impl atlas_graph_types::store::GraphQuery,
    p: &atlas_graph_types::id::Position,
    kind: atlas_graph_types::edge::EdgeKind,
) -> Vec<atlas_graph_types::explore::EdgeEntry> {
    let mut cursor = None;
    let mut out = Vec::new();
    loop {
        let page = snap.edges(p, &atlas_graph_types::explore::EdgeQuery { kind, cursor, limit: 200 });
        out.extend(page.entries);
        match page.next {
            Some(c) => cursor = Some(c),
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
        .routes(routes!(verse))
        .routes(routes!(xrefs))
}
