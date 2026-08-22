//! Parsers for `data/curated/*.toml` (schemas per spec §4.5 / the Task 5
//! brief): `eras.toml`, `books.toml`, one narrative per
//! `narratives/*.toml` file, and `events-extra.toml` for curator-defined
//! events that have no Theographic counterpart.
//!
//! Adapted name: the brief lists this group as
//! `parse_eras/parse_books/parse_narratives/parse_events_extra`, but each
//! `narratives/*.toml` file holds exactly one narrative (bare top-level
//! `id`/`name`/`color`/`legs`, not a `[[narrative]]` array-of-tables like
//! the other three schemas), so the function here is `parse_narrative`
//! (singular) — `main.rs` calls it once per file found under
//! `data/curated/narratives/`.
//!
//! `events-extra.toml`'s `verses` field accepts a curator-friendly range
//! (`"EXO.14.21-31"`, our own canonical codes — not raw OSIS) and is
//! expanded here into individual canonical verse ids, since the compiled
//! `Event.verses` contract (validated by `validate::run`) is single verses
//! only (mirrors the `Place.verse_links`/`Event.verses` "etl-validated"
//! trust class scene composition relies on).

use anyhow::{bail, Context, Result};
use atlas_core::data::{BookMeta, BookNarrationWindow, CatechismItem, CatechismPart, ChronologyAnchor, Era, Event, Landmark, LandMaskRegion, Narrative, PlaceBlurbEntry, PlaceDateClaim, PlaceHistory, PlaceNameAlias, PlaceNameEntry, Polity, PolityDelta, PolityEra};
use atlas_core::refs::ScriptureRef;
use atlas_core::time::TimeRange;
use serde::Deserialize;

use crate::catechism_map::{Deut5Entry, MappingFile, MappingOverride};

#[derive(Deserialize)]
struct ErasFile {
    era: Vec<Era>,
}

/// Parses `eras.toml`. Reuses `atlas_core::data::Era` directly for
/// deserialization since its field names (`id`, `name`, `from_year`,
/// `to_year`) already match the TOML schema exactly. Does not itself
/// validate contiguity/coverage/zero-years — that's `validate::run`'s job,
/// since it needs the full picture (and must report ALL violations, not
/// fail fast at the first bad era).
pub fn parse_eras(input: &str) -> Result<Vec<Era>> {
    let f: ErasFile = toml::from_str(input).context("eras.toml: invalid TOML or does not match the [[era]] schema")?;
    Ok(f.era)
}

#[derive(Deserialize)]
struct ChronologyAnchorsFile {
    anchor: Vec<ChronologyAnchor>,
}

/// Batch HOTFIX-6: parses `chronology-anchors.toml` (that file's own header
/// has the full schema/design rationale). Reuses `atlas_core::data::
/// ChronologyAnchor` directly for deserialization (its field names already
/// match the TOML schema exactly, same shape as `parse_eras` above). Pure
/// and STRUCTURAL only -- cross-checking each `event_id` against the real
/// compiled event set and each `era_boundary` row for a real bound
/// `event_id` are `validate::run_chronology_anchors`'s own job (needs the
/// fuller picture), matching every other curated schema in this module.
pub fn parse_chronology_anchors(input: &str) -> Result<Vec<ChronologyAnchor>> {
    let f: ChronologyAnchorsFile =
        toml::from_str(input).context("chronology-anchors.toml: invalid TOML or does not match the [[anchor]] schema")?;
    Ok(f.anchor)
}

#[derive(Deserialize)]
struct BookNarrationWindowsFile {
    window: Vec<BookNarrationWindow>,
}

/// Batch HOTFIX-6: parses `book-narration-windows.toml` (that file's own
/// header has the full design rationale). Same reuse-the-atlas-core-struct
/// shape as `parse_chronology_anchors` immediately above.
pub fn parse_book_narration_windows(input: &str) -> Result<Vec<BookNarrationWindow>> {
    let f: BookNarrationWindowsFile = toml::from_str(input)
        .context("book-narration-windows.toml: invalid TOML or does not match the [[window]] schema")?;
    Ok(f.window)
}

#[derive(Deserialize)]
struct BookToml {
    code: String,
    author: String,
    #[serde(default)]
    write_place: Option<String>,
    #[serde(default)]
    write_from: Option<i32>,
    #[serde(default)]
    write_to: Option<i32>,
}

#[derive(Deserialize)]
struct BooksFile {
    book: Vec<BookToml>,
}

/// Parses `books.toml`. The TOML field is named `code` (matching the other
/// curated schemas' terminology) while `BookMeta`'s field is named `book`;
/// this wrapper does that one rename. Does not check `write_place` against
/// the known place set — that cross-reference (WARN + drop on unknown,
/// per the brief) needs the compiled place list, so it lives in `main.rs`.
pub fn parse_books(input: &str) -> Result<Vec<BookMeta>> {
    let f: BooksFile = toml::from_str(input).context("books.toml: invalid TOML or does not match the [[book]] schema")?;
    Ok(f.book
        .into_iter()
        .map(|b| BookMeta { book: b.code, author: b.author, write_place: b.write_place, write_from: b.write_from, write_to: b.write_to })
        .collect())
}

/// Parses one `narratives/*.toml` file (bare top-level fields; `Narrative`'s
/// field names already match exactly, so this is a direct deserialize).
pub fn parse_narrative(input: &str) -> Result<Narrative> {
    toml::from_str(input).context("narrative TOML: invalid TOML or does not match the Narrative schema (id/name/color/legs)")
}

