//! `GET /api/xrefs/{sref}`'s aggregation core (batch-g1-brief.md requirement
//! 2, "passage context... passages give xrefs, not just geo"). Pure, no
//! `AtlasData` dependency (mirrors `crate::history`'s own narrow-slice-in,
//! narrow-value-out shape) -- `atlas_server::handlers::xrefs` is the thin
//! wrapper that parses the path param and hands this module `&data.cross_refs`/
//! `&data.verses` directly, per this crate's own "no business logic in
//! handlers.rs" rule.
//!
//! Reuses the loaded cross-ref data verbatim -- no new data source, no ETL
//! change. A cross-ref TARGET string is always one of the three canonical
//! shapes `atlas-etl::xrefs::parse_to_span` produces (mirrored here, same as
//! `atlas-server::handlers::first_verse_of_target` already duplicates for
//! the SAME reason documented there: this crate must not depend on the
//! ETL-only ATLAS-etl crate): a single verse (`"PSA.124.8"`), a same-chapter
//! span (`"COL.1.16-19"`), or a cross-chapter/book span (`"MAT.5.3-MAT.6.2"`).

use std::collections::HashMap;

use crate::data::CrossRef;
use crate::refs::{ScriptureRef, VerseId};

/// One cross-reference target aggregated across every member verse of a
/// queried span, carrying its own preview text (the same shape
/// `atlas-server::handlers::CrossRefOut` already serializes for
/// `/api/verse/{vref}`'s own per-verse list).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregatedXref {
    pub target: String,
    pub votes: i32,
    pub preview: String,
}

const MAX_RESULTS: usize = 20;

/// The verses making up `span`, in ascending order: a single verse for
/// `ScriptureRef::Verse`, `from_verse..=to_verse` (same book/chapter) for
/// `ScriptureRef::Passage`. `Book`/`Chapter` refs have no defined member-verse
/// list for this endpoint (`handlers::xrefs` 400s those as `bad_ref` before
/// this function is ever called -- see its own doc comment for why the
/// brief's own two examples, `GEN.1.1`/`GEN.1.1-5`, are read as an exhaustive
/// pair, not a representative sample); returns an empty vec for them rather
/// than panicking, so this function stays total.
pub fn span_member_verses(span: &ScriptureRef) -> Vec<VerseId> {
    match *span {
        ScriptureRef::Verse(v) => vec![v],
        ScriptureRef::Passage { book, chapter, from_verse, to_verse } => {
            (from_verse..=to_verse).map(|verse| VerseId { book, chapter, verse }).collect()
        }
        ScriptureRef::Book(_) | ScriptureRef::Chapter { .. } => Vec::new(),
    }
}

/// Parses an already-canonicalized cross-ref TARGET string into its own
/// (first, last) verse endpoints. Mirrors
/// `atlas-server::handlers::first_verse_of_target`'s three-shape parse, just
/// keeping the LAST verse too (needed for the subset check below) instead of
/// discarding it.
fn target_span(target: &str) -> Option<(VerseId, VerseId)> {
    if let Ok(v) = VerseId::parse_canonical(target) {
        return Some((v, v));
    }
    if let Ok(ScriptureRef::Passage { book, chapter, from_verse, to_verse }) = ScriptureRef::parse(target) {
        return Some((VerseId { book, chapter, verse: from_verse }, VerseId { book, chapter, verse: to_verse }));
    }
    let (left, right) = target.split_once('-')?;
    let lv = VerseId::parse_canonical(left).ok()?;
    let rv = VerseId::parse_canonical(right).ok()?;
    Some((lv, rv))
}

/// True when `target`'s own verse range is entirely contained within
/// `span`'s -- a "self-target" per the brief ("self-targets (targets inside
/// the span) dropped"): pointing back inside the very passage being explored
/// is not useful context, so it never survives aggregation. This is
/// deliberately a SUBSET check (both of the target's own endpoints must fall
/// inside the span), not a first-verse-only check -- XREF-2's own property is
/// "no target ⊆ the span": a target that starts inside the span but reaches
/// past its far end (e.g. target `GEN.1.3-10` against span `GEN.1.1-5`) is
/// still useful context pointing somewhere the span DOESN'T already cover,
/// so it is kept, not dropped. An unparseable target is never considered
/// inside the span (fails toward "keep" -- harmless either way here, since
/// such a target can never reach the final output anyway: building its
/// preview below re-parses it and drops it for good via the same failure).
fn target_within_span(target: &str, span: &ScriptureRef) -> bool {
    let Some((first, last)) = target_span(target) else { return false };
    let (book, chapter, from_verse, to_verse) = match *span {
        ScriptureRef::Verse(v) => (v.book, v.chapter, v.verse, v.verse),
        ScriptureRef::Passage { book, chapter, from_verse, to_verse } => (book, chapter, from_verse, to_verse),
        ScriptureRef::Book(_) | ScriptureRef::Chapter { .. } => return false,
    };
    first.book == book
        && first.chapter == chapter
        && last.book == book
        && last.chapter == chapter
        && first.verse >= from_verse
        && last.verse <= to_verse
}

