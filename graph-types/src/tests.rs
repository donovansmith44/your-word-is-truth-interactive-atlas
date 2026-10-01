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
    let a = rp(y(-1015), 1);
    let b = rp(y(-1015), 2);
    let c = rp(y(-1015), 3);
    let d = rp(y(-1015), 4);
    let e = rp(y(-1014), 0);
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

fn first_entry(g: &Graph, at_position: &Position, kind: EdgeKind) -> crate::explore::EdgeEntry {
    PositionRef(at_position.clone()).edges(g, &EdgeQuery { kind, cursor: None, limit: 1 }).entries.remove(0)
}

#[test]
fn an_edge_is_read_by_its_id_as_its_kind_its_two_ends_and_its_meta() {
    // Arrange
    let g = toy();
    let located = EdgeKind::Directed(RelationId::LocatedAt, Direction::Forward);
    let entry = first_entry(&g, &pos(&ev("baptism")), located);

    // Act
    let record = crate::store::GraphQuery::edge(&g, &entry.edge);

    // Assert
    assert_eq!(
        record,
        Some(crate::edge::EdgeRecord { id: entry.edge.clone(), kind: located, subject: pos(&ev("baptism")), object: at(&pl("jordan").erase()), meta: entry.meta })
    );
}

#[test]
fn an_edge_read_from_either_end_of_its_inverse_page_is_the_same_edge() {
    // Arrange
    let g = toy();
    let site_of = EdgeKind::Directed(RelationId::LocatedAt, Direction::Inverse);
    let entry = first_entry(&g, &at(&pl("jordan").erase()), site_of);

    // Act
    let record = crate::store::GraphQuery::edge(&g, &entry.edge).map(|r| (r.subject, r.object));

    // Assert
    assert_eq!(record, Some((pos(&ev("baptism")), at(&pl("jordan").erase()))));
}

#[test]
fn a_symmetric_edge_leads_from_the_lesser_of_its_ends() {
    // Arrange
    let mut g = Graph::default();
    g.analogue.push(crate::edge::Analogue { a: ev("temptation"), b: ev("baptism"), provenance: "curated".into() });
    g.build_indexes();
    let analogous = EdgeKind::Symmetric(SymRelationId::Analogue);
    let entry = first_entry(&g, &pos(&ev("temptation")), analogous);

    // Act
    let record = crate::store::GraphQuery::edge(&g, &entry.edge).map(|r| (r.kind, r.subject, r.object));

    // Assert
    assert_eq!(record, Some((analogous, pos(&ev("baptism")), pos(&ev("temptation")))));
}

#[test]
fn an_id_naming_no_edge_or_no_relation_reads_as_no_edge() {
    // Arrange
    let g = toy();
    let entry = first_entry(&g, &pos(&ev("baptism")), EdgeKind::Directed(RelationId::LocatedAt, Direction::Forward));
    let (_, hash) = entry.edge.0.split_once(':').unwrap();
    let asked = [
        crate::edge::EdgeId(format!("{}:{hash}", RelationId::Attests.name())),
        crate::edge::EdgeId(format!("Nothing:{hash}")),
        crate::edge::EdgeId(RelationId::LocatedAt.name().to_string()),
    ];

    // Act
    let read: Vec<bool> = asked.iter().map(|id| crate::store::GraphQuery::edge(&g, id).is_some()).collect();

    // Assert
    assert_eq!(read, vec![false, false, false]);
}

#[test]
fn an_edge_id_names_its_relation_read_forward() {
    // Arrange
    let g = toy();
    let entry = first_entry(&g, &at(&pl("jordan").erase()), EdgeKind::Directed(RelationId::LocatedAt, Direction::Inverse));
    let (_, hash) = entry.edge.0.split_once(':').unwrap();
    let asked = [
        entry.edge.clone(),
        crate::edge::EdgeId(format!("{}:{hash}", SymRelationId::Analogue.name())),
        crate::edge::EdgeId(format!("{}:not-hex", RelationId::LocatedAt.name())),
        crate::edge::EdgeId(format!("Nothing:{hash}")),
    ];

    // Act
    let kinds: Vec<Option<EdgeKind>> = asked.iter().map(crate::edge::EdgeId::kind).collect();

    // Assert
    assert_eq!(
        kinds,
        vec![Some(EdgeKind::Directed(RelationId::LocatedAt, Direction::Forward)), Some(EdgeKind::Symmetric(SymRelationId::Analogue)), None, None]
    );
}

#[test]
fn an_edges_ends_are_its_from_and_to_frontiers_of_one_each() {
    // Arrange
    let g = toy();
    let entry = first_entry(&g, &pos(&ev("baptism")), EdgeKind::Directed(RelationId::LocatedAt, Direction::Forward));
    let record = crate::store::GraphQuery::edge(&g, &entry.edge).unwrap();
    let here = Position::Edge(entry.edge.clone());

    // Act
    let ends: Vec<(EdgeKind, Vec<crate::explore::EdgeEntry>)> = record.ends().into_iter().map(|(kind, frontier)| (kind, frontier.edges().cloned().collect())).collect();

    // Assert
    assert_eq!(
        ends,
        vec![
            (crate::edge::EDGE_FROM, vec![crate::explore::EdgeEntry { edge: crate::edge::entry_id(RelationId::EdgeSource, &here, &record.subject), node: record.subject.clone(), meta: crate::explore::EdgeMeta::None }]),
            (crate::edge::EDGE_TO, vec![crate::explore::EdgeEntry { edge: crate::edge::entry_id(RelationId::EdgeTarget, &here, &record.object), node: record.object.clone(), meta: crate::explore::EdgeMeta::None }]),
        ]
    );
}

#[test]
fn a_position_is_the_from_end_of_a_forward_edge_the_to_end_of_an_inverse_one_and_the_lesser_end_of_a_symmetric_one() {
    // Arrange
    let (lesser, greater) = (pos(&ev("baptism")), pos(&ev("temptation")));
    let asked = [
        (EdgeKind::Directed(RelationId::LocatedAt, Direction::Forward), &greater, &lesser),
        (EdgeKind::Directed(RelationId::LocatedAt, Direction::Inverse), &lesser, &greater),
        (EdgeKind::Symmetric(SymRelationId::Analogue), &lesser, &greater),
        (EdgeKind::Symmetric(SymRelationId::Analogue), &greater, &lesser),
        (EdgeKind::Symmetric(SymRelationId::Analogue), &lesser, &lesser),
    ];

    // Act
    let ends: Vec<crate::edge::EdgeEnd> = asked.iter().map(|(kind, here, there)| kind.end_at(here, there)).collect();

    // Assert
    use crate::edge::EdgeEnd::{From, To};
    assert_eq!(ends, vec![From, To, From, To, From]);
}

#[test]
fn every_edge_kind_is_displayed_as_its_label_spoken_with_a_capital() {
    // Arrange
    let asked = [
        EdgeKind::Directed(RelationId::Attests, Direction::Forward),
        EdgeKind::Directed(RelationId::SpokenAt, Direction::Inverse),
        crate::edge::EDGE_FROM,
        EdgeKind::Symmetric(SymRelationId::Parallel),
    ];

    // Act
    let shown: Vec<String> = asked.iter().map(|kind| kind.display_label()).collect();

    // Assert
    assert_eq!(shown, vec!["Attested in", "Site of speech", "From", "Parallel"]);
}