#[derive(Deserialize)]
struct EventToml {
    id: String,
    label: String,
    /// Batch T2: OPTIONAL -- a `kind = "general"` row must OMIT both (see
    /// `parse_events_extra`'s own conditional-required logic below); a
    /// `kind = "event"` row (or `kind` absent, the default) still REQUIRES
    /// both, unchanged since Task 3.
    #[serde(default)]
    from_year: Option<i32>,
    #[serde(default)]
    to_year: Option<i32>,
    /// Batch T2: `#[serde(default)]` added so a `kind = "general"` row can
    /// omit this key entirely, same reasoning as `from_year`/`to_year`.
    #[serde(default)]
    places: Vec<String>,
    #[serde(default)]
    verses: Vec<String>,
    /// Batch T2: `"event"` (default, back-compat) | `"general"` -- see
    /// `atlas_core::data::Event::kind`'s own doc comment for the shape
    /// each implies. Re-validated against the full enum by
    /// `atlas_etl::validate::run` too (this parser only ever produces
    /// "event"/"general" itself, but `validate::run` is the single source
    /// of truth for the allowed set, same split every other curated enum
    /// in this app follows).
    #[serde(default)]
    kind: Option<String>,
    /// Batch T requirement 1/2: optional curator-authored provenance/
    /// ordering (see `atlas_core::data::Event`'s own doc comments for each).
    /// `#[serde(default)]` so every pre-Batch-T event in this file (the
    /// overwhelming majority) keeps parsing with no migration.
    #[serde(default)]
    robertson_section: Option<String>,
    /// Batch W1: inline `acts_section` provenance -- until this batch,
    /// `Event::acts_section` was only ever SET via the separate
    /// `data/curated/acts-sections.toml` merge file (every prior Acts
    /// event was a pre-existing Theographic id being enriched, never a
    /// brand-new `[[event]]` row -- see `parse_acts_sections`'s own doc
    /// comment). W1 closes a genuine coverage gap with NO pre-existing
    /// Theographic event to enrich (Acts 1:1-3, Luke's own preface to
    /// Theophilus, before the acts-sections.toml-enriched narrative even
    /// begins at 1:4) -- this field lets a brand-new Acts passage carry
    /// its own provenance inline, the SAME way `robertson_section` already
    /// does for brand-new Gospel events, without disturbing
    /// `acts-sections.toml`'s own existing enrichment-only role.
    #[serde(default)]
    acts_section: Option<String>,
    /// Batch W1: inline `atlas_section` provenance for a brand-new passage
    /// authored directly in a `data/curated/passages/*.toml` file (this
    /// batch's own new one-file-per-book directory, reusing this exact
    /// `EventToml`/`EventsFile`/`parse_events_extra` schema unmodified) --
    /// see `atlas_core::data::Event::atlas_section`'s own doc comment for
    /// the sibling "enrich a pre-existing Theographic event instead" path
    /// (`parse_atlas_sections`, below), which this field does NOT cover
    /// (that path has no `[[event]]` row to attach an inline field to in
    /// the first place).
    #[serde(default)]
    atlas_section: Option<String>,
    /// Batch W3: inline `kjv_superscription` provenance -- the KJV's own
    /// literal-citation sibling of `robertson_section`/`acts_section`/
    /// `atlas_section` above (see `atlas_core::data::Event::
    /// kjv_superscription`'s own doc comment). Inline-only, same as
    /// `acts_section`/`atlas_section` can also be set via their own merge
    /// files (`acts-sections.toml`/`atlas-sections.toml`) for promoting a
    /// bare pre-existing Theographic event -- this field has no such
    /// merge-file sibling, since every Batch W3 superscription-titled
    /// container is a brand-new `[[event]]` row, never a promotion (no
    /// pre-existing Theographic event models an individual Psalm).
    #[serde(default)]
    kjv_superscription: Option<String>,
    #[serde(default)]
    ref_note: Option<String>,
    #[serde(default)]
    order_key: i32,
}

fn default_event_kind() -> String {
    "event".to_string()
}

#[derive(Deserialize)]
struct EventsFile {
    event: Vec<EventToml>,
}

/// Expands one curated `verses` entry (a single canonical verse, e.g.
/// `"EXO.12.37"`, or a same-chapter range, e.g. `"EXO.14.21-31"`, using our
/// own book codes) into one-or-more canonical single-verse strings.
/// `context` names whatever curated record this ref came from (an event id,
/// a catechism item id, ...) purely for the error message -- shared by every
/// curated schema in this module that accepts a curator-friendly
/// single-verse-or-range string (originally `parse_events_extra`'s own
/// helper; Batch F's `parse_catechism` reuses it verbatim rather than
/// re-implementing the same expansion a second time).
/// `pub` (not private) specifically so `main.rs` -- a SEPARATE crate from
/// this library's own perspective, even though it's the same Cargo package
/// -- can reuse it for `catechism-deut5.toml`'s own `verses` field (see
/// `main.rs`'s own Batch F2 catechism section): those entries use this
/// project's OWN canonical-ref convention (e.g. `"DEU.5.9-10"`), identical
/// in shape to `catechism.toml`'s own `verses` field, so this is the exact
/// same expansion, not a second implementation of it.
pub fn expand_verse_ref(raw: &str, context: &str, out: &mut Vec<String>) -> Result<()> {
    match ScriptureRef::parse(raw) {
        Ok(ScriptureRef::Verse(v)) => out.push(format!("{}.{}.{}", v.book.code(), v.chapter, v.verse)),
        Ok(ScriptureRef::Passage { book, chapter, from_verse, to_verse }) => {
            for verse in from_verse..=to_verse {
                out.push(format!("{}.{}.{}", book.code(), chapter, verse));
            }
        }
        _ => bail!("curated data '{context}' has an unparseable verse ref '{raw}' (expected e.g. 'EXO.12.37' or 'EXO.14.21-31')"),
    }
    Ok(())
}

/// Parses `events-extra.toml`. Hard-errors (does not soft-drop) on an
/// invalid time range or a curated event with zero places, since this is
/// our own authored data and should be held to a higher bar than
/// third-party raw data with soft-dropped rows.
///
/// Batch T2 (general-kind PASSAGEs): `kind` gates which of `from_year`/
/// `to_year`/`places` are REQUIRED vs FORBIDDEN, in both directions --
/// "do not fabricate a date/place" is enforced structurally, not by
/// convention:
/// - `kind` absent or `"event"` (unchanged since Task 3): `places`
///   non-empty and `from_year`/`to_year` both present are REQUIRED; a
///   missing one of any of these is a hard error.
/// - `kind = "general"`: `places`/`from_year`/`to_year` must all be
///   ABSENT from the curated row -- a curator writing a date/place
///   TOGETHER WITH `kind = "general"` is a hard error too (one of the two
///   is a mistake; failing loud beats silently picking one). `when`
///   becomes `TimeRange::undated()` -- never a curator-typed number --
///   and `places` stays the empty `Vec` `#[serde(default)]` already gives
///   it when omitted.
pub fn parse_events_extra(input: &str) -> Result<Vec<Event>> {
    let f: EventsFile =
        toml::from_str(input).context("events-extra.toml: invalid TOML or does not match the [[event]] schema")?;

    let mut out = Vec::with_capacity(f.event.len());
    for e in f.event {
        let kind = e.kind.clone().unwrap_or_else(default_event_kind);
        let (when, places) = if kind == "general" {
            if e.from_year.is_some() || e.to_year.is_some() {
                bail!(
                    "curated event '{}' is kind=\"general\" but specifies from_year/to_year -- a general-kind passage must not claim a date (omit both fields; do not fabricate)",
                    e.id
                );
            }
            if !e.places.is_empty() {
                bail!(
                    "curated event '{}' is kind=\"general\" but specifies places -- a general-kind passage must not claim a place mapping (omit the field; do not fabricate)",
                    e.id
                );
            }
            (TimeRange::undated(), e.places)
        } else {
            if e.places.is_empty() {
                bail!("curated event '{}' has no places (places[0] is required as the narrative-arrow anchor)", e.id);
            }
            let (from_year, to_year) = match (e.from_year, e.to_year) {
                (Some(fy), Some(ty)) => (fy, ty),
                _ => bail!(
                    "curated event '{}' is missing from_year/to_year -- required unless kind=\"general\"",
                    e.id
                ),
            };
            let when = TimeRange::new(from_year, to_year).map_err(|src| {
                anyhow::anyhow!("curated event '{}' (from_year={}, to_year={}): {}", e.id, from_year, to_year, src)
            })?;
            (when, e.places)
        };
        let mut verses = Vec::new();
        for v in &e.verses {
            expand_verse_ref(v, &e.id, &mut verses)?;
        }
        out.push(Event {
            id: e.id,
            label: e.label,
            when,
            places,
            verses,
            kind,
            robertson_section: e.robertson_section,
            acts_section: e.acts_section,
            atlas_section: e.atlas_section,
            kjv_superscription: e.kjv_superscription,
            ref_note: e.ref_note,
            order_key: e.order_key,
            ..Default::default()
        });
    }
    Ok(out)
}

