//! M-C2 requirement 1: the decisive-title law re-homed as a graph query
//! (`atlas_graph::heading::build_heading_index`) -- red-then-green over
//! the batch's own two NAMED real-data cases (batch-mc2-brief.md), each
//! already independently pinned by an atlas-core unit test
//! (`AtlasData::heading_collision_tests`, `server/atlas-core/src/data.rs`)
//! and/or a live Playwright regression (`tests/ux/w3-passages.spec.ts`) --
//! this file proves the SAME two outcomes hold when computed straight off
//! the graph's own `Event` nodes, never `AtlasData`.
//!
//! JHN.12.1 (CONTRACT.md's own DECISIVE-CONTAINER MODEL example): `jm_bethany`
//! (a bare `jesus-ministry` narrative leg, heading-worthy only via the
//! "existing-title freebie" rule -- layer 0) and `pw_bethany` (a REAL
//! curated container, this batch's own flagship 3-witness passion-week leg
//! -- layer 1) both anchor JHN.12.1; the LAYER tier must decide the
//! collision outright, `pw_bethany` winning.
//!
//! PSA.53(.1) (`data/curated/passages/psalms.toml`'s own header note, Batch
//! W3's "Psalm 14 <-> Psalm 53" parallel): `psa_014` carries TWO witnesses
//! (self PSA.14 + parallel PSA.53), so its own heading-anchor set spans
//! BOTH psalms' own opening verses under ONE shared, both-true title --
//! proves multi-witness anchoring (one event, N heading anchors) survives
//! the re-homing, not just single-anchor collision resolution.

use std::collections::BTreeSet;
use std::path::Path;

use atlas_graph::build::build_graph_from_sources_with_eras;
use atlas_graph::heading::build_heading_index;
use atlas_graph_types::id::NodeKind;

// M-D3 (owner ruling R1 propagation): `build_heading_index` now takes the
// resolved-placement companion too (`NodePayload::Event` dropped its own
// from_year/order_key mirror -- see `heading::build_heading_index`'s own
// doc comment), so `real_graph` returns the chronology derivation
// alongside the graph rather than discarding it via `..`.
fn real_graph() -> (atlas_graph_types::graph::Graph, std::collections::HashMap<String, atlas_graph_types::chrono::ResolvedPlacement>) {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let raw_dir = data_dir.join("raw");
    let curated_dir = data_dir.join("curated");

    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    // M-C2 DELETION EVENT: `AtlasData::load`'s own five retiring-file
    // reads (places/events/narratives/verses-kjv/cross-refs.json) return
    // empty now -- `atlas_etl::compile::compile` is the real-data source
    // for this test's own two named cases (both need real curated
    // events/witnesses).
    let out = atlas_etl::compile::compile(&raw_dir, &curated_dir).expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify");
    let atlas = out.data;
    let eras = atlas.eras.clone();

    let (graph, _stats, _ews, chrono) = build_graph_from_sources_with_eras(&kjv_json, &xrefs_tsv, &atlas, &eras).expect("the real committed sources must build");
    (graph, chrono.resolved)
}

