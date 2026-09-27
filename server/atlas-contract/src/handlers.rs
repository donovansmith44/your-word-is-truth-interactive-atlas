//! Thin HTTP handlers: parse request params, call into `atlas_core`, wrap
//! the result in `Json`. No business logic lives here beyond response-shape
//! assembly and the out-of-canon policy documented at each handler that
//! needs one (controller ruling 3).

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Serialize;

use atlas_core::data::{AtlasData, BookMeta, CanonBook, Era, Event, Landmark, Narrative};
use atlas_core::history::{resolve_blurb, resolve_display_name, resolve_display_name_and_canonical};
use atlas_core::refs::{ScriptureRef, VerseId};
use atlas_core::scene::{compose_scripture_scene, compose_time_scene, to_scene_event};
use atlas_core::sources::SourcesDocument;
use atlas_core::time::TimeRange;
use atlas_core::wire::{Scene, SceneEvent, VerseGroup};
use atlas_core::xrefs::aggregate_span_xrefs;
use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::id::Position;
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::VerseRef;

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
///
/// OVERLAY-1 Task 5: the scene's DATA now comes from the graph port --
/// `graph.scene_source(&data)`, an `atlas_graph::scene_source::
/// GraphSceneSource` built once at load from `GraphService`'s own snapshot,
/// not from `AtlasData`'s former graph-derived `events`/`places`/
/// `narratives` fields (which `legacy::atlas_data_overlay` used to
/// reconstruct at boot, and which are no longer written on any serving
/// path). `data` is still extracted because that source reads two genuinely
/// curated-JSON sidecars through it (`place-history.json`,
/// `place-names-kjv.json`). The composed bytes are unchanged -- `tests/
/// scene_byte_identity.rs`'s 25 pinned hashes are the gate on that.
pub async fn scene_time(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Scene>, ApiError> {
    let from = parse_year(&params, "from")?;
    let to = parse_year(&params, "to")?;
    let window = TimeRange::new(from, to).map_err(|_| ApiError::bad_window())?;
    Ok(Json(compose_time_scene(graph.scene_source(&data), window)))
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
///
/// OVERLAY-1 Task 5: composes from `graph.scene_source(&data)`, exactly as
/// `scene_time` above does -- see that handler's own doc comment.
pub async fn scene_scripture(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Scene>, ApiError> {
    let raw = params.get("ref").map(String::as_str).unwrap_or("");
    let r = ScriptureRef::parse(raw).map_err(|_| ApiError::bad_ref(raw))?;
    Ok(Json(compose_scripture_scene(graph.scene_source(&data), &r)))
}

pub async fn books(State(data): State<Arc<AtlasData>>) -> Json<Vec<CanonBook>> {
    Json(data.canon.books.clone())
}

/// `GET /api/eras` (Batch M-C, controller decision 7: "map migration...
/// become views over graph queries"). BATCH M-C: re-implemented as a VIEW
/// over the graph's own Era nodes (`atlas_graph::era_adapter`) instead of
/// `AtlasData.eras` -- `GraphService.era_ids` is the companion enumeration
/// `GraphQuery`'s own minimal port surface doesn't model (no "list every
/// node of kind K" primitive; see that field's own doc comment), already
/// held in chronological order; every actual FIELD comes from a real
/// `GraphQuery::node` call on this request's own snapshot. WIRE SHAPE
/// UNCHANGED: `Era { id, name, from_year, to_year }`, same order, same
/// JSON -- `AtlasData.eras`/`eras.json` retire this batch (deletion
/// inventory) with this endpoint as their only production reader.
pub async fn eras(State(graph): State<Arc<GraphService>>) -> Json<Vec<Era>> {
    use atlas_graph_types::node::NodePayload;

    // DB-3: enumeration through the port (`nodes_of_kind`, id order); the
    // chronological wire order the retired `era_ids` companion carried is
    // this handler's own sort now -- by `(from_year, id)`, the same key the
    // companion sorted by, so the response bytes are unchanged
    // (`port_widening_real_data.rs` pins the equivalence).
    let snap = graph.snapshot();
    let mut eras: Vec<(i32, Era)> = graph
        .ids_of_kind(atlas_graph_types::id::NodeKind::Era)
        .into_iter()
        .filter_map(|id| {
            let node = snap.node(&id)?;
            match node.payload {
                NodePayload::Era { label, from_year, to_year } => Some((from_year, Era { id: id.raw.clone(), name: label, from_year, to_year })),
                _ => None,
            }
        })
        .collect();
    eras.sort_by(|a, b| (a.0, &a.1.id).cmp(&(b.0, &b.1.id)));
    Json(eras.into_iter().map(|(_, e)| e).collect())
}

/// `GET /api/narratives` (M-C2, definitive surface list). Re-implemented as
/// a VIEW over the graph's own Narrative nodes -- `GraphService.
/// narrative_ids` is the companion enumeration (same "port doesn't model
/// 'list every node of kind K'" class as `era_ids`/`polity_ids`, already
/// confirmed to match `data/curated/narratives/`'s own sorted-by-filename
/// compiled order -- see that field's own doc comment); `legs` comes from
/// `narrative_legs` (the `succession` relation's own row `chain`, the
/// single source -- never duplicated onto the payload). WIRE SHAPE
/// UNCHANGED: `atlas_core::data::Narrative { id, name, color, legs }`.
pub async fn narratives(State(graph): State<Arc<GraphService>>) -> Json<Vec<Narrative>> {
    let snap = graph.snapshot();
    let empty_legs: Vec<String> = Vec::new();
    let out: Vec<Narrative> = graph
        .ids_of_kind(atlas_graph_types::id::NodeKind::Narrative)
        .iter()
        .filter_map(|id| atlas_graph::legacy::narrative_from_node(id, &snap, graph.narrative_legs.get(&id.raw).unwrap_or(&empty_legs)))
        .collect();
    Json(out)
}

pub async fn landmarks(State(data): State<Arc<AtlasData>>) -> Json<Vec<Landmark>> {
    Json(data.landmarks.clone())
}

/// `GET /api/sources` (batch-s-brief.md requirement 3): the Sources
/// page's entire single source of truth, straight off
/// `data/compiled/sources.json` (itself generated 1:1 from LICENSES.md by
/// `atlas_etl::sources`'s own fail-loud drift check -- see the
/// `gen_sources` binary). The client renders this directly; nothing here
/// is a hardcoded duplicate list.
pub async fn sources(State(sources): State<Arc<SourcesDocument>>) -> Json<SourcesDocument> {
    Json((*sources).clone())
}

/// `GET /api/land-mask` (Batch R requirement 1, "borders become part of the
/// plate"): the curated land/coastline mask (`data/curated/land-mask.toml`)
/// used ONLY to clip polity washes so they never spill into open sea --
/// static geometry, no query params, no time dependence (unlike
/// `/api/polities`). `rings` is a flat `[[[lat,lon],...],...]` array -- every
/// region's own rings, already flattened at ETL time (see
/// `AtlasData::land_mask`'s own doc comment) -- the client never needs
/// region names/ref_notes, only the raw geometry to build an SVG clipPath
/// from.
#[derive(Debug, Serialize)]
pub struct LandMaskOut {
    pub rings: Vec<Vec<(f64, f64)>>,
}

pub async fn land_mask(State(data): State<Arc<AtlasData>>) -> Json<LandMaskOut> {
    Json(LandMaskOut { rings: data.land_mask.clone() })
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
/// Batch M requirement 1: the wire shape of one `atlas_core::data::PolityDelta`
/// -- a plain field-for-field copy (`event`/`verses`/`ref_note`), same "no
/// rename, no reshaping" convention `PolityOut` itself already follows for
/// `PolityEra`.
#[derive(Debug, Serialize)]
pub struct PolityDeltaOut {
    pub event: String,
    pub verses: Vec<String>,
    pub ref_note: String,
}

impl From<&atlas_core::data::PolityDelta> for PolityDeltaOut {
    fn from(d: &atlas_core::data::PolityDelta) -> Self {
        PolityDeltaOut { event: d.event.clone(), verses: d.verses.clone(), ref_note: d.ref_note.clone() }
    }
}

#[derive(Debug, Serialize)]
pub struct PolityOut {
    pub id: String,
    pub name: String,
    pub from: i32,
    pub to: i32,
    pub rings: Vec<Vec<(f64, f64)>>,
    pub color_key: u8,
    /// Batch M requirement 1: this era's own delta metadata, OMITTED (not
    /// null) when absent -- "an uneventful boundary stays visible but gets
    /// the minimal popover," so the client must be able to tell "no
    /// transition curated" apart from "transition curated with zero
    /// verses" (the latter still shows the event/grounding-note sections,
    /// just no THE SCRIPTURES section — see `PolityDelta::verses`' own doc
    /// comment). Mirrors `NarrativePositionOut.prior`/`.following`'s own
    /// `skip_serializing_if` precedent exactly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<PolityDeltaOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fall: Option<PolityDeltaOut>,
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
/// BATCH M-C (controller decision 7): re-implemented as a VIEW over the
/// graph's own Polity nodes (`atlas_graph::polity_adapter`) instead of
/// `AtlasData.polities` -- border data as node payloads (controller
/// decision 2), the map consuming payloads directly, per era, off each
/// Polity node's own `NodePayload::Polity.eras`. `GraphService.polity_ids`
/// is the companion enumeration (same status as `era_ids`, see that
/// field's own doc comment). WIRE SHAPE UNCHANGED: `PolityOut`/
/// `PolityDeltaOut`, same fields, same conditional presence, same
/// deterministic sort -- `AtlasData.polities`/`polities.json` stay
/// standing (deliberately NOT this batch's deletion target: still the
/// adapter's own curated source, same status as `event_world`'s own
/// `atlas.events`/`.narratives`).
pub async fn polities(
    State(graph): State<Arc<GraphService>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<PolitiesOut>, ApiError> {
    use atlas_graph_types::node::NodePayload;

    let from = parse_year(&params, "from")?;
    let to = parse_year(&params, "to")?;
    let window = TimeRange::new(from, to).map_err(|_| ApiError::bad_window())?;

    let snap = graph.snapshot();
    let mut out: Vec<PolityOut> = Vec::new();
    for id in &graph.ids_of_kind(atlas_graph_types::id::NodeKind::Polity) {
        let Some(node) = snap.node(id) else { continue };
        let NodePayload::Polity { color_key, eras, .. } = node.payload else { continue };
        for era in &eras {
            let era_range = TimeRange { from_year: era.from_year, to_year: era.to_year };
            if window.intersects(&era_range) {
                out.push(PolityOut {
                    id: id.raw.clone(),
                    name: era.name.clone(),
                    from: era.from_year,
                    to: era.to_year,
                    rings: era.rings.clone(),
                    color_key,
                    transition: era.transition.as_ref().map(|d| PolityDeltaOut { event: d.event.clone(), verses: d.verses.clone(), ref_note: d.ref_note.clone() }),
                    fall: era.fall.as_ref().map(|d| PolityDeltaOut { event: d.event.clone(), verses: d.verses.clone(), ref_note: d.ref_note.clone() }),
                });
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id).then(a.from.cmp(&b.from)));

    Ok(Json(PolitiesOut { polities: out }))
}

/// Batch R requirement 5 (place-in-verse hover -> marker blink): one place
/// mentioned in a verse, per `AtlasData::places_for_verse` (the reverse of
/// `Place::verse_links`). Deliberately lean -- id (to target a map marker)
/// and display name (for the client's own plain-text substring match against
/// the verse's rendered text, see `chapter`'s own doc comment for why there
/// is no richer per-mention offset data) -- mirrors `QuietPlace`'s own
/// "no more than the consumer needs" wire philosophy.
#[derive(Debug, Serialize)]
pub struct PlaceRefOut {
    pub id: String,
    pub name: String,
}

/// M-D3 (owner ruling U5, "in-text person and place name links,
/// mentions-attested ONLY -- no free-text matching ever"): one person the
/// graph's own `mentions` relation attests at a verse's own locus
/// (`GraphService::persons_by_verse`, precomputed off `Graph.mentions`,
/// NEVER a free-text/dictionary lookup against the verse's own rendered
/// words -- the exact "Sin"-the-city vs "sin"-the-noun hazard this
/// wording guards against: a name is only ever offered as a link because
/// a CURATED edge attests that specific entity at that specific verse,
/// not because some word in the text happens to match a name string
/// somewhere in the graph). Deliberately lean, mirroring `PlaceRefOut`'s
/// own "no more than the consumer needs" wire philosophy -- id (to open
/// the entity's own node popover) and display label (for the client's
/// own plain-text search locating WHERE in the verse's already-attested
/// text this entity's own name appears, the SAME PlaceMentions.cs
/// mechanism this batch generalizes rather than replaces -- attestation
/// is what THIS field guards; locating the substring within an ALREADY-
/// attested verse is a separate, later step, not a second matching
/// hazard).
#[derive(Debug, Serialize)]
pub struct PersonRefOut {
    pub id: String,
    pub name: String,
}

/// Batch RED-1 (owner order 2026-08-25, "Red letters on Jesus' words in
/// every translation"): one sub-verse red-letter span, CHAR (not byte)
/// offsets into this verse's own `text` field -- `graph.red_letter_spans`'s
/// own doc comment has the full "why char offsets" (C# `string` indexing is
/// UTF-16 code units; every character in this app's own KJV text is within
/// the Basic Multilingual Plane, so char count == UTF-16 code unit count
/// here, always). `start`/`end` are a half-open range (`text[start..end]`
/// in C# `Substring(start, end-start)` terms) -- KJV display renders this
/// EXACT sub-verse span; decision 5's own "ONE render rule."
#[derive(Debug, Serialize)]
pub struct WordsOfChristSpanOut {
    pub start: usize,
    pub end: usize,
}

/// Batch T requirement 5: one resolved pericope heading, folded onto its own
/// anchor verse (`VerseOut.heading`) -- the SAME "fold onto the already-
/// shared fetch" precedent `VerseDetailOut.catechism`/`.events` already
/// establish, here for the CHAPTER fetch instead of the verse-detail one
/// (a reader rendering a whole chapter needs to know, per verse, whether a
/// heading belongs above it -- a second per-verse round trip would be
/// absurd). `event_id` is what a click opens (a new `EventNode`, client-side);
/// `title` is rendered directly, so the reader never needs a second fetch
/// just to show the heading text itself.
#[derive(Debug, Serialize)]
pub struct HeadingOut {
    pub event_id: String,
    pub title: String,
    /// Batch HOTFIX-4 requirement 6: see `HeadingEntry::kind`'s own doc
    /// comment -- lets the reader's own heading rendering apply the quiet,
    /// non-traversable styling BEFORE a click, for a general-kind heading.
    pub kind: String,
    /// M-D1 requirement 1 (CHAPTER-BOUNDARY CONTINUATION): true iff this
    /// verse is NOT the container's own true first-covered verse but a
    /// later chapter its own coverage continues into -- lets the reader
    /// render a quiet continuation marker BEFORE a click, the same
    /// "affordance honesty travels as data, not inferred client-side"
    /// discipline `kind` above already establishes. Always present
    /// (`false` for every ordinary, PRIMARY heading -- the overwhelming
    /// majority), never omitted: unlike `heading` itself (conditional
    /// presence on `VerseOut`), once a heading exists at all its own
    /// continuation-ness is never in doubt.
    pub is_continuation: bool,
}

// M-D3 (owner ruling R5): `impl From<&atlas_core::data::HeadingEntry> for
// HeadingOut` retired -- genuinely orphaned (grep-proven: no call site
// anywhere in this workspace). `chapter` (below) has built `HeadingOut`
// directly from `graph.heading_index`'s own `atlas_graph::heading::
// HeadingEntry` since M-C2; this conversion's OWN source type
// (`atlas_core::data::HeadingEntry`, fed by `AtlasData::heading_for_verse`)
// has had no live reader since, and carried no lockstep test of its own
// (unlike `heading_for_verse` itself, which stays -- see that method's own
// doc comment / heading.rs's own module doc comment for the "dead, tested
// reference oracle" it remains). Recoverable from git history at the
// commit immediately preceding this one.

#[derive(Debug, Serialize)]
pub struct VerseOut {
    pub verse: u16,
    pub text: String,
    /// Batch R requirement 5: every curated place whose OWN `verse_links`
    /// names this verse, in place-id order (stable, not meaningful --
    /// there is no "primary mention" concept). Always present, possibly
    /// empty (most verses geocode to zero places) -- conditional presence
    /// lives on the CLIENT side (a mini-reader renders no hoverable span at
    /// all when this is empty), not as an omitted wire key, matching every
    /// other "always an array" field in this app's own wire (e.g.
    /// `Scene.quiet_places`).
    pub places: Vec<PlaceRefOut>,
    /// M-D3 (owner ruling U5): every person the graph's own `mentions`
    /// relation attests at this verse's own locus, in wire (row-insertion)
    /// order -- see `PersonRefOut`'s own doc comment for the full
    /// mentions-attested-only reasoning. Always present, possibly empty
    /// (most verses mention zero persons by name) -- SAME conditional-
    /// presence-lives-client-side convention `places` immediately above
    /// already establishes for this exact shape of field.
    pub persons: Vec<PersonRefOut>,
    /// Batch T requirement 5: this verse's own pericope heading, when it is
    /// a heading ANCHOR (`AtlasData::heading_for_verse`) -- omitted (not
    /// null), matching `PolityOut.transition`/`.fall`'s own conditional-
    /// presence convention, since the overwhelming majority of verses never
    /// anchor a heading at all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<HeadingOut>,
    /// Batch M-D2 (owner's cross-reference superscript directive, closed on
    /// the graph platform): this verse's own `cites` edge-summary COUNT at
    /// its TextUnit locus -- read straight off THE PORT (`GraphQuery::
    /// edge_summary`, design doc §5), the SAME generic query the `/api/node/
    /// {id}` handler itself calls (`graph_handlers::node_card`) -- never a
    /// second, bespoke tally. This is a genuinely NEW field (no bespoke
    /// predecessor existed), sourced NATIVELY from the generic contract from
    /// day one: the reader's own superscript markers ARE a query of the
    /// graph, per the design's own P1/P4 laws, just batched onto the
    /// EXISTING per-chapter response (one HTTP round trip for a whole
    /// chapter) rather than issuing 20-30 individual `GET /api/node/{id}`
    /// calls per chapter, which `client/IExplorableClient.cs`'s own concrete
    /// implementation is what a SINGLE node's lookup uses (see that file's
    /// own doc comment + `GraphExplorableClientTests` for the direct,
    /// unbatched proof the same contract is correct in isolation). Always
    /// present (0 for the overwhelming majority of verses, never omitted --
    /// the client's own superscript decision is `count == 0` -> nothing,
    /// never a missing-field special case).
    pub xref_count: usize,
    /// Batch RED-1: this verse's own aligned sub-verse red-letter spans
    /// (`GraphService.red_letter_spans`, precomputed once -- see that
    /// field's own doc comment for why this ONE companion is not
    /// graph-derivable), in ascending order. Always present, possibly
    /// empty -- SAME "always an array" convention `places`/`persons`
    /// above already establish (most verses carry none). KJV-ONLY by
    /// construction (decision 5: sub-verse precision is KJV-specific;
    /// this endpoint has never served any other translation -- ruling 5,
    /// "M1 is KJV-only").
    pub words_of_christ: Vec<WordsOfChristSpanOut>,
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
///
/// Batch M-A (brief requirement 5, "re-implement the OLD /api/chapter
/// handler as a VIEW over the window query"): the verse TEXT below now
/// comes from `GraphState::chapter_span` + `GraphState::window` -- the SAME
/// windowed reading-order query `GET /api/text?scope=chapter` calls --
/// instead of `data.verses.get(key)`. The verse-count bound and the
/// out-of-canon policy above are UNCHANGED and still sourced from
/// `AtlasData`; headings moved to `graph.heading_index` in M-C2, and
/// OVERLAY-1 Task 5 moved the PLACE-MENTION half onto the port too
/// (`graph.scene_source(&data)`'s own `places_for_verse`/`place`, the
/// materialised-from-the-graph successors of the deleted
/// `AtlasData::places_for_verse`/`place_by_id` -- identical ids in
/// identical order, see those methods' own doc comments). THIS endpoint
/// (the reader's own chapter view) was untouched by Batch M-B's own
/// event-world migration;
/// only `/api/narrative/event/{id}` (see that handler's own doc comment)
/// and the generic `/api/node`/`/edges` endpoints move to the graph this
/// batch. The WIRE SHAPE is byte-for-byte identical -- proven by
/// `tests/graph_equivalence.rs`'s own all-1,189-chapters comparison -- so
/// every existing caller of this endpoint (the reader's chapter view/
/// mini-reader/split view, `ChapterNode`, `PlaceCard`'s hover verse text,
/// `PassageBlock`, `PopoverSectionProviders`) now serves from the graph
/// with NO client-side change and no reader-visible behavior change.
pub async fn chapter(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Path(cref): Path<String>,
) -> Result<Json<ChapterOut>, ApiError> {
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

    // The graph's own view of this chapter's text, keyed by verse number --
    // empty (not an error) when the graph has no such chapter, mirroring
    // `data.verses.get(key)`'s own prior "absent means skip this verse"
    // tolerance below. Reached entirely through THE PORT (design doc §9a;
    // fix round 1, C1) -- `window::window`/`window::render` take only
    // `&dyn atlas_graph_types::store::GraphQuery`, never a concrete graph
    // struct; `chapter_span`/the opened snapshot are `GraphService`'s own
    // adapter-side companions (ref-resolution isn't part of the generic
    // port -- see that module's own doc comment).
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

    // OVERLAY-1 Task 5: the place-mention half's own source -- the
    // graph-backed scene source, resolved ONCE for the whole chapter rather
    // than per verse (it is a single `OnceLock` read, but hoisting it keeps
    // the hot loop below free of any repeated lookup).
    let scene_source = graph.scene_source(&data);

    let mut verses = Vec::new();
    for v in 1..=verse_count {
        let key = format!("{code}.{chapter}.{v}");
        if let Some(text) = graph_texts.get(&v) {
            let places = scene_source
                .places_for_verse(&key)
                .iter()
                .filter_map(|pid| scene_source.place(pid))
                .map(|p| PlaceRefOut {
                    id: p.id.clone(),
                    // Batch E3: resolved (period-history/KJV-alias-aware)
                    // name, not the bare Theographic default -- this is the
                    // "reader place mentions" surface (PlaceMentions.cs's own
                    // plain-text substring scan against THIS field is the
                    // app's only mention-detection mechanism, so an unaliased
                    // name here means a place whose KJV wording differs from
                    // its default name is silently never detected as
                    // mentioned in its own verse's text at all -- exactly
                    // the owner's bug report, one layer deeper). No window
                    // (`None`) -- same "scripture mode never resolves a
                    // period name" reasoning `compose_scripture_scene` uses;
                    // a chapter reading has no time window either.
                    name: resolve_display_name(&p.name, data.place_history_for(&p.id), None, data.place_name_alias_for(&p.id)),
                })
                .collect();
            // M-D3 (owner ruling U5): the SAME O(1) per-verse lookup
            // treatment as `heading`/`xref_count` below, off the
            // precomputed `graph.persons_by_verse` companion -- see that
            // field's own doc comment.
            // DB-3: through the port (`GraphService::persons_at_verse`,
            // `edges_with_nodes` over `mentions`), the retired
            // `persons_by_verse` companion's exact answer.
            let persons = graph
                .persons_at_verse(book.0, chapter, v)
                .into_iter()
                .map(|(id, name)| PersonRefOut { id, name })
                .collect();
            // M-C2 (requirement 1, decisive-title law re-homed as a graph
            // query): `graph.heading_index` (precomputed at `GraphService::
            // assemble` time by `heading::build_heading_index`), not
            // `data.heading_for_verse` -- see that module's own doc
            // comment for the full re-homing (kept in lockstep with
            // CONTRACT.md and the atlas-core original).
            let heading = graph
                .heading_index
                .get(&key)
                .map(|h| HeadingOut { event_id: h.event_id.clone(), title: h.title.clone(), kind: h.kind.clone(), is_continuation: h.continuation });
            // Batch M-D2: the generic port, inline -- `Position::Node` +
            // `GraphQuery::edge_summary` are the EXACT calls
            // `graph_handlers::node_card` makes for `GET /api/node/{id}`;
            // reused here as a library call (not a second HTTP round trip
            // per verse) so a whole chapter's worth of superscript counts
            // ships in the ONE fetch the reader already makes. `cites` is
            // ALWAYS the Forward direction from a verse's own locus (a verse
            // citing others, not "who cites me" -- `cited-by` is the
            // Inverse reading of the SAME relation, unused here).
            let verse_pos = Position::Node(atlas_graph::kjv_adapter::verse_node_id(book.0, chapter, v));
            let xref_count =
                snap.edge_summary(&verse_pos).get(&EdgeKind::Directed(RelationId::Cites, Direction::Forward)).copied().unwrap_or(0);
            // Batch RED-1: the SAME O(1) per-verse lookup treatment
            // `heading`/`xref_count`/`persons` above already get, off the
            // precomputed `graph.red_letter_spans` companion.
            let words_of_christ = graph.red_letter_spans.get(&key).map(|spans| spans.iter().map(|&(start, end)| WordsOfChristSpanOut { start, end }).collect()).unwrap_or_default();
            verses.push(VerseOut { verse: v, text: text.to_string(), places, persons, heading, xref_count, words_of_christ });
        }
    }

    Ok(Json(ChapterOut { sref: format!("{code}.{chapter}"), book: book.name().to_string(), chapter, verses }))
}

/// KRETZ-SCALE-1 (batch-corp1-review.md Q-1, batch-corp1-report.md §5,
/// batch-finalp1-brief.md ticket 2, SANCTIONED SERVER ADDITION): one
/// `CommentaryItem` row within one verse of `GET /api/kretzmann/chapter/{cref}`'s
/// own chapter-scoped response.
#[derive(Debug, Serialize)]
pub struct KretzmannChapterItemOut {
    pub id: String,
    pub heading: Option<String>,
}

/// One verse's own commentary items, in document order. Only verses with
/// >=1 item appear at all (mirrors `Kretzmann.razor`'s own retired
/// client-side "if (items.Count > 0)" filter).
#[derive(Debug, Serialize)]
pub struct KretzmannChapterVerseOut {
    pub verse: u16,
    pub items: Vec<KretzmannChapterItemOut>,
}

#[derive(Debug, Serialize)]
pub struct KretzmannChapterOut {
    pub verses: Vec<KretzmannChapterVerseOut>,
    pub version: String,
}

/// `GET /api/kretzmann/chapter/{cref}`. `cref` must parse as exactly a
/// `ScriptureRef::Chapter` (book + chapter, e.g. `PSA.119`) -- same 400
/// `bad_ref` convention as `GET /api/chapter/{cref}` for a book-only or
/// verse/passage-shaped path segment. Same ruling-3-policy as `chapter`
/// above: an out-of-range chapter number (or a book with no known chapters)
/// is NOT an error -- `verse_count` resolves to 0 and this 200s with an
/// empty `verses` list, never a 404 for "this chapter has no commentary."
///
/// Replaces `Kretzmann.razor`'s own retired client-side fan-out (one
/// `commented-on-by` edges HTTP call PER VERSE, concurrently -- 176
/// simultaneous requests on every locus change for a chapter like PSA 119)
/// with ONE request, computed by walking the SAME `commented-on-by` edge
/// machinery server-side, in-process, via `atlas_graph::kretzmann_adapter::
/// chapter_commentary` -- see that function's own doc comment for why this
/// is additive, not a types-crate or artifact change: the underlying
/// `CommentsOn`/`RelationId::CommentsOn` KRETZ-1 vocabulary is completely
/// unchanged, this is a new READ path over data the graph already carries.
pub async fn kretzmann_chapter(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Path(cref): Path<String>,
) -> Result<Json<KretzmannChapterOut>, ApiError> {
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

    let snap = graph.snapshot();
    let rows = atlas_graph::kretzmann_adapter::chapter_commentary(&snap, book.0, chapter, verse_count);

    // `chapter_commentary` already returns rows verse-ascending with every
    // verse's own items grouped consecutively (its own doc comment) -- this
    // loop just folds that flat Vec into the wire's own nested shape,
    // never re-sorting or re-grouping independently.
    let mut verses: Vec<KretzmannChapterVerseOut> = Vec::new();
    for row in rows {
        let item = KretzmannChapterItemOut { id: crate::graph_wire::encode_node_id(&row.item_id), heading: row.heading };
        match verses.last_mut() {
            Some(v) if v.verse == row.verse => v.items.push(item),
            _ => verses.push(KretzmannChapterVerseOut { verse: row.verse, items: vec![item] }),
        }
    }

    Ok(Json(KretzmannChapterOut { verses, version: atlas_graph::version_hex(graph.version()) }))
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
    /// Batch HOTFIX-4 (drive-by fix, found while adding `kind` below):
    /// OMITTED (not the server's own internal undated placeholder) for a
    /// general-kind passage -- matches `EventDetailOut.when`'s own
    /// established fabrication guard exactly (`to_scene_event`'s own `se.when`
    /// is `e.when` unconditionally, which for `kind == "general"` is
    /// `TimeRange::undated()`, never meant to be presented as a real date).
    /// Dormant until now (this row's own CLIENT rendering only ever read
    /// `Id`/`Label`, never `When`), but a real gap against house doctrine
    /// nonetheless -- fixed here rather than left for a future reader of
    /// this exact struct to copy the bad precedent forward.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<TimeRange>,
    pub verse_groups: Vec<VerseGroup>,
    pub places: Vec<String>,
    /// Batch HOTFIX-4 requirement 6 (AFFORDANCE HONESTY): this EVENT-kind
    /// PASSAGE's own `Event::kind` ("event" | "general") -- same reason as
    /// `HeadingOut::kind`, one surface over (a verse's own EVENT membership
    /// row, not a reader heading): the client applies the quiet,
    /// non-traversable styling to a general-kind row BEFORE the click, no
    /// second fetch needed to find out.
    pub kind: String,
    /// Batch PROV-1 (owner order 1): THIS ROW's own source -- the
    /// provenance id of the Event NODE this membership row points at, read
    /// straight off `Node::provenance` (`event_world::event_provenance`:
    /// `"theographic"` for an imported event, `"curated"` for one this
    /// project authored). Additive-only, the `words_of_christ` precedent --
    /// every field above serves byte-identically to before this batch.
    ///
    /// Deliberately the TARGET NODE's provenance, not the `Attests` row's:
    /// this row answers "which event is this verse part of," and the honest
    /// attribution for THAT claim is whoever says the event exists. The
    /// `Attests` rows' own provenance is served where it is actually
    /// rendered -- `EventDetailOut::witnesses_provenance`, on the PARALLEL
    /// ACCOUNTS section.
    ///
    /// This field IS the genuinely mixed one: a verse belonging to both an
    /// imported event and a hand-authored one names BOTH sources on its own
    /// section heading, which is the total-capture-honesty case at the
    /// verse frontier.
    pub provenance: String,
}

#[derive(Debug, Serialize)]
pub struct CrossRefOut {
    pub target: String,
    pub votes: i32,
    pub preview: String,
    /// PROV-1 FIX ROUND 1 (review M-3): the attribution for the CROSS
    /// REFERENCES section these rows belong to, so a PASSAGE node's section
    /// gets a "?" like a verse's.
    ///
    /// GRANULARITY, SAID ONCE AND PLAINLY (fix round 2, review M-NEW-1):
    /// this is a SECTION-level value carried on the ELEMENT because the
    /// endpoint is a bare array with no envelope to hang it on. Every
    /// element of one response receives the identical set. It is NOT
    /// per-row attribution and must never be described as such -- a
    /// comment in `handlers::verse` did describe it that way and was
    /// corrected. Per-row is not available here to be had:
    /// `graph.cross_refs_by_from` holds `atlas_core::data::CrossRef`, which
    /// carries `target` and `votes` only; the graph-types row that does
    /// carry `provenance` has it projected away before this call site sees
    /// it. Widening the companion index to keep it is a contract-shaped
    /// decision this batch refused, and nothing here should imply it
    /// already happened.
    ///
    /// THE ORIGINAL DISCLOSURE GAVE A FALSE REASON and the review was right
    /// to say so: `GET /api/xrefs/{sref}` returns a bare `Vec<CrossRefOut>`,
    /// but the array was never where the field goes -- the ELEMENT is a
    /// struct with room for an additive field, which is precisely the move
    /// this batch already made twice (`VerseEventOut.provenance`,
    /// `EventAnalogueOut.provenance`). No existing field moves; the response
    /// stays an array.
    ///
    /// A SET (`Vec<String>`), never one collapsed id, and that is measured
    /// rather than stylistic: `atlas_core::xrefs::aggregate_span_xrefs`
    /// UNIONS the `cites` rows of every member verse of the span and sums
    /// their votes, so one `CrossRefOut` is an aggregate of several
    /// underlying rows and there is no single row to attribute it to. The
    /// honest key is the `cross_refs` family's own distinct set -- the
    /// identical value `VerseDetailOut.cross_refs_provenance` already
    /// carries, pinned as `{openbible.info-cross-references}` by
    /// `the_per_family_provenance_map_of_the_real_artifact_is_pinned`. A
    /// second source landing in `cites` would surface on every row instead
    /// of hiding behind the first, which is the leper lesson in a type.
    pub provenance: Vec<String>,
}

/// Batch F ("the small catechism"), extended Batch F2 (question-level
/// citations): one catechism item citing a verse/span, as embedded in
/// `VerseDetailOut.catechism` AND returned (as a list) by `GET
/// /api/catechism/{sref}` -- deliberately lean (id + display name + an
/// optional question title, no preview text unlike `CrossRefOut`):
/// requirement 4's own UI description lists citing items (now
/// question-aware, "<Item> — <Question title>") as plain named entries,
/// nothing more, and the full item content is a separate fetch (`GET
/// /api/catechism/item/{id}`, see `CatechismItemOut` below), made only once
/// the user actually opens one. `question` is omitted from the wire
/// entirely (not null) when this hit came from Luther's own item-level
/// embedded citation rather than a question -- same conditional-presence
/// convention `where_written`/`blurb` etc. already use throughout this file.
#[derive(Debug, Serialize)]
pub struct CatechismRefOut {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    /// PROV-1 FIX ROUND 1 (review M-3): the `catechism` family's own
    /// distinct provenance set, so a PASSAGE node's THE SMALL CATECHISM
    /// section gets a "?" like a verse's. Same additive element-level move
    /// and same set-not-single-id reasoning as `CrossRefOut.provenance` --
    /// see that field's doc comment.
    ///
    /// The review judged deferring this half defensible because
    /// `catechism_for_span` took only `State<Arc<AtlasData>>`. Measured, it
    /// is not: `AppState` implements `FromRef` for `Arc<GraphService>` too,
    /// and six handlers in this file already take BOTH extractors
    /// (`handlers::verse` among them). So the second half was one extractor
    /// away, not a larger change, and it ships here.
    ///
    /// This family is the genuinely MULTI-sourced one --
    /// `{concord-sc-overlap, curated-catechism}`, pinned -- which is exactly
    /// why the field is a `Vec` on both endpoints.
    ///
    /// GRANULARITY (fix round 2, review M-NEW-1): SECTION-level, carried on
    /// the element because the endpoint is a bare array. Every element of
    /// one response receives the identical set. Not per-row, and not to be
    /// described as per-row -- see `CrossRefOut.provenance` for the full
    /// statement and for why per-row is unavailable at these call sites.
    pub provenance: Vec<String>,
}

impl CatechismRefOut {
    fn from_ref(c: atlas_core::catechism::CatechismRef, provenance: &[String]) -> Self {
        CatechismRefOut { id: c.id, name: c.name, question: c.question, provenance: provenance.to_vec() }
    }
}

#[derive(Debug, Serialize)]
pub struct VerseDetailOut {
    #[serde(rename = "ref")]
    pub sref: String,
    pub text: String,
    /// Batch RED-1: this verse's own aligned sub-verse red-letter spans --
    /// see `VerseOut.words_of_christ`'s own doc comment (identical shape/
    /// convention, this endpoint's own single-verse sibling).
    pub words_of_christ: Vec<WordsOfChristSpanOut>,
    pub book_meta: BookMetaOut,
    pub events: Vec<VerseEventOut>,
    pub cross_refs: Vec<CrossRefOut>,
    /// Batch F: catechism items citing this verse, via the SAME aggregation
    /// core `GET /api/catechism/{sref}` uses for a passage
    /// (`AtlasData::catechism_items_for_span`, a single-verse span here) --
    /// folded directly into this ALREADY-shared verse-detail fetch (rather
    /// than requiring a second round trip) so `CatechismSeamSection`
    /// (client) reads it off the exact same memoized `AtlasClient.Verse`
    /// call `VerseTextSectionProvider`/`CrossRefsSection` already share --
    /// "one fetch, not three," now not four either. Always present, possibly
    /// empty (most verses cite none) -- conditional presence lives on the
    /// CLIENT side (no section at all when empty), matching every other
    /// "always an array" field in this app's own wire.
    pub catechism: Vec<CatechismRefOut>,
    /// Batch PROV-1 (owner order 1, "the source from which it came"): THIS
    /// VERSE's own source -- the provenance id of the `TextUnit` node
    /// `text` above was rendered from, read straight off `Node::provenance`
    /// (`kjv_adapter`: `"kjv"`). The focus card's own attribution: the
    /// affordance beside a verse says "The King James Version" because THIS
    /// field said `kjv`, never because a client-side default assumed it.
    pub provenance: String,
    /// Batch PROV-1: every distinct provenance id behind the CROSS
    /// REFERENCES section -- the owner's own headline case ("sourced from
    /// openbible.com"). A LIST, not a string, deliberately: the day a
    /// second cross-reference corpus lands, this section starts naming
    /// both instead of quietly continuing to name the first. It is the
    /// `cross_refs` FAMILY's own distinct set (`ProvenanceIndex::
    /// by_family`), which is a true statement about every individual row
    /// exactly while the family is single-sourced -- pinned as such by
    /// `atlas-graph/tests/provenance_registry_real_data.rs`, and honest
    /// either way, since a family that gained a second source would serve
    /// both here rather than pick one.
    ///
    /// Always present, possibly empty (a verse with no cross references),
    /// same "always an array" convention as `cross_refs` itself.
    pub cross_refs_provenance: Vec<String>,
    /// Batch PROV-1: the same, for THE SMALL CATECHISM section -- the
    /// `catechism` family's own distinct provenance set.
    pub catechism_provenance: Vec<String>,
    // Batch T requirement 3 ("verse popover: event membership replaces
    // prev/next"): Batch N's own `narrative_positions` field (chronological
    // PRIOR/FOLLOWING, verse-keyed) is RETIRED here, cleanly -- verse-level
    // traversal no longer exists (see CONTRACT.md's own retirement note).
    // The PRE-EXISTING `events` field above (`Vec<VerseEventOut>`, id +
    // label + verse_groups + places, populated the same way since before
    // this batch) is what the client's own NEW "EVENT" section reads
    // instead: it already names every EVENT-kind PASSAGE citing this verse,
    // which is exactly "event membership" -- no new wire field needed for
    // that half. Chronological PRIOR/FOLLOWING now lives entirely on the
    // EVENT node (`GET /api/narrative/event/{id}`, unchanged plumbing,
    // called by a new client-side caller -- see `handlers::event` below for
    // the richer id-keyed EVENT fetch that node also uses).
}

/// Batch N: one event ADJACENT to a `NarrativePositionOut` (its own PRIOR or
/// FOLLOWING leg) -- id/label/places/verse_groups, per requirement 1
/// verbatim ("each adjacent event carrying its id, label, place(s), and
/// verse groups"). `verse_groups` is built by
/// `atlas_core::scene::to_scene_event` -- the SAME function every other
/// "an event's own verses on the wire" case in this file already calls
/// (`to_scene_event` above, `VerseEventOut`'s own construction) -- so this
/// is provably the same data a map arrow's own endpoint would show for the
/// identical event id, not a parallel re-derivation.
#[derive(Debug, Serialize)]
pub struct NarrativeAdjacentEventOut {
    pub id: String,
    pub label: String,
    pub places: Vec<String>,
    pub verse_groups: Vec<VerseGroup>,
}

impl From<atlas_core::narrative::NarrativeAdjacentEvent> for NarrativeAdjacentEventOut {
    fn from(e: atlas_core::narrative::NarrativeAdjacentEvent) -> Self {
        NarrativeAdjacentEventOut { id: e.id, label: e.label, places: e.places, verse_groups: e.verse_groups }
    }
}

/// Batch N: one (narrative, event) position a queried verse or event
/// touches -- `prior`/`following` omitted (not null) exactly at a
/// narrative's own first/last leg, same conditional-presence wire
/// convention `HistoryOut.blurb`/`CatechismRefOut.question` etc. already
/// use throughout this file. `event_id`/`event_label` are carried (map-
/// focus-sync + disambiguating two positions sharing one `narrative_id`)
/// even though they restate something the CALLER usually already knows.
#[derive(Debug, Serialize)]
pub struct NarrativePositionOut {
    pub narrative_id: String,
    pub narrative_name: String,
    pub event_id: String,
    pub event_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior: Option<NarrativeAdjacentEventOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<NarrativeAdjacentEventOut>,
}

// M-D3 (owner ruling R5): `impl From<atlas_core::narrative::NarrativePosition>
// for NarrativePositionOut` retired -- genuinely orphaned (grep-proven: no
// call site). `narrative_event_positions` (below) has built
// `NarrativePositionOut` directly, from the graph's own succession-edge
// topology, since M-B; this conversion's OWN source type
// (`atlas_core::narrative::NarrativePosition`, produced only by the
// now-retired `positions_for_events`) has had no live producer since.
// Recoverable from git history at the commit immediately preceding this
// one.

/// Batch HOTFIX-4 requirement 1: the GLOBAL chronological PRIOR/FOLLOWING
/// for one event id, independent of narrative membership -- see
/// `atlas_core::narrative::TimelinePosition`'s own doc comment for the full
/// ordering rule. Reuses `NarrativeAdjacentEventOut` (same shape, same
/// "id/label/places/verse_groups" the narrative rows already send) -- one
/// computation, one wire type, two consumers.
#[derive(Debug, Serialize)]
pub struct TimelinePositionOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior: Option<NarrativeAdjacentEventOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<NarrativeAdjacentEventOut>,
}

impl From<atlas_core::narrative::TimelinePosition> for TimelinePositionOut {
    fn from(p: atlas_core::narrative::TimelinePosition) -> Self {
        TimelinePositionOut { prior: p.prior.map(Into::into), following: p.following.map(Into::into) }
    }
}

/// Batch HOTFIX-4 requirement 1: `GET /api/narrative/event/{id}`'s own
/// extended wire shape -- WAS a bare `Vec<NarrativePositionOut>` (Batch N);
/// NOW an object, `narrative` carrying EXACTLY that same array (unchanged
/// shape, unchanged rows, every existing narrative-scoped consumer keeps
/// reading it unmodified) alongside the NEW `timeline` field. `timeline` is
/// OMITTED (not `null`) entirely for a general-kind or unknown event id
/// (requirement 2: "general-kind containers... NOT part of time traversal"),
/// present otherwise with `prior`/`following` each independently omitted
/// only at the atlas's own true first/last dated event (conditional
/// presence, matching every other optional field on this wire). Every
/// consumer of the OLD bare-array shape is migrated in this same commit:
/// `AtlasClient.NarrativeEventPositions` (client), `EventNode`/
/// `INarrativeAware` (client), `PlaceCard.LoadNarrativePositions` (client,
/// reads `.Narrative` — TRAVERSAL-1 logic unchanged), and the Playwright
/// helper call sites in `world-pin.spec.ts`/`popover-sections.spec.ts`.
#[derive(Debug, Serialize)]
pub struct NarrativeEventPositionsOut {
    pub narrative: Vec<NarrativePositionOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline: Option<TimelinePositionOut>,
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
pub async fn verse(State(data): State<Arc<AtlasData>>, State(graph): State<Arc<GraphService>>, Path(vref): Path<String>) -> Result<Json<VerseDetailOut>, ApiError> {
    let vid = VerseId::parse_canonical(&vref).map_err(|_| ApiError::bad_ref(&vref))?;
    let canonical = format!("{}.{}.{}", vid.book.code(), vid.chapter, vid.verse);

    // M-C2 (definitive surface list): the verse's own text now comes from
    // the graph's own TextUnit node -- the SAME `window::render` primitive
    // `handlers::chapter` already uses -- not `data.verses.get(key)`.
    let snap = graph.snapshot();
    let text_id = atlas_graph::kjv_adapter::verse_node_id(vid.book.0, vid.chapter, vid.verse);
    let text = window::render(&snap, &text_id).ok_or_else(|| ApiError::not_found("verse"))?;
    // Batch PROV-1: the verse's own attribution, off the SAME node
    // `window::render` just read. `window::render` returning `Some` means
    // the node exists, so this lookup cannot legitimately miss -- but it is
    // written fail-loud rather than defaulted: an unattributed verse is a
    // 500 naming the id, never a silent blank at the reader (the fail-loud
    // law -- "never a silent blank and never a fabricated label").
    //
    // FIX ROUND 1 (review H-1): the node-absence guard was already here, but
    // a node carrying a BLANK provenance string slipped through it. The
    // `filter` closes that, so "no PROV-1 wire field is ever empty" is one
    // uniform rule across all four of them rather than three-quarters of one
    // -- `graph_api.rs::no_provenance_field_the_wire_serves_is_ever_blank`
    // is the standing assertion of it.
    let provenance = snap
        .node(&text_id)
        .map(|n| n.provenance)
        .filter(|p| !p.trim().is_empty())
        .ok_or_else(|| ApiError::internal(&format!("verse {canonical} rendered text with no node to attribute it to")))?;

    let book_meta = data.books_meta.iter().find(|b| b.book == vid.book.code()).cloned().unwrap_or_else(|| BookMeta {
        book: vid.book.code().to_string(),
        author: String::new(),
        write_place: None,
        write_from: None,
        write_to: None,
    });

    // M-C2: "which events attest this verse" is now the INVERSE
    // `attested-in` frontier at the verse's own TextUnit position -- real
    // per-verse `attests` rows (event_world.rs's own M-C2 fix, one row per
    // witness VERSE rather than per verse GROUP) close the exact gap that
    // would otherwise drop a witness-interior verse like `MAT.26.6`.
    // Deduped by event id (an event whose OWN top-level `verses` and a
    // witness both happen to name the identical verse must still surface
    // once, mirroring `AtlasData::finish()`'s own `verse_to_events`
    // dedup); sorted by `from_year` to match that index's own inherited
    // (from `self.events`'s own from_year-sorted order) iteration order.
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
    let events: Vec<VerseEventOut> = attesting_events
        .into_iter()
        .map(|e| {
            // Batch PROV-1: this row's own attribution -- the Event NODE's
            // provenance, off the same snapshot.
            //
            // FIX ROUND 1 (review H-1, HIGH). This read used to end in
            // `.unwrap_or_default()`, and three comments (here, at
            // `EventAnalogueOut` below, and in `client/Dtos.cs`) justified
            // the empty string on the ground that the client renders it as
            // a LOUD unresolved notice. THAT WAS FALSE: every client path
            // to `ProvenanceResolver.Resolve` filtered whitespace ids out
            // FIRST, so a blank provenance rendered as no "?" at all --
            // the exact silent blank requirement 3 forbids, and exactly how
            // the ATTEST-1 leper row hid in the first place. Both halves
            // are fixed: the client no longer filters an id it was HANDED
            // (a blank one now renders the loud notice), and the wire can
            // no longer emit the blank at all. `ApiError::internal` is the
            // same answer `handlers::event` already gives for the event's
            // own node provenance -- the resource exists, our data about it
            // is incomplete, which is our bug and not the caller's.
            let node_provenance = snap
                .node(&atlas_graph::event_world::event_node_id(&e.id))
                .map(|n| n.provenance)
                .filter(|p| !p.trim().is_empty())
                .ok_or_else(|| ApiError::internal(&format!("event {} has no provenance to attribute this membership row to", e.id)))?;
            let se = to_scene_event(&e);
            let when = if e.kind == "event" { Some(se.when) } else { None };
            Ok(VerseEventOut {
                id: se.id,
                label: se.label,
                when,
                verse_groups: se.verse_groups,
                places: e.places.clone(),
                kind: e.kind.clone(),
                provenance: node_provenance,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    // M-C2 (requirement 2): `graph.cross_refs_by_from` (the graph's own
    // `cites` rows, dot-ref keyed, `target` = each row's own
    // `target_display` -- the honest original citation string) instead of
    // `data.cross_refs` -- see that field's own doc comment.
    //
    // OVERLAY-1 Task 2: the preview TEXT itself now comes from
    // `graph.verse_text_of` -- a real, on-demand graph query (one node
    // lookup per row), not the retired `graph.verse_text` whole-spine
    // companion, and not `data.verses`. Same fail-soft behavior as before
    // (a missing preview skips the row, per this endpoint's own "ruling 4"
    // doc comment above): only the DATA SOURCE moved.
    // PROV-1 FIX ROUND 1 (review M-3): the family set, read ONCE off the
    // load-time companion index and cloned onto each element, so the two
    // endpoints that serve `CrossRefOut` say the identical thing about the
    // identical rows and a PASSAGE node (served by the bare-array endpoint,
    // which has no envelope) can carry a "?" at all.
    //
    // FIX ROUND 2 (review M-NEW-1) -- THE GRANULARITY, STATED CORRECTLY.
    // This comment used to end "so the rows INSIDE `VerseDetailOut` are
    // attributed per row, not only per section." THAT WAS FALSE, and the
    // field's own DTO doc comment (`CrossRefOut.provenance`) had it right
    // all along: this is SECTION-level attribution DUPLICATED onto each
    // element for transport, not per-row attribution.
    //
    // And per-row is not available here to be had. Measured, not assumed:
    // `graph.cross_refs_by_from` is `HashMap<String,
    // Vec<atlas_core::data::CrossRef>>` (`service.rs`), and
    // `atlas_core::data::CrossRef` (`data.rs`) carries `target` and `votes`
    // and NOTHING ELSE -- the graph-types row that does carry
    // `provenance: ProvenanceId` (`edge.rs`) has it projected away before it
    // reaches this map. Serving the family set is the honest available
    // answer; claiming it is per-row is not. Getting real per-row values
    // would mean widening the companion index, which is the contract-shaped
    // decision this batch deliberately refused, and it must not be implied
    // to have already happened.
    let cross_refs_provenance = graph.provenance.by_family(atlas_graph::provenance::family::CROSS_REFS);
    // DB-4c: a seek on the kjv section's `xref_by_from` (or the Mem arm's
    // retained map), never a whole-corpus companion.
    let by_from = graph.cross_refs_for_span(&ScriptureRef::Verse(vid));
    let cross_refs: Vec<CrossRefOut> = by_from
        .get(&canonical)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(|cr| {
            let first = first_verse_of_target(&cr.target)?;
            let preview = graph.verse_text_of(&VerseRef { book: first.book.0, chapter: first.chapter, verse: first.verse })?;
            Some(CrossRefOut { target: cr.target.clone(), votes: cr.votes, preview, provenance: cross_refs_provenance.clone() })
        })
        .collect();

    // Batch F: same aggregation core `catechism_for_span` below uses for a
    // passage, called here with a single-verse span -- see
    // `VerseDetailOut.catechism`'s own doc comment for why this is folded
    // into the already-shared verse-detail fetch rather than a second call.
    let catechism_provenance = graph.provenance.by_family(atlas_graph::provenance::family::CATECHISM);
    let catechism: Vec<CatechismRefOut> = data
        .catechism_items_for_span(&ScriptureRef::Verse(vid))
        .into_iter()
        .map(|c| CatechismRefOut::from_ref(c, &catechism_provenance))
        .collect();

    // Batch RED-1: the SAME per-verse lookup `handlers::chapter` uses, off
    // the precomputed `graph.red_letter_spans` companion.
    let words_of_christ: Vec<WordsOfChristSpanOut> = graph.red_letter_spans.get(&canonical).map(|spans| spans.iter().map(|&(start, end)| WordsOfChristSpanOut { start, end }).collect()).unwrap_or_default();

    Ok(Json(VerseDetailOut {
        sref: canonical,
        text,
        words_of_christ,
        book_meta: BookMetaOut {
            author: book_meta.author,
            write_place: book_meta.write_place,
            write_from: book_meta.write_from,
            write_to: book_meta.write_to,
        },
        events,
        cross_refs,
        catechism,
        provenance,
        // Batch PROV-1: the two section-level attributions, straight off
        // the load-time companion index -- no scan, no fetch, nothing added
        // to the per-request path (the sub-100ms frontier law binds here).
        // FIX ROUND 1: the same two values now also ride each `cross_refs`/
        // `catechism` ELEMENT (review M-3), which is what lets a PASSAGE
        // node -- served by the bare-array endpoints -- carry a "?" too.
        // These section fields stay for the wire-compatibility they always
        // had; nothing about them moved.
        cross_refs_provenance,
        catechism_provenance,
    }))
}

/// `GET /api/narrative/event/{id}` (Batch N requirement 1's own "endpoint/
/// payload also supports event-id lookup" half; Batch T requirement 2: the
/// resolver itself is UNCHANGED -- `positions_for_events`'s own leg-array-
/// adjacency walk was already exactly "chronologically adjacent given a
/// validated leg order," so no new logic was needed; what changed is WHO
/// calls this endpoint -- `client/Explore/EventNode.cs` replaces the
/// retired `NarrativeEventNode.cs` as its own caller -- and that ETL now
/// ALSO validates same-year legs via `Event::order_key`, not just
/// `when.from_year`, see `atlas_etl::validate::run`): every narrative
/// position the given event id occupies -- mirrors `GET
/// /api/catechism/item/{id}`'s own precedent exactly (an id-keyed follow-on
/// lookup). Reached by the client ONLY with an event id already handed back
/// by a prior response (never typed by a user), so an id that names no real
/// event at all is a genuine "not found," same `place`/`catechism_item`
/// precedent as every other exact-identifier lookup in this file;
/// ruling-3-policy still applies one layer in -- a REAL event that simply
/// isn't a leg of any narrative 200s with an empty array (the "no results"
/// case, not the "bad identifier" case), same as `positions_for_events`
/// itself naturally returns for a bare, narrative-less event (see
/// `narrative::tests::event_in_no_narrative_returns_no_positions`).
///
/// BATCH M-B (controller decision 3): re-implemented as a VIEW over graph
/// queries -- temporal neighbors come from `atlas_graph::Chronology::
/// temporal_neighbors` (built from `ChronologyDerivation::order`, which
/// `tests/timeline_equivalence.rs` proves is EXACTLY
/// `atlas_core::narrative::global_timeline_position`'s own timeline order --
/// the acceptance centerpiece). The bespoke resolvers this endpoint used to
/// call (`positions_for_events`/`global_timeline_position`) RETIRE from this
/// production call site -- `atlas_core::narrative`'s own module is
/// otherwise completely untouched (its OWN tests, including E1-E5, stay
/// green, unmodified) and its two topology functions remain `pub`, still
/// directly unit-tested, simply no longer reached by any live server
/// response.
///
/// BATCH M-C (controller decision 1): succession duals now come straight
/// off the GENERIC PORT instead of a companion index. `graph-types` commit
/// `13184e1` (owner-approved, "EdgeMeta -- per-entry relation metadata")
/// tags every `follows-in`/`precedes-in` entry with the `NarrativeId` it
/// belongs to (`Graph::build_indexes`'s own `Succession` pairing), so "every
/// narrative this event is a leg of, and its neighbor in each" is answerable
/// by draining both direction's pages at this event's own Position and
/// grouping entries by `EdgeMeta::Narrative` -- exactly the SAME
/// `Narrative.legs`-derived data the graph's own `Succession` rows were
/// always built from, just read back through the port instead of a
/// second, hand-maintained index (`atlas_graph::event_world::EventWorld`'s
/// own `narrative_positions` field, RETIRED this batch -- see that
/// module's own `Chronology` doc comment). This closes the M-B review's I-3
/// (validation bypass): there is now only one representation of "which
/// narrative is this leg in," so it cannot silently diverge from the
/// graph's own rows.
///
/// WIRE SHAPE IS BYTE-IDENTICAL (hard requirement, verified by the
/// pre-existing Playwright `event-timeline.spec.ts`/`popover-sections.spec.ts`
/// suites, which exercise this exact endpoint through the unmodified
/// client): `NarrativeEventPositionsOut`/`NarrativePositionOut`/
/// `TimelinePositionOut`/`NarrativeAdjacentEventOut` are UNCHANGED. Each
/// adjacent event's own presentation (label/places/verse_groups) still
/// calls `atlas_core::narrative::adjacent_event` directly (made `pub` at
/// M-B, see that function's own doc comment) -- the SAME presentation
/// builder the OLD resolver used, so label/places/verse_group formatting
/// cannot drift from what shipped before; only the TOPOLOGY (which ids are
/// prior/following, in which narratives) now originates from the graph.
pub async fn narrative_event_positions(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Path(id): Path<String>,
) -> Result<Json<NarrativeEventPositionsOut>, ApiError> {
    use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
    use atlas_graph_types::explore::EdgeMeta;
    use atlas_graph_types::id::{NarrativeId, Position};
    use atlas_graph_types::node::NodePayload;
    use std::collections::BTreeSet;

    let snap = graph.snapshot();
    let event_id = atlas_graph::event_world::event_node_id(&id);
    // OVERLAY-1 Task 5: the existence check and the label are ONE node
    // fetch now -- the label comes straight off the Event node's own
    // payload, so this handler needs no materialised event collection at
    // all (it replaces `data.event_by_id(&id).label`, which the deleted
    // overlay used to populate).
    let Some(node) = snap.node(&event_id) else {
        return Err(ApiError::not_found("event"));
    };
    let event_label = match node.payload {
        NodePayload::Event { label, .. } => label,
        _ => String::new(),
    };
    let event_pos = Position::Node(event_id);

    // "follows-in" (Forward) is THIS event's own following-event page;
    // "precedes-in" (Inverse) is its own prior-event page -- see
    // `graph_types::graph::Graph::build_indexes`'s own Succession pairing
    // (subject = the earlier leg, object = the later leg; `fwd` reads
    // subject -> object, `inv` reads object -> subject).
    let following_entries = drain_edges(&snap, &event_pos, EdgeKind::Directed(RelationId::Succession, Direction::Forward));
    let prior_entries = drain_edges(&snap, &event_pos, EdgeKind::Directed(RelationId::Succession, Direction::Inverse));

    let mut narrative_ids: BTreeSet<NarrativeId> = following_entries
        .iter()
        .chain(prior_entries.iter())
        .filter_map(|e| match &e.meta {
            EdgeMeta::Narrative(nid) => Some(nid.clone()),
            _ => None,
        })
        .collect();
    // THE ONE PRESENTATION SOURCE for this whole handler (OVERLAY-1-HOTFIX-1).
    //
    // `atlas_core::narrative::adjacent_event` -- the shared builder that turns
    // an event id into {label, places, verse_groups} for every `prior`,
    // `following` and `timeline.*` below -- used to take `&AtlasData` and
    // resolve through `AtlasData.events`. OVERLAY-1 Task 5 left that vec
    // permanently EMPTY on every serving path (the boot-time overlay that
    // filled it is gone), so all three fields came back blank for every event
    // in the real atlas, while `tests/api.rs`'s fixture test -- whose
    // `demo_fixture()` hand-fills `events` -- stayed green. Three Playwright
    // specs were the only gate that caught it.
    //
    // It takes the SceneSource now, and this is the same object
    // `handlers::scene_time`/`scene_scripture` compose the map from
    // (materialised from the port with `finish()`'s merges and sort replayed),
    // so an adjacent event's own label/places/verse_groups are LITERALLY the
    // map arrow endpoint's own -- the ONE-GRAPH property
    // `atlas_core::narrative`'s own header states, now true on the serving
    // path too.
    //
    // Fix round 1 (review I-1): "true by construction" is what the first
    // version of this comment said, and it was not. `impl SceneSource for
    // AtlasData` exists, so a future edit CAN hand this function an
    // `AtlasData` again (`adjacent_event(&*data, ..)` compiles) and get the
    // same empty answer back. Two standing laws are the real guard --
    // `tests/no_legacy_event_reads.rs` (no serving source reads the emptied
    // collections or their derived accessors) and
    // `atlas-core/tests/no_atlas_data_in_public_signatures.rs` (no new public
    // fn in atlas-core takes an `AtlasData` at all) -- and the cure is
    // ETL-INPUT-1, deleting the three fields.
    let src = graph.scene_source(&data);

    // A narrative whose `legs` names exactly ONE event (a real, if rare,
    // shape -- e.g. `demo_fixture()`'s own "patriarchs-demo") produces NO
    // `Succession` row pair at all: `chain.windows(2)` on a one-element
    // chain is empty by construction (a doubly-linked list of one node has
    // no links), so its membership is genuinely invisible to the port's
    // EdgeMeta-tagged succession pages -- a real, structural gap in the
    // `succession` relation's own shape (it communicates SEQUENCE, not bare
    // membership), not a bug in this batch's port-based rewrite. Solo-leg
    // narratives are enumerated directly off the narrative list (a small,
    // in-memory scan -- narrative counts stay in the tens, never paged) so
    // this event's own membership in one is never silently dropped; every
    // narrative reached this way that DOES have a real prior/following
    // still gets it from the port entries above.
    // OVERLAY-1 Task 5: that list is `graph.scene_source(&data)`'s own,
    // materialised from `gs.narrative_ids` + `gs.narrative_legs` through
    // `legacy::narrative_from_node` (and post-`apply_event_merges`, so a leg
    // naming an absorbed event is already repointed) -- the exact content
    // and order the deleted `AtlasData.narratives` carried.
    for n in src.narrative_list() {
        if n.legs.len() == 1 && n.legs[0] == id {
            narrative_ids.insert(NarrativeId::new(n.id.clone()));
        }
    }

    let leg_event_id = |entries: &[atlas_graph_types::explore::EdgeEntry], nid: &NarrativeId| -> Option<String> {
        entries.iter().find(|e| matches!(&e.meta, EdgeMeta::Narrative(n) if n == nid)).and_then(|e| match &e.node {
            Position::Node(id) => Some(id.raw.clone()),
            Position::Edge(_) => None,
        })
    };

    let narrative: Vec<NarrativePositionOut> = narrative_ids
        .into_iter()
        .map(|nid| {
            let narrative_name = snap
                .node(&nid.erase())
                .map(|n| match n.payload {
                    NodePayload::Narrative { label, .. } => label,
                    _ => String::new(),
                })
                .unwrap_or_default();
            let prior = leg_event_id(&prior_entries, &nid);
            let following = leg_event_id(&following_entries, &nid);
            NarrativePositionOut {
                narrative_id: nid.0,
                narrative_name,
                event_id: id.clone(),
                event_label: event_label.clone(),
                prior: prior.and_then(|pid| atlas_core::narrative::adjacent_event(src, &pid)).map(Into::into),
                following: following.and_then(|pid| atlas_core::narrative::adjacent_event(src, &pid)).map(Into::into),
            }
        })
        .collect();

    // Batch HOTFIX-4 requirement 1's own "global chronological PRIOR/
    // FOLLOWING" half. TRAV-1 (controller decision 2, "the graph serves
    // it"): temporal-adjacency IS a materialized graph edge now
    // (`RelationId`'s symmetric sibling `SymRelationId::TemporalAdjacency`,
    // TRAV-1's crate patch) -- this reads `GraphService::temporal_neighbors`,
    // built once from the real `temporal_adjacency` rows' own honest
    // `earlier`/`later` ends (service.rs's own doc comment), never
    // re-derived from a position index. `None` (field omitted) for a
    // general-kind passage, by construction -- a general-kind event never
    // gets a `ChronologyDerivation` entry at all (`derive_chronology`
    // filters to `kind == "event"`), so it never gets a `temporal_adjacency`
    // row either, hence absent from `temporal_neighbors` exactly like it
    // was absent from the old `timeline_index`.
    // DB-3: through the port (`GraphService::temporal_neighbors_of`):
    // membership from the chronology order, adjacency from the
    // temporal-adjacency edges, direction from that order.
    let timeline = graph.temporal_neighbors_of(&id).map(|(prior, following)| TimelinePositionOut {
        prior: prior.as_deref().and_then(|pid| atlas_core::narrative::adjacent_event(src, pid)).map(Into::into),
        following: following.as_deref().and_then(|pid| atlas_core::narrative::adjacent_event(src, pid)).map(Into::into),
    });

    Ok(Json(NarrativeEventPositionsOut { narrative, timeline }))
}

/// Drains every page of one edge kind at one position -- the SAME
/// cursor-loop shape `atlas_graph_types::store`'s own (private) conformance
/// harness uses internally, needed here because a handler-side consumer
/// (unlike the generic `/api/node/{id}/edges` endpoint, which hands back
/// exactly one page) sometimes genuinely needs the WHOLE frontier of one
/// kind (e.g. every narrative an event is a leg of) rather than one honest
/// page of it.
fn drain_edges(
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

/// Batch T requirement 4: one EVENT-kind PASSAGE's own resolved place --
/// id (to open a `PlaceNode`/target the map) + display name (so the client
/// never needs a second lookup just to label an explorable place row).
#[derive(Debug, Serialize)]
pub struct EventPlaceOut {
    pub id: String,
    pub name: String,
}

/// Batch T requirement 4: one resolved witness -- "book, verse-range,
/// translation-mapped," the owner's own words verbatim, ALREADY resolved to
/// this app's one compiled translation (`atlas_core::translation::resolve`,
/// real fail-loud lookup, not a silent default) and grouped via the SAME
/// `verse_groups_for` every other verse list on this wire already uses
/// (`atlas_core::scene::witnesses_for` -- one function, so a heading's own
/// anchor verse and this section's own witness list can never disagree).
#[derive(Debug, Serialize)]
pub struct EventWitnessOut {
    pub book: String,
    pub verse_groups: Vec<VerseGroup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub robertson_section: Option<String>,
}

impl From<atlas_core::scene::ResolvedWitness> for EventWitnessOut {
    fn from(w: atlas_core::scene::ResolvedWitness) -> Self {
        EventWitnessOut { book: w.book, verse_groups: w.verse_groups, ref_note: w.ref_note, robertson_section: w.robertson_section }
    }
}

/// `GET /api/event/{id}`'s own wire shape (Batch T requirement 4, "EVENT
/// node popover"): `title` (this event's own `Event::label`), `kind`
/// (Batch T2: `"event"` | `"general"`, ALWAYS present -- the client's own
/// signal for which sections apply), `when`, every resolved place, every
/// resolved witness (ALWAYS >=1 -- see `scene::witnesses_for`'s own doc
/// comment for the single-implicit-witness synthesis; requirement 4's
/// "single-witness events show the one passage, no parallel framing when
/// n=1" is a CLIENT-side rendering decision keyed off `witnesses.len()`,
/// not a server-side omission), and provenance (`robertson_section`/
/// `ref_note`, each omitted, not null, when this event's own date/grouping
/// needed no note beyond the other).
///
/// Batch T2: `when` is OMITTED (not null) for a `kind == "general"`
/// passage -- the internal `Event::when` still holds
/// `TimeRange::undated()` (a structurally-required field, see that
/// function's own doc comment), but a general-kind passage has no
/// defensible date, so nothing here may ever present that sentinel to a
/// reader as a real claim. `places` stays an always-present, possibly-
/// empty array (unchanged pattern -- a general-kind passage's own
/// `Event::places` is always empty by construction, so this needs no
/// separate gating).
#[derive(Debug, Serialize)]
pub struct EventDetailOut {
    pub id: String,
    pub title: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<TimeRange>,
    pub places: Vec<EventPlaceOut>,
    pub witnesses: Vec<EventWitnessOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub robertson_section: Option<String>,
    /// Batch T2 (Acts provenance): Acts's own sibling provenance field to
    /// `robertson_section` above -- see `atlas_core::data::Event::
    /// acts_section`'s own doc comment for why it's separate, not reused.
    /// Omitted (not null) when absent, same convention.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acts_section: Option<String>,
    /// Batch W1 (whole-Bible titled verse containers): the general,
    /// whole-Bible sibling of `acts_section` above -- see
    /// `atlas_core::data::Event::atlas_section`'s own doc comment. Omitted
    /// (not null) when absent, same convention.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas_section: Option<String>,
    /// Batch W3: the KJV's own literal-citation sibling of `robertson_section`/
    /// `acts_section`/`atlas_section` above -- see `atlas_core::data::Event::
    /// kjv_superscription`'s own doc comment. Omitted (not null) when absent,
    /// same convention.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kjv_superscription: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_note: Option<String>,
    /// ATTEST-1 (L3, "Mentioned in"): the verses that MENTION this event
    /// without narrating it -- canonical verse ids, ascending. Distinct
    /// from `witnesses` ON PURPOSE, and that distinction IS the batch: an
    /// account NARRATES the event and belongs under PARALLEL ACCOUNTS; a
    /// mention merely REFERENCES it while narrating something else, and
    /// rendering one as the other is exactly the "fundamental error" the
    /// owner reported. An event whose whole scriptural basis is mentions
    /// (the Espousal of Mary) serves an EMPTY `witnesses` and a non-empty
    /// list here -- a real node with a real frontier, never a fabricated
    /// parallel-accounts section.
    ///
    /// OMITTED (not `[]`) when empty, the same convention every optional
    /// field above follows -- so an event with no mentions serves
    /// byte-identically to before this batch.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mentioned_in: Vec<String>,
    /// ATTEST-1 (L4, "Similar Accounts"): events joined to this one by an
    /// `Analogue` row -- "distinct events whose accounts are similar in
    /// form or content, NEVER two accounts of one event." Deliberately a
    /// SEPARATE field from `witnesses` rather than a flag on it: the
    /// owner's own report was that a similar-but-distinct story was being
    /// rendered as a parallel account, and one field cannot carry two
    /// claims. Omitted when empty, same convention.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub analogues: Vec<EventAnalogueOut>,
    /// Batch PROV-1 (owner order 1): THIS EVENT's own source -- the Event
    /// node's `provenance`, off `Node::provenance`
    /// (`event_world::event_provenance`: `"theographic"` for an imported
    /// event, `"curated"` for one this project authored). The focus card's
    /// own attribution, and the TOTAL-CAPTURE HONESTY case in one field: a
    /// hand-authored event says `curated`, which resolves to the registry's
    /// own "Our Own Curated Work" category, and therefore cannot silently
    /// wear Theographic's clothes at the reader.
    pub provenance: String,
    /// Batch PROV-1: every distinct provenance id behind THIS EVENT's
    /// PARALLEL ACCOUNTS section -- the `Attests` rows for this event
    /// specifically, not the family average.
    ///
    /// A list, and per-EVENT rather than per-family, even though the real
    /// `attests` table is single-sourced TODAY (`{event-witnesses}`,
    /// measured by `the_per_family_provenance_map_of_the_real_artifact_
    /// is_pinned`; ATTEST-1's `attestation-corrections` rows land on
    /// `mentions`/`analogue`, not here -- an earlier version of this
    /// comment said otherwise and was wrong). Per-event is what keeps this
    /// TRUE if a second source ever lands in the table: a family average
    /// would start lying the moment it did, and THE LEPER LESSON is exactly
    /// that a hand-authored row must never be attributed to an importer.
    ///
    /// Omitted (not `[]`) when empty -- an event with no accounts (the
    /// Espousal of Mary) renders no PARALLEL ACCOUNTS section at all, so it
    /// needs no attribution for one; same convention as `mentioned_in`
    /// above, so such an event serves byte-identically to before this
    /// batch but for the two unconditional fields.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub witnesses_provenance: Vec<String>,
    /// Batch PROV-1: the same, for the "Mentioned in" section -- the
    /// `Mentions` rows naming THIS event.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mentions_provenance: Vec<String>,
}

/// ATTEST-1: one end of an `Analogue` -- enough to render and to explore
/// (`/api/event/{id}` takes this `id` straight back).
#[derive(Debug, Serialize)]
pub struct EventAnalogueOut {
    pub id: String,
    pub title: String,
    /// Batch PROV-1: genuinely PER-ROW -- the `Analogue` row joining these
    /// two events carries its own provenance, and this is that value, not
    /// either end's node provenance and not the family's. An `Analogue` is
    /// a curatorial CLAIM about two events ("similar in form or content,
    /// never two accounts of one event"); attributing it to whoever
    /// supplied the events would name the wrong asserter.
    pub provenance: String,
}

/// `GET /api/event/{id}` (Batch T requirement 4): the EVENT node's own rich
/// fetch -- id-keyed, same exact-identifier "unknown id -> 404 not_found"
/// precedent `narrative_event_positions`/`catechism_item`/`place` already
/// set (never a user-typed id; always one a prior response, or a reader
/// heading, already handed back). Reads the graph's own Event node directly
/// (M-C2: via `legacy::event_from_node`, not the scene/narrative machinery,
/// and no longer `data.event_by_id` -- see the M-C2 comment inside this
/// function) since this is a passage's own STANDALONE content --
/// title/date/places/witnesses -- not anything scoped to a window or a
/// narrative position.
pub async fn event(State(data): State<Arc<AtlasData>>, State(graph): State<Arc<GraphService>>, Path(id): Path<String>) -> Result<Json<EventDetailOut>, ApiError> {
    // M-C2 (definitive surface list): reconstructed from the graph's own
    // Event node (`NodePayload::Event`'s own M-C2 widening carries every
    // field this handler needs) instead of `data.event_by_id`.
    // `place_history_for`/`place_name_alias_for` below stay on `AtlasData`
    // deliberately -- `place-history.json`/`place-names-kjv.json` are not
    // this batch's deletion target, unaffected by the migration.
    let snap = graph.snapshot();
    let e: Event = atlas_graph::legacy::event_from_node(&atlas_graph::event_world::event_node_id(&id), &snap, &graph.chronology.chrono).ok_or_else(|| ApiError::not_found("event"))?;
    let e: &Event = &e;

    // Batch E3: resolved name (period-history- and KJV-alias-aware), not the
    // bare Theographic default -- this is the "PARALLEL ACCOUNTS place
    // lines" surface (an EVENT node's own `event-places`/`event-place-{id}`
    // rows render right alongside its PARALLEL ACCOUNTS witness section).
    // Window = this event's own `e.when`, gated on `e.kind == "event"` --
    // Fix round 1 (I-1): a `kind != "event"` ("general") passage's `e.when`
    // is `TimeRange::undated()` (the WHOLE atlas span, [-4004,100] -- see
    // its own doc comment), not an out-of-range sentinel, so passing it as a
    // real window trivially intersects every curated period-name range and
    // lets `resolve_display_name` spuriously pick a period name (or an
    // arbitrary one among several) for a passage that structurally has no
    // date at all. Mirrors the SAME kind-gate this handler already applies
    // to the wire `when` field below (the SAME "no real window here"
    // reasoning `handlers::chapter`/`compose_scripture_scene` already use)
    // -- computed once here, reused for both the places resolution and
    // `when`, so the two can never drift apart again.
    let window = if e.kind == "event" { Some(e.when) } else { None };
    let places = e
        .places
        .iter()
        .filter_map(|pid| atlas_graph::legacy::place_from_node(&atlas_graph::event_world::place_stub_node_id(pid), &snap))
        .map(|p| EventPlaceOut {
            id: p.id.clone(),
            name: resolve_display_name(&p.name, data.place_history_for(&p.id), window, data.place_name_alias_for(&p.id)),
        })
        .collect();
    let witnesses = atlas_core::scene::witnesses_for(e).into_iter().map(EventWitnessOut::from).collect();
    // Batch T2: never surface the undated() sentinel to the wire for a
    // general-kind passage -- see EventDetailOut's own doc comment.
    let when = window;

    // ATTEST-1: the two new frontier sections, both read straight off the
    // graph's own indexes through the SAME generic `drain_edges` walk
    // every other relation in this file uses -- no second path, no
    // re-derivation from AtlasData.
    let event_pos = Position::Node(atlas_graph::event_world::event_node_id(&e.id));
    // (L3) "Mentioned in": `Mentions` INVERSE, event -> the text units
    // that reference it. Rendered as canonical verse ids so the client
    // can hand them straight back to `/api/verse/{sref}`; sorted for a
    // stable reading order (the index's own order is insertion order,
    // which is curated-file order, not canonical order).
    let mut mentioned_in: Vec<String> = drain_edges(&snap, &event_pos, EdgeKind::Directed(RelationId::Mentions, Direction::Inverse))
        .into_iter()
        .filter_map(|entry| match entry.node {
            Position::Node(id) => id.raw.strip_prefix("bible/").and_then(|rest| {
                let mut parts = rest.split('.');
                let book: u8 = parts.next()?.parse().ok()?;
                let chapter: u16 = parts.next()?.parse().ok()?;
                let verse: u16 = parts.next()?.parse().ok()?;
                let code = atlas_core::refs::BookId(book).code();
                Some(((book, chapter, verse), format!("{code}.{chapter}.{verse}")))
            }),
            Position::Edge(_) => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>()
        .into_values()
        .collect();
    mentioned_in.dedup();
    // (L4) "Similar Accounts": the SYMMETRIC `Analogue` relation, walked
    // from this end. Titles come from the neighbour's own Event node, so
    // the client never needs a second fetch just to label the row.
    let analogues: Vec<EventAnalogueOut> = drain_edges(&snap, &event_pos, EdgeKind::Symmetric(atlas_graph_types::edge::SymRelationId::Analogue))
        .into_iter()
        .filter_map(|entry| match entry.node {
            Position::Node(id) => atlas_graph::legacy::event_from_node(&id, &snap, &graph.chronology.chrono).map(|other| {
                // Batch PROV-1: THIS ROW's own provenance, looked up by the
                // pair it joins. `analogue_for_pair` is symmetric (both
                // orderings are stored), so walking the relation from
                // either end resolves the same single row.
                //
                // FIX ROUND 1 (review H-1, HIGH): this used to be
                // `.unwrap_or_default()`, defended by the same false claim
                // corrected at `VerseEventOut` above -- the client filtered
                // the blank into silence rather than shouting about it. The
                // miss IS unreachable for a pair we just WALKED an Analogue
                // edge to reach, which is exactly why a 500 costs nothing
                // and makes the fail-loud claim TRUE. The failure it now
                // catches is the real one the review named: an id
                // normalization or `EventId` alias change that leaves the
                // walked edge and the row key disagreeing.
                // DB-3: `row_provenance` through the port
                // (`GraphService::analogue_provenance`); the fail-loud
                // claim is unchanged.
                let provenance = graph
                    .analogue_provenance(&e.id, &other.id)
                    .filter(|p| !p.trim().is_empty())
                    .ok_or_else(|| ApiError::internal(&format!("analogue row {} <-> {} has no provenance to attribute it to", e.id, other.id)))?;
                Ok(EventAnalogueOut { id: other.id.clone(), title: other.label.clone(), provenance })
            }),
            Position::Edge(_) => None,
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    Ok(Json(EventDetailOut {
        id: e.id.clone(),
        title: e.label.clone(),
        kind: e.kind.clone(),
        when,
        places,
        witnesses,
        robertson_section: e.robertson_section.clone(),
        acts_section: e.acts_section.clone(),
        atlas_section: e.atlas_section.clone(),
        kjv_superscription: e.kjv_superscription.clone(),
        ref_note: e.ref_note.clone(),
        mentioned_in,
        analogues,
        // Batch PROV-1: the event's own node provenance. `event_from_node`
        // above already proved the node exists (it reconstructed `e` from
        // it), so this second read cannot legitimately miss -- fail-loud
        // rather than defaulted, same reasoning as `handlers::verse`.
        // FIX ROUND 1 (review H-1): `.filter` added for the same reason as
        // `handlers::verse`'s -- a node whose provenance is a blank string
        // used to pass the absence guard and reach the wire as "".
        provenance: snap
            .node(&atlas_graph::event_world::event_node_id(&e.id))
            .map(|n| n.provenance)
            .filter(|p| !p.trim().is_empty())
            .ok_or_else(|| ApiError::internal(&format!("event {} has no node to attribute it to", e.id)))?,
        // DB-3: both through `GraphQuery::row_provenance`, one lookup per
        // walked edge (`GraphService::{attests_provenance,
        // event_mentions_provenance}`), instead of the retired load-time
        // per-event maps.
        witnesses_provenance: graph.attests_provenance(&e.id),
        mentions_provenance: graph.event_mentions_provenance(&e.id),
    }))
}

/// `GET /api/catechism/{sref}` (Batch F, "verse -> citing catechism items
/// lookup" -- requirement 3): `sref` must parse as exactly a
/// `ScriptureRef::Verse` or `ScriptureRef::Passage`, mirroring
/// `handlers::xrefs`'s own accepted-shapes precedent exactly (a bare book or
/// chapter ref has no defined "member verses" to aggregate over, so both 400
/// as `bad_ref`). ruling-3-policy: an sref with no citing catechism items at
/// all is NOT an error -- 200 with an empty list, same "gracefully empty,
/// never a 404" policy `xrefs`/`chapter`/`scene_scripture` already follow.
/// Business logic (the union-across-member-verses aggregation) lives in
/// `atlas_core::catechism::items_for_span`, reached here via
/// `AtlasData::catechism_items_for_span` -- this handler is pure
/// response-shape assembly, same as every other handler in this file.
///
/// PROV-1 FIX ROUND 1 (review M-3): now takes `State<Arc<GraphService>>`
/// too, purely to attribute each row. That is the SECOND extractor the
/// review thought would make this half "a genuinely larger change" -- it is
/// not: `AppState` already implements `FromRef<AppState>` for
/// `Arc<GraphService>`, and six handlers in this file already take both.
/// The aggregation itself is untouched; `AtlasData` is still where the
/// business logic lives.
pub async fn catechism_for_span(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Path(sref): Path<String>,
) -> Result<Json<Vec<CatechismRefOut>>, ApiError> {
    let span = match ScriptureRef::parse(&sref) {
        Ok(span @ (ScriptureRef::Verse(_) | ScriptureRef::Passage { .. })) => span,
        _ => return Err(ApiError::bad_ref(&sref)),
    };

    let provenance = graph.provenance.by_family(atlas_graph::provenance::family::CATECHISM);
    let out = data.catechism_items_for_span(&span).into_iter().map(|c| CatechismRefOut::from_ref(c, &provenance)).collect();
    Ok(Json(out))
}

/// Batch F: one resolved proof verse -- `vref` + its own FULL KJV text
/// (design-direction.md's house rendering, per requirement 4: "THE
/// SCRIPTURES -- the item's proof verses... full verse text per house
/// rendering" -- not a truncated preview the way `CrossRefOut.preview` is).
/// Batch F2: `question` (omitted when absent, same convention as
/// `CatechismRefOut.question`) names which question this proof verse came
/// from, when it came from one -- `None` for Luther's own item-level
/// embedded citations. This is requirement 4's own "if cheap, highlight/
/// deep-link the question context": a small caption next to the verse in
/// THE SCRIPTURES, cheap because it needs no new fetch or scroll machinery,
/// just this one extra field already available at merge time.
#[derive(Debug, Serialize)]
pub struct CatechismProofVerseOut {
    pub vref: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
}

/// `GET /api/catechism/item/{id}`'s own wire shape -- `part_title` alongside
/// the item's own fields so the client never needs a second fetch to show
/// "Baptism" as this item's own chief-part context. `where_written` is
/// omitted (not `null`) when absent -- same conditional-presence wire
/// convention `HistoryOut.blurb`/`established`/`destroyed` already use.
#[derive(Debug, Serialize)]
pub struct CatechismItemOut {
    pub id: String,
    pub name: String,
    pub part_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub explanation_heading: String,
    pub explanation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub where_written: Option<String>,
    pub verses: Vec<CatechismProofVerseOut>,
}

/// `GET /api/catechism/item/{id}` (Batch F, "item fetch by id" --
/// requirement 3). Unknown id -> 404 `not_found`, same precedent
/// `handlers::place` already set for an exact-identifier lookup (not a ref
/// with its own "out of canon but still valid shape" middle ground). Each
/// proof verse resolves to its own full KJV text (`CatechismProofVerseOut`'s
/// own doc comment); a verse id that fails to resolve (should never happen
/// -- `validate::run_catechism` already guarantees every curated verse
/// exists in the compiled KJV text) is skipped rather than panicking, same
/// ruling-4 soft-fail policy `handlers::verse`'s own cross-ref preview
/// lookup already follows.
///
/// OVERLAY-1 Task 2: proof-verse text now comes from `graph.verse_text_of`
/// (a real, on-demand graph query), not the retired `AtlasData.verses` --
/// this handler picks up a second extractor, `State<Arc<GraphService>>`,
/// the same combined-state pattern `handlers::verse` already uses.
pub async fn catechism_item(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Path(id): Path<String>,
) -> Result<Json<CatechismItemOut>, ApiError> {
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
    let mut verses: Vec<CatechismProofVerseOut> = Vec::new();
    for v in &item.verses {
        if !seen.insert((v.clone(), None)) {
            continue;
        }
        if let Some(text) = text_of(v) {
            verses.push(CatechismProofVerseOut { vref: v.clone(), text, question: None });
        }
    }
    for q in &item.questions {
        for v in &q.verses {
            if !seen.insert((v.clone(), Some(q.title.clone()))) {
                continue;
            }
            if let Some(text) = text_of(v) {
                verses.push(CatechismProofVerseOut { vref: v.clone(), text, question: Some(q.title.clone()) });
            }
        }
    }

    Ok(Json(CatechismItemOut {
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

/// `GET /api/xrefs/{sref}` (batch-g1-brief.md requirement 2, "passage
/// context -- passages give xrefs, not just geo"). `sref` must parse as
/// exactly a `ScriptureRef::Verse` or `ScriptureRef::Passage` -- the brief's
/// own two given examples, `GEN.1.1` and `GEN.1.1-5`, are read as an
/// exhaustive pair (a single verse or a same-chapter span) rather than a
/// representative sample of every `ScriptureRef` shape: a bare book or
/// chapter ref (`GEN`, `GEN.1`) has no defined "member verses" for this
/// endpoint to aggregate over, so both 400 as `bad_ref`, the same typed
/// error every other ref-shaped endpoint already uses (requirement 2:
/// "Typed errors (bad_ref) unchanged").
///
/// ruling-3-policy: unlike `/api/verse/{vref}`, an sref with no recorded
/// cross-references at all -- including one naming a verse outside this
/// atlas's compiled canon -- is NOT an error: 200 with an empty list, the
/// same "gracefully empty, never a 404" policy `scene_scripture`/`chapter`
/// already follow. This falls out of the aggregation itself needing no
/// special-casing: `aggregate_span_xrefs` only ever reads `cross_refs` by
/// key and calls `verse_text` by key, and a key simply absent/`None`
/// contributes nothing, which is exactly as true for a real, canonical
/// verse with zero curated cross-references (the overwhelmingly common
/// case) as for an out-of-canon one.
///
/// Business logic (the union-and-sum aggregation, self-target drop, sort,
/// cap-at-20) lives in `atlas_core::xrefs::aggregate_span_xrefs` -- this
/// handler is pure response-shape assembly, per this module's own file
/// header.
/// OVERLAY-1 Task 2: `aggregate_span_xrefs`'s own preview-text parameter
/// is now `impl Fn(&str) -> Option<String>`, not `&HashMap<String,
/// String>` -- `atlas_core` still has no `graph-types` dependency of its
/// own (the closure type crosses the boundary, not a graph type), and the
/// aggregation logic itself is unchanged. `graph.cross_refs_by_from` (the
/// graph's own `cites` rows -- `target` carries each row's own
/// `target_display`, the honest original citation string, graph_types::
/// edge::CrossRef's own M-C2 widening) still supplies the rows; the
/// preview text now comes from `graph.verse_text_of` called per candidate
/// key, on demand, instead of the retired `graph.verse_text` whole-spine
/// companion. `AtlasData` is still not read anywhere in this handler.
pub async fn xrefs(State(graph): State<Arc<GraphService>>, Path(sref): Path<String>) -> Result<Json<Vec<CrossRefOut>>, ApiError> {
    let span = match ScriptureRef::parse(&sref) {
        Ok(span @ (ScriptureRef::Verse(_) | ScriptureRef::Passage { .. })) => span,
        _ => return Err(ApiError::bad_ref(&sref)),
    };

    let by_from = graph.cross_refs_for_span(&span);
    let aggregated = aggregate_span_xrefs(&span, &by_from, |key| {
        let v = VerseId::parse_canonical(key).ok()?;
        graph.verse_text_of(&VerseRef { book: v.book.0, chapter: v.chapter, verse: v.verse })
    });
    // PROV-1 FIX ROUND 1 (review M-3): one read off the load-time companion
    // index, cloned per row -- no scan, no fetch. See
    // `CrossRefOut.provenance` for why the value is the family SET.
    let provenance = graph.provenance.by_family(atlas_graph::provenance::family::CROSS_REFS);
    let out = aggregated
        .into_iter()
        .map(|x| CrossRefOut { target: x.target, votes: x.votes, preview: x.preview, provenance: provenance.clone() })
        .collect();
    Ok(Json(out))
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
    /// Batch E3 (requirement 2's quiet provenance note): the bare, stripped,
    /// un-aliased, un-period-resolved canonical name. ALWAYS COMPUTED --
    /// unlike `history`, which stays absent for a place with no curated
    /// `PlaceHistory` record at all (e.g. `cush-2`), this field's own
    /// resolution never depends on `history` existing -- but only ever
    /// SERIALIZED (`Some`) when it differs from whatever name is actually
    /// showing this request (a curated period name OR a curated KJV alias
    /// resolved to something else). Omitted (`None`, not a repeat of the
    /// title) whenever this place's displayed name already IS its canonical
    /// name, so the client's own popover renders NO provenance note rather
    /// than a vacuous "known elsewhere as X" that just repeats the title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_name: Option<String>,
    /// ENT-1a (owner order: "we actually want meaningful information about
    /// who or what someone is, having that be backed by scripture"):
    /// Easton's Bible Dictionary (1897, public domain) prose, source-
    /// attested, `None` until a match exists -- never fabricated. ADDITIVE
    /// JSON (batch-ent1a-brief.md controller decision 3): the current
    /// client ignores unknown/absent fields; the EntityProfile presentation
    /// half that renders this is HELD for the frontend-elegance brainstorm.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
/// M-C2 (definitive surface list): `place`/`events` now come from the
/// graph -- `atlas_graph::legacy::place_from_node` for the place itself,
/// the `site-of` (`located-at` INVERSE) frontier at the place's own
/// position for its own events (every LocatedAt row event_world.rs builds
/// for this place, order-independent here since the explicit `sort_by_key`
/// below already re-establishes the from_year order regardless).
/// `place_history_for`/`place_name_alias_for` stay on `AtlasData`
/// deliberately -- `place-history.json`/`place-names-kjv.json` are not
/// this batch's deletion target.
pub async fn place(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<PlaceDetailOut>, ApiError> {
    let snap = graph.snapshot();
    let place_id = atlas_graph::event_world::place_stub_node_id(&id);
    let place = atlas_graph::legacy::place_from_node(&place_id, &snap).ok_or_else(|| ApiError::not_found("place"))?;

    let mut events: Vec<SceneEvent> = drain_edges(&snap, &Position::Node(place_id.clone()), EdgeKind::Directed(RelationId::LocatedAt, Direction::Inverse))
        .into_iter()
        .filter_map(|entry| match entry.node {
            Position::Node(eid) => atlas_graph::legacy::event_from_node(&eid, &snap, &graph.chronology.chrono),
            Position::Edge(_) => None,
        })
        .map(|e| to_scene_event(&e))
        .collect();
    events.sort_by_key(|e| e.when.from_year);

    let window = match (params.get("from").and_then(|s| s.parse::<i32>().ok()), params.get("to").and_then(|s| s.parse::<i32>().ok()))
    {
        (Some(from), Some(to)) => TimeRange::new(from, to).ok(),
        _ => None,
    };

    // Batch E3: resolved ONCE here (not re-derived inside the `history`
    // closure below, and not gated on `history` existing at all -- a place
    // like `cush-2` has no curated `PlaceHistory` record whatsoever, so
    // `history` stays `None`, but its KJV alias must still resolve).
    let alias = data.place_name_alias_for(&id);
    let (display_name, canonical_name) = resolve_display_name_and_canonical(&place.name, data.place_history_for(&id), window, alias);

    let history = data.place_history_for(&id).map(|h| HistoryOut {
        display_name: display_name.clone(),
        blurb: window.and_then(|w| resolve_blurb(&h.blurbs, w)).map(|b| b.text.clone()),
        established: h.established.as_ref().map(|c| DateClaimOut { when: c.when, verses: c.verses.clone(), note: c.note.clone() }),
        destroyed: h.destroyed.as_ref().map(|c| DateClaimOut { when: c.when, verses: c.verses.clone(), note: c.note.clone() }),
    });

    // ENT-1a: a separate, tiny lookup (not threaded through `place`, the
    // legacy `atlas_core::data::Place` reconstruction above) -- see
    // `graph_handlers::node_description`'s own doc comment (batch-
    // polish1-brief.md ENT1A-m4: generalized from the old Place-only
    // `place_description`, unified with `graph_handlers::node_card`'s own
    // near-twin accessor; batch-finalp2's own layering cleanup relocated
    // it from `atlas_graph::legacy` to this crate -- this call site's own
    // behavior is unchanged either way).
    let description = crate::graph_handlers::node_description(&place_id, &snap);

    Ok(Json(PlaceDetailOut { id: place.id.clone(), name: place.name.clone(), lat: place.lat, lon: place.lon, events, history, canonical_name, description }))
}