// --- Batch T requirement 1: event-witnesses.toml ("parallel witnesses") ---

#[derive(Deserialize)]
struct EventWitnessesFile {
    witness: Vec<WitnessToml>,
}

#[derive(Deserialize)]
struct WitnessToml {
    event_id: String,
    book: String,
    /// Curator-friendly single-verse-or-range strings (this project's own
    /// canonical codes, e.g. `"MAT.27.33-50"`) -- expanded the SAME way
    /// `parse_events_extra`'s own `verses` field already is (`expand_verse_ref`,
    /// reused verbatim, not re-implemented).
    verses: Vec<String>,
    #[serde(default)]
    ref_note: Option<String>,
    #[serde(default)]
    robertson_section: Option<String>,
}

/// Parses `event-witnesses.toml` (Batch T requirement 1: "PARALLEL
/// WITNESSES -- the set of per-book passages that recount the same event").
/// Schema: a FLAT `[[witness]]` array, each row explicitly naming its own
/// `event_id` -- deliberately NOT nested under a per-event `[[event]]`
/// group the way `polities/*.toml`'s `[era.transition]`/`[era.fall]` are
/// (see `atlas_core::data::PolityDelta::for_era_from`'s own doc comment for
/// the exact TOML array-of-tables mis-attachment bug that shipped once,
/// live, from that nested shape -- 7 of Batch M's own 22 deltas). A FLAT
/// list with an explicit id per row has NO equivalent risk to guard against
/// in the first place -- there is no "most recently opened parent" for a
/// row to silently attach to instead of the one a curator's own comment
/// describes, so no echo-field discipline is needed here; the batch report
/// discloses this as the reason, not an oversight.
///
/// Pure and STRUCTURAL only, same split every other curated schema in this
/// module follows: a malformed file bails immediately; cross-checking each
/// `event_id` against the real compiled event set, each `book` against the
/// real canon, and each verse against the compiled KJV text all need the
/// fuller picture and are `main.rs`'s / `validate::run`'s own job (matching
/// `parse_place_history`'s own pure-parse-then-cross-validate precedent).
pub fn parse_event_witnesses(input: &str) -> Result<Vec<(String, atlas_core::data::EventWitness)>> {
    let f: EventWitnessesFile =
        toml::from_str(input).context("event-witnesses.toml: invalid TOML or does not match the [[witness]] schema")?;

    let mut out = Vec::with_capacity(f.witness.len());
    for w in f.witness {
        let mut verses = Vec::new();
        for v in &w.verses {
            expand_verse_ref(v, &format!("witness for event '{}' ({})", w.event_id, w.book), &mut verses)?;
        }
        let translations =
            std::collections::HashMap::from([(atlas_core::translation::DEFAULT_TRANSLATION.to_string(), verses)]);
        out.push((
            w.event_id,
            atlas_core::data::EventWitness { book: w.book, translations, ref_note: w.ref_note, robertson_section: w.robertson_section },
        ));
    }
    Ok(out)
}

// --- Batch T2: acts-sections.toml (Acts provenance) ---

#[derive(Deserialize)]
struct ActsSectionsFile {
    #[serde(default)]
    section: Vec<ActsSectionToml>,
}

#[derive(Deserialize)]
struct ActsSectionToml {
    event_id: String,
    acts_section: String,
}

/// Parses `acts-sections.toml` (Batch T2, Acts provenance -- owner's own
/// ambiguity ruling: "acts sections get their own provenance key, NOT
/// robertson_section"). Schema: a FLAT `[[section]]` array, each row
/// explicitly naming its own `event_id` -- the SAME flat, no-mis-attachment-
/// risk shape `event-witnesses.toml`/`parse_event_witnesses` already
/// establishes (see that function's own doc comment for the full
/// reasoning), reused here rather than re-invented, since this file has
/// the identical "one curated fact per named event id" structure. This
/// file exists specifically so a bare Theographic-sourced event (which
/// has no `[[event]]` row of its own in `events-extra.toml` to add a field
/// to directly) can still gain Acts provenance -- `main.rs` merges it onto
/// the FULL combined event set (Theographic + events-extra.toml) by id,
/// the same merge timing/mechanism `event-witnesses.toml` already uses.
///
/// Pure and STRUCTURAL only, same split every other curated schema in this
/// module follows: a malformed file bails immediately; cross-checking each
/// `event_id` against the real compiled event set is `main.rs`'s own job.
pub fn parse_acts_sections(input: &str) -> Result<Vec<(String, String)>> {
    let f: ActsSectionsFile =
        toml::from_str(input).context("acts-sections.toml: invalid TOML or does not match the [[section]] schema")?;
    Ok(f.section.into_iter().map(|s| (s.event_id, s.acts_section)).collect())
}

// --- Batch W1: atlas-sections.toml (whole-Bible provenance, the general
// sibling of acts-sections.toml) ---

#[derive(Deserialize)]
struct AtlasSectionsFile {
    #[serde(default)]
    section: Vec<AtlasSectionToml>,
}

#[derive(Deserialize)]
struct AtlasSectionToml {
    event_id: String,
    atlas_section: String,
}

/// Parses `atlas-sections.toml` (Batch W1, whole-Bible titled verse
/// containers -- req 1's own provenance vocabulary: "atlas_section (our own
/// sectioning, the sanctioned Acts-precedent fallback)"). Schema and
/// purpose are IDENTICAL to `parse_acts_sections` immediately above, just
/// for `Event::atlas_section` instead of `Event::acts_section` -- a flat
/// `[[section]]` array, each row explicitly naming its own `event_id`,
/// letting a bare pre-existing Theographic-sourced event (already real
/// title/date/place, CC BY-SA 4.0, already credited in LICENSES.md) gain
/// heading-worthy provenance with no duplicate `[[event]]` row. `main.rs`
/// merges it onto the FULL combined event set (Theographic +
/// events-extra.toml + passages/*.toml) by id, the same merge timing/
/// mechanism `acts-sections.toml`/`event-witnesses.toml` already use.
///
/// Pure and STRUCTURAL only, same split every other curated schema in this
/// module follows: a malformed file bails immediately; cross-checking each
/// `event_id` against the real compiled event set is `main.rs`'s own job.
pub fn parse_atlas_sections(input: &str) -> Result<Vec<(String, String)>> {
    let f: AtlasSectionsFile =
        toml::from_str(input).context("atlas-sections.toml: invalid TOML or does not match the [[section]] schema")?;
    Ok(f.section.into_iter().map(|s| (s.event_id, s.atlas_section)).collect())
}

// --- Batch W1: coverage-manifest.toml (the whole-Bible coverage-manifest
// infrastructure the W series builds on) ---

#[derive(Deserialize)]
struct CoverageManifestFile {
    declared: Vec<String>,
}

