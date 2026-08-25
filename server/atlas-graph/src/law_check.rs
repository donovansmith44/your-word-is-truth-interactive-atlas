//! The pipeline's LAW-CHECK stage (pipeline.rs's own stage mapping): a
//! generic, cross-adapter REFERENTIAL-INTEGRITY law, alongside the KJV
//! adapter's own bijection/reconstruction fidelity law (fidelity.rs).
//!
//! Scope, disclosed: checks every NODE-TYPED endpoint of every authored
//! row -- Event/Narrative/Anchor/Place ids on `attests`/`succession`/
//! `dated_by`/`located_at`, (M-C2, folding M-C review M-1) `mentions.entity`
//! (`MentionedEntity`) and `catechism.item` (`CatechismItemId`), and
//! (PG-1a) `named_after`'s own `namesake` (`Namesake::{PeopleGroup, Place,
//! Polity}`) and `eponym` (`PersonId`) -- resolves to a real node in the
//! built graph. This is now every node-typed endpoint this crate's own
//! adapters emit; the
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

use atlas_graph_types::edge::MentionedEntity;
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
    // M-D3 (owner ruling R2): the `named` relation (and its own
    // `named.place` dangling-reference check, formerly here) retired whole
    // -- a place's aliases are `NodePayload::Place::aliases`'s own payload
    // field, plain strings with no node reference of their own to dangle,
    // so there is nothing left for a check at this shape to verify (the
    // vacuous branch of the ruling: "re-points... or deletes it if
    // vacuous").
    // M-C2 (folded M-C review M-1): the two node-typed endpoints this
    // law's own module doc comment previously disclosed as unchecked --
    // now real checks, not just a documented coincidence.
    for row in &graph.mentions {
        let id = match &row.entity {
            MentionedEntity::Place(p) => p.erase(),
            MentionedEntity::Person(p) => p.erase(),
            MentionedEntity::PeopleGroup(g) => g.erase(),
        };
        check("mentions", "entity", id)?;
    }
    for row in &graph.catechism {
        check("catechism", "item", row.item.erase())?;
    }
    // KRETZ-1: `comments_on`'s own node-typed endpoint (`item`) -- the SAME
    // class of check `catechism`'s own `item` gets immediately above. The
    // `on` field (a `BibleLocusRange`) is NOT checked here, matching this
    // law's own disclosed scope (module doc comment): every adapter that
    // emits a TextLocus/BibleLocusRange-shaped row already derives it
    // directly from a real, just-built TextUnit position, and `kretzmann_
    // adapter::normalize` is no exception (`VerseRef { book, chapter, verse
    // }` built straight from the SAME parsed unit the KJV TextUnit nodes
    // themselves are keyed by).
    for row in &graph.comments_on {
        check("comments_on", "item", row.item.erase())?;
    }
    // PG-1a: `named_after`'s two node-typed endpoints (`namesake`/`eponym`)
    // -- newly authored this batch, closing the SAME class of gap M-C2's
    // own `mentions`/`catechism` extension above closed first (this law's
    // own scope grows with every new node-typed authored relation, per its
    // own module doc comment).
    for row in &graph.named_after {
        let namesake_id = match &row.namesake {
            atlas_graph_types::edge::Namesake::PeopleGroup(g) => g.erase(),
            atlas_graph_types::edge::Namesake::Place(p) => p.erase(),
            atlas_graph_types::edge::Namesake::Polity(p) => p.erase(),
        };
        check("named_after", "namesake", namesake_id)?;
        check("named_after", "eponym", row.eponym.erase())?;
    }

    Ok(())
}

