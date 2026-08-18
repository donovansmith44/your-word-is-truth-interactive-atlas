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
use atlas_core::data::{BookMeta, Era, Event, Narrative};
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

#[cfg(test)]
mod tests {
    use super::*;

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
