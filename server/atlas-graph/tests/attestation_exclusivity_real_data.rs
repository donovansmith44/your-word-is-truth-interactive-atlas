//! Batch ATTEST-1 real-data laws: ACCOUNTS vs. MENTIONS over the real
//! committed corpus, plus the owner-ratified `Analogue` relation.
//!
//! THE OWNER'S DIAGNOSIS (verbatim): "I'm seeing a fundamental error. The
//! Espousal of Mary event has parallel accounts Mat.1.18 + Luke.1.27, and
//! that's a distinct event from The Angel Gabriel Announces Jesus'... which
//! has Luk1.26-36; Even worse, the appearance of Gabriel to Zacharias is
//! BETWEEN the espousal of mary and the announcement of Gabriel to Mary."
//! And separately: "A leper healed; a great popular excitement is given a
//! parallel where there shouldn't be from Mat.8.1-4; another leprosy
//! story... let's call it Analogue; that's ok for now."
//!
//! Both reports are one defect class: `Attests` -- the relation that means
//! "this passage NARRATES this event" -- was carrying verses that merely
//! REFERENCE an event, and joining two similar-but-distinct stories. This
//! file pins the corrections over the real corpus, and pins the SWEEP that
//! found the rest of the class (see `attestation_pending::PENDING`, the
//! compiled curation queue).
//!
//! Same per-file real-graph construction every other `_real_data.rs` file
//! in this crate uses (deliberately duplicated, not shared -- see
//! `bible_containers_real_data.rs`'s own header).

use std::path::Path;

use atlas_graph_types::edge::MentionedEntity;

fn real_atlas_data() -> atlas_core::data::AtlasData {
    static CACHED: std::sync::OnceLock<atlas_core::data::AtlasData> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile")
                .data
        })
        .clone()
}

fn real_graph() -> &'static atlas_graph_types::graph::Graph {
    static GRAPH: std::sync::OnceLock<atlas_graph_types::graph::Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
        let xrefs_tsv = std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
        let atlas = real_atlas_data();
        let (graph, ..) = atlas_graph::build::build_graph_from_sources_with_eras(&kjv_json, &xrefs_tsv, &atlas, &atlas.eras)
            .expect("the real committed sources must build");
        graph
    })
}

/// THE FOUNDING CASE, over the real corpus (L1 + L3). `theo-249` "Espousal
/// of Mary" carried LUK.1.27 and MAT.1.18 as its two "parallel accounts";
/// neither narrates the espousal. After the retype it has ZERO `Attests`
/// rows and exactly its two `Mentions` rows -- TOTAL CAPTURE: the facts
/// changed type, they were not dropped. Both halves are asserted because
/// either alone passes on a broken shape (zero attests AND zero mentions
/// would be silent data loss).
#[test]
fn the_espousal_of_mary_is_mention_only_and_keeps_both_facts() {
    let graph = real_graph();

    let attests: Vec<String> = graph.attests.iter().filter(|r| r.event.0 == "theo-249").map(|r| format!("{:?}", r.attestation.from.unit)).collect();
    assert!(
        attests.is_empty(),
        "theo-249 must have NO narrative accounts -- LUK.1.27 (\"To a virgin espoused to a man whose name was Joseph\") \
         and MAT.1.18 (\"When as his mother Mary was espoused to Joseph... she was found with child\") both MENTION the \
         espousal while narrating something else. Still attested: {attests:?}"
    );

    let mut mentioned: Vec<(u8, u16, u16)> = graph
        .mentions
        .iter()
        .filter(|r| matches!(&r.entity, MentionedEntity::Event(e) if e.0 == "theo-249"))
        .filter_map(|r| match &r.locus.at {
            atlas_graph_types::text::TextRef::Bible(v) => Some((v.book, v.chapter, v.verse)),
            atlas_graph_types::text::TextRef::Concord(_) => None,
        })
        .collect();
    mentioned.sort();
    assert_eq!(
        mentioned,
        vec![(39, 1, 18), (41, 1, 27)],
        "total capture: MAT.1.18 (book 39) and LUK.1.27 (book 41) must BOTH still reach the espousal, as mentions"
    );

    // And the collision that started it all is gone: LUK.1.27 now attests
    // exactly ONE event, the annunciation whose narrative it sits inside.
    let luk_1_27: Vec<&str> = graph
        .attests
        .iter()
        .filter(|r| {
            let v = &r.attestation.from.unit;
            (v.book, v.chapter, v.verse) == (41, 1, 27)
        })
        .map(|r| r.event.0.as_str())
        .collect();
    assert_eq!(
        luk_1_27,
        vec!["rob_annunciation_mary"],
        "LUK.1.27 must belong to exactly one event's Attests set -- two was the fabricated parallel the owner reported"
    );
}

