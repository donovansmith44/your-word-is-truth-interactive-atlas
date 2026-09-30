mod common;

use std::collections::BTreeSet;

use atlas_graph_types::edge::MentionedEntity;

fn real_graph() -> &'static atlas_graph_types::graph::Graph {
    static GRAPH: std::sync::OnceLock<atlas_graph_types::graph::Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| common::kjv_and_atlas_build(&common::real_atlas().eras).0)
}

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

#[test]
fn the_sermon_on_the_mount_is_attested_by_every_verse_of_its_account_past_the_twentieth_of_a_chapter() {
    // Arrange
    let sermon = common::real_atlas().event_by_id("rob_sermon_on_the_mount").expect("the Sermon on the Mount is a curated event");
    let expected: BTreeSet<(u8, u16, u16)> = sermon
        .witnesses
        .iter()
        .flat_map(|w| atlas_core::translation::resolve(&w.translations, atlas_core::translation::DEFAULT_TRANSLATION).expect("every witness has a KJV reading"))
        .map(|v| {
            let id = atlas_core::refs::VerseId::parse_canonical(v).expect("etl-validated verse id");
            (id.book.0, id.chapter, id.verse)
        })
        .collect();
    // Act
    let attested: BTreeSet<(u8, u16, u16)> = real_graph()
        .attests
        .iter()
        .filter(|r| r.event.0 == sermon.id)
        .map(|r| {
            let u = &r.attestation.from.unit;
            (u.book, u.chapter, u.verse)
        })
        .collect();
    // Assert
    assert_eq!(attested, expected);
}

#[test]
fn the_attestation_laws_hold_over_the_real_corpus() {
    let graph = real_graph();
    atlas_graph::law_check::attestation_is_exclusive(graph).expect("L2 must hold over the real corpus");
    atlas_graph::law_check::attestation_inventory_has_no_stale_rows(graph)
        .expect("L2's stale-declaration direction -- asserted HERE because it is a claim about the real corpus, not about the synthetic fixtures the per-build pass also sees");
    atlas_graph::law_check::analogue_rows_join_two_distinct_events(graph).expect("L4's distinctness gate must hold over the real corpus");
}

#[test]
fn the_corpus_sweep_totals_are_pinned() {
    use atlas_graph::attestation_pending::{Class, PENDING};

    let containment = PENDING.iter().filter(|p| p.class == Class::Containment).count();
    let overlap = PENDING.iter().filter(|p| p.class == Class::Overlap).count();
    assert_eq!(containment + overlap, PENDING.len(), "every declared row carries a class");

    assert_eq!(PENDING.len(), SWEEP_PENDING_PAIRS, "the curation queue's size changed -- re-run the sweep and record the change");
    assert_eq!(containment, SWEEP_CONTAINMENT_PAIRS, "the containment-class count changed");
    assert_eq!(overlap, SWEEP_OVERLAP_PAIRS, "the overlap-class count changed");

    let mut keys: Vec<(&str, &str)> = PENDING.iter().map(|p| (p.a, p.b)).collect();
    let before = keys.len();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), before, "attestation_pending::PENDING contains a duplicate pair");
    assert!(PENDING.iter().all(|p| p.a < p.b), "every row must be stored with its ends in lexicographic order -- law_check folds observed collisions the same way");

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

const SWEEP_PENDING_PAIRS: usize = 889;
const SWEEP_CONTAINMENT_PAIRS: usize = 719;
const SWEEP_OVERLAP_PAIRS: usize = 170;
