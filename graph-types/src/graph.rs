//! The graph: per-relation typed tables (the tables ARE the indexes),
//! per-corpus reading spines, bidirectional indexes built in one pass.

use std::collections::BTreeMap;

use crate::edge::{
    at, Attests, BiIndex, CatechismLink, CommentsOn, Confesses, Contains, Corresponds, CrossRef,
    Fulfills,
    LocatedAt, MentionedEntity, Mentions, NamedAfter, Namesake, Quotes, RelationId,
    Succession, TemporalAdjacency, Typology,
};
use crate::chrono::DatedBy;
use crate::id::{AnyNodeId, NodeKind, Position};
use crate::node::Node;
use crate::text::{BibleTag, Corpus, TextLocus, TextRef};

/// Reading order: each corpus's spine — a total order per skeleton,
/// window-queried, NOT a paged relation (deliberate manifest exclusion).
#[derive(Debug, Default)]
pub struct ReadingSpine {
    pub order: Vec<AnyNodeId>, // canonical order of materialized units
}

#[derive(Debug, Default)]
pub struct Graph {
    pub nodes: BTreeMap<AnyNodeId, Node>,

    // -------- authored --------
    pub contains_bible: Vec<Contains<BibleTag>>,
    pub contains_concord: Vec<Contains<crate::text::ConcordTag>>,
    pub attests: Vec<Attests>,
    pub succession: Vec<Succession>,
    pub dated_by: Vec<DatedBy>,
    pub located_at: Vec<LocatedAt>,
    pub fulfills: Vec<Fulfills>,
    pub typology: Vec<Typology>,
    pub named_after: Vec<NamedAfter>,
    pub catechism: Vec<CatechismLink>,
    /// KRETZ-1: verse-anchored commentary targets (annotation shape;
    /// the units themselves are CommentaryItem nodes).
    pub comments_on: Vec<CommentsOn>,
    // -------- imported --------
    pub mentions: Vec<Mentions>,
    pub cross_refs: Vec<CrossRef>,
    pub quotes: Vec<Quotes>,
    pub confesses: Vec<Confesses>,
    pub corresponds_bible: Vec<Corresponds<BibleTag>>,
    /// TRAV-1: machine-DERIVED at compile from `temporal_order`
    /// (consecutive pairs) -- lives with imported rows because the ETL
    /// authors it, not a human.
    pub temporal_adjacency: Vec<TemporalAdjacency>,

    // -------- spines & indexes (built, never authored) --------
    pub reading: BTreeMap<&'static str, ReadingSpine>,
    pub indexes: BTreeMap<RelationId, BiIndex>,
    /// M-C: the symmetric sibling of `indexes` -- closes the "Symmetric
    /// relations: skeleton serves none yet" gap `explore.rs`'s own
    /// `raw_neighbors` has documented since M-A (a disclosed, standing gap,
    /// not a law; M-B's own report named it explicitly for
    /// `temporal-adjacency`). Built the SAME way (one `BiIndex::
    /// build_symmetric` pass per inhabited `SymRelationId`, from the row
    /// tables below) -- `catechism-link` populated it first, and TRAV-1's
    /// own `temporal-adjacency` rows CLOSED the M-B gap by joining it.
    pub symmetric_indexes: BTreeMap<crate::edge::SymRelationId, BiIndex>,
    /// pid -> node id, built alongside the other indexes: derive() is a
    /// lookup, not a scan (same derived-state class as `indexes`;
    /// content addressing makes it deterministic).
    pub pid_index: BTreeMap<crate::id::Pid, AnyNodeId>,
}

fn text_node(kind_hint: &TextLocus) -> AnyNodeId {
    // Skeleton mapping from a locus to its unit node id.
    let raw = match &kind_hint.at {
        TextRef::Bible(v) => format!("bible/{}.{}.{}", v.book, v.chapter, v.verse),
        TextRef::Concord(c) => format!("concord/{}.{}.{}", c.part, c.article, c.paragraph),
    };
    AnyNodeId { kind: NodeKind::TextUnit, raw }
}

