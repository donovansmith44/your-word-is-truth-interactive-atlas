use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;

use atlas_core::data::AtlasData;
use atlas_core::refs::{ScriptureRef, VerseId};
use atlas_graph::GraphService;
use atlas_graph_types::text::VerseRef;

use crate::error::ApiError;
use crate::wire;

/// `GET /api/catechism/{sref}` (Batch F, "verse -> citing catechism items
/// lookup" -- requirement 3): `sref` must parse as exactly a
/// `ScriptureRef::Verse` or `ScriptureRef::Passage`, mirroring
/// `reading::xrefs`'s own accepted-shapes precedent exactly (a bare book or
/// chapter ref has no defined "member verses" to aggregate over, so both 400
/// as `bad_ref`). ruling-3-policy: an sref with no citing catechism items at
/// all is NOT an error -- 200 with an empty list, same "gracefully empty,
/// never a 404" policy `xrefs`/`chapter`/`scene_scripture` already follow.
/// Business logic (the union-across-member-verses aggregation) lives in
/// `atlas_core::catechism::items_for_span`, reached here via
/// `AtlasData::catechism_items_for_span` -- this handler is pure
/// response-shape assembly, same as every other handler in this crate.
///
/// PROV-1 FIX ROUND 1 (review M-3): now takes `State<Arc<GraphService>>`
/// too, purely to attribute each row. That is the SECOND extractor the
/// review thought would make this half "a genuinely larger change" -- it is
/// not: `AppState` already implements `FromRef<AppState>` for
/// `Arc<GraphService>`, and six handlers in this crate already take both.
/// The aggregation itself is untouched; `AtlasData` is still where the
/// business logic lives.
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

/// `GET /api/catechism/item/{id}` (Batch F, "item fetch by id" --
/// requirement 3). Unknown id -> 404 `not_found`, same precedent
/// `places::place` already set for an exact-identifier lookup (not a ref
/// with its own "out of canon but still valid shape" middle ground). Each
/// proof verse resolves to its own full KJV text (`CatechismProofVerse`'s
/// own doc comment); a verse id that fails to resolve (should never happen
/// -- `validate::run_catechism` already guarantees every curated verse
/// exists in the compiled KJV text) is skipped rather than panicking, same
/// ruling-4 soft-fail policy `reading::verse`'s own cross-ref preview
/// lookup already follows.
///
/// OVERLAY-1 Task 2: proof-verse text now comes from `graph.verse_text_of`
/// (a real, on-demand graph query), not the retired `AtlasData.verses` --
/// this handler picks up a second extractor, `State<Arc<GraphService>>`,
/// the same combined-state pattern `reading::verse` already uses.
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

    // Batch F2: THE SCRIPTURES is the item-level `verses` (Luther's own
    // embedded citations, Batch F, `question: None`, listed FIRST -- "items
    // keep their F-batch embedded-citation links too" reads naturally as
    // the primary, first-listed source) followed by each of `questions[]`,
    // in curated order, each contributing its OWN verses tagged with its
    // OWN question title. Deduped by (vref, question) -- a verse cited
    // twice under the exact same question (or twice with no question) never
    // renders as two identical rows; a verse legitimately cited BOTH ways
    // (once bare, once under a question) still shows once per way, since
    // that's genuinely two different pieces of information (see this
    // handler's own module-level citation-integrity discipline: never
    // silently drop a real distinction).
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