/// Parses `coverage-manifest.toml` -- a flat list of canonical 3-letter book
/// codes this project claims are FULLY covered (every one of that book's
/// own verses belongs to >=1 titled container). Pure and STRUCTURAL only,
/// same split every other curated schema in this module follows -- a
/// malformed file bails immediately; checking each declared code is a real
/// canon code, has no duplicates, and is ACTUALLY fully covered against the
/// real compiled data is `server/atlas-etl/tests/coverage.rs`'s own job
/// (it needs the compiled `events.json`/`canon.json`, which this pure
/// parser deliberately does not read).
pub fn parse_coverage_manifest(input: &str) -> Result<Vec<String>> {
    let f: CoverageManifestFile =
        toml::from_str(input).context("coverage-manifest.toml: invalid TOML or does not match the 'declared' schema")?;
    Ok(f.declared)
}

#[derive(Deserialize)]
struct LandmarkToml {
    name: String,
    kind: String,
    lat: f64,
    lon: f64,
    // Batch C2: optional far-field size hint ("sm"/"md"/"lg") — see
    // atlas_core::data::Landmark::size's own doc comment. `Option<T>`
    // fields are optional-by-default under serde's derive (missing key ->
    // None) even without an explicit `#[serde(default)]`, so every
    // pre-Batch-C2 landmarks.toml entry (no `size = ...` line at all)
    // keeps parsing exactly as before.
    size: Option<String>,
}

#[derive(Deserialize)]
struct LandmarksFile {
    landmark: Vec<LandmarkToml>,
}

/// Parses `landmarks.toml` (schema: `[[landmark]]` with `name`/`kind`/
/// `lat`/`lon`/optional `size`). Pure — does not validate `kind`/`size`
/// against their allowed enums or check `lat`/`lon` against the clip bbox;
/// that's `validate::run_landmarks`'s job (needs the bbox, which this
/// module doesn't own), matching the brief's "curated::parse_landmarks
/// (pure) -> validate" pipeline.
pub fn parse_landmarks(input: &str) -> Result<Vec<Landmark>> {
    let f: LandmarksFile =
        toml::from_str(input).context("landmarks.toml: invalid TOML or does not match the [[landmark]] schema")?;
    Ok(f.landmark.into_iter().map(|l| Landmark { name: l.name, kind: l.kind, lat: l.lat, lon: l.lon, size: l.size }).collect())
}

// --- Batch E: place-history.toml -------------------------------------------

#[derive(Deserialize)]
struct PlaceHistoryFile {
    place: Vec<PlaceToml>,
}

#[derive(Deserialize)]
struct PlaceToml {
    id: String,
    #[serde(default, rename = "name")]
    names: Vec<NameToml>,
    #[serde(default, rename = "blurb")]
    blurbs: Vec<BlurbToml>,
    #[serde(default)]
    established: Option<DateClaimToml>,
    #[serde(default)]
    destroyed: Option<DateClaimToml>,
}

#[derive(Deserialize)]
struct NameToml {
    name: String,
    from: i32,
    to: i32,
    #[serde(default)]
    verses: Vec<String>,
}

#[derive(Deserialize)]
struct BlurbToml {
    text: String,
    from: i32,
    to: i32,
    breadth: String,
}

/// `year` OR `from`+`to` (the brief's "year OR from/to range") -- exactly
/// one of the two shapes is required; ambiguous (both) or empty (neither)
/// TOML tables are a curator authoring mistake, held to the same
/// hard-error-not-soft-drop bar `parse_events_extra` already applies to our
/// own hand-authored data (see this module's file header).
#[derive(Deserialize)]
struct DateClaimToml {
    year: Option<i32>,
    from: Option<i32>,
    to: Option<i32>,
    #[serde(default)]
    verses: Vec<String>,
    #[serde(default)]
    note: Option<String>,
}

fn resolve_date_claim(claim: DateClaimToml, place_id: &str, field: &str) -> Result<PlaceDateClaim> {
    let when = match (claim.year, claim.from, claim.to) {
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => {
            bail!("place '{place_id}' {field}: has BOTH 'year' and 'from'/'to' -- use exactly one shape")
        }
        (Some(y), None, None) => TimeRange::new(y, y),
        (None, Some(f), Some(t)) => TimeRange::new(f, t),
        (None, Some(_), None) | (None, None, Some(_)) => {
            bail!("place '{place_id}' {field}: 'from'/'to' range needs BOTH bounds")
        }
        (None, None, None) => {
            bail!("place '{place_id}' {field}: needs either 'year' or 'from'+'to'")
        }
    }
    .map_err(|src| anyhow::anyhow!("place '{place_id}' {field}: {src}"))?;
    Ok(PlaceDateClaim { when, verses: claim.verses, note: claim.note })
}

/// Parses `place-history.toml` (Batch E: `[[place]]` per curated place id,
/// with nested `[[place.name]]` / `[[place.blurb]]` arrays and singular
/// `[place.established]` / `[place.destroyed]` tables). Pure — bails
/// immediately (matching `parse_events_extra`'s precedent) only on
/// STRUCTURAL shape problems a `TimeRange` can catch by itself (a zero year
/// or an inverted range) or an ambiguous/empty established/destroyed table;
/// does NOT check the place id is real, that cited verses parse and exist
/// in the compiled KJV text, or that ranges within one place don't overlap
/// — those need the full merged `AtlasData` (or the compiled verse map) and
/// so are `validate::run_place_history`'s job instead, same pure-parse-then-
/// cross-validate split every other curated schema in this module follows.
pub fn parse_place_history(input: &str) -> Result<Vec<PlaceHistory>> {
    let f: PlaceHistoryFile =
        toml::from_str(input).context("place-history.toml: invalid TOML or does not match the [[place]] schema")?;

    let mut out = Vec::with_capacity(f.place.len());
    for p in f.place {
        let mut names = Vec::with_capacity(p.names.len());
        for n in p.names {
            let when = TimeRange::new(n.from, n.to).map_err(|src| {
                anyhow::anyhow!("place '{}' name '{}' (from={}, to={}): {}", p.id, n.name, n.from, n.to, src)
            })?;
            names.push(PlaceNameEntry { name: n.name, when, verses: n.verses });
        }

        let mut blurbs = Vec::with_capacity(p.blurbs.len());
        for b in p.blurbs {
            let when = TimeRange::new(b.from, b.to).map_err(|src| {
                anyhow::anyhow!("place '{}' blurb '{}...' (from={}, to={}): {}", p.id, &b.text.chars().take(24).collect::<String>(), b.from, b.to, src)
            })?;
            blurbs.push(PlaceBlurbEntry { text: b.text, when, breadth: b.breadth });
        }

        let established = p.established.map(|c| resolve_date_claim(c, &p.id, "established")).transpose()?;
        let destroyed = p.destroyed.map(|c| resolve_date_claim(c, &p.id, "destroyed")).transpose()?;

        out.push(PlaceHistory { id: p.id, names, blurbs, established, destroyed });
    }
    Ok(out)
}

// --- Batch E3: place-names-kjv.toml (KJV display-name alias layer) --------

#[derive(Deserialize)]
struct PlaceNamesKjvFile {
    alias: Vec<AliasToml>,
}

#[derive(Deserialize)]
struct AliasToml {
    id: String,
    name: String,
    #[serde(default)]
    verses: Vec<String>,
}

