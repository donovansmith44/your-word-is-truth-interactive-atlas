//! The pipeline's LAW-CHECK stage: the generic, cross-adapter referential-integrity law. Every
//! NODE-TYPED endpoint of every authored row is checked; a TextLocus-typed endpoint is not -- resolving
//! one needs a private helper, and every adapter derives its loci from real, just-built node ids.
use std::collections::BTreeSet;

use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::AnyNodeId;

#[derive(Debug, PartialEq, Eq)]
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

/// Every node-typed endpoint of every authored row resolves to a real node. Fail-loud on the FIRST
/// dangling reference, naming the relation, the field and the missing id.
pub fn every_row_reference_resolves(graph: &Graph) -> Result<(), DanglingReference> {
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
        // `ChronoTarget::Era` is deliberately excluded: the degenerate placement fallback, reachable
        // only when the anchor table is empty, mints a synthetic era id that is intentionally
        // unresolvable, and flagging it would turn a disclosed honesty gap into a build failure.
        let target = match row.placement.target() {
            ChronoTarget::Anchor(a) => Some(a.erase()),
            ChronoTarget::Prior(p) => Some(p.erase()),
            ChronoTarget::Era(_) => None,
        };
        if let Some(target) = target {
            check("dated_by", "target", target)?;
        }
    }
    for row in &graph.mentions {
        check("mentions", "entity", row.entity.node_id())?;
    }
    for row in &graph.catechism {
        check("catechism", "item", row.item.erase())?;
    }
    for row in &graph.shown {
        check("shown", "map", row.map.erase())?;
        check("shown", "node", row.node.clone())?;
    }
    for row in &graph.map_succession {
        check("map_succession", "prior", row.prior.erase())?;
        check("map_succession", "next", row.next.erase())?;
    }
    for row in &graph.comments_on {
        check("comments_on", "item", row.item.erase())?;
    }
    for row in &graph.spoken_by {
        check("spoken_by", "speaker", row.speaker.erase())?;
    }
    for row in &graph.spoken_at {
        check("spoken_at", "place", row.place.erase())?;
    }
    for row in &graph.named_after {
        let namesake_id = match &row.namesake {
            atlas_graph_types::edge::Namesake::PeopleGroup(g) => g.erase(),
            atlas_graph_types::edge::Namesake::Place(p) => p.erase(),
            atlas_graph_types::edge::Namesake::Polity(p) => p.erase(),
        };
        check("named_after", "namesake", namesake_id)?;
        check("named_after", "eponym", row.eponym.erase())?;
    }
    for row in &graph.contains_bible {
        check("contains_bible", "container", row.container.erase())?;
        if let atlas_graph_types::edge::ContainerContent::Container(child) = &row.content {
            check("contains_bible", "content.container", child.erase())?;
        }
    }
    for row in &graph.contains_concord {
        check("contains_concord", "container", row.container.erase())?;
        if let atlas_graph_types::edge::ContainerContent::Container(child) = &row.content {
            check("contains_concord", "content.container", child.erase())?;
        }
    }
    for row in &graph.canon_succession {
        check("canon_succession", "prior", row.prior.erase())?;
        check("canon_succession", "next", row.next.erase())?;
    }
    for row in &graph.analogue {
        check("analogue", "a", row.a.erase())?;
        check("analogue", "b", row.b.erase())?;
    }
    // Both ends of an `Occurs` row are node-typed once lowered: the entry, and the verse its word
    // locus lowers to.
    for row in &graph.occurs {
        check("occurs", "entry", row.entry.erase())?;
        let raw = match &row.locus.at {
            atlas_graph_types::text::TextRef::Bible(v) => format!("bible/{}.{}.{}", v.book, v.chapter, v.verse),
            atlas_graph_types::text::TextRef::Concord(c) => format!("concord/{}.{}.{}", c.part, c.article, c.paragraph),
        };
        check("occurs", "locus", AnyNodeId { kind: atlas_graph_types::id::NodeKind::TextUnit, raw })?;
    }
    for row in &graph.parent_of {
        check("parent_of", "parent", row.parent.erase())?;
        check("parent_of", "child", row.child.erase())?;
    }
    for row in &graph.spouses {
        check("spouses", "a", row.a.erase())?;
        check("spouses", "b", row.b.erase())?;
    }
    for row in &graph.brethren {
        check("brethren", "a", row.a.erase())?;
        check("brethren", "b", row.b.erase())?;
    }
    for row in &graph.participates {
        check("participates", "person", row.person.erase())?;
        check("participates", "event", row.event.erase())?;
    }
    for row in &graph.authored {
        check("authored", "book", row.book.erase())?;
        check("authored", "person", row.person.erase())?;
    }

    Ok(())
}