#[test]
fn named_case_jhn_12_1_the_real_container_beats_the_freebie() {
    let (graph, resolved) = real_graph();
    let index = build_heading_index(&graph, &resolved);

    let heading = index.get("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
    assert_eq!(heading.event_id, "pw_bethany", "the real, witness-bearing container must win over the bare narrative-leg freebie (jm_bethany)");
    assert_ne!(heading.event_id, "jm_bethany", "the freebie must not win the collision");
}

#[test]
fn named_case_psa_53_1_the_shared_fool_incipit_container_anchors_both_psalms() {
    let (graph, resolved) = real_graph();
    let index = build_heading_index(&graph, &resolved);

    let psa53 = index.get("PSA.53.1").expect("PSA.53.1 must anchor a heading -- psa_014's own second (parallel) witness");
    assert_eq!(psa53.event_id, "psa_014");
    assert_eq!(psa53.title, "The fool hath said in his heart, There is no God.");
    assert_eq!(psa53.kind, "general");

    // The SAME container also anchors its own psalm's own verse 1 -- one
    // event, two heading anchors (multi-witness anchoring, not a
    // single-anchor coincidence).
    let psa14 = index.get("PSA.14.1").expect("PSA.14.1 must anchor a heading -- psa_014's own first (self) witness");
    assert_eq!(psa14.event_id, "psa_014");
    assert_eq!(psa14.title, psa53.title, "one shared container, one shared title, true at both anchors (fix round 1, batch-w3-review.md Important-1)");
}

// -----------------------------------------------------------------------
// M-D1 requirement 1 (owner live report #2, 2026-08-21, verbatim: "genesis
// 6 the first verses have no container label... i'm assuming this isn't an
// isolated case"): canonically-first anchoring + chapter-boundary
// continuation + an every-covered-chapter assertion, all against the real
// compiled graph.
// -----------------------------------------------------------------------

/// RED before this batch: `theo-32` ("God decides to destroy every living
/// thing," GEN.6.1-7) has no curated `witnesses` -- its own original
/// Theographic verse link was GEN.6.7, and Batch W1's own enrichment pass
/// APPENDED 6.1-6 afterward, so the OLD `heading_anchors_for` (first verse
/// in CURATED/IMPORTED array order) anchored at GEN.6.7, leaving GEN.6.1-6
/// unlabeled -- exactly the owner's own live report. GREEN after: the
/// container's own CANONICALLY FIRST covered verse, GEN.6.1, is the anchor.
#[test]
fn named_case_gen_6_1_anchors_canonically_first_not_curated_import_order() {
    let (graph, resolved) = real_graph();
    let index = build_heading_index(&graph, &resolved);

    let heading = index.get("GEN.6.1").expect("GEN.6.1 must anchor a heading -- theo-32's own canonically first covered verse (M-D1 req 1, owner live report #2)");
    assert_eq!(heading.event_id, "theo-32");
    assert!(!heading.continuation, "GEN.6.1 is theo-32's own TRUE first-covered verse -- a PRIMARY anchor, not a continuation");

    // GEN.6.7 (the pre-fix anchor) must NOT still claim the heading --
    // theo-32's own real anchor moved to 6.1, so 6.7 is un-anchored by
    // theo-32 (it may still be covered by some OTHER container's own
    // continuation or a different primary anchor; the only hard law here
    // is that theo-32 itself no longer claims 6.7 as ITS anchor).
    if let Some(still_there) = index.get("GEN.6.7") {
        assert_ne!(still_there.event_id, "theo-32", "theo-32's own anchor must have moved off GEN.6.7 to the canonically first verse, GEN.6.1");
    }
}

/// RED before this batch: chapter-boundary continuation headings did not
/// exist at all -- a container spanning multiple chapters (e.g.
/// `ezr_temple_completed`, `data/curated/passages/ezra.toml`, whose own
/// `verses` are `["EZR.5.1-17", "EZR.6.1-22"]`) anchored ONLY at its own
/// true first verse, EZR.5.1; EZR.6.1 -- a covered chapter's own opening
/// verse, mid-container -- rendered NO heading at all. GREEN after: EZR.6.1
/// carries a CONTINUATION heading for the SAME container.
#[test]
fn named_case_ezr_temple_completed_spans_chapters_5_and_6_with_a_continuation_heading_at_6_1() {
    let (graph, resolved) = real_graph();
    let index = build_heading_index(&graph, &resolved);

    let primary = index.get("EZR.5.1").expect("EZR.5.1 must anchor ezr_temple_completed's own PRIMARY heading");
    assert_eq!(primary.event_id, "ezr_temple_completed");
    assert!(!primary.continuation, "the container's own true first-covered verse is a PRIMARY anchor");

    let continuation = index.get("EZR.6.1").expect("EZR.6.1 -- a covered chapter's own opening verse, mid-container -- must render SOME heading (M-D1 req 1: 'no covered chapter may open with unlabeled verses')");
    assert_eq!(continuation.event_id, "ezr_temple_completed", "the SAME container continues at the chapter boundary");
    assert_eq!(continuation.title, primary.title, "one container, one title, true at both the anchor and its own continuation");
    assert!(continuation.continuation, "EZR.6.1 is NOT ezr_temple_completed's own true first verse -- it must render as a CONTINUATION, not a second primary");
}

/// The every-covered-chapter assertion (M-D1 requirement 1, verbatim:
/// "an every-covered-chapter assertion over the graph (all 66 books now)"),
/// derived from the graph itself -- never a hardcoded book/chapter list.
///
/// SCOPE, precisely (matching `build_heading_index`'s own law, CONTRACT.md's
/// own HEADING-WORTHY RULE): "covered" here means touched by >=1
/// HEADING-WORTHY container -- a narrative leg, or a real curated container
/// (witnesses/robertson_section/acts_section/atlas_section/
/// kjv_superscription). A bare Theographic freebie that is a leg of no
/// narrative and carries none of those (e.g. `theo-120` "Birth of Moses,"
/// EXO.2.1-10 -- real, live, checked against the real compiled data before
/// this test was written) is, by CONTRACT.md's own pre-existing, deliberate
/// rule, "correctly anchor[ing] NO heading anywhere" -- that is a SEPARATE,
/// disclosed curation-coverage question (W-series book-by-book authoring),
/// not this requirement's own defect class, and this test does not chase
/// it. What THIS test proves is req 1's own actual promise: wherever
/// heading-worthy coverage exists at all, no chapter boundary inside it
/// goes unlabeled -- exactly what the continuation pass fixes.
///
/// The heading-worthy chapter set is derived INDEPENDENTLY here (a second,
/// from-scratch walk over the same graph, not a call into
/// `build_heading_index`'s own internals) -- the same "two independently-
/// derived values, assert against each other" discipline
/// `adjacent_event_verse_groups_equal_the_map_arrows_own_scene_event`
/// already established elsewhere in this codebase, so a real gap in the
/// continuation pass has somewhere to show up rather than being
/// tautologically hidden inside the one function under test.
#[test]
fn m_d1_every_chapter_with_heading_worthy_coverage_opens_with_a_real_heading() {
    let (graph, resolved) = real_graph();
    let index = build_heading_index(&graph, &resolved);
    let narrative_legs = atlas_graph::heading::narrative_leg_event_ids(&graph);

    let mut worthy_chapters: BTreeSet<(String, u16)> = BTreeSet::new();
    for (id, node) in &graph.nodes {
        if id.kind != NodeKind::Event {
            continue;
        }
        let atlas_graph_types::node::NodePayload::Event { verses, witnesses, robertson_section, acts_section, atlas_section, kjv_superscription, .. } =
            &node.payload
        else {
            continue;
        };
        let is_real_container =
            !witnesses.is_empty() || robertson_section.is_some() || acts_section.is_some() || atlas_section.is_some() || kjv_superscription.is_some();
        if !(narrative_legs.contains(&id.raw) || is_real_container) {
            continue; // NOT heading-worthy -- out of this assertion's own scope, per CONTRACT.md's own rule
        }

        // Precisely: a chapter's own OPENING verse (verse == 1) must itself
        // be a MEMBER of this heading-worthy group's own coverage -- not
        // merely "some verse of this chapter is touched somewhere." A
        // group whose coverage jumps INTO a chapter mid-way (its own
        // earlier verses covered by a DIFFERENT container) contributes
        // nothing to THAT chapter's own boundary -- correctly: whichever
        // container truly opens it (if any) is what this test asks about,
        // for that container's own group. Since `chapter.1` is always the
        // numerically LOWEST verse in its chapter, membership here is
        // UNAMBIGUOUS: either it is this group's own overall minimum (the
        // PRIMARY anchor `heading_anchors_for` computes) or it is not (a
        // CONTINUATION candidate `continuation_candidates_for` computes) --
        // never neither, which is exactly why this scoped set is the
        // correct one to check against the built index below.
        //
        // WITNESSES-OR-VERSES, never both -- mirroring `heading_anchors_for`'s
        // own branch exactly (a real, live-caught mismatch during this
        // test's own authoring: `theo-162`'s TOP-LEVEL `verses` still
        // carries its full raw Theographic import span, 1KI.5.1-7.51, but
        // its own CURATED `witnesses` were deliberately narrowed to 1 Kings
        // 6 alone -- see that event's own `ref_note` -- specifically to
        // avoid a within-layer anchor collision with the finer
        // `1ki_hiram_temple_prep`/`1ki_temple_furnishings` siblings; the
        // production law correctly ignores the stale wide `verses` field
        // once real witnesses exist, so this test must too, or it silently
        // asks for headings production law never promises).
        let mut note = |v: &str| {
            if let Ok(vid) = atlas_core::refs::VerseId::parse_canonical(v) {
                if vid.verse == 1 {
                    worthy_chapters.insert((vid.book.code().to_string(), vid.chapter));
                }
            }
        };
        if !witnesses.is_empty() {
            for w in witnesses {
                if let Some(vs) = w.translations.get(atlas_graph::kjv_adapter::KJV_TRANSLATION) {
                    for v in vs {
                        note(v);
                    }
                }
            }
        } else {
            for v in verses {
                note(v);
            }
        }
    }
    // A sanity floor proving this independent walk actually found real,
    // substantial heading-worthy coverage, not an empty/near-empty set
    // that would make the assertion below vacuous.
    assert!(worthy_chapters.len() >= 500, "expected a substantial set of heading-worthy chapters (all 66 books), got {}", worthy_chapters.len());

    let missing: Vec<String> = worthy_chapters.iter().map(|(b, c)| format!("{b}.{c}.1")).filter(|v| !index.contains_key(v)).collect();
    assert!(
        missing.is_empty(),
        "every chapter touched by heading-worthy coverage must open with a real heading (primary or continuation) -- {} chapter(s) open unlabeled: {:?}",
        missing.len(),
        missing
    );
}
