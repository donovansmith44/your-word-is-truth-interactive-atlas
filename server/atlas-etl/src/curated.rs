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
use atlas_core::data::{BookMeta, Era, Event, Landmark, Narrative, PlaceBlurbEntry, PlaceDateClaim, PlaceHistory, PlaceNameEntry, Polity, PolityEra};
use atlas_core::refs::ScriptureRef;
use atlas_core::time::TimeRange;
use serde::Deserialize;

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
fn expand_verse_ref(raw: &str, event_id: &str, out: &mut Vec<String>) -> Result<()> {
    match ScriptureRef::parse(raw) {
        Ok(ScriptureRef::Verse(v)) => out.push(format!("{}.{}.{}", v.book.code(), v.chapter, v.verse)),
        Ok(ScriptureRef::Passage { book, chapter, from_verse, to_verse }) => {
            for verse in from_verse..=to_verse {
                out.push(format!("{}.{}.{}", book.code(), chapter, verse));
            }
        }
        _ => bail!("curated event '{event_id}' has an unparseable verse ref '{raw}' (expected e.g. 'EXO.12.37' or 'EXO.14.21-31')"),
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
        .map(|e| PolityEra { name: e.name, from: e.from, to: e.to, ref_note: e.ref_note, rings: e.rings })
        .collect();
    Ok(Polity { id: f.id, color_key: 0, eras })
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