/// Parses `place-names-kjv.toml` (Batch E3: a FLAT `[[alias]]` array, each
/// row naming its own `id`) -- the SAME flat, no-mis-attachment-risk shape
/// `event-witnesses.toml`/`parse_event_witnesses` and `acts-sections.toml`/
/// `parse_acts_sections` already establish (see `parse_event_witnesses`'s
/// own doc comment for the TOML array-of-tables mis-attachment class this
/// avoids entirely), reused here for the identical reason: one curated fact
/// per named place id. Each row's plain `name` is wrapped into a
/// `{"kjv": name}` translation map here (same indirection
/// `parse_event_witnesses` wraps its own single-translation curator field
/// into) -- "kjv is the only key today; identity survives future
/// translations" per batch-e3-brief.md requirement 1. `verses` are passed
/// through AS-IS (plain single-verse citation ids, e.g. `place-history.toml`'s
/// own `[[place.name]]` `verses` field -- not a curator-friendly RANGE like
/// `events-extra.toml`'s own `verses`, so no `expand_verse_ref` here).
///
/// Pure and STRUCTURAL only, same split every other curated schema in this
/// module follows: a malformed file bails immediately; cross-checking `id`
/// against the real compiled place set, rejecting duplicate ids, rejecting
/// an alias equal to its own place's canonical name, and checking each
/// cited verse parses and exists in the compiled KJV text are all
/// `validate::run_place_names_kjv`'s job instead (needs the compiled places
/// list and KJV text, which this pure parse step doesn't own).
pub fn parse_place_names_kjv(input: &str) -> Result<Vec<PlaceNameAlias>> {
    let f: PlaceNamesKjvFile =
        toml::from_str(input).context("place-names-kjv.toml: invalid TOML or does not match the [[alias]] schema")?;

    Ok(f.alias
        .into_iter()
        .map(|a| PlaceNameAlias {
            id: a.id,
            translations: std::collections::HashMap::from([(atlas_core::translation::DEFAULT_TRANSLATION.to_string(), a.name)]),
            verses: a.verses,
        })
        .collect())
}

// --- Batch B2: polities/{id}.toml ("borders v2, the cartographer's edition") ---

#[derive(Deserialize)]
struct PolityToml {
    id: String,
    #[serde(rename = "era")]
    eras: Vec<PolityEraToml>,
}

#[derive(Deserialize)]
struct PolityEraToml {
    name: String,
    from: i32,
    to: i32,
    ref_note: String,
    rings: Vec<Vec<(f64, f64)>>,
    /// Batch M requirement 1: nested `[era.transition]`/`[era.fall]` tables
    /// -- TOML's own standard "subtable of the most recently opened
    /// array-of-tables element" shape, so these attach to exactly the
    /// `[[era]]` entry they're written under with no id/index matching. See
    /// `atlas_core::data::PolityEra::transition`/`::fall`'s own doc comments
    /// for what each means; both are `#[serde(default)]` so every existing
    /// era (the overwhelming majority, honestly omitted per the citation-
    /// integrity rule) keeps parsing with neither present.
    #[serde(default)]
    transition: Option<PolityDeltaToml>,
    #[serde(default)]
    fall: Option<PolityDeltaToml>,
}

#[derive(Deserialize)]
struct PolityDeltaToml {
    event: String,
    #[serde(default)]
    verses: Vec<String>,
    ref_note: String,
    /// Fix round 1 (I1): required, no `#[serde(default)]` -- a curator MUST
    /// name which era's own `from` year this block belongs to. See
    /// `atlas_core::data::PolityDelta::for_era_from`'s own doc comment for
    /// why (the actual mis-attachment bug this batch's own self-review
    /// found, and the structural check this field exists to make possible).
    for_era_from: i32,
}

/// Parses one `data/curated/polities/{id}.toml` file (schema: top-level
/// `id`, one or more `[[era]]` tables -- `name`/`from`/`to`/`ref_note`/
/// `rings`, see `atlas_core::data::PolityEra`'s own doc comment for the
/// exact field shape and why `rings` is `[lat, lon]`, not GeoJSON's
/// `[lon, lat]`). Pure and STRUCTURAL only, same split every other curated
/// schema in this module follows (`parse_landmarks`/`parse_place_history`):
/// a TOML file that doesn't even parse into this shape is a curator
/// authoring mistake, held to the same immediate-bail bar
/// `parse_place_history` already applies to hand-authored data, but zero
/// years, inverted ranges, era overlap, ring closure/simplicity, and the
/// bbox check all need the FULL picture (every era at once, geometry math) and
/// so are deliberately deferred to `validate::run_polities` instead, same
/// "parse then cross-validate, don't fail fast mid-file" reasoning as
/// `parse_place_history`'s own doc comment.
///
/// `color_key` is LEFT PROVISIONAL (`0`) here -- fix round 1 (M1): it used
/// to be computed eagerly in this function (a pure hash of `id` alone), but
/// a collision-free assignment needs to see every OTHER polity in the same
/// roster too, which a single file being parsed in isolation never has
/// visibility into (exactly the same "needs the FULL picture" reasoning the
/// doc comment above already gives for deferring the zero-year/overlap/
/// ring-closure checks to `validate::run_polities` instead of checking them
/// here). `process_polities` (`server/atlas-etl/src/main.rs`) overwrites
/// every polity's `color_key` in one pass, via
/// `polities::assign_color_keys`, once the full sorted roster this
/// function's own caller reads is available -- see that function's own doc
/// comment for the collision-free assignment algorithm itself.
pub fn parse_polity(input: &str) -> Result<Polity> {
    let f: PolityToml = toml::from_str(input).context("polity TOML: invalid TOML or does not match the id/[[era]] schema")?;
    let eras = f
        .eras
        .into_iter()
        .map(|e| PolityEra {
            name: e.name,
            from: e.from,
            to: e.to,
            ref_note: e.ref_note,
            rings: e.rings,
            transition: e.transition.map(|d| PolityDelta { event: d.event, verses: d.verses, ref_note: d.ref_note, for_era_from: d.for_era_from }),
            fall: e.fall.map(|d| PolityDelta { event: d.event, verses: d.verses, ref_note: d.ref_note, for_era_from: d.for_era_from }),
        })
        .collect();
    Ok(Polity { id: f.id, color_key: 0, eras })
}

// --- Batch R requirement 1: land-mask.toml ("borders become part of the plate") ---

#[derive(Deserialize)]
struct LandMaskFile {
    region: Vec<LandMaskRegionToml>,
}

#[derive(Deserialize)]
struct LandMaskRegionToml {
    name: String,
    ref_note: String,
    rings: Vec<Vec<(f64, f64)>>,
}

/// Parses `land-mask.toml` (schema: one or more `[[region]]` tables --
/// `name`/`ref_note`/`rings`, mirroring `PolityEra`'s own `rings` shape
/// exactly -- see `atlas_core::data::LandMaskRegion`'s own doc comment).
/// Pure and STRUCTURAL only, same split every other curated geometry schema
/// in this module follows (`parse_polity`'s own doc comment): a TOML file
/// that doesn't even parse into this shape is a curator authoring mistake,
/// held to the same immediate-bail bar; ring closure/simplicity/bbox
/// containment all need the full picture and so are deliberately deferred to
/// `validate::run_land_mask` instead.
pub fn parse_land_mask(input: &str) -> Result<Vec<LandMaskRegion>> {
    let f: LandMaskFile =
        toml::from_str(input).context("land-mask.toml: invalid TOML or does not match the [[region]] schema")?;
    Ok(f.region.into_iter().map(|r| LandMaskRegion { name: r.name, ref_note: r.ref_note, rings: r.rings }).collect())
}