/// The container-containment rows must form a FOREST: every container appearing as a `Container` child
/// in either corpus has at most ONE parent, and following parents never returns to a visited
/// container. A violation is a fail-loud build failure, never shipped data.
pub fn container_containment_is_a_forest(graph: &Graph) -> Result<(), String> {
    use std::collections::BTreeMap;
    let mut edges: Vec<(&'static str, &str, &str)> = Vec::new();
    for row in &graph.contains_bible {
        if let atlas_graph_types::edge::ContainerContent::Container(child) = &row.content {
            edges.push(("contains_bible", row.container.0.as_str(), child.0.as_str()));
        }
    }
    for row in &graph.contains_concord {
        if let atlas_graph_types::edge::ContainerContent::Container(child) = &row.content {
            edges.push(("contains_concord", row.container.0.as_str(), child.0.as_str()));
        }
    }

    // child -> parent, with single-parent enforced as the map is built.
    let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
    for (relation, container, child) in edges {
        if let Some(existing) = parent.insert(child, container) {
            if existing != container {
                return Err(format!(
                    "container containment is not single-parent: '{child}' is a Container child of BOTH '{existing}' and '{container}' ({relation})"
                ));
            }
            // The same (parent, child) row twice is a duplicate edge, and cheaper to name here than to
            // let the index silently double it.
            return Err(format!(
                "duplicate container-containment row: '{container}' ⊃ '{child}' appears more than once ({relation})"
            ));
        }
    }
    // With single-parent already enforced, a walk up the parent map either terminates at a root or
    // revisits a container, and a revisit is a cycle.
    for start in parent.keys() {
        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        let mut cur: &str = start;
        seen.insert(cur);
        while let Some(next) = parent.get(cur) {
            if !seen.insert(next) {
                return Err(format!(
                    "container containment has a CYCLE reachable from '{start}' (revisited '{next}') -- containment must be a forest"
                ));
            }
            cur = next;
        }
    }
    Ok(())
}

/// An `Analogue` row asserts its two ends are DISTINCT events, so a self-loop asserts the opposite of what the
/// relation means, and a duplicate row for one unordered pair would double the symmetric edge. Kinship is
/// likewise acyclic, and no `parent-of` pair is stated twice: the source states each link from both ends.
pub fn kinship_is_acyclic(graph: &Graph) -> Result<(), String> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut seen_pairs: BTreeSet<(&str, &str)> = BTreeSet::new();
    for row in &graph.parent_of {
        let (p, c) = (row.parent.0.as_str(), row.child.0.as_str());
        if p == c {
            return Err(format!("kinship: '{p}' is stated as their own parent"));
        }
        if !seen_pairs.insert((p, c)) {
            return Err(format!("duplicate parent-of row: '{p}' -> '{c}'"));
        }
        children.entry(p).or_default().push(c);
    }
    // Colour 1 is on the stack, colour 2 is done.
    let mut colour: BTreeMap<&str, u8> = BTreeMap::new();
    for start in children.keys().copied() {
        if colour.get(start).is_some() {
            continue;
        }
        let mut stack: Vec<(&str, usize)> = vec![(start, 0)];
        colour.insert(start, 1);
        while let Some((node, idx)) = stack.last_mut() {
            let kids = children.get(node).map(|v| v.as_slice()).unwrap_or(&[]);
            if *idx < kids.len() {
                let next = kids[*idx];
                *idx += 1;
                match colour.get(next) {
                    Some(1) => return Err(format!("kinship has a CYCLE: '{next}' is an ancestor of themselves (reached again from '{node}')")),
                    Some(_) => {}
                    None => {
                        colour.insert(next, 1);
                        stack.push((next, 0));
                    }
                }
            } else {
                colour.insert(node, 2);
                stack.pop();
            }
        }
    }
    Ok(())
}

pub fn analogue_rows_join_two_distinct_events(graph: &Graph) -> Result<(), String> {
    let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
    for row in &graph.analogue {
        let (a, b) = (row.a.0.as_str(), row.b.0.as_str());
        if a == b {
            return Err(format!(
                "analogue row joins '{a}' to ITSELF -- an Analogue asserts two DISTINCT events; two accounts of ONE event are Attests rows on that one event"
            ));
        }
        let key = if a <= b { (a, b) } else { (b, a) };
        if !seen.insert(key) {
            return Err(format!(
                "duplicate analogue row: '{}' <-> '{}' appears more than once (the symmetric index would double the edge)",
                key.0, key.1
            ));
        }
    }
    Ok(())
}

