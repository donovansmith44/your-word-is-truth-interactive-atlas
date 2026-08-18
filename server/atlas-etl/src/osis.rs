//! Shared helper for parsing OSIS-style `Book.Chapter.Verse` references, as
//! used by the OpenBible geocoding bundle (`ancient.jsonl`'s `verses[].osis`),
//! Theographic's `verses.json` (`fields.osisRef`), and the openbible.info
//! cross-references TSV (`Gen.1.1` style).
//!
//! These raw datasets use OSIS book abbreviations (`Gen`, `1Sam`, `2Kgs`, ...)
//! which do NOT always match atlas-core's 3-letter `code`s
//! (`GEN`, `1SA`, `2KI`, ...) even case-insensitively, so
//! `atlas_core::refs::ScriptureRef::parse` (which only matches `.code`)
//! cannot be used directly on raw osis text. `canon::resolve_alias` matches
//! code OR osis OR name, so we resolve the book that way and build the
//! `VerseId` by hand.

use atlas_core::canon::resolve_alias;
use atlas_core::refs::VerseId;

/// Parses a strict `Book.Chapter.Verse` OSIS reference (e.g. `"2Kgs.5.12"`)
/// into a `VerseId`. Returns `None` (not an error) on anything that isn't
/// exactly that shape — callers decide whether a non-match is a hard error
/// or a droppable row.
pub fn parse_verse(s: &str) -> Option<VerseId> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let book = resolve_alias(parts[0])?;
    let chapter: u16 = parts[1].parse().ok()?;
    let verse: u16 = parts[2].parse().ok()?;
    if chapter == 0 || verse == 0 {
        return None;
    }
    Some(VerseId { book, chapter, verse })
}

/// Canonical string form of a `VerseId`, e.g. `"2KI.5.12"` (our 3-letter code,
/// not the input's OSIS abbreviation).
pub fn canonical(v: &VerseId) -> String {
    format!("{}.{}.{}", v.book.code(), v.chapter, v.verse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_osis_abbreviation_to_our_code() {
        let v = parse_verse("2Kgs.5.12").unwrap();
        assert_eq!(v.book.code(), "2KI");
        assert_eq!(canonical(&v), "2KI.5.12");
    }

    #[test]
    fn rejects_bad_shapes() {
        assert!(parse_verse("Gen.1").is_none()); // not verse-granularity
        assert!(parse_verse("Gen.1.1.1").is_none());
        assert!(parse_verse("Zzz.1.1").is_none()); // unknown book
        assert!(parse_verse("Gen.0.1").is_none()); // chapter 0
        assert!(parse_verse("Gen.1.0").is_none()); // verse 0
    }
}
