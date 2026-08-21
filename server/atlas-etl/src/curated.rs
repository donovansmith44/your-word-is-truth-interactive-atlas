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
use atlas_core::data::{BookMeta, CatechismItem, CatechismPart, Era, Event, Landmark, LandMaskRegion, Narrative, PlaceBlurbEntry, PlaceDateClaim, PlaceHistory, PlaceNameEntry, Polity, PolityDelta, PolityEra};
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
    from_year: i32,
    to_year: i32,
    places: Vec<String>,
    #[serde(default)]
    verses: Vec<String>,
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
pub fn parse_events_extra(input: &str) -> Result<Vec<Event>> {
    let f: EventsFile =
        toml::from_str(input).context("events-extra.toml: invalid TOML or does not match the [[event]] schema")?;

    let mut out = Vec::with_capacity(f.event.len());
    for e in f.event {
        if e.places.is_empty() {
            bail!("curated event '{}' has no places (places[0] is required as the narrative-arrow anchor)", e.id);
        }
        let when = TimeRange::new(e.from_year, e.to_year).map_err(|src| {
            anyhow::anyhow!("curated event '{}' (from_year={}, to_year={}): {}", e.id, e.from_year, e.to_year, src)
        })?;
        let mut verses = Vec::new();
        for v in &e.verses {
            expand_verse_ref(v, &e.id, &mut verses)?;
        }
        out.push(Event { id: e.id, label: e.label, when, places: e.places, verses });
    }
    Ok(out)
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
}