// --- Batch F: catechism.toml ("the small catechism") -----------------------

#[derive(Deserialize)]
struct CatechismFile {
    part: Vec<CatechismPartToml>,
}

#[derive(Deserialize)]
struct CatechismPartToml {
    id: String,
    title: String,
    item: Vec<CatechismItemToml>,
}

#[derive(Deserialize)]
struct CatechismItemToml {
    id: String,
    name: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    explanation_heading: Option<String>,
    explanation: String,
    #[serde(default)]
    where_written: Option<String>,
    #[serde(default)]
    verses: Vec<String>,
    #[serde(default)]
    ref_note: Option<String>,
}

/// Parses `catechism.toml` (schema: `[[part]]` -- `id`/`title` -> nested
/// `[[part.item]]` -- `id`/`name`/optional `text`/optional
/// `explanation_heading`/`explanation`/optional `where_written`/`verses`/
/// optional `ref_note`, see `CatechismItem`'s own doc comment for why `text`
/// is optional and `explanation_heading` defaults). Pure and STRUCTURAL
/// only, same split every other curated schema in this module follows: a
/// TOML file that doesn't even parse into this shape is a curator authoring
/// mistake, held to the same immediate-bail bar `parse_place_history`/
/// `parse_polity` already apply to hand-authored data. `verses` accepts the
/// SAME curator-friendly single-verse-or-range strings `parse_events_extra`
/// does (reuses `expand_verse_ref` verbatim), expanded here into individual
/// canonical verse ids; a verse ref's own validity against the compiled KJV
/// text (does it parse AND actually exist) needs the full picture and so is
/// deliberately deferred to `validate::run_catechism` instead, same
/// pure-parse-then-cross-validate split `parse_place_history`'s own doc
/// comment already establishes. Duplicate part/item ids across the whole
/// file are ALSO `validate::run_catechism`'s job (needs the full roster at
/// once), not checked here.
pub fn parse_catechism(input: &str) -> Result<Vec<CatechismPart>> {
    let f: CatechismFile =
        toml::from_str(input).context("catechism.toml: invalid TOML or does not match the [[part]]/[[part.item]] schema")?;

    let mut parts = Vec::with_capacity(f.part.len());
    for p in f.part {
        let mut items = Vec::with_capacity(p.item.len());
        for it in p.item {
            let mut verses = Vec::new();
            for v in &it.verses {
                expand_verse_ref(v, &it.id, &mut verses)?;
            }
            items.push(CatechismItem {
                id: it.id,
                name: it.name,
                text: it.text,
                explanation_heading: it.explanation_heading.unwrap_or_else(|| "What does this mean?".to_string()),
                explanation: it.explanation,
                where_written: it.where_written,
                verses,
                ref_note: it.ref_note,
                // Batch F2: question-level citations are merged in SEPARATELY,
                // by main.rs (after this pure parse of catechism.toml itself),
                // from the brain-fuel/catechism mapping + the Deut5 supplement
                // -- see `merge_catechism_questions` there. Always empty here.
                questions: Vec::new(),
            });
        }
        parts.push(CatechismPart { id: p.id, title: p.title, items });
    }
    Ok(parts)
}

// --- Batch F2: catechism-mapping.toml (requirement 3) + catechism-deut5.toml (requirement 5b) ---

#[derive(Deserialize)]
struct CatechismMappingFileToml {
    file: Vec<MappingFileToml>,
}

#[derive(Deserialize)]
struct MappingFileToml {
    path: String,
    item: String,
    #[serde(default, rename = "override")]
    overrides: Vec<MappingOverrideToml>,
}

#[derive(Deserialize)]
struct MappingOverrideToml {
    item: String,
    questions: Vec<u32>,
}

/// Parses `catechism-mapping.toml` (schema: `[[file]]` -- `path`/`item`/
/// optional `[[file.override]]` -- `item`/`questions`, see that file's own
/// header comment for the full convention). Pure and STRUCTURAL only, same
/// split every other curated schema in this module follows: a TOML file
/// that doesn't even parse into this shape is a curator authoring mistake,
/// held to the same immediate-bail bar `parse_catechism`/`parse_polity`
/// already apply. Does NOT check that every named `item`/override `item`
/// actually exists in `catechism.toml`, or that every named `path` actually
/// exists under `data/raw/` -- both need the fuller picture (the compiled
/// catechism parts, and the filesystem) and so are
/// `catechism_map::merge_questions_into_parts`'s / `catechism_map::
/// build_questions_from_mapping`'s own job respectively, same
/// pure-parse-then-cross-validate split every other curated schema here
/// follows.
pub fn parse_catechism_mapping(input: &str) -> Result<Vec<MappingFile>> {
    let f: CatechismMappingFileToml =
        toml::from_str(input).context("catechism-mapping.toml: invalid TOML or does not match the [[file]] schema")?;
    Ok(f.file
        .into_iter()
        .map(|row| MappingFile {
            path: row.path,
            item: row.item,
            overrides: row.overrides.into_iter().map(|o| MappingOverride { item: o.item, questions: o.questions }).collect(),
        })
        .collect())
}

#[derive(Deserialize)]
struct CatechismDeut5File {
    entry: Vec<Deut5EntryToml>,
}

#[derive(Deserialize)]
struct Deut5EntryToml {
    item: String,
    verses: Vec<String>,
    ref_note: String,
}