impl Graph {
    /// Build every bidirectional index from the row tables — one pass
    /// per relation; both directions are projections of the same rows.
    /// Also builds the pid index (derive() as lookup).
    pub fn build_indexes(&mut self) {
        use crate::id::ContentAddressed;
        self.pid_index = self
            .nodes
            .values()
            .map(|n| (n.pid(), n.id.clone()))
            .collect();

        use RelationId as R;

        use crate::explore::EdgeMeta as M;
        let mut pairs: BTreeMap<RelationId, Vec<(Position, Position, M)>> = BTreeMap::new();

        for row in &self.contains_bible {
            let c = at(&row.container.erase());
            for l in &row.content.0 {
                let tl: TextLocus = l.clone().into();
                pairs.entry(R::Contains).or_default().push((c.clone(), at(&text_node(&tl)), M::None));
            }
        }
        // CORP-2a: the Concord sibling of the `contains_bible` loop
        // immediately above -- SAME shape, the OTHER `Contains<C>`
        // instantiation this struct carries (graph.rs's own field-level
        // doc comment: "authored"). Declared alongside `contains_bible`
        // since M-A but never lowered into `pairs` until now: `Contains<
        // ConcordTag>` had no real caller before `concord_adapter.rs`
        // (this batch's own first-real-caller note, matching artifact.rs's
        // identical "deliberately incomplete until a real caller arrives"
        // shape for `concord_locus_to_dto`) -- both are precedented,
        // mechanical completions of an already-declared field, not a new
        // relation or a type-shape change.
        for row in &self.contains_concord {
            let c = at(&row.container.erase());
            for l in &row.content.0 {
                let tl: TextLocus = l.clone().into();
                pairs.entry(R::Contains).or_default().push((c.clone(), at(&text_node(&tl)), M::None));
            }
        }
        for row in &self.attests {
            let e = at(&row.event.erase());
            let tl: TextLocus = row.attestation.from.clone().into();
            pairs.entry(R::Attests).or_default().push((e, at(&text_node(&tl)), M::None));
        }
        for row in &self.succession {
            for w in row.chain.windows(2) {
                pairs.entry(R::Succession).or_default().push((
                    at(&w[0].erase()),
                    at(&w[1].erase()),
                    M::Narrative(row.narrative.clone()),
                ));
            }
        }
        for row in &self.dated_by {
            let e = at(&row.event.erase());
            let t = match row.placement.target() {
                crate::chrono::ChronoTarget::Anchor(a) => at(&a.erase()),
                crate::chrono::ChronoTarget::Prior(p) => at(&p.erase()),
                crate::chrono::ChronoTarget::Era(er) => at(&er.erase()),
            };
            pairs.entry(R::DatedBy).or_default().push((e, t, M::None));
        }
        for row in &self.located_at {
            pairs.entry(R::LocatedAt).or_default().push((
                at(&row.event.erase()),
                at(&row.place.erase()),
                M::None,
            ));
        }
        for row in &self.named_after {
            let s = match &row.namesake {
                Namesake::PeopleGroup(g) => at(&g.erase()),
                Namesake::Place(p) => at(&p.erase()),
                Namesake::Polity(p) => at(&p.erase()),
            };
            pairs.entry(R::NamedAfter).or_default().push((s, at(&row.eponym.erase()), M::None));
        }
        for row in &self.mentions {
            let s = at(&text_node(&row.locus));
            let o = match &row.entity {
                MentionedEntity::Place(p) => at(&p.erase()),
                MentionedEntity::Person(p) => at(&p.erase()),
                MentionedEntity::PeopleGroup(g) => at(&g.erase()),
            };
            pairs.entry(R::Mentions).or_default().push((s, o, M::None));
        }
        for row in &self.cross_refs {
            pairs.entry(R::Cites).or_default().push((
                at(&text_node(&row.from)),
                at(&text_node(&row.to)),
                M::Votes(row.votes),
            ));
        }
        for row in &self.quotes {
            let s = at(&text_node(&row.quoting));
            let tl: TextLocus = row.quoted.from.clone().into();
            pairs.entry(R::Quotes).or_default().push((s, at(&text_node(&tl)), M::None));
        }
        for row in &self.confesses {
            let s: TextLocus = row.confessing.clone().into();
            let o: TextLocus = row.confessed.from.clone().into();
            pairs.entry(R::Confesses).or_default().push((
                at(&text_node(&s)),
                at(&text_node(&o)),
                M::None,
            ));
        }
        // EDGE-1: prophecy/fulfillment and typology lower exactly like
        // the other text-to-text relations -- edge endpoint is each
        // range's FIRST verse (the cites/quotes/confesses precedent);
        // the full ranges stay on the rows for display.
        for row in &self.fulfills {
            let s: TextLocus = row.prophecy.from.clone().into();
            let o: TextLocus = row.fulfillment.from.clone().into();
            pairs.entry(R::Fulfillment).or_default().push((
                at(&text_node(&s)),
                at(&text_node(&o)),
                M::None,
            ));
        }
        for row in &self.typology {
            let s: TextLocus = row.type_passage.from.clone().into();
            let o: TextLocus = row.antitype_passage.from.clone().into();
            pairs.entry(R::Typology).or_default().push((
                at(&text_node(&s)),
                at(&text_node(&o)),
                M::None,
            ));
        }
        // M-D3 (owner ruling R2) closed M-C's long-disclosed `named` shape
        // gap by RETIRING the relation: a `Named` row's object was a bare
        // `String` with no `Position` representation, so the relation
        // could never lower into `pairs` -- and the serving path was
        // always `NodePayload::Place`'s own `aliases` payload field
        // (node.rs). Manifest row, row struct, and the `graph.named`
        // table are gone; aliases remain a fact ABOUT the place.

        self.indexes = pairs
            .into_iter()
            .map(|(rel, ps)| (rel, BiIndex::build(rel, &ps)))
            .collect();

        // M-C: the symmetric sibling of the directed pass above -- closes
        // the "Symmetric relations: skeleton serves none yet" gap
        // (`explore.rs`'s own `raw_neighbors`, disclosed since M-A).
        // `catechism` (`CatechismLink { locus: TextLocus, item:
        // CatechismItemId, .. }`) is the first symmetric relation with
        // real rows to index; both ends resolve to real `Position`s (a
        // TextUnit and a CatechismItem node respectively), unlike
        // `named`'s own bare-string object (see this function's own note
        // above) -- there is nothing blocking this one.
        use crate::edge::SymRelationId as S;
        let mut sym_pairs: BTreeMap<S, Vec<(Position, Position, M)>> = BTreeMap::new();
        for row in &self.catechism {
            let locus = at(&text_node(&row.locus));
            let item = at(&row.item.erase());
            sym_pairs.entry(S::CatechismLink).or_default().push((locus, item, M::None));
        }
        // TRAV-1: the second inhabited symmetric relation -- the exact
        // `temporal-adjacency` gap the doc comment above carried since
        // M-B, now closed. Rows are compile-derived (see the struct's
        // own doc); the symmetric index serves "adjacent-in-time"
        // traversal both ways, and the honest `earlier`/`later` row
        // ends carry direction for the Chronology block's display.
        for row in &self.temporal_adjacency {
            sym_pairs.entry(S::TemporalAdjacency).or_default().push((
                at(&row.earlier.erase()),
                at(&row.later.erase()),
                M::None,
            ));
        }
        self.symmetric_indexes = sym_pairs
            .into_iter()
            .map(|(rel, ps)| (rel, BiIndex::build_symmetric(rel, &ps)))
            .collect();
    }

