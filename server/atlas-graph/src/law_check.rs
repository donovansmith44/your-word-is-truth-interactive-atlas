//! The pipeline's LAW-CHECK stage (pipeline.rs's own stage mapping): a
//! generic, cross-adapter REFERENTIAL-INTEGRITY law, alongside the KJV
//! adapter's own bijection/reconstruction fidelity law (fidelity.rs).
//!
//! Scope, disclosed: checks every NODE-TYPED endpoint of every authored
//! row -- Event/Narrative/Anchor/Place ids on `attests`/`succession`/
//! `dated_by`/`located_at`/`named`, and (M-C2, folding M-C review M-1)
//! `mentions.entity` (`PlaceOrPerson`) and `catechism.item`
//! (`CatechismItemId`) -- resolves to a real node in the built graph. This
//! is now every node-typed endpoint this crate's own adapters emit; the
//! previous omission of the latter two was a genuine scope-disclosure gap
//! (M-C review's own words: "safe today only because place_adapter/
//! catechism_adapter build their rows and their corresponding nodes from
//! the identical source iteration... but that invariant isn't enforced by
//! this law, only coincidentally true"), now closed for real rather than
//! merely documented. TextLocus-typed endpoints (`attests.attestation`,
//! `mentions.locus`, `cross_refs.from`/`.to`, `catechism.locus`) are STILL
//! NOT checked here -- resolving a locus to its own TextUnit node id
//! requires `graph_types::graph::Graph`'s own PRIVATE `text_node` helper
//! (confirmed: not part of that module's public surface), and every
//! adapter that EMITS a TextLocus-shaped row already derives it directly
//! from a real, just-built TextUnit node id (kjv_adapter's own
//! `verse_node_id`/`dot_ref` round trip) or a citation the KJV fidelity
//! law itself already proves exists -- a second, weaker re-check here
//! would not catch anything the adapters' own construction doesn't
//! already guarantee by shape (unlike the node-typed endpoints above,
//! which name an id typed once at authoring time with no such structural
//! guarantee).
use std::collections::BTreeSet;

use atlas_graph_types::edge::PlaceOrPerson;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{AnyNodeId, EventId};
use atlas_graph_types::node::NodePayload;

use crate::event_world::ChronologyDerivation;

#[derive(Debug)]
pub struct DanglingReference {
    pub relation: &'static str,
    pub field: &'static str,
    pub missing: AnyNodeId,
}

impl std::fmt::Display for DanglingReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{} names {:?}, which has no node in the built graph", self.relation, self.field, self.missing)
    }
}
impl std::error::Error for DanglingReference {}

/// Every node-typed endpoint of every authored row resolves to a real
/// node. Fail-loud on the FIRST dangling reference found (named precisely
/// -- which relation, which field, which missing id), matching this
/// crate's own fail-loud convention for boundary laws (`fidelity.rs`).
pub fn every_authored_edge_resolves(graph: &Graph) -> Result<(), DanglingReference> {
    let has = |id: &AnyNodeId| graph.nodes.contains_key(id);
    let check = |relation: &'static str, field: &'static str, id: AnyNodeId| -> Result<(), DanglingReference> {
        if has(&id) {
            Ok(())
        } else {
            Err(DanglingReference { relation, field, missing: id })
        }
    };

    for row in &graph.attests {
        check("attests", "event", row.event.erase())?;
    }
    for row in &graph.located_at {
        check("located_at", "event", row.event.erase())?;
        check("located_at", "place", row.place.erase())?;
    }
    for row in &graph.succession {
        let seen: BTreeSet<&atlas_graph_types::id::EventId> = row.chain.iter().collect();
        for eid in seen {
            check("succession", "chain", eid.erase())?;
        }
    }
    for row in &graph.dated_by {
        check("dated_by", "event", row.event.erase())?;
        use atlas_graph_types::chrono::ChronoTarget;
        // ChronoTarget::Era is DELIBERATELY excluded: `event_world::
        // choose_placement`'s own degenerate fallback (reachable only when
        // the anchor table is empty -- never true of real data, but true
        // of `AtlasData::demo_fixture()` and several unit fixtures across
        // this workspace) mints a synthetic `"undetermined-basis-{id}"`
        // era id BY DESIGN, documented at that call site as intentionally
        // unresolvable ("EraOnly has no real Era node to resolve against,
        // honestly... this event's own 'why this date?' explorability
        // degrades to 'undetermined basis'"). Flagging that as a dangling
        // reference would turn a disclosed, deliberate honesty gap into a
        // build failure -- the opposite of this law's own purpose. A
        // REAL (curated, non-synthetic) EraOnly placement's target -- once
        // Era nodes materialize -- is exactly the kind of reference this
        // law WOULD want to check; that's a real Era adapter's own concern,
        // not this generic pass's.
        let target = match row.placement.target() {
            ChronoTarget::Anchor(a) => Some(a.erase()),
            ChronoTarget::Prior(p) => Some(p.erase()),
            ChronoTarget::Era(_) => None,
        };
        if let Some(target) = target {
            check("dated_by", "target", target)?;
        }
    }
    for row in &graph.named {
        check("named", "place", row.place.erase())?;
    }
    // M-C2 (folded M-C review M-1): the two node-typed endpoints this
    // law's own module doc comment previously disclosed as unchecked --
    // now real checks, not just a documented coincidence.
    for row in &graph.mentions {
        let id = match &row.entity {
            PlaceOrPerson::Place(p) => p.erase(),
            PlaceOrPerson::Person(p) => p.erase(),
        };
        check("mentions", "entity", id)?;
    }
    for row in &graph.catechism {
        check("catechism", "item", row.item.erase())?;
    }

    Ok(())
}