// M-D3 (owner ruling R1): `payload_years_match_resolved_placements` (the
// M-C2 "verified-cache law," which verified `NodePayload::Event.from_year`
// never diverged from its own resolved `dated_by` placement) RETIRES here,
// WITH the payload fields it existed to police -- `NodePayload::Event` no
// longer carries a from_year/to_year/order_key mirror at all (the crate
// patch this batch applies), so there is no longer a second, independently-
// computed copy for this law to cross-check; deletion is the stronger fix
// (a law with nothing left to guard is not a law, it is dead weight). The
// deleted code (struct, fn, and its own tests) is recoverable from git
// history at the commit immediately preceding this one -- see the batch
// report for the exact SHA, the P7 way (no runtime flag).

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
            entity: MentionedEntity::Place(atlas_graph_types::id::PlaceId::new("nowhere")),
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

    /// KRETZ-1: the SAME red-then-green shape as `catechism` immediately
    /// above, for `comments_on`'s own node-typed `item` endpoint.
    #[test]
    fn red_when_a_comments_on_row_names_an_item_with_no_node() {
        use atlas_graph_types::text::{BibleLocusRange, VerseRef};
        let mut graph = Graph::default();
        let range = BibleLocusRange::new(atlas_graph_types::text::Locus::whole(VerseRef { book: 0, chapter: 1, verse: 1 }), atlas_graph_types::text::Locus::whole(VerseRef { book: 0, chapter: 1, verse: 1 })).unwrap();
        graph.comments_on.push(atlas_graph_types::edge::CommentsOn {
            item: atlas_graph_types::id::CommentaryItemId::new("nowhere"),
            on: range,
            provenance: "test".into(),
            justification: Justification::default(),
        });
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling comments_on.item reference");
        assert_eq!(err.relation, "comments_on");
        assert_eq!(err.field, "item");
    }

    #[test]
    fn green_when_a_comments_on_row_resolves_to_a_real_commentary_item_node() {
        use atlas_graph_types::id::{CommentaryItemId, NodeKind, SourceId};
        use atlas_graph_types::node::{Node, NodePayload};
        use atlas_graph_types::text::{BibleLocusRange, VerseRef};

        let mut graph = Graph::default();
        let item_id = CommentaryItemId::new("kretzmann/0.1.0").erase();
        assert_eq!(item_id.kind, NodeKind::CommentaryItem);
        graph.nodes.insert(item_id.clone(), Node { id: item_id.clone(), payload: NodePayload::CommentaryItem { work: SourceId::new("kretzmann-popular-commentary"), heading: None, text: "prose".into() }, provenance: "test".into() });
        let range = BibleLocusRange::new(atlas_graph_types::text::Locus::whole(VerseRef { book: 0, chapter: 1, verse: 1 }), atlas_graph_types::text::Locus::whole(VerseRef { book: 0, chapter: 1, verse: 1 })).unwrap();
        graph.comments_on.push(atlas_graph_types::edge::CommentsOn { item: CommentaryItemId::new("kretzmann/0.1.0"), on: range, provenance: "test".into(), justification: Justification::default() });

        assert!(every_authored_edge_resolves(&graph).is_ok());
    }

    #[test]
    fn red_when_a_named_after_row_names_a_namesake_with_no_node() {
        let mut graph = Graph::default();
        graph.named_after.push(atlas_graph_types::edge::NamedAfter {
            namesake: atlas_graph_types::edge::Namesake::PeopleGroup(atlas_graph_types::id::PeopleGroupId::new("nowhere")),
            eponym: atlas_graph_types::id::PersonId::new("also-nowhere"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        // Both endpoints dangle; the first-checked field (`namesake`) is
        // what the error names, the same "first field wins" convention
        // `red_when_a_located_at_row_names_a_place_with_no_node` proves.
        let err = every_authored_edge_resolves(&graph).expect_err("must catch the dangling named_after.namesake reference");
        assert_eq!(err.relation, "named_after");
        assert_eq!(err.field, "namesake");
    }

    #[test]
    fn red_when_a_named_after_row_names_an_eponym_with_no_node() {
        use atlas_graph_types::id::PeopleGroupId;
        use atlas_graph_types::node::{Node, NodePayload};

        let mut graph = Graph::default();
        let group_id = PeopleGroupId::new("ammonites").erase();
        graph.nodes.insert(group_id.clone(), Node { id: group_id, payload: NodePayload::PeopleGroup { label: "Ammonites".into(), description: None }, provenance: "test".into() });
        graph.named_after.push(atlas_graph_types::edge::NamedAfter {
            namesake: atlas_graph_types::edge::Namesake::PeopleGroup(PeopleGroupId::new("ammonites")),
            eponym: atlas_graph_types::id::PersonId::new("nowhere"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        let err = every_authored_edge_resolves(&graph).expect_err("the namesake resolves; the eponym must still be caught");
        assert_eq!(err.relation, "named_after");
        assert_eq!(err.field, "eponym");
    }

    #[test]
    fn green_when_mentions_catechism_and_named_after_rows_resolve_to_real_nodes() {
        use atlas_graph_types::id::{NodeKind, PeopleGroupId, PersonId, PlaceId};
        use atlas_graph_types::node::{Node, NodePayload};

        let mut graph = Graph::default();
        let place_id = PlaceId::new("hebron").erase();
        graph.nodes.insert(
            place_id.clone(),
            Node { id: place_id.clone(), payload: NodePayload::Place { canonical: "Hebron".into(), lat: 0.0, lon: 0.0, aliases: vec![], description: None }, provenance: "test".into() },
        );
        graph.mentions.push(atlas_graph_types::edge::Mentions { locus: locus(), entity: MentionedEntity::Place(PlaceId::new("hebron")), provenance: "test".into() });

        let item_id = atlas_graph_types::id::CatechismItemId::new("commandment-1").erase();
        assert_eq!(item_id.kind, NodeKind::CatechismItem);
        graph.nodes.insert(item_id.clone(), Node { id: item_id.clone(), payload: NodePayload::CatechismItem { label: "The First Commandment".into() }, provenance: "test".into() });
        graph.catechism.push(atlas_graph_types::edge::CatechismLink {
            locus: locus(),
            item: atlas_graph_types::id::CatechismItemId::new("commandment-1"),
            provenance: "test".into(),
            justification: Justification::default(),
        });

        let group_id = PeopleGroupId::new("ammonites").erase();
        graph.nodes.insert(group_id.clone(), Node { id: group_id, payload: NodePayload::PeopleGroup { label: "Ammonites".into(), description: None }, provenance: "test".into() });
        let person_id = PersonId::new("ben-ammi_451").erase();
        graph.nodes.insert(person_id.clone(), Node { id: person_id, payload: NodePayload::Person { label: "Ben-ammi".into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None }, provenance: "test".into() });
        graph.named_after.push(atlas_graph_types::edge::NamedAfter {
            namesake: atlas_graph_types::edge::Namesake::PeopleGroup(PeopleGroupId::new("ammonites")),
            eponym: PersonId::new("ben-ammi_451"),
            provenance: "test".into(),
            justification: Justification::default(),
        });

        assert!(every_authored_edge_resolves(&graph).is_ok());
    }

    // M-D3: the former "verified-cache law" tests (`one_dated_event` +
    // four `payload_years_match_resolved_placements` cases) are deleted
    // alongside the function itself -- see this file's own retirement note
    // above.
}
