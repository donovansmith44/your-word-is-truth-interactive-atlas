//! The laws, run: monad laws, dual involution, bijection witness,
//! temporal totality — over a toy graph. If these pass, the design
//! composes.

use std::collections::BTreeSet;

use crate::chrono::{
    temporal_order, PlacementBasis, ResolvedDate, ResolvedPlacement, SeqKey, TimePoint, Year,
};
use crate::edge::{
    dual, at, Attests, Direction, EdgeKind, Justification, LocatedAt, RelationId, Succession,
    SymRelationId,
};
use crate::explore::{EdgeQuery, Explorable, Holdings, PositionRef};
use crate::graph::Graph;
use crate::id::{AnyNodeId, EventId, NodeKind, PlaceId, Position};
use crate::text::{BibleLocus, BibleLocusRange, VerseRef};

fn ev(name: &str) -> EventId {
    EventId::new(name)
}
fn pl(name: &str) -> PlaceId {
    PlaceId::new(name)
}
fn vr(b: u8, c: u16, v: u16) -> VerseRef {
    VerseRef { book: b, chapter: c, verse: v }
}
fn brange(b: u8, c: u16, v1: u16, v2: u16) -> BibleLocusRange {
    BibleLocusRange::new(BibleLocus::whole(vr(b, c, v1)), BibleLocus::whole(vr(b, c, v2))).unwrap()
}

/// Toy graph: a three-event chain, each attested, two located.
fn toy() -> Graph {
    let mut g = Graph::default();
    g.succession.push(
        Succession::new(
            crate::id::NarrativeId::new("passion"),
            vec![ev("baptism"), ev("temptation"), ev("cana")],
            "curated".into(),
            Justification::default(),
        )
        .unwrap(),
    );
    g.attests.push(Attests {
        event: ev("baptism"),
        attestation: brange(40, 3, 13, 17),
        provenance: "curated".into(),
        justification: Justification::default(),
    });
    g.attests.push(Attests {
        event: ev("temptation"),
        attestation: brange(40, 4, 1, 11),
        provenance: "curated".into(),
        justification: Justification::default(),
    });
    g.located_at.push(LocatedAt {
        event: ev("baptism"),
        place: pl("jordan"),
        provenance: "curated".into(),
        justification: Justification::default(),
    });
    g.located_at.push(LocatedAt {
        event: ev("temptation"),
        place: pl("wilderness"),
        provenance: "curated".into(),
        justification: Justification::default(),
    });
    g.build_indexes();
    g
}

fn pos(e: &EventId) -> Position {
    at(&e.erase())
}

const FOLLOWS: EdgeKind = EdgeKind::Directed(RelationId::Succession, Direction::Forward);
const PRECEDES: EdgeKind = EdgeKind::Directed(RelationId::Succession, Direction::Inverse);
const SITE_OF: EdgeKind = EdgeKind::Directed(RelationId::LocatedAt, Direction::Inverse);
const LOCATED: EdgeKind = EdgeKind::Directed(RelationId::LocatedAt, Direction::Forward);

#[test]
fn dual_is_involutive_over_every_kind() {
    for r in RelationId::ALL {
        for d in [Direction::Forward, Direction::Inverse] {
            let k = EdgeKind::Directed(*r, d);
            assert_eq!(dual(dual(k)), k);
            assert_ne!(dual(k), k, "directed kinds flip");
        }
    }
    for s in SymRelationId::ALL {
        let k = EdgeKind::Symmetric(*s);
        assert_eq!(dual(k), k, "symmetric kinds are fixed points");
    }
}

#[test]
fn labels_exist_for_every_reading() {
    for r in RelationId::ALL {
        assert!(!r.forward_label().is_empty());
        assert!(!r.inverse_label().is_empty());
        assert_ne!(r.forward_label(), r.inverse_label());
    }
    for s in SymRelationId::ALL {
        assert!(!s.label().is_empty());
    }
}

#[test]
fn monad_left_identity_focus_then_bind_is_apply() {
    let g = toy();
    let f = |p: &Position| Holdings::focus(p.clone()).step(&g, FOLLOWS);
    let n = pos(&ev("baptism"));
    assert_eq!(Holdings::focus(n.clone()).bind(f), f(&n));
}

#[test]
fn monad_right_identity_bind_focus_changes_nothing() {
    let g = toy();
    let h = Holdings::focus(pos(&ev("baptism"))).step(&g, FOLLOWS);
    assert_eq!(h.bind(|p| Holdings::focus(p.clone())), h);
}

#[test]
fn monad_associativity_grouping_is_irrelevant() {
    let g = toy();
    let f = |p: &Position| Holdings::focus(p.clone()).step(&g, FOLLOWS);
    let k = |p: &Position| Holdings::focus(p.clone()).step(&g, SITE_OF);
    let h = Holdings::focus(pos(&ev("baptism")));
    let left = h.bind(&f).bind(&k);
    let right = h.bind(|p| f(p).bind(&k));
    assert_eq!(left, right);
}