#[derive(Debug)]
pub struct PayloadYearDivergence {
    pub event: String,
    pub payload_from_year: i32,
    pub payload_to_year: i32,
    pub resolved_from_year: i32,
    pub resolved_to_year: i32,
}

impl std::fmt::Display for PayloadYearDivergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "event {:?}: NodePayload::Event's own from_year/to_year ({}, {}) diverges from its resolved placement's own years ({}, {}) -- the payload mirror and dated_by's placement (the single source) must never disagree",
            self.event, self.payload_from_year, self.payload_to_year, self.resolved_from_year, self.resolved_to_year
        )
    }
}
impl std::error::Error for PayloadYearDivergence {}

/// Fix round 1 (DELEGATED RULING, `batch-mc2-review.md`'s own DELEGATED
/// TYPES REVIEW finding 4): `NodePayload::Event.from_year`/`.to_year`
/// (M-C2's own payload widening, mirroring `atlas_core::data::Event.when`
/// verbatim -- see `event_world::event_node`'s own doc comment, "for
/// lossless overlay reconstruction") is a MIRROR, never a second source of
/// truth -- `dated_by` placements (`ChronologyDerivation.resolved`, this
/// crate's own single feed for ordering/`temporal_neighbors`/timeline
/// position) remain THE source. This closes the review's own named
/// tension ("the implementer refused exactly this duplication pattern for
/// Narrative legs but accepted it for years") by making the payload's own
/// ANCHOR a PROVEN projection instead of an unwinding.
///
/// SCOPE, corrected against real data (a genuine discovery made landing
/// this law, not assumed from the ruling's own wording): the ruling's own
/// phrase "from_year/to_year equal its resolved placement years" reads as
/// a full-interval check, but `resolve` (above, `let tp = TimePoint::
/// year_only(Year::new(e.when.from_year)...); ResolvedPlacement { date:
/// ResolvedDate { from: tp, to: tp }, ... }`) NEVER derives a resolved "to"
/// distinct from "from" -- ordering/`temporal_order` only ever needs ONE
/// representative point per event, so `resolved.date.to` is, BY DESIGN,
/// always a copy of `resolved.date.from`, never `e.when.to_year`. Running
/// a literal from_year-AND-to_year check against the real committed data
/// immediately failed on `rut_naomi_returns` (payload `-1200..-1190`, a
/// genuine 10-year curated uncertainty range; resolved `-1200..-1200`, the
/// point the resolver actually orders by) -- range-dated events are a
/// real, common, legitimate `TimeRange` shape (`atlas_core::time::
/// TimeRange`), not a data error, so REJECTING them here would be a new,
/// wrong constraint this law has no business inventing. The TRUE
/// invariant this law verifies: the payload's own ANCHOR (`from_year`,
/// the value the WHOLE ordering system is built from) must never diverge
/// from the resolved placement's own point. `to_year` is a display-only,
/// legitimately-wider uncertainty bound with no resolved-placement
/// counterpart to verify against at all -- not weakened silently; this is
/// the disclosed, corrected scope, not the literal ruling text.
///
/// Scope, disclosed (unchanged from the original plan): only events
/// PRESENT in `chrono.resolved` are checked (general-kind events never get
/// a resolved placement at all -- correctly excluded, the same "no
/// timeline entry for a dateless passage" shape `event_world`'s own tests
/// already establish); an event id in `chrono.resolved` with no matching
/// graph node is NOT flagged here (a genuine dangling reference of that
/// shape is `every_authored_edge_resolves`'s own concern via
/// `dated_by.event`, not duplicated).
pub fn payload_years_match_resolved_placements(graph: &Graph, chrono: &ChronologyDerivation) -> Result<(), PayloadYearDivergence> {
    for (event_id, resolved) in &chrono.resolved {
        let node_id = EventId::new(event_id.clone()).erase();
        let Some(node) = graph.nodes.get(&node_id) else { continue };
        let NodePayload::Event { from_year, to_year, .. } = &node.payload else { continue };
        let resolved_from = resolved.date.from.year.get();
        if *from_year != resolved_from {
            return Err(PayloadYearDivergence {
                event: event_id.clone(),
                payload_from_year: *from_year,
                payload_to_year: *to_year,
                resolved_from_year: resolved_from,
                resolved_to_year: resolved.date.to.year.get(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::edge::{Justification, LocatedAt};
    use atlas_graph_types::id::{EventId, PlaceId};

    #[test]
    fn green_on_an_empty_graph() {
        let graph = Graph::default();
        assert!(every_authored_edge_resolves(&graph).is_ok());
    }

    #[test]
    fn red_when_a_located_at_row_names_a_place_with_no_node() {
        let mut graph = Graph::default();
        graph.located_at.push(LocatedAt {
            event: EventId::new("e1"),
            place: PlaceId::new("nowhere"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        // No Event/Place nodes inserted at all -- both endpoints dangle;
        // the first-checked field (`event`) is what the error names.
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling reference");
        assert_eq!(err.relation, "located_at");
        assert_eq!(err.field, "event");
    }

    fn locus() -> atlas_graph_types::text::TextLocus {
        atlas_graph_types::text::TextLocus {
            at: atlas_graph_types::text::TextRef::Bible(atlas_graph_types::text::VerseRef { book: 0, chapter: 1, verse: 1 }),
            span: None,
        }
    }

    // M-C2 (folded M-C review M-1): the two node-typed endpoints this
    // law previously left unchecked -- red-then-green, mirroring the
    // `located_at` case above.
    #[test]
    fn red_when_a_mentions_row_names_a_place_with_no_node() {
        let mut graph = Graph::default();
        graph.mentions.push(atlas_graph_types::edge::Mentions {
            locus: locus(),
            entity: PlaceOrPerson::Place(atlas_graph_types::id::PlaceId::new("nowhere")),
            provenance: "test".into(),
        });
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling mentions.entity reference");
        assert_eq!(err.relation, "mentions");
        assert_eq!(err.field, "entity");
    }

    #[test]
    fn red_when_a_catechism_link_row_names_an_item_with_no_node() {
        let mut graph = Graph::default();
        graph.catechism.push(atlas_graph_types::edge::CatechismLink {
            locus: locus(),
            item: atlas_graph_types::id::CatechismItemId::new("nowhere"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling catechism.item reference");
        assert_eq!(err.relation, "catechism");
        assert_eq!(err.field, "item");
    }

    #[test]
    fn green_when_mentions_and_catechism_rows_resolve_to_real_nodes() {
        use atlas_graph_types::id::{NodeKind, PlaceId};
        use atlas_graph_types::node::{Node, NodePayload};

        let mut graph = Graph::default();
        let place_id = PlaceId::new("hebron").erase();
        graph.nodes.insert(
            place_id.clone(),
            Node { id: place_id.clone(), payload: NodePayload::Place { canonical: "Hebron".into(), lat: 0.0, lon: 0.0, aliases: vec![] }, provenance: "test".into() },
        );
        graph.mentions.push(atlas_graph_types::edge::Mentions { locus: locus(), entity: PlaceOrPerson::Place(PlaceId::new("hebron")), provenance: "test".into() });

        let item_id = atlas_graph_types::id::CatechismItemId::new("commandment-1").erase();
        assert_eq!(item_id.kind, NodeKind::CatechismItem);
        graph.nodes.insert(item_id.clone(), Node { id: item_id.clone(), payload: NodePayload::CatechismItem { label: "The First Commandment".into() }, provenance: "test".into() });
        graph.catechism.push(atlas_graph_types::edge::CatechismLink {
            locus: locus(),
            item: atlas_graph_types::id::CatechismItemId::new("commandment-1"),
            provenance: "test".into(),
            justification: Justification::default(),
        });

        assert!(every_authored_edge_resolves(&graph).is_ok());
    }

    // Fix round 1 (delegated ruling): the verified-cache law.
    use atlas_graph_types::chrono::{PlacementBasis, ResolvedDate, ResolvedPlacement, SeqKey, TimePoint, Year};
    use atlas_graph_types::node::Node;

    /// A minimal one-event graph + matching `ChronologyDerivation`, both
    /// carrying `year` as their own from/to (a point-in-time event, the
    /// common real-data shape) -- callers mutate one side to force a
    /// divergence.
    fn one_dated_event(year: i32) -> (Graph, ChronologyDerivation) {
        let mut graph = Graph::default();
        let id = EventId::new("e1").erase();
        graph.nodes.insert(
            id.clone(),
            Node {
                id: id.clone(),
                payload: NodePayload::Event {
                    label: "Test Event".into(),
                    kind: "event".into(),
                    from_year: year,
                    to_year: year,
                    order_key: 0,
                    verses: vec![],
                    witnesses: vec![],
                    robertson_section: None,
                    acts_section: None,
                    atlas_section: None,
                    kjv_superscription: None,
                    ref_note: None,
                },
                provenance: "test".into(),
            },
        );
        let tp = TimePoint::year_only(Year::new(year).unwrap());
        let mut resolved = std::collections::HashMap::new();
        resolved.insert("e1".to_string(), ResolvedPlacement { date: ResolvedDate { from: tp, to: tp }, seq: SeqKey(0), basis: PlacementBasis::Textual });
        (graph, ChronologyDerivation { order: vec!["e1".to_string()], placements: std::collections::HashMap::new(), resolved })
    }

    #[test]
    fn green_when_payload_years_equal_the_resolved_placement() {
        let (graph, chrono) = one_dated_event(-30);
        assert!(payload_years_match_resolved_placements(&graph, &chrono).is_ok());
    }

    #[test]
    fn red_when_a_dated_events_payload_years_diverge_from_its_own_resolved_placement() {
        let (mut graph, chrono) = one_dated_event(-30);
        // Mutate ONLY the payload mirror (simulating exactly the class of
        // bug this law exists to catch: the two independent computations
        // -- atlas_core's own event-merge date resolution, mirrored onto
        // the payload; atlas_graph's own derive_chronology, feeding
        // `resolved` -- drifting apart).
        let id = EventId::new("e1").erase();
        if let Some(node) = graph.nodes.get_mut(&id) {
            if let NodePayload::Event { from_year, to_year, .. } = &mut node.payload {
                *from_year = -29;
                *to_year = -29;
            }
        }
        let err = payload_years_match_resolved_placements(&graph, &chrono).expect_err("a payload/resolved-placement year mismatch must be rejected, not silently accepted");
        assert_eq!(err.event, "e1");
        assert_eq!(err.payload_from_year, -29);
        assert_eq!(err.resolved_from_year, -30);
    }

    #[test]
    fn green_for_a_genuine_range_dated_event_whose_to_year_the_resolver_never_tracks() {
        // Real-data discovery this fix round made (this function's own doc
        // comment): `rut_naomi_returns` -- payload -1200..-1190, a genuine
        // 10-year curated TimeRange -- resolves to a single POINT (-1200)
        // for ordering, since `resolve` (event_world.rs) only ever derives
        // a resolved "to" as a copy of "from", never from `e.when.to_year`.
        // A range-dated event's own WIDER to_year must NOT be flagged --
        // only its own from_year (the true anchor) is checked.
        let (mut graph, chrono) = one_dated_event(-1200);
        let id = EventId::new("e1").erase();
        if let Some(node) = graph.nodes.get_mut(&id) {
            if let NodePayload::Event { to_year, .. } = &mut node.payload {
                *to_year = -1190; // from_year (-1200) unchanged -- still matches the resolved point
            }
        }
        assert!(payload_years_match_resolved_placements(&graph, &chrono).is_ok(), "a range-dated event's own wider to_year must never be flagged -- the resolver has no opinion on it at all");
    }

    #[test]
    fn green_when_an_event_id_in_resolved_has_no_matching_node() {
        // Disclosed scope (this function's own doc comment): a dangling
        // reference of THIS shape is every_authored_edge_resolves's own
        // concern (via dated_by.event), not duplicated here -- this law
        // must not itself panic or false-positive on it.
        let (_graph, chrono) = one_dated_event(-30);
        let empty_graph = Graph::default();
        assert!(payload_years_match_resolved_placements(&empty_graph, &chrono).is_ok());
    }
}
