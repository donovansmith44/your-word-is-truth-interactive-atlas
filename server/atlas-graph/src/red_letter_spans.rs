//! The KJV sub-verse span table, keyed by dot-ref, in CHAR offsets rather than UTF-8 byte offsets:
//! the client indexes strings as UTF-16 code units, and every character in this text is inside the
//! BMP, so one scalar is one code unit and a char count is what a client-side substring needs.

use std::collections::BTreeMap;

use atlas_etl::red_letter::RedLetterCorpus;

fn char_offset(s: &str, byte_offset: usize) -> usize {
    s.char_indices().take_while(|(b, _)| *b < byte_offset).count()
}

/// Verses whose own alignment failed entirely are excluded from the map, honestly: a verse absent
/// here renders no red at all.
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