/// THE ANALOGUE CHARTER CASE, over the real corpus (L4). Matthew's leper
/// (MAT.8.1-4, "when he was come down from the mountain") and Mark's/Luke's
/// (MRK.1.40-45 / LUK.5.12-16, during the first Galilean tour, with the
/// publishing-abroad aftermath the event is titled for) are two events, and
/// the relation that joins them says similar, not same.
#[test]
fn the_two_leprosy_events_are_distinct_and_joined_by_an_analogue() {
    let graph = real_graph();

    let verses_of = |id: &str| -> Vec<(u8, u16, u16)> {
        let mut v: Vec<(u8, u16, u16)> = graph
            .attests
            .iter()
            .filter(|r| r.event.0 == id)
            .map(|r| {
                let u = &r.attestation.from.unit;
                (u.book, u.chapter, u.verse)
            })
            .collect();
        v.sort();
        v.dedup();
        v
    };

    let matthews = verses_of("mat_leper_healed");
    let marks = verses_of("rob_leper_healed");
    assert_eq!(matthews, vec![(39, 8, 1), (39, 8, 2), (39, 8, 3), (39, 8, 4)], "Matthew's leper keeps its whole pericope -- MAT.8.1-4, the span CHRON-1's own fix round widened it to; zero coverage lost by the split");
    assert!(!marks.is_empty(), "Mark's/Luke's occasion must keep its own accounts");
    assert!(
        marks.iter().all(|(b, _, _)| *b != 39),
        "MAT.8.1-4 must no longer attest rob_leper_healed -- that WAS the false parallel: \"A leper healed; a great \
         popular excitement is given a parallel where there shouldn't be from Mat.8.1-4; another leprosy story\""
    );
    assert!(marks.iter().any(|(b, c, _)| (*b, *c) == (40, 1)), "Mark 1 must be among rob_leper_healed's accounts");
    assert!(marks.iter().any(|(b, c, _)| (*b, *c) == (41, 5)), "Luke 5 must be among rob_leper_healed's accounts");

    let joined = graph.analogue.iter().any(|r| {
        let (a, b) = (r.a.0.as_str(), r.b.0.as_str());
        (a == "rob_leper_healed" && b == "mat_leper_healed") || (a == "mat_leper_healed" && b == "rob_leper_healed")
    });
    assert!(joined, "the two leprosy events must be joined by an Analogue row -- the owner's own ratified idiom for 'stories that are very similar in this regard, but distinct events'");
}

/// THE STANDING LAWS, asserted over the REAL corpus rather than a fixture
/// (the same discipline `bible_containers_real_data.rs` applies to the
/// forest gate and the index≡rows law):
///   * L2, attestation exclusivity -- fail-loud against the declared
///     curation queue, in both directions (an undeclared collision, or a
///     stale/drifted declaration, is a build failure);
///   * L4's companion -- every `Analogue` row joins two distinct events,
///     exactly once.
/// These are the SAME functions `pipeline::LawCheckPass` runs on every
/// build; asserting them here as well is what makes a failure legible as a
/// named test rather than only as a build abort.
#[test]
fn the_attestation_laws_hold_over_the_real_corpus() {
    let graph = real_graph();
    atlas_graph::law_check::attestation_is_exclusive(graph).expect("L2 must hold over the real corpus");
    atlas_graph::law_check::attestation_inventory_has_no_stale_rows(graph)
        .expect("L2's stale-declaration direction -- asserted HERE because it is a claim about the real corpus, not about the synthetic fixtures the per-build pass also sees");
    atlas_graph::law_check::analogue_rows_join_two_distinct_events(graph).expect("L4's distinctness gate must hold over the real corpus");
}