/// Parses `catechism-deut5.toml` (requirement 5b; schema: `[[entry]]` --
/// `item`/`verses`/`ref_note`, all required -- see that file's own header).
/// Pure and STRUCTURAL only, same split as `parse_catechism_mapping` above:
/// verse expansion/validation against the compiled KJV text and the item-id
/// cross-check both happen later (`curated::expand_verse_ref` is reused
/// verbatim for the actual range expansion, at the SAME call site
/// `main.rs` already uses for `catechism.toml`'s own `verses` field --
/// these are OUR OWN canonical-ref strings, e.g. `"DEU.5.9-10"`, not the
/// brain-fuel repo's human-readable form, so `catechism_map::
/// canonicalize_ref` is deliberately NOT used here).
pub fn parse_catechism_deut5(input: &str) -> Result<Vec<Deut5Entry>> {
    let f: CatechismDeut5File =
        toml::from_str(input).context("catechism-deut5.toml: invalid TOML or does not match the [[entry]] schema")?;
    Ok(f.entry.into_iter().map(|e| Deut5Entry { item: e.item, verses: e.verses, ref_note: e.ref_note }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_landmarks_reads_valid_toml() {
        let landmarks = parse_landmarks(include_str!("../tests/fixtures/landmarks-sample.toml")).unwrap();
        assert_eq!(landmarks.len(), 3);
        let jordan = landmarks.iter().find(|l| l.name == "Jordan River").unwrap();
        assert_eq!(jordan.kind, "water");
        assert_eq!(jordan.lat, 31.76);
        assert_eq!(jordan.lon, 35.55);
        assert_eq!(jordan.size, None); // Batch C2: no `size` line in the fixture -> None, not an error

        // Batch C2: the Negev entry DOES carry a `size = "lg"` line -- proves
        // parse_landmarks actually threads the optional field through
        // (rather than silently dropping it), the real bug a naive
        // `Landmark { ..., size: None }` literal in the map closure would
        // have caused with zero compile-time signal.
        let negev = landmarks.iter().find(|l| l.name == "Negev").unwrap();
        assert_eq!(negev.size.as_deref(), Some("lg"));
    }

    // --- Batch B2: parse_polity ---------------------------------------------

    #[test]
    fn parse_polity_reads_valid_toml_and_computes_color_key() {
        let polity = parse_polity(include_str!("../tests/fixtures/polities-sample.toml")).unwrap();
        assert_eq!(polity.id, "testland");
        assert_eq!(polity.eras.len(), 2);
        assert_eq!(polity.eras[0].name, "Testland");
        assert_eq!(polity.eras[0].from, -2000);
        assert_eq!(polity.eras[0].to, -1500);
        assert_eq!(polity.eras[0].rings.len(), 1);
        assert_eq!(polity.eras[0].rings[0].len(), 5);
        assert_eq!(polity.eras[0].rings[0][0], (10.0, 10.0), "rings are [lat, lon], first pair verbatim");
        assert_eq!(polity.eras[1].name, "Greater Testland");
        assert!(polity.eras[0].transition.is_none(), "first era's own [era.transition] is honestly absent in the fixture");
        assert!(polity.eras[0].fall.is_none());

        // Batch M requirement 1: [era.transition]/[era.fall] under the
        // fixture's SECOND [[era]] -- proves both nested tables attach to
        // the era they're written under (TOML's own array-of-tables
        // subtable rule), not the first/wrong one.
        let transition = polity.eras[1].transition.as_ref().expect("second era carries a transition in the fixture");
        assert_eq!(transition.event, "Testland expands");
        assert_eq!(transition.verses, vec!["GEN.1.1".to_string()]);
        assert_eq!(transition.ref_note, "synthetic fixture, not a real citation");
        assert_eq!(transition.for_era_from, -1499, "fix round 1 (I1): echoes the SAME era's own from it's actually attached to");

        let fall = polity.eras[1].fall.as_ref().expect("second era carries a fall in the fixture");
        assert_eq!(fall.event, "Greater Testland falls");
        assert_eq!(fall.verses, vec!["GEN.1.2".to_string(), "GEN.1.3".to_string()]);
        assert_eq!(fall.for_era_from, -1499);

        // Fix round 1 (M1): color_key is now LEFT PROVISIONAL (0) by
        // parse_polity -- a single file has no visibility into the rest of
        // the roster it might collide with, so the real, collision-free
        // value is assigned later by `process_polities` (main.rs), over the
        // FULL sorted roster at once, via `polities::assign_color_keys`
        // (see that function's own unit tests for the collision-free
        // assignment algorithm itself). This just proves parse_polity
        // doesn't reach for the old per-file hash anymore.
        assert_eq!(polity.color_key, 0);
    }

    #[test]
    fn parse_polity_rejects_malformed_toml() {
        assert!(parse_polity("not = [valid").is_err());
        assert!(parse_polity("id = \"x\"").is_err(), "missing [[era]] array entirely");
    }

    // --- Batch R requirement 1: parse_land_mask -----------------------------

    #[test]
    fn parse_land_mask_reads_valid_toml() {
        let toml = r#"
[[region]]
name = "Testland coast"
ref_note = "test fixture, not a real coastline"
rings = [
  [[10.0, 10.0], [10.0, 20.0], [20.0, 20.0], [20.0, 10.0], [10.0, 10.0]]
]

[[region]]
name = "Testisle"
ref_note = "test fixture"
rings = [
  [[30.0, 30.0], [30.0, 32.0], [32.0, 31.0], [30.0, 30.0]]
]
"#;
        let regions = parse_land_mask(toml).unwrap();
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].name, "Testland coast");
        assert_eq!(regions[0].rings.len(), 1);
        assert_eq!(regions[0].rings[0][0], (10.0, 10.0), "rings are [lat, lon], first pair verbatim");
        assert_eq!(regions[1].name, "Testisle");
    }

    #[test]
    fn parse_land_mask_rejects_malformed_toml() {
        assert!(parse_land_mask("not = [valid").is_err());
        assert!(parse_land_mask("foo = 1").is_err(), "missing [[region]] array entirely");
    }

    // --- Batch F: parse_catechism --------------------------------------------

    #[test]
    fn parse_catechism_reads_valid_toml_with_defaults_applied() {
        let toml = r#"
[[part]]
id = "ten-commandments"
title = "The Ten Commandments"

  [[part.item]]
  id = "commandment-1"
  name = "The First Commandment"
  text = "Thou shalt have no other gods."
  explanation = "We should fear, love, and trust in God above all things."

  [[part.item]]
  id = "commandments-close"
  name = "What Does God Say of All These Commandments?"
  text = "I the LORD thy God am a jealous God..."
  explanation = "God threatens to punish all that transgress these commandments."
  verses = ["EXO.20.5-6"]
  ref_note = "f. read as covering v.6"

[[part]]
id = "baptism"
title = "The Sacrament of Holy Baptism"

  [[part.item]]
  id = "baptism-1"
  name = "Baptism — Part the First"
  explanation_heading = "What is Baptism?"
  explanation = "Baptism is not simple water only..."
  where_written = "Christ, our Lord, says..."
  verses = ["MAT.28.19"]
"#;
        let parts = parse_catechism(toml).unwrap();
        assert_eq!(parts.len(), 2);

        let commandments = &parts[0];
        assert_eq!(commandments.id, "ten-commandments");
        assert_eq!(commandments.title, "The Ten Commandments");
        assert_eq!(commandments.items.len(), 2);

        let first = &commandments.items[0];
        assert_eq!(first.id, "commandment-1");
        assert_eq!(first.text.as_deref(), Some("Thou shalt have no other gods."));
        // Default applied: no explanation_heading line in the TOML above.
        assert_eq!(first.explanation_heading, "What does this mean?");
        assert_eq!(first.where_written, None);
        assert!(first.verses.is_empty());
        assert_eq!(first.ref_note, None);

        let close = &commandments.items[1];
        // "EXO.20.5-6" expands to two individual canonical verse ids, same
        // range-expansion expand_verse_ref already gives parse_events_extra.
        assert_eq!(close.verses, vec!["EXO.20.5".to_string(), "EXO.20.6".to_string()]);
        assert_eq!(close.ref_note.as_deref(), Some("f. read as covering v.6"));

        let baptism = &parts[1];
        let b1 = &baptism.items[0];
        assert_eq!(b1.text, None, "Baptism items have no separate prompt text -- see CatechismItem's own doc comment");
        // Explicit override, NOT the default.
        assert_eq!(b1.explanation_heading, "What is Baptism?");
        assert_eq!(b1.where_written.as_deref(), Some("Christ, our Lord, says..."));
        assert_eq!(b1.verses, vec!["MAT.28.19".to_string()]);
    }

    #[test]
    fn parse_catechism_rejects_malformed_toml() {
        assert!(parse_catechism("not = [valid").is_err());
        assert!(parse_catechism("id = \"x\"").is_err(), "missing [[part]] array entirely");
    }

    #[test]
    fn parse_catechism_rejects_an_unparseable_verse_ref() {
        let toml = r#"
[[part]]
id = "p"
title = "P"
  [[part.item]]
  id = "i1"
  name = "N"
  explanation = "E"
  verses = ["not-a-ref"]
"#;
        let err = parse_catechism(toml).unwrap_err();
        assert!(err.to_string().contains("i1"), "{err}");
    }

    // --- Batch F2: parse_catechism_mapping / parse_catechism_deut5 ---------

    #[test]
    fn parse_catechism_mapping_reads_valid_toml_with_and_without_overrides() {
        let toml = r#"
[[file]]
path = "resources/02.1-The-First-Commandment.yaml"
item = "commandment-1"

[[file]]
path = "resources/05.2.1-Confession-and-Absolution.yaml"
item = "confession-1"

  [[file.override]]
  item = "confession-2"
  questions = [6, 7, 9]
"#;
        let rows = parse_catechism_mapping(toml).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].path, "resources/02.1-The-First-Commandment.yaml");
        assert_eq!(rows[0].item, "commandment-1");
        assert!(rows[0].overrides.is_empty());
        assert_eq!(rows[1].overrides.len(), 1);
        assert_eq!(rows[1].overrides[0].item, "confession-2");
        assert_eq!(rows[1].overrides[0].questions, vec![6, 7, 9]);
    }

    #[test]
    fn parse_catechism_mapping_rejects_malformed_toml() {
        assert!(parse_catechism_mapping("not = [valid").is_err());
        assert!(parse_catechism_mapping("foo = 1").is_err(), "missing [[file]] array entirely");
    }

    #[test]
    fn parse_catechism_deut5_reads_valid_toml() {
        let toml = r#"
[[entry]]
item = "commandment-1"
verses = ["DEU.5.7"]
ref_note = "test note"

[[entry]]
item = "commandments-close"
verses = ["DEU.5.9-10"]
ref_note = "another note"
"#;
        let entries = parse_catechism_deut5(toml).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].item, "commandment-1");
        assert_eq!(entries[0].verses, vec!["DEU.5.7".to_string()]);
        assert_eq!(entries[0].ref_note, "test note");
        assert_eq!(entries[1].verses, vec!["DEU.5.9-10".to_string()]);
    }

    #[test]
    fn parse_catechism_deut5_rejects_malformed_toml() {
        assert!(parse_catechism_deut5("not = [valid").is_err());
        assert!(parse_catechism_deut5("item = \"x\"").is_err(), "missing [[entry]] array entirely");
    }

    #[test]
    fn expand_verse_ref_handles_single_and_range() {
        let mut out = Vec::new();
        expand_verse_ref("EXO.12.37", "e1", &mut out).unwrap();
        assert_eq!(out, vec!["EXO.12.37".to_string()]);

        let mut out2 = Vec::new();
        expand_verse_ref("GEN.12.14-20", "e2", &mut out2).unwrap();
        assert_eq!(out2.len(), 7);
        assert_eq!(out2.first().unwrap(), "GEN.12.14");
        assert_eq!(out2.last().unwrap(), "GEN.12.20");
    }

    // --- Batch T requirement 1: parse_event_witnesses -----------------------

    #[test]
    fn parse_event_witnesses_reads_a_flat_witness_list_with_translation_indirection() {
        let toml = r#"
[[witness]]
event_id = "pw_golgotha"
book = "MAT"
verses = ["MAT.27.33-50"]
ref_note = "Matthew 27:33-50 read directly"
robertson_section = "Robertson (1922) §164"

[[witness]]
event_id = "pw_golgotha"
book = "JHN"
verses = ["JHN.19.17-30"]
"#;
        let rows = parse_event_witnesses(toml).unwrap();
        assert_eq!(rows.len(), 2);

        let (event_id, mat) = &rows[0];
        assert_eq!(event_id, "pw_golgotha");
        assert_eq!(mat.book, "MAT");
        // "kjv" translation-mapped, expanded from the curator-friendly range
        // string the SAME way events-extra.toml's own `verses` field is
        // (expand_verse_ref, reused verbatim).
        let kjv = mat.translations.get("kjv").expect("kjv translation must be populated");
        assert_eq!(kjv.len(), 18); // MAT.27.33..=50
        assert_eq!(kjv.first().unwrap(), "MAT.27.33");
        assert_eq!(kjv.last().unwrap(), "MAT.27.50");
        assert_eq!(mat.ref_note.as_deref(), Some("Matthew 27:33-50 read directly"));
        assert_eq!(mat.robertson_section.as_deref(), Some("Robertson (1922) §164"));

        let (event_id2, jhn) = &rows[1];
        assert_eq!(event_id2, "pw_golgotha");
        assert_eq!(jhn.book, "JHN");
        assert_eq!(jhn.ref_note, None); // optional field, genuinely absent here
    }

    #[test]
    fn parse_event_witnesses_rejects_malformed_toml() {
        assert!(parse_event_witnesses("not = [valid").is_err());
        assert!(parse_event_witnesses("foo = 1").is_err(), "missing [[witness]] array entirely");
    }

    #[test]
    fn parse_event_witnesses_rejects_an_unparseable_verse_ref() {
        let toml = r#"
[[witness]]
event_id = "e1"
book = "MAT"
verses = ["not-a-ref"]
"#;
        let err = parse_event_witnesses(toml).unwrap_err();
        assert!(err.to_string().contains("e1"), "{err}");
    }

    // --- Batch E3: parse_place_names_kjv -------------------------------------

    #[test]
    fn parse_place_names_kjv_reads_a_flat_alias_list_with_translation_indirection() {
        let toml = r#"
[[alias]]
id = "cush-2"
name = "Ethiopia"
verses = ["GEN.2.13"]

[[alias]]
id = "tigris"
name = "Hiddekel"
verses = ["GEN.2.14", "DAN.10.4"]
"#;
        let rows = parse_place_names_kjv(toml).unwrap();
        assert_eq!(rows.len(), 2);

        assert_eq!(rows[0].id, "cush-2");
        // "kjv" translation-mapped, same indirection parse_event_witnesses
        // wraps its own single-translation curator field into.
        assert_eq!(rows[0].translations.get("kjv").map(String::as_str), Some("Ethiopia"));
        assert_eq!(rows[0].verses, vec!["GEN.2.13".to_string()]);

        assert_eq!(rows[1].id, "tigris");
        assert_eq!(rows[1].translations.get("kjv").map(String::as_str), Some("Hiddekel"));
        assert_eq!(rows[1].verses, vec!["GEN.2.14".to_string(), "DAN.10.4".to_string()]);
    }

    #[test]
    fn parse_place_names_kjv_rejects_malformed_toml() {
        assert!(parse_place_names_kjv("not = [valid").is_err());
        assert!(parse_place_names_kjv("foo = 1").is_err(), "missing [[alias]] array entirely");
    }
}