    /// Windowed reading query — the reader's only primitive. Any
    /// partition into windows concatenates to the same sequence.
    pub fn reading_window(
        &self,
        corpus: &'static str,
        start: usize,
        n: usize,
    ) -> Vec<AnyNodeId> {
        self.reading
            .get(corpus)
            .map(|s| s.order.iter().skip(start).take(n).cloned().collect())
            .unwrap_or_default()
    }
}

/// Marker: corpus id helper for spines.
pub fn corpus_key<C: Corpus>() -> &'static str {
    C::ID
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge::{Contains, Direction, EdgeKind, RelationId};
    use crate::explore::{EdgeQuery, Explorable, PositionRef};
    use crate::id::{ContainerNodeId, NodeKind};
    use crate::ingest::ProvenanceId;
    use crate::node::{Node, NodePayload};
    use crate::text::{ConcordRef, ConcordTag, Locus, LocusSet, TranslationId};
    use std::collections::BTreeSet;

    /// CORP-2a: `contains_concord` was declared on this struct since M-A
    /// but never lowered into `pairs` until this batch (see this file's
    /// own `build_indexes` doc comment on the loop immediately after
    /// `contains_bible`'s) -- this proves the wiring, mirroring the
    /// `contains_bible` shape exactly: a container's own forward
    /// "contains" frontier lists its member paragraphs, and each
    /// paragraph's inverse "member-of" frontier lists the SAME container
    /// back, under the SAME EdgeId either way (the bijection witness).
    #[test]
    fn contains_concord_rows_lower_into_the_directed_contains_index_both_ways() {
        let mut g = Graph::default();
        let container_id = ContainerNodeId::new("concord-ac-iv");
        let p1 = ConcordRef { part: 3, article: 4, paragraph: 1 };
        let p2 = ConcordRef { part: 3, article: 4, paragraph: 2 };
        for p in [&p1, &p2] {
            let raw = format!("concord/{}.{}.{}", p.part, p.article, p.paragraph);
            g.nodes.insert(
                crate::id::AnyNodeId { kind: NodeKind::TextUnit, raw: raw.clone() },
                Node {
                    id: crate::id::AnyNodeId { kind: NodeKind::TextUnit, raw },
                    payload: NodePayload::TextUnit { corpus: "concord", renderings: [(TranslationId("bente-dau".into()), "text".into())].into_iter().collect() },
                    provenance: "test".into(),
                },
            );
        }
        g.nodes.insert(
            container_id.erase(),
            Node { id: container_id.erase(), payload: NodePayload::Container { title: "Article IV. Of Justification.".into() }, provenance: "test".into() },
        );
        let mut content: BTreeSet<Locus<ConcordTag>> = BTreeSet::new();
        content.insert(Locus::whole(p1.clone()));
        content.insert(Locus::whole(p2.clone()));
        g.contains_concord.push(Contains { container: container_id.clone(), content: LocusSet(content), provenance: ProvenanceId::from("test"), justification: Default::default() });

        g.build_indexes();

        let forward = EdgeKind::Directed(RelationId::Contains, Direction::Forward);
        let page = PositionRef(crate::id::Position::Node(container_id.erase())).edges(&g, &EdgeQuery { kind: forward, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 2, "the container's own forward 'contains' frontier lists both paragraphs");

        let p1_node = crate::id::AnyNodeId { kind: NodeKind::TextUnit, raw: "concord/3.4.1".into() };
        let inverse = EdgeKind::Directed(RelationId::Contains, Direction::Inverse);
        let back = PositionRef(crate::id::Position::Node(p1_node.clone())).edges(&g, &EdgeQuery { kind: inverse, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 1, "the paragraph's own inverse 'member-of' frontier lists its container");
        assert_eq!(back.entries[0].node, crate::id::Position::Node(container_id.erase()));

        let from_container_entry = page.entries.iter().find(|e| e.node == crate::id::Position::Node(p1_node.clone())).expect("the container's own page must list paragraph 1");
        assert_eq!(from_container_entry.edge, back.entries[0].edge, "the SAME edge id, from either end -- the bijection witness");
    }
}