/// THE CORPUS SWEEP, pinned (the brief's own requirement 4: "find EVERY
/// verse attested to >=2 distinct events"). These are real numbers off the
/// real corpus, and they are the batch's most important finding: the
/// account/mention confusion the owner caught is NOT a handful of typos --
/// the `Attests` relation is systematically carrying a CONTAINMENT
/// relationship it was never meant to carry.
///
/// Every pair below is declared in `attestation_pending::PENDING` with its
/// mechanical structural class and nothing else; the semantic disposition
/// is the owner's, one pair at a time, against the KJV text
/// (`batch-attest1-report.md` carries the full table). Pinning the totals
/// here means the queue's own SIZE cannot drift silently in either
/// direction -- it shrinks only when someone resolves a pair and records
/// the shrink.
#[test]
fn the_corpus_sweep_totals_are_pinned() {
    use atlas_graph::attestation_pending::{Class, PENDING};

    let containment = PENDING.iter().filter(|p| p.class == Class::Containment).count();
    let overlap = PENDING.iter().filter(|p| p.class == Class::Overlap).count();
    assert_eq!(containment + overlap, PENDING.len(), "every declared row carries a class");

    // The pinned inventory shape (see the module's own doc comment for how
    // these were derived and why the semantic call is deliberately absent).
    assert_eq!(PENDING.len(), SWEEP_PENDING_PAIRS, "the curation queue's size changed -- re-run the sweep and record the change");
    assert_eq!(containment, SWEEP_CONTAINMENT_PAIRS, "the containment-class count changed");
    assert_eq!(overlap, SWEEP_OVERLAP_PAIRS, "the overlap-class count changed");

    // Sorted and unique, so a hand-edit cannot introduce a shadowed
    // duplicate the law would then silently accept twice.
    let mut keys: Vec<(&str, &str)> = PENDING.iter().map(|p| (p.a, p.b)).collect();
    let before = keys.len();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), before, "attestation_pending::PENDING contains a duplicate pair");
    assert!(PENDING.iter().all(|p| p.a < p.b), "every row must be stored with its ends in lexicographic order -- law_check folds observed collisions the same way");

    // The pairs THIS batch resolved must be ABSENT: resolving a pair means
    // deleting its row, and a queue that keeps resolved entries stops being
    // a queue. Absence here is also the regression pin that says "this was
    // resolved, do not re-create it" -- without the row, a future re-attest
    // would come back as a NEW undeclared collision, which reds, but the
    // resolution itself would be recorded nowhere.
    //
    // TWO of these were genuine L2 collisions at BASE (`62e49e6`'s
    // `data/exports/chronology.json`, checked directly): LUK.1.27 sat in
    // both `theo-249`'s and `rob_annunciation_mary`'s `Attests` sets, and
    // MAT.1.18 sat in both `theo-249`'s and `rob_joseph_annunciation`'s.
    // Both are retyped to `Mentions` on `theo-249` (L1), leaving the
    // espousal mention-only (L3).
    //
    // The THIRD, `mat_leper_healed`/`rob_leper_healed`, was never an L2
    // collision -- MAT.8.1-4 attested exactly one event at BASE
    // (`rob_leper_healed`). It was a FALSE PARALLEL INSIDE one event, and
    // the split that fixed it is what makes the pair possible at all. It is
    // pinned absent here for the forward direction: the two leprosy events
    // are joined by an `Analogue` row and must NEVER come to share an
    // attestation, which would re-fabricate the parallel the owner reported.
    // (ATTEST-1 fix round 1, review finding M-3.)
    for resolved in [
        ("rob_annunciation_mary", "theo-249"),
        ("rob_joseph_annunciation", "theo-249"),
        ("mat_leper_healed", "rob_leper_healed"),
    ] {
        assert!(
            !PENDING.iter().any(|p| (p.a, p.b) == resolved),
            "{resolved:?} was resolved by this batch and must not be in the pending queue"
        );
    }
}

// Pinned sweep totals -- see `the_corpus_sweep_totals_are_pinned` above.
// Kept as named constants so a future batch that legitimately shrinks the
// queue changes ONE line per number, with the change visible in the diff.
const SWEEP_PENDING_PAIRS: usize = 824;
const SWEEP_CONTAINMENT_PAIRS: usize = 590;
const SWEEP_OVERLAP_PAIRS: usize = 234;
