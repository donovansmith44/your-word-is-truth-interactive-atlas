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
use atlas_graph_types::id::AnyNodeId;

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
}
