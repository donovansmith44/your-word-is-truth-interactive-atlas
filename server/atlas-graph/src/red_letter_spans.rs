//! Batch RED-1, decision 4 ("Compiled data (NOT graph)"): the KJV
//! sub-verse span table -- CHAR offsets, not byte offsets (module doc
//! comment below has the why), into OUR canonical KJV verse text, keyed by
//! dot-ref, shipping in `data/compiled/red-letter-spans.json` --
//! compiled-data-side,
//! never the graph artifact (`artifact.rs`'s own `FORMAT_VERSION` doc
//! comment: "the span table is compiled-data-side, not artifact"). Written
//! once, by `bins/compile_graph.rs` (the SAME binary that already writes
//! `data/exports/*.json` as a terminal pass, decision 4's own precedent);
//! loaded once, at `GraphService` construction, on EITHER real path
//! (`from_artifact`: reads the file; `build`'s own `--build-from-raw` dev
//! fallback: recomputed directly from a freshly-parsed+aligned corpus, no
//! file needed at all -- raw bytes are already in hand on that path).
//!
//! CHAR OFFSETS, NOT UTF-8 BYTE OFFSETS, disclosed: `red_letter.rs`'s own
//! alignment produces Rust byte offsets (natural for `&str` slicing
//! server-side). The CLIENT is a C#/Blazor app, and C# `string` indexing is
//! UTF-16 CODE UNITS -- identical to a byte offset only for pure ASCII text.
//! Real KJV text occasionally carries a non-ASCII character (an en dash in
//! a compound Hebrew name, e.g.) BEFORE a red span, which would silently
//! misalign a raw byte offset against the client's own string index. Every
//! character in this app's own KJV text is within the Basic Multilingual
//! Plane (confirmed: no astral/surrogate-pair characters anywhere in the
//! compiled text), so ONE Unicode scalar == ONE UTF-16 code unit here,
//! always -- converting byte offsets to CHAR COUNTS (`char_offset` below)
//! at this exact serialization boundary is what keeps a client-side
//! `Text.Substring(start, end - start)` correct even on a verse whose text
//! isn't pure ASCII.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use atlas_etl::red_letter::RedLetterCorpus;

pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RedLetterSpansFile {
    format_version: u32,
    /// dot-ref -> `[start_char, end_char]` pairs, ascending, non-overlapping
    /// (the SAME order `red_letter::RedLetterVerse.spans` already
    /// guarantees -- this is a pure re-keying + byte-to-char conversion,
    /// never a re-sort).
    spans: BTreeMap<String, Vec<(usize, usize)>>,
}

/// Converts a BYTE offset into `s` to a CHAR (Unicode scalar) offset --
/// module doc comment has the full "why" for the client-side UTF-16
/// alignment this exists to guarantee. `O(n)` in the byte offset's own
/// position, which is fine here: called at most a handful of times per
/// verse, over verse-length strings, once, at compile time.
fn char_offset(s: &str, byte_offset: usize) -> usize {
    s.char_indices().take_while(|(b, _)| *b < byte_offset).count()
}

/// Builds the dot-ref -> char-offset-span map straight off a parsed
/// `RedLetterCorpus`, against the SAME canonical verse text the corpus was
/// itself aligned against (needed here only for the byte->char conversion,
/// never re-aligned). Verses whose OWN alignment fully failed (`spans` is
/// empty -- `red_letter.rs`'s own doc comment: the verse still belongs to
/// the SpokenBy verse SET, just contributes no sub-verse row) are excluded
/// from this map entirely, honestly -- a verse absent here renders NO red
/// in the KJV sub-verse surface, the real, small, disclosed residual
/// (batch-red1-report.md has the exact count).
pub fn spans_by_dot_ref(corpus: &RedLetterCorpus, kjv_verses: &std::collections::HashMap<String, String>) -> BTreeMap<String, Vec<(usize, usize)>> {
    let mut out = BTreeMap::new();
    for v in &corpus.verses {
        if v.spans.is_empty() {
            continue;
        }
        let dot_ref = format!("{}.{}.{}", atlas_core::canon::BOOKS[v.book_index as usize].code, v.chapter, v.verse);
        let Some(text) = kjv_verses.get(&dot_ref) else { continue };
        let char_spans: Vec<(usize, usize)> = v.spans.iter().map(|&(s, e)| (char_offset(text, s), char_offset(text, e))).collect();
        out.insert(dot_ref, char_spans);
    }
    out
}

