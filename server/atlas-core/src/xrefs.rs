//! A cross-reference target is always one of three canonical shapes: a single verse
//! (`PSA.124.8`), a same-chapter span (`COL.1.16-19`), or a cross-chapter span
//! (`MAT.5.3-MAT.6.2`).

use std::collections::HashMap;

use crate::data::CrossRef;
use crate::identity::CrossReferenceTarget;
use crate::refs::{ScriptureRef, VerseId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregatedXref {
    pub target: CrossReferenceTarget,
    pub votes: i32,
    pub preview: String,
}

const MAX_RESULTS: usize = 20;

/// Ascending. A book or chapter ref has no defined member-verse list, so it yields an
/// empty vec rather than panicking and this function stays total.
pub fn span_member_verses(span: &ScriptureRef) -> Vec<VerseId> {
    match *span {
        ScriptureRef::Verse(v) => vec![v],
        ScriptureRef::Passage(passage) => (passage.from_verse..=passage.to_verse).map(|verse| VerseId { book: passage.book, chapter: passage.chapter, verse }).collect(),
        ScriptureRef::Book(_) | ScriptureRef::Chapter(..) => Vec::new(),
    }
}

fn target_span(target: &str) -> Option<(VerseId, VerseId)> {
    target.parse::<CrossReferenceTarget>().ok().map(|target| target.first_and_last())
}

/// A SUBSET check, not a first-verse-only one: a target that starts inside the span
/// but reaches past its far end still points somewhere the span does not cover, so it
/// survives. An unparseable target is never considered inside the span.
fn target_within_span(target: &str, span: &ScriptureRef) -> bool {
    let Some((first, last)) = target_span(target) else { return false };
    let (book, chapter, from_verse, to_verse) = match *span {
        ScriptureRef::Verse(v) => (v.book, v.chapter, v.verse, v.verse),
        ScriptureRef::Passage(passage) => (passage.book, passage.chapter, passage.from_verse, passage.to_verse),
        ScriptureRef::Book(_) | ScriptureRef::Chapter(..) => return false,
    };
    first.book == book
        && first.chapter == chapter
        && last.book == book
        && last.chapter == chapter
        && first.verse >= from_verse
        && last.verse <= to_verse
}

/// Ties are broken by each target's first-seen position, so the sort by votes is
/// deterministic rather than dependent on `HashMap` iteration order.
pub fn aggregate_span_xrefs(
    span: &ScriptureRef,
    cross_refs: &HashMap<String, Vec<CrossRef>>,
    verse_text: impl Fn(&str) -> Option<String>,
) -> Vec<AggregatedXref> {
    let mut votes_by_target: HashMap<String, i32> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for member in span_member_verses(span) {
        let key = format!("{}.{}.{}", member.book.code(), member.chapter, member.verse);
        let Some(refs) = cross_refs.get(&key) else { continue };
        for cr in refs {
            if target_within_span(&cr.target, span) {
                continue;
            }
            match votes_by_target.get_mut(&cr.target) {
                Some(v) => *v += cr.votes,
                None => {
                    votes_by_target.insert(cr.target.clone(), cr.votes);
                    order.push(cr.target.clone());
                }
            }
        }
    }

    let mut out: Vec<AggregatedXref> = order
        .into_iter()
        .filter_map(|target| {
            let votes = *votes_by_target.get(&target)?;
            let target: CrossReferenceTarget = target.parse().ok()?;
            let (first, _) = target.first_and_last();
            let preview = verse_text(&first.to_string())?;
            Some(AggregatedXref { target, votes, preview })
        })
        .collect();

    out.sort_by(|a, b| b.votes.cmp(&a.votes));
    out.truncate(MAX_RESULTS);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn book(code: &str) -> crate::refs::BookId {
        crate::canon::resolve_alias(code).unwrap()
    }

    fn verse(code: &str, chapter: u16, v: u16) -> VerseId {
        VerseId { book: book(code), chapter, verse: v }
    }

    fn cr(target: &str, votes: i32) -> CrossRef {
        CrossRef { target: target.into(), votes }
    }

    fn fixture_cross_refs() -> HashMap<String, Vec<CrossRef>> {
        let mut m = HashMap::new();
        m.insert("GEN.1.1".to_string(), vec![cr("JHN.1.1", 10), cr("GEN.1.3", 7), cr("PSA.33.6", 3)]);
        m.insert("GEN.1.2".to_string(), vec![cr("JHN.1.1", 4), cr("ISA.45.18", 2)]);
        m.insert("GEN.1.3".to_string(), vec![cr("2CO.4.6", 6), cr("GEN.1.1-2", 5)]);
        m.insert("GEN.1.4".to_string(), vec![cr("1JN.1.5", 1)]);
        m.insert("GEN.1.5".to_string(), vec![cr("PSA.74.16", 8), cr("GEN.1.3-10", 9)]);
        m
    }

    fn fixture_verses() -> HashMap<String, String> {
        let mut v = HashMap::new();
        for (k, text) in [
            ("GEN.1.1", "In the beginning God created the heaven and the earth."),
            ("GEN.1.2", "And the earth was without form, and void."),
            ("GEN.1.3", "And God said, Let there be light: and there was light."),
            ("JHN.1.1", "In the beginning was the Word."),
            ("PSA.33.6", "By the word of the LORD were the heavens made."),
            ("ISA.45.18", "I am the LORD; and there is none else."),
            ("2CO.4.6", "God, who commanded the light to shine out of darkness."),
            ("1JN.1.5", "God is light, and in him is no darkness at all."),
            ("PSA.74.16", "The day is thine, the night also is thine."),
        ] {
            v.insert(k.to_string(), text.to_string());
        }
        v
    }

    fn gen_1_span(from_verse: u16, to_verse: u16) -> ScriptureRef {
        if from_verse == to_verse {
            ScriptureRef::Verse(verse("GEN", 1, from_verse))
        } else {
            ScriptureRef::Passage(crate::identity::PassageReference { book: book("GEN"), chapter: 1, from_verse, to_verse })
        }
    }

    #[test]
    fn span_member_verses_covers_verse_and_passage() {
        assert_eq!(span_member_verses(&gen_1_span(3, 3)), vec![verse("GEN", 1, 3)]);
        assert_eq!(
            span_member_verses(&gen_1_span(1, 3)),
            vec![verse("GEN", 1, 1), verse("GEN", 1, 2), verse("GEN", 1, 3)]
        );
        assert!(span_member_verses(&ScriptureRef::Book(book("GEN"))).is_empty());
        assert!(span_member_verses(&ScriptureRef::Chapter(crate::identity::ChapterReference { book: book("GEN"), chapter: 1 })).is_empty());
    }

    #[test]
    fn target_span_handles_all_three_canonical_shapes() {
        assert_eq!(target_span("PSA.124.8"), Some((verse("PSA", 124, 8), verse("PSA", 124, 8))));
        assert_eq!(target_span("COL.1.16-19"), Some((verse("COL", 1, 16), verse("COL", 1, 19))));
        assert_eq!(target_span("MAT.5.3-MAT.6.2"), Some((verse("MAT", 5, 3), verse("MAT", 6, 2))));
        assert_eq!(target_span("garbage"), None);
    }

    #[test]
    fn votes_sum_across_member_verses_not_just_the_first_hit() {
        let out = aggregate_span_xrefs(&gen_1_span(1, 5), &fixture_cross_refs(), |k| fixture_verses().get(k).cloned());
        let jhn = out.iter().find(|x| x.target.to_string() == "JHN.1.1").expect("JHN.1.1 must survive aggregation");
        assert_eq!(jhn.votes, 14, "10 (from GEN.1.1) + 4 (from GEN.1.2)");
    }

    #[test]
    fn self_targets_are_dropped_but_a_target_merely_starting_inside_the_span_survives() {
        let out = aggregate_span_xrefs(&gen_1_span(1, 5), &fixture_cross_refs(), |k| fixture_verses().get(k).cloned());
        let targets: Vec<String> = out.iter().map(|x| x.target.to_string()).collect();
        assert!(!targets.contains(&"GEN.1.3".to_string()), "{targets:?}");
        assert!(!targets.contains(&"GEN.1.1-2".to_string()), "{targets:?}");
        assert!(targets.contains(&"GEN.1.3-10".to_string()), "{targets:?}");
    }

    #[test]
    fn sorted_votes_descending_and_preview_is_targets_own_first_verse_text() {
        let out = aggregate_span_xrefs(&gen_1_span(1, 5), &fixture_cross_refs(), |k| fixture_verses().get(k).cloned());
        for pair in out.windows(2) {
            assert!(pair[0].votes >= pair[1].votes, "{out:?}");
        }
        let jhn = out.iter().find(|x| x.target.to_string() == "JHN.1.1").unwrap();
        assert_eq!(jhn.preview, "In the beginning was the Word.");
    }

    #[test]
    fn single_verse_span_keeps_targets_that_only_look_self_against_a_wider_span() {
        let out = aggregate_span_xrefs(&gen_1_span(1, 1), &fixture_cross_refs(), |k| fixture_verses().get(k).cloned());
        let targets: Vec<String> = out.iter().map(|x| x.target.to_string()).collect();
        assert_eq!(targets, vec!["JHN.1.1", "GEN.1.3", "PSA.33.6"]);
    }

    #[test]
    fn missing_member_verse_in_cross_refs_map_is_a_graceful_no_contribution() {
        let out = aggregate_span_xrefs(&gen_1_span(4, 6), &fixture_cross_refs(), |k| fixture_verses().get(k).cloned());
        let targets: Vec<String> = out.iter().map(|x| x.target.to_string()).collect();
        assert_eq!(targets, vec!["GEN.1.3-10", "PSA.74.16", "1JN.1.5"]);
    }

    #[test]
    fn cap_at_20_keeps_the_top_20_by_votes() {
        let mut cross_refs = HashMap::new();
        let refs: Vec<CrossRef> = (1..=25).map(|i| CrossRef { target: format!("PSA.{i}.1"), votes: i }).collect();
        cross_refs.insert("GEN.1.1".to_string(), refs);
        let mut verses = HashMap::new();
        for i in 1..=25 {
            verses.insert(format!("PSA.{i}.1"), format!("verse {i}"));
        }

        let out = aggregate_span_xrefs(&gen_1_span(1, 1), &cross_refs, |k| verses.get(k).cloned());
        assert_eq!(out.len(), 20);
        assert_eq!(out[0].target.to_string(), "PSA.25.1");
        assert_eq!(out[19].target.to_string(), "PSA.6.1");
    }

    #[test]
    fn determinism_repeated_calls_yield_the_same_result() {
        let cross_refs = fixture_cross_refs();
        let verses = fixture_verses();
        let a = aggregate_span_xrefs(&gen_1_span(1, 5), &cross_refs, |k| verses.get(k).cloned());
        let b = aggregate_span_xrefs(&gen_1_span(1, 5), &cross_refs, |k| verses.get(k).cloned());
        assert_eq!(a, b);
    }

    fn span_strategy() -> impl Strategy<Value = ScriptureRef> {
        let g = book("GEN");
        (1u16..=6, 0u16..=6).prop_map(move |(a, b)| {
            if b == 0 || b == a {
                ScriptureRef::Verse(VerseId { book: g, chapter: 1, verse: a })
            } else {
                let (lo, hi) = (a.min(b), a.max(b));
                ScriptureRef::Passage(crate::identity::PassageReference { book: g, chapter: 1, from_verse: lo, to_verse: hi })
            }
        })
    }

    proptest! {
        #[test]
        fn xref_2_span_aggregation(span in span_strategy()) {
            let cross_refs = fixture_cross_refs();
            let verses = fixture_verses();
            let result = aggregate_span_xrefs(&span, &cross_refs, |k| verses.get(k).cloned());

            prop_assert!(result.len() <= MAX_RESULTS);

            for x in &result {
                prop_assert!(target_span(&x.target.to_string()).is_some(), "unparseable target {}", x.target);
                prop_assert!(!target_within_span(&x.target.to_string(), &span), "target {} should have been dropped (subset of span)", x.target);
            }

            for pair in result.windows(2) {
                prop_assert!(pair[0].votes >= pair[1].votes);
            }

            let members = span_member_verses(&span);
            for x in &result {
                let seen_somewhere = members.iter().any(|m| {
                    let key = format!("{}.{}.{}", m.book.code(), m.chapter, m.verse);
                    cross_refs.get(&key).is_some_and(|v| v.iter().any(|c| c.target == x.target.to_string()))
                });
                prop_assert!(seen_somewhere, "target {} not present in any member's own xref list", x.target);
            }
        }
    }
}
