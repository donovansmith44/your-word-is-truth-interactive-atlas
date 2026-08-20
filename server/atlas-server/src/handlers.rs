//! Thin HTTP handlers: parse request params, call into `atlas_core`, wrap
//! the result in `Json`. No business logic lives here beyond response-shape
//! assembly and the out-of-canon policy documented at each handler that
//! needs one (controller ruling 3).

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Serialize;

use atlas_core::data::{AtlasData, BookMeta, CanonBook, Era, Landmark, Narrative};
use atlas_core::history::{resolve_blurb, resolve_display_name};
use atlas_core::refs::{ScriptureRef, VerseId};
use atlas_core::scene::{compose_scripture_scene, compose_time_scene, to_scene_event};
use atlas_core::time::TimeRange;
use atlas_core::wire::{Scene, SceneEvent, VerseGroup};

use crate::error::ApiError;

pub async fn health() -> &'static str {
    "ok"
}

fn parse_year(params: &HashMap<String, String>, key: &str) -> Result<i32, ApiError> {
    params.get(key).and_then(|s| s.parse::<i32>().ok()).ok_or_else(ApiError::bad_window)
}

/// `GET /api/scene?from=&to=`. `from`/`to` are read out of a
/// `Query<HashMap<String, String>>` rather than a strongly-typed `Query<T>`
/// specifically so a missing or unparseable value can never trigger axum's
/// own extractor-rejection response (ruling 1) — `HashMap<String, String>`
/// cannot itself fail to deserialize on these inputs, so every failure mode
/// (missing, non-integer, zero, inverted) is handled by this function and
/// always yields the typed `bad_window` body.
pub async fn scene_time(
    State(data): State<Arc<AtlasData>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Scene>, ApiError> {
    let from = parse_year(&params, "from")?;
    let to = parse_year(&params, "to")?;
    let window = TimeRange::new(from, to).map_err(|_| ApiError::bad_window())?;
    Ok(Json(compose_time_scene(&data, window)))
}

/// `GET /api/scene/scripture?ref=`.
///
/// ruling-3-policy: a `ref` that fails to *parse* (unknown book code, zero
/// chapter/verse, empty segment, inverted range — i.e. `ScriptureRef::parse`
/// returns `Err`) is a structurally bad ref and always 400s as `bad_ref`,
/// same as a missing `ref` param (treated as parsing the empty string, which
/// also fails to parse). A `ref` that *parses* but names coordinates outside
/// the loaded canon (e.g. a chapter number past the end of the book) is
/// deliberately NOT an error: `compose_scripture_scene` only ever matches it
/// against verses that actually exist on events/places, so an out-of-canon
/// ref naturally composes an empty-but-valid scene, exactly mirroring
/// ruling 2's "don't reject out-of-span time windows" for the sibling
/// endpoint. This needs no extra bounds-checking code — it falls out of not
/// adding any.
pub async fn scene_scripture(
    State(data): State<Arc<AtlasData>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Scene>, ApiError> {
    let raw = params.get("ref").map(String::as_str).unwrap_or("");
    let r = ScriptureRef::parse(raw).map_err(|_| ApiError::bad_ref(raw))?;
    Ok(Json(compose_scripture_scene(&data, &r)))
}

pub async fn books(State(data): State<Arc<AtlasData>>) -> Json<Vec<CanonBook>> {
    Json(data.canon.books.clone())
}

pub async fn eras(State(data): State<Arc<AtlasData>>) -> Json<Vec<Era>> {
    Json(data.eras.clone())
}

pub async fn narratives(State(data): State<Arc<AtlasData>>) -> Json<Vec<Narrative>> {
    Json(data.narratives.clone())
}

pub async fn landmarks(State(data): State<Arc<AtlasData>>) -> Json<Vec<Landmark>> {
    Json(data.landmarks.clone())
}

/// Batch B2 ("borders v2, the cartographer's edition"): one polity-era row
/// on the wire, `{ id, name, from, to, rings, color_key }` per the batch
/// brief verbatim -- field names deliberately match
/// `atlas_core::data::PolityEra`'s own (`from`/`to`, not `from_year`/
/// `to_year`; see that struct's doc comment for why) so this is a plain
/// copy, not a rename. `id` is the POLITY's id (constant across every era
/// row this same polity contributes to a response), `name`/`from`/`to`/
/// `rings` are this specific ERA's own fields, and `color_key` is the
/// polity's own precomputed hash (`Polity::color_key` -- copied here
/// unchanged, never rehashed per-request).
#[derive(Debug, Serialize)]
pub struct PolityOut {
    pub id: String,
    pub name: String,
    pub from: i32,
    pub to: i32,
    pub rings: Vec<Vec<(f64, f64)>>,
    pub color_key: u8,
}

#[derive(Debug, Serialize)]
pub struct PolitiesOut {
    pub polities: Vec<PolityOut>,
}

/// `GET /api/polities?from=&to=`. `from`/`to` share `scene_time`'s lenient
/// parsing (ruling 1: missing/unparseable/zero/inverted -> 400
/// `bad_window`), via the same `parse_year` helper and `TimeRange::new`
/// validity check.
///
/// Once the window is valid, emits every era (of every polity) whose own
/// `[from,to]` intersects it -- a polity with several eras in view (a
/// window spanning a border change) contributes one row PER intersecting
/// era, all sharing that polity's own `id`/`color_key`; a window matching no
/// era at all (out-of-span, or the `demo_fixture()`/pre-ETL case where
/// `data.polities` is empty) is not an error, mirroring `scene_time`'s own
/// ruling-2 spirit -- it 200s with an empty `polities` array. Deterministic
/// order: by polity id, then by era `from` -- so a multi-era window always
/// lists a polity's OLDER era before its newer one (the exact order
/// map.js's `BorderLayer` needs to paint older-under-newer and pick the
/// dotted/lightest era correctly without re-sorting client-side).
pub async fn polities(
    State(data): State<Arc<AtlasData>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<PolitiesOut>, ApiError> {
    let from = parse_year(&params, "from")?;
    let to = parse_year(&params, "to")?;
    let window = TimeRange::new(from, to).map_err(|_| ApiError::bad_window())?;

    let mut out: Vec<PolityOut> = Vec::new();
    for p in &data.polities {
        for era in &p.eras {
            let era_range = TimeRange { from_year: era.from, to_year: era.to };
            if window.intersects(&era_range) {
                out.push(PolityOut {
                    id: p.id.clone(),
                    name: era.name.clone(),
                    from: era.from,
                    to: era.to,
                    rings: era.rings.clone(),
                    color_key: p.color_key,
                });
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id).then(a.from.cmp(&b.from)));

    Ok(Json(PolitiesOut { polities: out }))
}

#[derive(Debug, Serialize)]
pub struct VerseOut {
    pub verse: u16,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct ChapterOut {
    #[serde(rename = "ref")]
    pub sref: String,
    pub book: String,
    pub chapter: u16,
    pub verses: Vec<VerseOut>,
}

/// `GET /api/chapter/{cref}`. `cref` must parse as exactly a
/// `ScriptureRef::Chapter` (book + chapter, e.g. `EXO.14`) — a book-only or
/// verse/passage-shaped path segment is the wrong shape for this endpoint
/// and 400s as `bad_ref`, same as an unparseable one. The optional
/// `?translation=kjv` query param (ruling 5, M1 is KJV-only) is never
/// extracted, so its presence or absence cannot affect this handler at all.
///
/// ruling-3-policy: once `cref` parses as a `Chapter`, an out-of-range
/// chapter number (or a book with no known chapters in this atlas) is NOT an
/// error — the verse-count bound comes from `canon.books[].chapters`, and an
/// unknown/short chapter just yields `verse_count = 0`, i.e. a 200 response
/// with an empty `verses` list. Same rationale as `scene_scripture`: a
/// reader showing "no verses in this chapter" is a meaningful response, not
/// a failure.
pub async fn chapter(State(data): State<Arc<AtlasData>>, Path(cref): Path<String>) -> Result<Json<ChapterOut>, ApiError> {
    let (book, chapter) = match ScriptureRef::parse(&cref) {
        Ok(ScriptureRef::Chapter { book, chapter }) => (book, chapter),
        _ => return Err(ApiError::bad_ref(&cref)),
    };
    let code = book.code();

    let verse_count = data
        .canon
        .books
        .iter()
        .find(|b| b.code == code)
        .and_then(|b| b.chapters.get((chapter - 1) as usize))
        .copied()
        .unwrap_or(0);

    let mut verses = Vec::new();
    for v in 1..=verse_count {
        let key = format!("{code}.{chapter}.{v}");
        if let Some(text) = data.verses.get(&key) {
            verses.push(VerseOut { verse: v, text: text.clone() });
        }
    }

    Ok(Json(ChapterOut { sref: format!("{code}.{chapter}"), book: book.name().to_string(), chapter, verses }))
}

#[derive(Debug, Serialize)]
pub struct BookMetaOut {
    pub author: String,
    pub write_place: Option<String>,
    pub write_from: Option<i32>,
    pub write_to: Option<i32>,
}

/// The verse-detail endpoint's event shape: `SceneEvent`'s fields
/// (id/label/when/verse_groups) plus the event's place ids, so the client
/// can jump from a verse to "explore this event on the map" without a
/// second round trip.
#[derive(Debug, Serialize)]
pub struct VerseEventOut {
    pub id: String,
    pub label: String,
    pub when: TimeRange,
    pub verse_groups: Vec<VerseGroup>,
    pub places: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CrossRefOut {
    pub target: String,
    pub votes: i32,
    pub preview: String,
}

#[derive(Debug, Serialize)]
pub struct VerseDetailOut {
    #[serde(rename = "ref")]
    pub sref: String,
    pub text: String,
    pub book_meta: BookMetaOut,
    pub events: Vec<VerseEventOut>,
    pub cross_refs: Vec<CrossRefOut>,
}

/// Mirrors atlas-etl's private `xrefs::first_verse_of_target` (duplicated,
/// not shared, because atlas-server does not and should not depend on
/// atlas-etl — that crate is a build-time-only ETL binary, not a runtime
/// library). Extracts the first verse id referenced by an
/// already-canonicalized cross-ref target string, which is either a single
/// verse (`"PSA.124.8"`), a same-chapter span (`"COL.1.16-19"`), or a
/// cross-chapter/book span (`"MAT.5.3-MAT.6.2"`).
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

/// `GET /api/verse/{vref}`. `vref` must parse as exactly a
/// `ScriptureRef::Verse` (`VerseId::parse_canonical` enforces this) — any
/// other shape 400s as `bad_ref`.
///
/// ruling-3-policy: unlike the scene/chapter endpoints, a structurally valid
/// vref whose text is absent from this atlas's compiled KJV map is 404
/// `not_found`, not a 200-with-placeholder. A single verse is an
/// individually-addressed resource (like `/api/place/{id}`), not a
/// list/scene that can be gracefully empty — there is no non-misleading way
/// to represent "this verse doesn't exist" other than "not found", so this
/// endpoint intentionally follows `/api/place/{id}`'s precedent rather than
/// `scene_scripture`'s/`chapter`'s "out-of-canon is still 200" policy.
///
/// Cross-ref preview rows fail soft (ruling 4): ETL guarantees every
/// compiled cross-ref target's first verse exists in the verses map, but if
/// that's ever violated the row is skipped rather than panicking.
pub async fn verse(State(data): State<Arc<AtlasData>>, Path(vref): Path<String>) -> Result<Json<VerseDetailOut>, ApiError> {
    let vid = VerseId::parse_canonical(&vref).map_err(|_| ApiError::bad_ref(&vref))?;
    let canonical = format!("{}.{}.{}", vid.book.code(), vid.chapter, vid.verse);

    let text = data.verses.get(&canonical).cloned().ok_or_else(|| ApiError::not_found("verse"))?;

    let book_meta = data.books_meta.iter().find(|b| b.book == vid.book.code()).cloned().unwrap_or_else(|| BookMeta {
        book: vid.book.code().to_string(),
        author: String::new(),
        write_place: None,
        write_from: None,
        write_to: None,
    });

    let events: Vec<VerseEventOut> = data
        .events_for_verse(&canonical)
        .iter()
        .filter_map(|id| data.event_by_id(id))
        .map(|e| {
            let se = to_scene_event(e);
            VerseEventOut { id: se.id, label: se.label, when: se.when, verse_groups: se.verse_groups, places: e.places.clone() }
        })
        .collect();

    let cross_refs: Vec<CrossRefOut> = data
        .cross_refs
        .get(&canonical)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(|cr| {
            let first = first_verse_of_target(&cr.target)?;
            let key = format!("{}.{}.{}", first.book.code(), first.chapter, first.verse);
            let preview = data.verses.get(&key)?;
            Some(CrossRefOut { target: cr.target.clone(), votes: cr.votes, preview: preview.clone() })
        })
        .collect();

    Ok(Json(VerseDetailOut {
        sref: canonical,
        text,
        book_meta: BookMetaOut {
            author: book_meta.author,
            write_place: book_meta.write_place,
            write_from: book_meta.write_from,
            write_to: book_meta.write_to,
        },
        events,
        cross_refs,
    }))
}

/// Batch E: one curated established/destroyed date claim, as served by
/// `/api/place/{id}`. `when` reuses `TimeRange`'s own wire shape
/// (`from_year`/`to_year`) rather than a separate "year" field -- the
/// client's `YearText.FormatRange` already collapses equal endpoints to a
/// single-year display, so a genuine year and a range need no separate flag.
#[derive(Debug, Serialize)]
pub struct DateClaimOut {
    pub when: TimeRange,
    pub verses: Vec<String>,
    pub note: Option<String>,
}

/// Batch E: `/api/place/{id}`'s optional `history` payload, present only
/// when this place has a curated `PlaceHistory` record at all (`when
/// curated`, per the brief). `display_name` and `blurb` are resolved
/// against the request's `?from=&to=` window when given (else `display_name`
/// falls back to the place's own default `name` and `blurb` is omitted --
/// see `history::resolve_display_name`/`resolve_blurb`'s own doc comments);
/// `established`/`destroyed` are window-independent static facts, always
/// included verbatim whenever curated.
#[derive(Debug, Serialize)]
pub struct HistoryOut {
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blurb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub established: Option<DateClaimOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destroyed: Option<DateClaimOut>,
}

#[derive(Debug, Serialize)]
pub struct PlaceDetailOut {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub events: Vec<SceneEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<HistoryOut>,
}

/// `GET /api/place/{id}?from=&to=`. Unknown id -> 404 `not_found` (a place
/// id is an exact identifier, not a ref that can be "out of canon but still
/// valid shape" — there's no parsing/shape question here at all).
///
/// `from`/`to` are OPTIONAL (Batch E) and, unlike `scene_time`/`borders`'s
/// own `from`/`to`, never themselves cause a 400: this endpoint's core
/// resource (the place, its events) is fully meaningful with neither
/// present, so a missing or malformed window just means `history` (when the
/// place has one at all) reports its default display name and no blurb,
/// rather than rejecting the whole request over an optional refinement.
pub async fn place(
    State(data): State<Arc<AtlasData>>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<PlaceDetailOut>, ApiError> {
    let place = data.place_by_id(&id).ok_or_else(|| ApiError::not_found("place"))?;

    let mut events: Vec<SceneEvent> = data.events.iter().filter(|e| e.places.contains(&id)).map(to_scene_event).collect();
    events.sort_by_key(|e| e.when.from_year);

    let window = match (params.get("from").and_then(|s| s.parse::<i32>().ok()), params.get("to").and_then(|s| s.parse::<i32>().ok()))
    {
        (Some(from), Some(to)) => TimeRange::new(from, to).ok(),
        _ => None,
    };

    let history = data.place_history_for(&id).map(|h| HistoryOut {
        display_name: resolve_display_name(&place.name, Some(h), window),
        blurb: window.and_then(|w| resolve_blurb(&h.blurbs, w)).map(|b| b.text.clone()),
        established: h.established.as_ref().map(|c| DateClaimOut { when: c.when, verses: c.verses.clone(), note: c.note.clone() }),
        destroyed: h.destroyed.as_ref().map(|c| DateClaimOut { when: c.when, verses: c.verses.clone(), note: c.note.clone() }),
    });

    Ok(Json(PlaceDetailOut { id: place.id.clone(), name: place.name.clone(), lat: place.lat, lon: place.lon, events, history }))
}