pub fn write_file(path: &Path, spans: &BTreeMap<String, Vec<(usize, usize)>>) -> Result<()> {
    let file = RedLetterSpansFile { format_version: FORMAT_VERSION, spans: spans.clone() };
    let json = serde_json::to_string_pretty(&file).context("serializing red-letter-spans.json")?;
    std::fs::write(path, format!("{json}\n")).with_context(|| format!("writing {}", path.display()))
}

/// `None` when the file simply doesn't exist (an older `data/compiled/`
/// snapshot that predates this batch, or a test fixture directory) -- the
/// SAME graceful-absence treatment `service.rs`'s own `load_kretzmann`/
/// `load_concord`/`load_brainfuel` already establish; `Some(Err(..))`
/// propagated fail-loud when it exists but is malformed.
pub fn read_file(path: &Path) -> Result<Option<BTreeMap<String, Vec<(usize, usize)>>>> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let file: RedLetterSpansFile = serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    if file.format_version != FORMAT_VERSION {
        anyhow::bail!("{} carries format_version {} (this build understands {FORMAT_VERSION})", path.display(), file.format_version);
    }
    Ok(Some(file.spans))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_offset_matches_byte_offset_for_pure_ascii() {
        let s = "Follow me, and I will make you fishers of men.";
        assert_eq!(char_offset(s, 0), 0);
        assert_eq!(char_offset(s, 7), 7);
        assert_eq!(char_offset(s, s.len()), s.chars().count());
    }

    #[test]
    fn char_offset_diverges_from_byte_offset_after_a_multibyte_character() {
        // En dash (U+2013) is 3 UTF-8 bytes but ONE UTF-16 code unit --
        // the exact case this module's own doc comment names.
        let s = "Bar\u{2013}jona said this.";
        // Byte offset of "said" (after "Bar" (3) + en-dash (3 bytes) + "jona" (4) + " " (1) = 11 bytes).
        let byte_of_said = s.find("said").unwrap();
        assert_eq!(byte_of_said, 3 + 3 + 4 + 1);
        // Char offset must be 3 (Bar) + 1 (en dash, ONE char) + 4 (jona) + 1 (space) = 9.
        assert_eq!(char_offset(s, byte_of_said), 9);
    }

    #[test]
    fn spans_by_dot_ref_excludes_verses_with_zero_aligned_spans() {
        use atlas_etl::red_letter::{RedLetterCorpus, RedLetterVerse};
        let corpus = RedLetterCorpus {
            verses: vec![
                RedLetterVerse { book_index: 39, chapter: 4, verse: 19, spans: vec![(24, 70)] },
                RedLetterVerse { book_index: 39, chapter: 5, verse: 10, spans: vec![] }, // alignment totally failed
            ],
            stats: Default::default(),
        };
        let verses: std::collections::HashMap<String, String> =
            [("MAT.4.19".to_string(), "And he saith unto them, Follow me, and I will make you fishers of men.".to_string()), ("MAT.5.10".to_string(), "irrelevant".to_string())].into_iter().collect();
        let out = spans_by_dot_ref(&corpus, &verses);
        assert!(out.contains_key("MAT.4.19"));
        assert!(!out.contains_key("MAT.5.10"), "a verse with zero aligned spans must not enter the compiled table at all");
    }

    #[test]
    fn write_then_read_round_trips_losslessly() {
        let dir = std::env::temp_dir().join(format!("red1-spans-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("red-letter-spans.json");
        let mut spans = BTreeMap::new();
        spans.insert("MAT.4.19".to_string(), vec![(24, 70)]);
        spans.insert("MAT.5.4".to_string(), vec![(0, 15)]);
        write_file(&path, &spans).unwrap();
        let read_back = read_file(&path).unwrap().expect("file must exist");
        assert_eq!(read_back, spans);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_file_returns_none_for_a_missing_file_not_an_error() {
        let missing = std::env::temp_dir().join("this-file-really-should-not-exist-red1.json");
        assert!(read_file(&missing).unwrap().is_none());
    }
}