/// The aggregation itself (batch-g1-brief.md requirement 2, verbatim):
/// "union of member verses' xrefs, per-target votes SUMMED, self-targets
/// (targets inside the span) dropped, sorted votes desc, capped at 20, each
/// with the existing preview text."
///
/// `order` tracks each target's first-seen position (iterating member verses
/// in span order, each verse's own xref list in its stored -- already
/// votes-descending, per `atlas-etl::xrefs::parse`'s own sort -- order) so
/// the final sort-by-votes is stable and deterministic on ties, rather than
/// depending on `HashMap`'s unspecified iteration order.
pub fn aggregate_span_xrefs(
    span: &ScriptureRef,
    cross_refs: &HashMap<String, Vec<CrossRef>>,
    verses: &HashMap<String, String>,
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
            // Preview fails soft (same ruling-4 policy `handlers::verse`'s own
            // cross-ref preview already follows): a target whose own first
            // verse is unexpectedly missing from `verses` is skipped, not
            // panicked on.
            let (first, _) = target_span(&target)?;
            let preview_key = format!("{}.{}.{}", first.book.code(), first.chapter, first.verse);
            let preview = verses.get(&preview_key)?;
            Some(AggregatedXref { target, votes, preview: preview.clone() })
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

    // A small, hand-built cross-ref fixture over GEN.1.1..GEN.1.5, richer
    // than demo_fixture()'s single-entry one -- deliberately covers every
    // XREF-2 case in one shot: an overlapping target across two member
    // verses (JHN.1.1, to prove SUMMING), a same-chapter self-target fully
    // inside the span (GEN.1.3, GEN.1.1-2), a target that starts inside the
    // span but reaches past its far end (GEN.1.3-10, to prove the subset --
    // not merely intersects -- reading of "self-target"), and ordinary
    // outside-span targets (JHN.1.1, PSA.33.6, ISA.45.18, 2CO.4.6, 1JN.1.5,
    // PSA.74.16).
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
            ScriptureRef::Passage { book: book("GEN"), chapter: 1, from_verse, to_verse }
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
        assert!(span_member_verses(&ScriptureRef::Chapter { book: book("GEN"), chapter: 1 }).is_empty());
    }

    #[test]
    fn target_span_handles_all_three_canonical_shapes() {
        assert_eq!(target_span("PSA.124.8"), Some((verse("PSA", 124, 8), verse("PSA", 124, 8))));
        assert_eq!(target_span("COL.1.16-19"), Some((verse("COL", 1, 16), verse("COL", 1, 19))));
        assert_eq!(target_span("MAT.5.3-MAT.6.2"), Some((verse("MAT", 5, 3), verse("MAT", 6, 2))));
        assert_eq!(target_span("garbage"), None);
    }

    // The hand-verifiable "sum property spot-check" the brief calls for:
    // JHN.1.1 is cited by BOTH GEN.1.1 (10) and GEN.1.2 (4) -- its aggregated
    // vote count over span GEN.1.1-5 must be exactly their sum, 14, not just
    // "some positive number" or the max of the two.
    #[test]
    fn votes_sum_across_member_verses_not_just_the_first_hit() {
        let out = aggregate_span_xrefs(&gen_1_span(1, 5), &fixture_cross_refs(), &fixture_verses());
        let jhn = out.iter().find(|x| x.target == "JHN.1.1").expect("JHN.1.1 must survive aggregation");
        assert_eq!(jhn.votes, 14, "10 (from GEN.1.1) + 4 (from GEN.1.2)");
    }

    #[test]
    fn self_targets_are_dropped_but_a_target_merely_starting_inside_the_span_survives() {
        let out = aggregate_span_xrefs(&gen_1_span(1, 5), &fixture_cross_refs(), &fixture_verses());
        let targets: Vec<&str> = out.iter().map(|x| x.target.as_str()).collect();
        // GEN.1.3 (exactly one member verse) and GEN.1.1-2 (wholly inside
        // the span) are both self-targets -- dropped.
        assert!(!targets.contains(&"GEN.1.3"), "{targets:?}");
        assert!(!targets.contains(&"GEN.1.1-2"), "{targets:?}");
        // GEN.1.3-10 starts inside the span but reaches to verse 10, past
        // the span's own to_verse=5 -- NOT a subset, so it survives (this is
        // XREF-2's own "no target ⊆ the span" wording, not a first-verse-
        // only check).
        assert!(targets.contains(&"GEN.1.3-10"), "{targets:?}");
    }

    #[test]
    fn sorted_votes_descending_and_preview_is_targets_own_first_verse_text() {
        let out = aggregate_span_xrefs(&gen_1_span(1, 5), &fixture_cross_refs(), &fixture_verses());
        for pair in out.windows(2) {
            assert!(pair[0].votes >= pair[1].votes, "{out:?}");
        }
        let jhn = out.iter().find(|x| x.target == "JHN.1.1").unwrap();
        assert_eq!(jhn.preview, "In the beginning was the Word.");
    }

    #[test]
    fn single_verse_span_keeps_targets_that_only_look_self_against_a_wider_span() {
        // GEN.1.1 alone (span [1,1]): its own raw list is [JHN.1.1:10,
        // GEN.1.3:7, PSA.33.6:3]. GEN.1.3 is a self-target against the WIDER
        // GEN.1.1-5 span (verse 3 falls inside [1,5]) but is NOT one here --
        // verse 3 is outside this span's own [1,1] -- so all three survive.
        // Pinning this distinction is the point of the test: "self" is
        // relative to the QUERIED span, not to whichever verse happens to
        // cite the target.
        let out = aggregate_span_xrefs(&gen_1_span(1, 1), &fixture_cross_refs(), &fixture_verses());
        let targets: Vec<&str> = out.iter().map(|x| x.target.as_str()).collect();
        assert_eq!(targets, vec!["JHN.1.1", "GEN.1.3", "PSA.33.6"]);
    }

    #[test]
    fn missing_member_verse_in_cross_refs_map_is_a_graceful_no_contribution() {
        // Span GEN.1.4-6: GEN.1.6 has no entry in the fixture cross_refs map
        // at all (contributes nothing, no crash); GEN.1.4/GEN.1.5 (both
        // real members) still contribute normally -- GEN.1.3-10 survives
        // (starts at verse 3, before this span's own from_verse=4, so it is
        // not a subset) alongside PSA.74.16 and 1JN.1.5.
        let out = aggregate_span_xrefs(&gen_1_span(4, 6), &fixture_cross_refs(), &fixture_verses());
        let targets: Vec<&str> = out.iter().map(|x| x.target.as_str()).collect();
        assert_eq!(targets, vec!["GEN.1.3-10", "PSA.74.16", "1JN.1.5"]); // votes 9, 8, 1
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

        let out = aggregate_span_xrefs(&gen_1_span(1, 1), &cross_refs, &verses);
        assert_eq!(out.len(), 20);
        assert_eq!(out[0].target, "PSA.25.1"); // highest votes (25) first
        assert_eq!(out[19].target, "PSA.6.1"); // 25 down to 6 is exactly the top 20
    }

    #[test]
    fn determinism_repeated_calls_yield_the_same_result() {
        let cross_refs = fixture_cross_refs();
        let verses = fixture_verses();
        let a = aggregate_span_xrefs(&gen_1_span(1, 5), &cross_refs, &verses);
        let b = aggregate_span_xrefs(&gen_1_span(1, 5), &cross_refs, &verses);
        assert_eq!(a, b);
    }

    fn span_strategy() -> impl Strategy<Value = ScriptureRef> {
        let g = book("GEN");
        (1u16..=6, 0u16..=6).prop_map(move |(a, b)| {
            if b == 0 || b == a {
                ScriptureRef::Verse(VerseId { book: g, chapter: 1, verse: a })
            } else {
                let (lo, hi) = (a.min(b), a.max(b));
                ScriptureRef::Passage { book: g, chapter: 1, from_verse: lo, to_verse: hi }
            }
        })
    }

    // XREF-2 (batch-g1-brief.md requirement 2, verbatim): "for random valid
    // spans, every returned target parses, no target ⊆ the span, votes
    // non-increasing, and every target appears in at least one member
    // verse's own xref list (sum property spot-checkable)".
    proptest! {
        #[test]
        fn xref_2_span_aggregation(span in span_strategy()) {
            let cross_refs = fixture_cross_refs();
            let verses = fixture_verses();
            let result = aggregate_span_xrefs(&span, &cross_refs, &verses);

            prop_assert!(result.len() <= MAX_RESULTS);

            for x in &result {
                // every returned target parses
                prop_assert!(target_span(&x.target).is_some(), "unparseable target {}", x.target);
                // no target subset of the span
                prop_assert!(!target_within_span(&x.target, &span), "target {} should have been dropped (subset of span)", x.target);
            }

            // votes non-increasing
            for pair in result.windows(2) {
                prop_assert!(pair[0].votes >= pair[1].votes);
            }

            // every returned target appears in at least one member verse's
            // own (raw, pre-aggregation) xref list
            let members = span_member_verses(&span);
            for x in &result {
                let seen_somewhere = members.iter().any(|m| {
                    let key = format!("{}.{}.{}", m.book.code(), m.chapter, m.verse);
                    cross_refs.get(&key).is_some_and(|v| v.iter().any(|c| c.target == x.target))
                });
                prop_assert!(seen_somewhere, "target {} not present in any member's own xref list", x.target);
            }
        }
    }
}
