use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;

use atlas_core::data::AtlasData;
use atlas_core::refs::{ScriptureRef, VerseId};
use atlas_graph::GraphService;
use atlas_graph_types::text::VerseRef;

use crate::error::ApiError;
use crate::wire;

/// The catechism items that cite a verse or a span, each named and tied to the question it was cited under.
///
/// `{sref}` is `BOOK.CHAPTER.VERSE` or a same-chapter span such as `GEN.1.1-5`;
/// a book-only or chapter-only reference is `bad_ref`. A reference no item cites
/// answers an empty list, and each `id` fetches the whole item from
/// `/api/catechism/item/{id}`.
#[utoipa::path(get, path = "/api/catechism/{sref}", params(("sref" = String, Path)), responses((status = 200, body = Vec<wire::CatechismRef>), ApiError), tag = "catechism")]
pub async fn catechism_for_span(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Path(sref): Path<String>,
) -> Result<Json<Vec<wire::CatechismRef>>, ApiError> {
    let span = match ScriptureRef::parse(&sref) {
        Ok(span @ (ScriptureRef::Verse(_) | ScriptureRef::Passage { .. })) => span,
        _ => return Err(ApiError::bad_ref(&sref)),
    };

    let provenance = graph.provenance.by_family(atlas_graph::provenance::family::CATECHISM);
    let out = data.catechism_items_for_span(&span).into_iter().map(|c| wire::CatechismRef::attributed(c, &provenance)).collect();
    Ok(Json(out))
}

/// One catechism item in full: its own words, its explanation, the chief part it belongs to, and every proof verse with the verse text spelled out.
///
/// `{id}` is an item id handed back by `/api/catechism/{sref}` or by a verse's
/// own catechism list; an id naming no item is `not_found`.
#[utoipa::path(get, path = "/api/catechism/item/{id}", params(("id" = String, Path)), responses((status = 200, body = wire::CatechismItem), ApiError), tag = "catechism")]
pub async fn catechism_item(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Path(id): Path<String>,
) -> Result<Json<wire::CatechismItem>, ApiError> {
    let (part, item) = data.catechism_item_by_id(&id).ok_or_else(|| ApiError::not_found("catechism item"))?;

    let text_of = |v: &str| -> Option<String> {
        let vid = VerseId::parse_canonical(v).ok()?;
        graph.verse_text_of(&VerseRef { book: vid.book.0, chapter: vid.chapter, verse: vid.verse })
    };

    // Keyed by (verse, question) rather than by verse: the same verse cited once
    // bare and once under a question is two distinct citations, not a duplicate.
    let mut seen: std::collections::HashSet<(String, Option<String>)> = std::collections::HashSet::new();
    let mut verses: Vec<wire::CatechismProofVerse> = Vec::new();
    for v in &item.verses {
        if !seen.insert((v.clone(), None)) {
            continue;
        }
        if let Some(text) = text_of(v) {
            verses.push(wire::CatechismProofVerse { vref: v.clone(), text, question: None });
        }
    }
    for q in &item.questions {
        for v in &q.verses {
            if !seen.insert((v.clone(), Some(q.title.clone()))) {
                continue;
            }
            if let Some(text) = text_of(v) {
                verses.push(wire::CatechismProofVerse { vref: v.clone(), text, question: Some(q.title.clone()) });
            }
        }
    }

    Ok(Json(wire::CatechismItem {
        id: item.id.clone(),
        name: item.name.clone(),
        part_title: part.title.clone(),
        text: item.text.clone(),
        explanation_heading: item.explanation_heading.clone(),
        explanation: item.explanation.clone(),
        where_written: item.where_written.clone(),
        verses,
    }))
}

pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(catechism_item))
        .routes(routes!(catechism_for_span))
}