/// No verse belongs to the `Attests` set of two distinct events: a shared verse fabricates a parallel, since
/// `Attests` is what the parallels capability walks. Stated against the declared inventory -- an undeclared
/// collision or a drifted count fails the build -- and a declared row pins the COUNT, not the verse SET.
pub fn attestation_is_exclusive(graph: &Graph) -> Result<(), String> {
    use std::collections::BTreeMap;

    // One `Attests` row is emitted per verse, so a range's own `from` unit IS the attested verse: no
    // range expansion is needed, or would be honest.
    let mut by_verse: BTreeMap<(u8, u16, u16), BTreeSet<&str>> = BTreeMap::new();
    for row in &graph.attests {
        let v = &row.attestation.from.unit;
        by_verse.entry((v.book, v.chapter, v.verse)).or_default().insert(row.event.0.as_str());
    }

    // Observed collisions, folded to unordered event pairs with the count of verses they truly share.
    let mut observed: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for events in by_verse.values() {
        if events.len() < 2 {
            continue;
        }
        let ids: Vec<&str> = events.iter().copied().collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                *observed.entry((ids[i], ids[j])).or_insert(0) += 1;
            }
        }
    }

    let declared: BTreeMap<(&str, &str), usize> = crate::attestation_pending::PENDING
        .iter()
        .map(|p| ((p.a, p.b), p.shared_verses))
        .collect();

    for (pair, count) in &observed {
        match declared.get(pair) {
            None => {
                return Err(format!(
                    "ATTESTATION EXCLUSIVITY (L2) VIOLATED: '{}' and '{}' both attest {} shared verse(s), and this pair is NOT in the declared pending inventory. An event's Attests edges carry ONLY narrative accounts: if one side merely REFERENCES the event, author a Mentions row instead; if the two are similar-but-distinct, author an Analogue row; if they are one event, merge them. Silently sharing an attestation fabricates a parallel account.",
                    pair.0, pair.1, count
                ));
            }
            Some(declared_count) if declared_count != count => {
                return Err(format!(
                    "ATTESTATION EXCLUSIVITY (L2) INVENTORY DRIFT: '{}' and '{}' now share {} verse(s), but attestation_pending::PENDING declares {}. Re-pin the row (and record the shrink) rather than letting the inventory drift.",
                    pair.0, pair.1, count, declared_count
                ));
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// The other direction, over the REAL corpus only: a DECLARED pair that no longer collides is a
/// failure, so the queue can only shrink deliberately. It is not part of the per-build pass because
/// every small synthetic fixture collides on nothing, which would make the whole inventory look stale.
pub fn attestation_inventory_has_no_stale_rows(graph: &Graph) -> Result<(), String> {
    use std::collections::BTreeMap;

    let mut by_verse: BTreeMap<(u8, u16, u16), BTreeSet<&str>> = BTreeMap::new();
    for row in &graph.attests {
        let v = &row.attestation.from.unit;
        by_verse.entry((v.book, v.chapter, v.verse)).or_default().insert(row.event.0.as_str());
    }
    let mut observed: BTreeSet<(&str, &str)> = BTreeSet::new();
    for events in by_verse.values() {
        if events.len() < 2 {
            continue;
        }
        let ids: Vec<&str> = events.iter().copied().collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                observed.insert((ids[i], ids[j]));
            }
        }
    }
    for p in crate::attestation_pending::PENDING {
        if !observed.contains(&(p.a, p.b)) {
            return Err(format!(
                "ATTESTATION EXCLUSIVITY (L2) STALE DECLARATION: '{}' and '{}' are declared as sharing {} verse(s) but no longer collide at all. Delete the row and record the shrink.",
                p.a, p.b, p.shared_verses
            ));
        }
    }
    Ok(())
}

/// Rebuilding the indexes from this graph's row tables alone must reproduce the indexes it serves, entry for
/// entry and in order, catching any step that writes into them outside `build_indexes` and its one row-derived
/// post-step. Not a per-build pass: that would double every build's index cost for a code-change-only risk.
pub fn indexes_derive_exactly_from_rows(graph: &Graph) -> Result<(), String> {
    use std::collections::BTreeMap;
    // The indexes are a pure function of exactly the rows and nodes cloned here.
    let mut fresh = Graph::default();
    fresh.nodes = graph.nodes.clone();
    fresh.contains_bible = graph.contains_bible.clone();
    fresh.contains_concord = graph.contains_concord.clone();
    fresh.attests = graph.attests.clone();
    fresh.succession = graph.succession.clone();
    fresh.canon_succession = graph.canon_succession.clone();
    fresh.dated_by = graph.dated_by.clone();
    fresh.located_at = graph.located_at.clone();
    fresh.fulfills = graph.fulfills.clone();
    fresh.typology = graph.typology.clone();
    fresh.named_after = graph.named_after.clone();
    fresh.catechism = graph.catechism.clone();
    fresh.comments_on = graph.comments_on.clone();
    fresh.spoken_by = graph.spoken_by.clone();
    fresh.spoken_at = graph.spoken_at.clone();
    fresh.mentions = graph.mentions.clone();
    fresh.cross_refs = graph.cross_refs.clone();
    fresh.quotes = graph.quotes.clone();
    fresh.confesses = graph.confesses.clone();
    fresh.corresponds_bible = graph.corresponds_bible.clone();
    fresh.temporal_adjacency = graph.temporal_adjacency.clone();
    fresh.analogue = graph.analogue.clone();
    fresh.occurs = graph.occurs.clone();
    fresh.parent_of = graph.parent_of.clone();
    fresh.spouses = graph.spouses.clone();
    fresh.participates = graph.participates.clone();
    fresh.authored = graph.authored.clone();
    fresh.shown = graph.shown.clone();
    fresh.map_succession = graph.map_succession.clone();
    fresh.brethren = graph.brethren.clone();
    fresh.build_indexes();
    crate::event_world::add_justified_by(&mut fresh);

    let compare = |name: &str, served: &BTreeMap<atlas_graph_types::id::Position, Vec<(atlas_graph_types::edge::EdgeId, atlas_graph_types::id::Position, atlas_graph_types::explore::EdgeMeta)>>, rebuilt: &BTreeMap<atlas_graph_types::id::Position, Vec<(atlas_graph_types::edge::EdgeId, atlas_graph_types::id::Position, atlas_graph_types::explore::EdgeMeta)>>| -> Result<(), String> {
        if served != rebuilt {
            return Err(format!("{name}: the served index diverges from a pure rebuild from rows -- something wrote into the indexes outside build_indexes/add_justified_by"));
        }
        Ok(())
    };
    for (rel, served) in &graph.indexes {
        let rebuilt = fresh.indexes.get(rel).ok_or_else(|| format!("served index for {rel:?} has no row-derived counterpart at all"))?;
        compare(&format!("indexes[{rel:?}].fwd"), &served.fwd, &rebuilt.fwd)?;
        compare(&format!("indexes[{rel:?}].inv"), &served.inv, &rebuilt.inv)?;
    }
    if graph.indexes.len() != fresh.indexes.len() {
        return Err("the rebuilt graph carries a relation index the served graph lacks".into());
    }
    for (rel, served) in &graph.symmetric_indexes {
        let rebuilt = fresh.symmetric_indexes.get(rel).ok_or_else(|| format!("served symmetric index for {rel:?} has no row-derived counterpart at all"))?;
        compare(&format!("symmetric_indexes[{rel:?}].fwd"), &served.fwd, &rebuilt.fwd)?;
    }
    if graph.symmetric_indexes.len() != fresh.symmetric_indexes.len() {
        return Err("the rebuilt graph carries a symmetric index the served graph lacks".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::edge::{Justification, LocatedAt, MentionedEntity};
    use atlas_graph_types::id::{EventId, PlaceId};
    use atlas_graph_types::node::{Node, NodePayload};

    #[test]
    fn green_on_an_empty_graph() {
        let graph = Graph::default();
        assert!(every_row_reference_resolves(&graph).is_ok());
    }

    #[test]
    fn red_when_a_person_is_stated_as_their_own_parent() {
        // Arrange
        let mut graph = Graph::default();
        graph.parent_of.push(atlas_graph_types::edge::ParentOf {
            parent: atlas_graph_types::id::PersonId::new("adam_1"),
            child: atlas_graph_types::id::PersonId::new("adam_1"),
            parentage: atlas_graph_types::edge::Parentage::Natural,
            provenance: "test".into(),
            justification: Default::default(),
        });

        // Act
        let refusal = kinship_is_acyclic(&graph).expect_err("no one is their own parent");

        // Assert
        assert_eq!(refusal, "kinship: 'adam_1' is stated as their own parent");
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
        let err = every_row_reference_resolves(&graph).expect_err("must catch the dangling reference");
        assert_eq!(err.relation, "located_at");
        assert_eq!(err.field, "event");
    }

    fn locus() -> atlas_graph_types::text::TextLocus {
        atlas_graph_types::text::TextLocus {
            at: atlas_graph_types::text::TextRef::Bible(atlas_graph_types::text::VerseRef { book: 0, chapter: 1, verse: 1 }),
            span: None,
        }
    }

    #[test]
    fn red_when_a_mentions_row_names_a_place_with_no_node() {
        let mut graph = Graph::default();
        graph.mentions.push(atlas_graph_types::edge::Mentions {
            locus: locus(),
            entity: MentionedEntity::Place(atlas_graph_types::id::PlaceId::new("nowhere")),
            provenance: "test".into(),
        });
        let err = every_row_reference_resolves(&graph).expect_err("must catch the dangling mentions.entity reference");
        assert_eq!(err.relation, "mentions");
        assert_eq!(err.field, "entity");
    }

    #[test]
    fn red_when_a_shown_row_names_a_node_the_map_has_no_node_for() {
        // Arrange
        let mut graph = Graph::default();
        let map = atlas_graph_types::id::MapId::new("era-primeval");
        graph.nodes.insert(map.erase(), Node { id: map.erase(), payload: NodePayload::Map { label: "Primeval".into(), from_year: -4004, to_year: -2167 }, provenance: "test".into() });
        graph.shown.push(atlas_graph_types::edge::Shown { map, node: PlaceId::new("nowhere").erase(), provenance: "test".into() });

        // Act
        let err = every_row_reference_resolves(&graph).expect_err("must catch the dangling shown.node reference");

        // Assert
        assert_eq!(err, DanglingReference { relation: "shown", field: "node", missing: PlaceId::new("nowhere").erase() });
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
        let err = every_row_reference_resolves(&graph).expect_err("must catch the dangling catechism.item reference");
        assert_eq!(err.relation, "catechism");
        assert_eq!(err.field, "item");
    }

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
        let err = every_row_reference_resolves(&graph).expect_err("must catch the dangling comments_on.item reference");
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

        assert!(every_row_reference_resolves(&graph).is_ok());
    }

    #[test]
    fn red_when_a_spoken_by_row_names_a_speaker_with_no_node() {
        use atlas_graph_types::text::{BibleLocusRange, VerseRef};
        let mut graph = Graph::default();
        let range = BibleLocusRange::new(atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 }), atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 })).unwrap();
        graph.spoken_by.push(atlas_graph_types::edge::SpokenBy {
            locus: range,
            speaker: atlas_graph_types::id::PersonId::new("nowhere"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        let err = every_row_reference_resolves(&graph).expect_err("must catch the dangling spoken_by.speaker reference");
        assert_eq!(err.relation, "spoken_by");
        assert_eq!(err.field, "speaker");
    }

    #[test]
    fn green_when_a_spoken_by_row_resolves_to_a_real_person_node() {
        use atlas_graph_types::id::{NodeKind, PersonId};
        use atlas_graph_types::node::{Node, NodePayload};
        use atlas_graph_types::text::{BibleLocusRange, VerseRef};

        let mut graph = Graph::default();
        let person_id = PersonId::new("jesus_905").erase();
        assert_eq!(person_id.kind, NodeKind::Person);
        graph.nodes.insert(person_id.clone(), Node { id: person_id, payload: NodePayload::Person { label: "Jesus".into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None, first_year: None, last_year: None, eternal: false, eternal_grounds: vec![] }, provenance: "test".into() });
        let range = BibleLocusRange::new(atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 }), atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 })).unwrap();
        graph.spoken_by.push(atlas_graph_types::edge::SpokenBy { locus: range, speaker: PersonId::new("jesus_905"), provenance: "test".into(), justification: Justification::default() });

        assert!(every_row_reference_resolves(&graph).is_ok());
    }

    #[test]
    fn red_when_a_spoken_at_row_names_a_place_with_no_node() {
        use atlas_graph_types::text::{BibleLocusRange, VerseRef};
        let mut graph = Graph::default();
        let range = BibleLocusRange::new(atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 }), atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 })).unwrap();
        graph.spoken_at.push(atlas_graph_types::edge::SpokenAt {
            locus: range,
            place: atlas_graph_types::id::PlaceId::new("nowhere"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        let err = every_row_reference_resolves(&graph).expect_err("must catch the dangling spoken_at.place reference");
        assert_eq!(err.relation, "spoken_at");
        assert_eq!(err.field, "place");
    }

    #[test]
    fn green_when_a_spoken_at_row_resolves_to_a_real_place_node() {
        use atlas_graph_types::id::PlaceId;
        use atlas_graph_types::node::{Node, NodePayload};
        use atlas_graph_types::text::{BibleLocusRange, VerseRef};

        let mut graph = Graph::default();
        let place_id = PlaceId::new("capernaum").erase();
        graph.nodes.insert(place_id.clone(), Node { id: place_id, payload: NodePayload::Place { canonical: "Capernaum".into(), lat: 0.0, lon: 0.0, aliases: vec![], description: None }, provenance: "test".into() });
        let range = BibleLocusRange::new(atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 }), atlas_graph_types::text::Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 })).unwrap();
        graph.spoken_at.push(atlas_graph_types::edge::SpokenAt { locus: range, place: PlaceId::new("capernaum"), provenance: "test".into(), justification: Justification::default() });

        assert!(every_row_reference_resolves(&graph).is_ok());
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
        let err = every_row_reference_resolves(&graph).expect_err("must catch the dangling named_after.namesake reference");
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
        let err = every_row_reference_resolves(&graph).expect_err("the namesake resolves; the eponym must still be caught");
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
        graph.nodes.insert(person_id.clone(), Node { id: person_id, payload: NodePayload::Person { label: "Ben-ammi".into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None, first_year: None, last_year: None, eternal: false, eternal_grounds: vec![] }, provenance: "test".into() });
        graph.named_after.push(atlas_graph_types::edge::NamedAfter {
            namesake: atlas_graph_types::edge::Namesake::PeopleGroup(PeopleGroupId::new("ammonites")),
            eponym: PersonId::new("ben-ammi_451"),
            provenance: "test".into(),
            justification: Justification::default(),
        });

        assert!(every_row_reference_resolves(&graph).is_ok());
    }

    fn authored(book: &atlas_graph_types::id::ContainerNodeId, person: &str) -> atlas_graph_types::edge::Authored {
        atlas_graph_types::edge::Authored {
            book: book.clone(),
            person: atlas_graph_types::id::PersonId::new(person),
            provenance: "test".into(),
            justification: Justification::default(),
        }
    }

    #[test]
    fn red_when_an_authored_row_names_a_person_with_no_node() {
        // Arrange
        let mut graph = Graph::default();
        let genesis = container_node(&mut graph, "bible-book-GEN");
        graph.authored.push(authored(&genesis, "nowhere"));

        // Act
        let err = every_row_reference_resolves(&graph).expect_err("must catch the dangling person");

        // Assert
        assert_eq!(
            err,
            DanglingReference { relation: "authored", field: "person", missing: atlas_graph_types::id::PersonId::new("nowhere").erase() }
        );
    }

    #[test]
    fn red_when_an_authored_row_names_a_book_with_no_node() {
        // Arrange
        let mut graph = Graph::default();
        let unbuilt = atlas_graph_types::id::ContainerNodeId::new("bible-book-XXX");
        graph.authored.push(authored(&unbuilt, "moses_2108"));

        // Act
        let err = every_row_reference_resolves(&graph).expect_err("must catch the dangling book");

        // Assert
        assert_eq!(err, DanglingReference { relation: "authored", field: "book", missing: unbuilt.erase() });
    }

    #[test]
    fn green_when_an_authored_row_resolves_to_its_book_and_its_person() {
        // Arrange
        let mut graph = Graph::default();
        let genesis = container_node(&mut graph, "bible-book-GEN");
        let moses = atlas_graph_types::id::PersonId::new("moses_2108").erase();
        graph.nodes.insert(moses.clone(), Node { id: moses, payload: NodePayload::Person { label: "Moses".into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None, first_year: None, last_year: None, eternal: false, eternal_grounds: vec![] }, provenance: "test".into() });
        graph.authored.push(authored(&genesis, "moses_2108"));

        // Act
        let verdict = every_row_reference_resolves(&graph);

        // Assert
        assert!(verdict.is_ok());
    }

    fn container_node(graph: &mut Graph, raw: &str) -> atlas_graph_types::id::ContainerNodeId {
        let id = atlas_graph_types::id::ContainerNodeId::new(raw);
        graph.nodes.insert(
            id.erase(),
            Node { id: id.erase(), payload: NodePayload::Container { title: raw.to_string() }, provenance: "test".into() },
        );
        id
    }

    fn child_row(parent: &atlas_graph_types::id::ContainerNodeId, child: &atlas_graph_types::id::ContainerNodeId) -> atlas_graph_types::edge::Contains<atlas_graph_types::text::BibleTag> {
        atlas_graph_types::edge::Contains {
            container: parent.clone(),
            content: atlas_graph_types::edge::ContainerContent::Container(child.clone()),
            provenance: "test".into(),
            justification: Justification::default(),
        }
    }

    #[test]
    fn red_when_a_contains_row_names_a_container_with_no_node() {
        let mut graph = Graph::default();
        graph.contains_bible.push(child_row(&atlas_graph_types::id::ContainerNodeId::new("bible-book-GEN"), &atlas_graph_types::id::ContainerNodeId::new("bible-chapter-GEN-1")));
        let err = every_row_reference_resolves(&graph).expect_err("must catch the dangling container");
        assert_eq!(err.relation, "contains_bible");
        assert_eq!(err.field, "container");
    }

    #[test]
    fn red_when_a_container_child_dangles_and_when_a_canon_step_dangles() {
        let mut graph = Graph::default();
        let book = container_node(&mut graph, "bible-book-GEN");
        graph.contains_bible.push(child_row(&book, &atlas_graph_types::id::ContainerNodeId::new("bible-chapter-GEN-99")));
        let err = every_row_reference_resolves(&graph).expect_err("must catch the dangling child");
        assert_eq!(err.relation, "contains_bible");
        assert_eq!(err.field, "content.container");

        let mut graph2 = Graph::default();
        let ch1 = container_node(&mut graph2, "bible-chapter-GEN-1");
        graph2.canon_succession.push(atlas_graph_types::edge::CanonSuccession {
            prior: ch1,
            next: atlas_graph_types::id::ContainerNodeId::new("bible-chapter-GEN-2"),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        let err2 = every_row_reference_resolves(&graph2).expect_err("must catch the dangling next");
        assert_eq!(err2.relation, "canon_succession");
        assert_eq!(err2.field, "next");
    }

    #[test]
    fn forest_law_green_on_a_real_forest() {
        let mut graph = Graph::default();
        let book = container_node(&mut graph, "bible-book-GEN");
        let ch1 = container_node(&mut graph, "bible-chapter-GEN-1");
        let ch2 = container_node(&mut graph, "bible-chapter-GEN-2");
        graph.contains_bible.push(child_row(&book, &ch1));
        graph.contains_bible.push(child_row(&book, &ch2));
        assert!(container_containment_is_a_forest(&graph).is_ok());
    }

    #[test]
    fn forest_law_red_on_two_parents() {
        let mut graph = Graph::default();
        let gen = container_node(&mut graph, "bible-book-GEN");
        let exo = container_node(&mut graph, "bible-book-EXO");
        let ch = container_node(&mut graph, "bible-chapter-GEN-1");
        graph.contains_bible.push(child_row(&gen, &ch));
        graph.contains_bible.push(child_row(&exo, &ch));
        let err = container_containment_is_a_forest(&graph).expect_err("two parents must be caught");
        assert!(err.contains("not single-parent"), "{err}");
    }

    #[test]
    fn forest_law_red_on_a_duplicate_row_and_on_a_cycle() {
        let mut graph = Graph::default();
        let book = container_node(&mut graph, "bible-book-GEN");
        let ch = container_node(&mut graph, "bible-chapter-GEN-1");
        graph.contains_bible.push(child_row(&book, &ch));
        graph.contains_bible.push(child_row(&book, &ch));
        let err = container_containment_is_a_forest(&graph).expect_err("a duplicate row must be caught");
        assert!(err.contains("duplicate"), "{err}");

        let mut graph2 = Graph::default();
        let a = container_node(&mut graph2, "a");
        let b = container_node(&mut graph2, "b");
        let c = container_node(&mut graph2, "c");
        graph2.contains_bible.push(child_row(&a, &b));
        graph2.contains_bible.push(child_row(&b, &c));
        graph2.contains_bible.push(child_row(&c, &a));
        let err2 = container_containment_is_a_forest(&graph2).expect_err("a cycle must be caught");
        assert!(err2.contains("CYCLE"), "{err2}");
    }

    fn event_node(graph: &mut Graph, raw: &str) -> EventId {
        let id = EventId::new(raw);
        graph.nodes.insert(
            id.erase(),
            Node {
                id: id.erase(),
                payload: NodePayload::Event { label: raw.to_string(), kind: atlas_core::data::EventKind::Event.name().to_string(), verses: vec![], witnesses: vec![], robertson_section: None, acts_section: None, atlas_section: None, kjv_superscription: None, ref_note: None },
                provenance: "test".into(),
            },
        );
        id
    }

    fn attests(event: &EventId, book: u8, chapter: u16, verse: u16) -> atlas_graph_types::edge::Attests {
        use atlas_graph_types::text::{BibleLocusRange, Locus, VerseRef};
        let l = Locus::whole(VerseRef { book, chapter, verse });
        atlas_graph_types::edge::Attests {
            event: event.clone(),
            attestation: BibleLocusRange::new(l.clone(), l).unwrap(),
            provenance: "test".into(),
            justification: Justification::default(),
        }
    }

    #[test]
    fn attestation_exclusivity_green_when_no_verse_is_shared() {
        let mut graph = Graph::default();
        let a = event_node(&mut graph, "rob_leper_healed");
        let b = event_node(&mut graph, "mat_leper_healed");
        graph.attests.push(attests(&a, 40, 1, 40));
        graph.attests.push(attests(&b, 39, 8, 1));
        assert!(attestation_is_exclusive(&graph).is_ok());
    }

    #[test]
    fn attestation_exclusivity_red_on_an_undeclared_shared_verse() {
        let mut graph = Graph::default();
        let espousal = event_node(&mut graph, "theo-249");
        let annunciation = event_node(&mut graph, "rob_annunciation_mary");
        graph.attests.push(attests(&espousal, 41, 1, 27));
        graph.attests.push(attests(&annunciation, 41, 1, 27));
        let err = attestation_is_exclusive(&graph).expect_err("a shared attestation must fail the build");
        assert!(err.contains("ATTESTATION EXCLUSIVITY (L2) VIOLATED"), "{err}");
        assert!(err.contains("theo-249") && err.contains("rob_annunciation_mary"), "the error must name BOTH events: {err}");
    }

    #[test]
    fn stale_declarations_fail_the_real_corpus_law_but_never_the_per_build_pass() {
        let first = crate::attestation_pending::PENDING.first().expect("the shipped inventory is non-empty -- if it ever empties, L2 is fully satisfied and this module should be deleted with it");
        let err = attestation_inventory_has_no_stale_rows(&Graph::default()).expect_err("a declared pair that no longer collides must fail");
        assert!(err.contains("STALE DECLARATION"), "{err}");
        assert!(err.contains(first.a), "the error must name the stale pair: {err}");

        assert!(
            attestation_is_exclusive(&Graph::default()).is_ok(),
            "the PER-BUILD pass must stay green on a collision-free fixture graph -- otherwise every pipeline test in this workspace reds on an inventory that is about the real corpus, not about them"
        );
    }

    #[test]
    fn attestation_exclusivity_red_on_inventory_drift() {
        let row = crate::attestation_pending::PENDING.first().expect("the shipped inventory is non-empty");
        let mut graph = Graph::default();
        let a = event_node(&mut graph, row.a);
        let b = event_node(&mut graph, row.b);
        assert_ne!(row.shared_verses, 1, "pick a different row: this test needs a declared count that differs from 1");
        graph.attests.push(attests(&a, 40, 1, 1));
        graph.attests.push(attests(&b, 40, 1, 1));
        let err = attestation_is_exclusive(&graph).expect_err("a drifted count must fail the build");
        assert!(err.contains("INVENTORY DRIFT"), "{err}");
    }

    #[test]
    fn analogue_gate_green_then_red_on_a_self_loop_and_on_a_duplicate() {
        use atlas_graph_types::edge::Analogue;
        let mut graph = Graph::default();
        let a = event_node(&mut graph, "rob_leper_healed");
        let b = event_node(&mut graph, "mat_leper_healed");
        graph.analogue.push(Analogue { a: a.clone(), b: b.clone(), provenance: "test".into() });
        assert!(analogue_rows_join_two_distinct_events(&graph).is_ok(), "two distinct events is the whole point");

        let mut loops = Graph::default();
        let s = event_node(&mut loops, "rob_leper_healed");
        loops.analogue.push(Analogue { a: s.clone(), b: s, provenance: "test".into() });
        let err = analogue_rows_join_two_distinct_events(&loops).expect_err("a self-loop must be caught");
        assert!(err.contains("ITSELF"), "{err}");

        let mut dupes = Graph::default();
        let x = event_node(&mut dupes, "rob_leper_healed");
        let y = event_node(&mut dupes, "mat_leper_healed");
        dupes.analogue.push(Analogue { a: x.clone(), b: y.clone(), provenance: "test".into() });
        dupes.analogue.push(Analogue { a: y, b: x, provenance: "test".into() });
        let err2 = analogue_rows_join_two_distinct_events(&dupes).expect_err("a duplicate pair must be caught");
        assert!(err2.contains("duplicate"), "{err2}");
    }

    #[test]
    fn red_when_an_analogue_or_an_event_mention_names_a_missing_node() {
        use atlas_graph_types::edge::Analogue;
        let mut graph = Graph::default();
        let a = event_node(&mut graph, "rob_leper_healed");
        graph.analogue.push(Analogue { a, b: EventId::new("nowhere"), provenance: "test".into() });
        let err = every_row_reference_resolves(&graph).expect_err("the dangling analogue end must be caught");
        assert_eq!(err.relation, "analogue");
        assert_eq!(err.field, "b");

        let mut graph2 = Graph::default();
        graph2.mentions.push(atlas_graph_types::edge::Mentions {
            locus: locus(),
            entity: MentionedEntity::Event(EventId::new("nowhere")),
            provenance: "test".into(),
        });
        let err2 = every_row_reference_resolves(&graph2).expect_err("the dangling mentions.entity Event must be caught");
        assert_eq!(err2.relation, "mentions");
        assert_eq!(err2.field, "entity");
    }

    #[test]
    fn indexes_derive_exactly_from_rows_catches_a_post_build_write() {
        let mut graph = Graph::default();
        let book = container_node(&mut graph, "bible-book-GEN");
        let ch = container_node(&mut graph, "bible-chapter-GEN-1");
        graph.contains_bible.push(child_row(&book, &ch));
        graph.build_indexes();
        assert!(indexes_derive_exactly_from_rows(&graph).is_ok(), "an untampered graph must pass");

        use atlas_graph_types::edge::{at, entry_id, RelationId};
        let s = at(&book.erase());
        let o = at(&ch.erase());
        let eid = entry_id(RelationId::Succession, &s, &o);
        graph
            .indexes
            .entry(RelationId::Succession)
            .or_default()
            .fwd
            .entry(s)
            .or_default()
            .push((eid, o, atlas_graph_types::explore::EdgeMeta::None));
        assert!(indexes_derive_exactly_from_rows(&graph).is_err(), "a post-build index write must be caught");
    }
}
