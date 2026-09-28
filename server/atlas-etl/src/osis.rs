//! Parsing OSIS-style `Book.Chapter.Verse` references, as the raw geocoding, Theographic and
//! cross-reference sources all spell them. Their book abbreviations do not match atlas-core's codes
//! even case-insensitively, so the book is resolved through `canon::resolve_alias`, which matches code,
//! osis or name, and the `VerseId` is built by hand.

use atlas_core::canon::resolve_alias;
use atlas_core::refs::VerseId;

/// Returns `None`, not an error, for anything that is not exactly that shape: the caller decides
/// whether a non-match is a hard error or a droppable row.
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

/// Canonical string form of a `VerseId`, in our 3-letter code rather than the input's OSIS
/// abbreviation.
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
        assert!(parse_verse("Gen.1").is_none());
        assert!(parse_verse("Gen.1.1.1").is_none());
        assert!(parse_verse("Zzz.1.1").is_none());
        assert!(parse_verse("Gen.0.1").is_none());
        assert!(parse_verse("Gen.1.0").is_none());
    }
}