#[test]
fn step_page_agreement_pages_are_windows_over_the_total() {
    let g = toy();
    let n = pos(&ev("baptism"));
    let total = Holdings::focus(n.clone()).step(&g, FOLLOWS);

    let r = PositionRef(n);
    let mut cursor = None;
    let mut paged: BTreeSet<Position> = BTreeSet::new();
    loop {
        let page = r.edges(&g, &EdgeQuery { kind: FOLLOWS, cursor, limit: 1 });
        paged.extend(page.entries.iter().map(|e| e.node.clone()));
        match page.next {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    assert_eq!(Holdings(paged), total);
}

#[test]
fn bijection_witness_same_edge_id_from_either_end() {
    let g = toy();
    let pairs = crate::explore::inverse_entry_ids(&g, &pos(&ev("baptism")), FOLLOWS);
    assert!(!pairs.is_empty());
    for (fwd, inv) in pairs {
        assert_eq!(fwd, inv, "one row, two projections, one id");
    }
}

#[test]
fn walk_baptism_to_temptation_and_back() {
    let g = toy();
    let fwd = Holdings::focus(pos(&ev("baptism"))).step(&g, FOLLOWS);
    assert!(fwd.0.contains(&pos(&ev("temptation"))));
    let back = fwd.step(&g, PRECEDES);
    assert!(back.0.contains(&pos(&ev("baptism"))));
}

#[test]
fn pooled_two_hop_what_happened_where_this_happened() {
    let g = toy();
    let out = Holdings::focus(pos(&ev("baptism"))).steps(&g, &[LOCATED, SITE_OF]);
    assert!(out.0.contains(&pos(&ev("baptism"))), "reachable via its own place");
}

#[test]
fn edge_summary_includes_symmetric_kinds() {
    // M-C review I-1 pin: a node with real symmetric-relation edges
    // must report them in its summary — omitting them would let the
    // honesty policy (render iff count > 0) hide real connections.
    use crate::edge::{CatechismLink, Justification};
    use crate::text::{TextLocus, TextRef};

    let mut g = toy();
    g.catechism.push(CatechismLink {
        locus: TextLocus { at: TextRef::Bible(vr(40, 3, 13)), span: None },
        item: crate::id::CatechismItemId::new("baptism-part"),
        provenance: "curated".into(),
        justification: Justification::default(),
    });
    g.build_indexes();

    let item = Position::Node(crate::id::CatechismItemId::new("baptism-part").erase());
    let summary = crate::explore::PositionRef(item.clone()).edge_summary(&g);
    let sym = EdgeKind::Symmetric(SymRelationId::CatechismLink);
    assert_eq!(summary.get(&sym), Some(&1), "symmetric kinds appear in summaries");
    let total = Holdings::focus(item).step(&g, sym);
    assert_eq!(total.0.len(), 1, "and the summary agrees with the frontier");
}

#[test]
fn edge_summary_counts_match_frontiers() {
    let g = toy();
    let n = pos(&ev("baptism"));
    let summary = PositionRef(n.clone()).edge_summary(&g);
    for (kind, count) in summary {
        let total = Holdings::focus(n.clone()).step(&g, kind);
        assert_eq!(total.0.len(), count);
        assert!(count > 0, "summary lists only inhabited kinds (honesty)");
    }
}

#[test]
fn temporal_order_is_total_and_antisymmetric() {
    let y = |n: i32| TimePoint::year_only(Year::new(n).unwrap());
    let rp = |from: TimePoint, seq: u32| ResolvedPlacement {
        date: ResolvedDate { from, to: from },
        seq: SeqKey(seq),
        basis: PlacementBasis::Traditional,
    };
    let a = rp(y(-1015), 1); // coup
    let b = rp(y(-1015), 2); // anointing
    let c = rp(y(-1015), 3); // charge
    let d = rp(y(-1015), 4); // death
    let e = rp(y(-1014), 0); // gibeon dream
    let all = [a, b, c, d, e];
    for (i, x) in all.iter().enumerate() {
        for (j, y2) in all.iter().enumerate() {
            let o = temporal_order(x, y2);
            if i == j {
                assert_eq!(o, std::cmp::Ordering::Equal);
            } else {
                assert_ne!(o, std::cmp::Ordering::Equal, "total: distinct placements order");
                assert_eq!(o, temporal_order(y2, x).reverse(), "antisymmetric");
            }
        }
    }
    assert!(temporal_order(&d, &e).is_lt(), "David's death precedes the dream at Gibeon");
    assert!(temporal_order(&c, &d).is_lt(), "the charge precedes the death");
}

#[test]
fn kind_mismatch_is_a_checked_parse_not_a_surprise() {
    let n = AnyNodeId { kind: NodeKind::Place, raw: "sidon".into() };
    assert!(n.narrow::<crate::id::EventTag>().is_err());
    assert!(n.narrow::<crate::id::PlaceTag>().is_ok());
}

#[test]
fn malformed_chains_fail_to_construct() {
    let j = Justification::default;
    assert!(Succession::new(crate::id::NarrativeId::new("n"), vec![], "p".into(), j()).is_err());
    assert!(Succession::new(
        crate::id::NarrativeId::new("n"),
        vec![ev("a"), ev("a")],
        "p".into(),
        j()
    )
    .is_err());
}
