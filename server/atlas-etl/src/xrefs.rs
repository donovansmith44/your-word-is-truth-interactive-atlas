//! Parser for the openbible.info cross-references TSV. Line 1 is a 4-field header while every data row
//! has exactly 3, so it is skipped unconditionally rather than inferring columns from it. A target may
//! be a same-chapter range, canonicalized to `COL.1.16-17`, or a wider one, `MAT.5.3-MAT.6.2`.

use std::collections::HashMap;

use anyhow::Result;
use atlas_core::data::CrossRef;
use atlas_core::refs::{ScriptureRef, VerseId};

use crate::osis;

#[derive(Debug, Clone, Default)]
pub struct XrefStats {
    /// Rows dropped for structural reasons: a wrong field count, a `From`/`To` that is not a canon verse
    /// or span, or a non-integer vote count.
    pub dropped_unparseable: usize,
    /// Rows dropped because `To` (as a single verse) was identical to `From`.
    pub dropped_self: usize,
}

/// Tries the whole string as one verse first, the common case, then falls back to splitting on the first
/// `-` and parsing both halves as OSIS verses.
fn parse_to_span(raw: &str) -> Option<(String, VerseId, bool)> {
    if let Some(v) = osis::parse_verse(raw) {
        let canon = osis::canonical(&v);
        return Some((canon, v, true));
    }
    let (left, right) = raw.split_once('-')?;
    let lv = osis::parse_verse(left)?;
    let rv = osis::parse_verse(right)?;
    let canon = if lv.book == rv.book && lv.chapter == rv.chapter {
        format!("{}.{}.{}-{}", lv.book.code(), lv.chapter, lv.verse, rv.verse)
    } else {
        format!("{}-{}", osis::canonical(&lv), osis::canonical(&rv))
    };
    Some((canon, lv, false))
}

/// Each `From` verse's cross-refs come back sorted by votes descending. Self-references and structurally
/// unparseable rows are dropped and counted, never silently ignored.
pub fn parse(input: &str) -> Result<(HashMap<String, Vec<CrossRef>>, XrefStats)> {
    let mut map: HashMap<String, Vec<CrossRef>> = HashMap::new();
    let mut stats = XrefStats::default();

    for line in input.lines().skip(1) {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 3 {
            stats.dropped_unparseable += 1;
            continue;
        }
        let (from_s, to_s, votes_s) = (fields[0], fields[1], fields[2]);

        let Some(from_v) = osis::parse_verse(from_s) else {
            stats.dropped_unparseable += 1;
            continue;
        };
        let Ok(votes) = votes_s.trim().parse::<i32>() else {
            stats.dropped_unparseable += 1;
            continue;
        };
        let Some((target, _first_verse, is_single)) = parse_to_span(to_s) else {
            stats.dropped_unparseable += 1;
            continue;
        };

        let from_canon = osis::canonical(&from_v);
        if is_single && target == from_canon {
            stats.dropped_self += 1;
            continue;
        }

        map.entry(from_canon).or_default().push(CrossRef { target, votes });
    }

    for refs in map.values_mut() {
        refs.sort_by(|a, b| b.votes.cmp(&a.votes));
    }

    Ok((map, stats))
}

/// Extracts the first verse of an already-canonicalized target, in any of its three forms.
fn first_verse_of_target(target: &str) -> Option<VerseId> {
    if let Ok(v) = VerseId::parse_canonical(target) {
        return Some(v);
    }
    if let Ok(ScriptureRef::Passage(atlas_core::identity::PassageReference { book, chapter, from_verse, .. })) = ScriptureRef::parse(target) {
        return Some(VerseId { book, chapter, verse: from_verse });
    }
    let (left, _right) = target.split_once('-')?;
    VerseId::parse_canonical(left).ok()
}

/// A second pure pass, standalone so it stays unit-testable without I/O: drops any cross-ref whose
/// target's first verse is not a key in the compiled verses map, since the preview needs that verse's
/// text, and drops the `From` entry entirely when nothing survives.
pub fn filter_missing_first_verse(
    map: HashMap<String, Vec<CrossRef>>,
    verses: &HashMap<String, String>,
) -> (HashMap<String, Vec<CrossRef>>, usize) {
    let mut dropped = 0usize;
    let mut out: HashMap<String, Vec<CrossRef>> = HashMap::with_capacity(map.len());
    for (from, refs) in map {
        let kept: Vec<CrossRef> = refs
            .into_iter()
            .filter(|c| {
                let exists = first_verse_of_target(&c.target).is_some_and(|v| verses.contains_key(&osis::canonical(&v)));
                if !exists {
                    dropped += 1;
                }
                exists
            })
            .collect();
        if !kept.is_empty() {
            out.insert(from, kept);
        }
    }
    (out, dropped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_verse_of_target_handles_all_three_shapes() {
        assert_eq!(first_verse_of_target("PSA.124.8").unwrap().verse, 8);
        assert_eq!(first_verse_of_target("COL.1.16-19").unwrap().verse, 16);
        assert_eq!(first_verse_of_target("MAT.5.3-MAT.6.2").unwrap().verse, 3);
        assert!(first_verse_of_target("garbage").is_none());
    }
}
