//! Parser for the scrollmapper `KJV.json` public-domain text (saved locally
//! as `data/raw/kjv.json`; see `data/raw/README.md`). Single JSON object
//! (not array) shaped `{ translation, books: [{ name, chapters: [{ chapter,
//! verses: [{ verse, text }] }] }] }`.
//!
//! This dataset's book names use old-style forms ("I Samuel", "II Kings",
//! "Revelation of John") rather than the OSIS/USX abbreviations the other
//! datasets use, so `canon::resolve_alias` won't match them directly. We
//! normalize Roman-numeral prefixes to Arabic digits and strip the
//! "of John" suffix before resolving.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use atlas_core::canon::resolve_alias;
use atlas_core::data::{Canon, CanonBook};
use serde::Deserialize;

#[derive(Deserialize)]
struct RawKjv {
    books: Vec<RawBook>,
}

#[derive(Deserialize)]
struct RawBook {
    name: String,
    chapters: Vec<RawChapter>,
}

#[derive(Deserialize)]
struct RawChapter {
    chapter: u16,
    verses: Vec<RawVerse>,
}

#[derive(Deserialize)]
struct RawVerse {
    verse: u16,
    text: String,
}

/// Normalizes this dataset's book-name spelling into a form
/// `canon::resolve_alias` can match: `"I Samuel"` -> `"1 Samuel"`,
/// `"II Kings"` -> `"2 Kings"`, `"III John"` -> `"3 John"`,
/// `"Revelation of John"` -> `"Revelation"`. Everything else passes through
/// unchanged (it already matches a canonical name, e.g. `"Genesis"`,
/// `"Song of Solomon"`).
/// `pub(crate)`, not private: Batch CORP-1a's `brainfuel` module reuses this
/// EXACT normalization for `data/raw/brain-fuel-bible/data/books.json`'s own
/// `kjv_name` field, which uses the IDENTICAL old-style convention this
/// dataset does ("I Samuel", "Revelation of John", ...) -- verified against
/// the real vendored file (see `brainfuel.rs`'s own module doc comment). One
/// normalizer, not two independently-authored copies that could drift.
pub(crate) fn normalize_book_name(raw: &str) -> String {
    let s = raw.trim();
    let s = s.strip_suffix(" of John").unwrap_or(s);
    if let Some(rest) = s.strip_prefix("III ") {
        format!("3 {rest}")
    } else if let Some(rest) = s.strip_prefix("II ") {
        format!("2 {rest}")
    } else if let Some(rest) = s.strip_prefix("I ") {
        format!("1 {rest}")
    } else {
        s.to_string()
    }
}

/// Parses `kjv.json` into the compiled `Canon` (book codes/names + verse
/// counts per chapter, in canonical `canon::BOOKS` order) and a canonical
/// `"BOOK.CH.V" -> text` verse map. Hard-errors if any book name fails to
/// resolve even after normalization.
pub fn parse(input: &str) -> Result<(Canon, HashMap<String, String>)> {
    let raw: RawKjv = serde_json::from_str(input).context("kjv.json is not valid JSON")?;

    let mut books: Vec<(u8, CanonBook)> = Vec::with_capacity(raw.books.len());
    let mut verses: HashMap<String, String> = HashMap::new();

    for book in &raw.books {
        let normalized = normalize_book_name(&book.name);
        let book_id = match resolve_alias(&normalized) {
            Some(id) => id,
            None => bail!(
                "kjv.json book name '{}' (normalized to '{}') does not resolve to any known canon book",
                book.name,
                normalized
            ),
        };
        let code = book_id.code();

        let mut chapter_verse_counts: Vec<u16> = Vec::with_capacity(book.chapters.len());
        for chapter in &book.chapters {
            chapter_verse_counts.push(chapter.verses.len() as u16);
            for verse in &chapter.verses {
                let key = format!("{code}.{}.{}", chapter.chapter, verse.verse);
                verses.insert(key, verse.text.clone());
            }
        }

        books.push((
            book_id.0,
            CanonBook { code: code.to_string(), name: book_id.name().to_string(), chapters: chapter_verse_counts },
        ));
    }

    // Order by canonical BookId index regardless of input file order (the
    // real kjv.json already happens to be in this order, but don't rely on it).
    books.sort_by_key(|(idx, _)| *idx);
    let canon = Canon { books: books.into_iter().map(|(_, b)| b).collect() };

    Ok((canon, verses))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_roman_numerals_and_of_john_suffix() {
        assert_eq!(normalize_book_name("I Samuel"), "1 Samuel");
        assert_eq!(normalize_book_name("II Kings"), "2 Kings");
        assert_eq!(normalize_book_name("III John"), "3 John");
        assert_eq!(normalize_book_name("Revelation of John"), "Revelation");
        assert_eq!(normalize_book_name("Genesis"), "Genesis");
        assert_eq!(normalize_book_name("Song of Solomon"), "Song of Solomon");
    }
}
