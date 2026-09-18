//! Batch RED-1, decision 4 ("Compiled data (NOT graph)"): the KJV
//! sub-verse span table -- CHAR offsets, not byte offsets (module doc
//! comment below has the why), into OUR canonical KJV verse text, keyed by
//! dot-ref. DB-5: it is compiled into the kjv section's `red_letter_span`
//! table (`sqlite::extras`) straight from memory by `bins/compile_graph.rs`
//! and read back by `sqlite::serve::load_red_letter_spans`; the former
//! `data/compiled/red-letter-spans.json` sidecar is gone. The
//! `--build-from-raw` dev fallback recomputes it from the corpus.
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


use atlas_etl::red_letter::RedLetterCorpus;

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
