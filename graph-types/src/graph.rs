//! The graph: per-relation typed tables (the tables ARE the indexes),
//! per-corpus reading spines, bidirectional indexes built in one pass.

use std::collections::BTreeMap;

use crate::edge::{
    at, Attests, BiIndex, CatechismLink, Confesses, Contains, Corresponds, CrossRef, Fulfills,
    LocatedAt, MentionedEntity, Mentions, NamedAfter, Namesake, Quotes, RelationId,
    Succession, Typology,
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
    // -------- imported --------
    pub mentions: Vec<Mentions>,
    pub cross_refs: Vec<CrossRef>,
    pub quotes: Vec<Quotes>,
    pub confesses: Vec<Confesses>,
    pub corresponds_bible: Vec<Corresponds<BibleTag>>,

    // -------- spines & indexes (built, never authored) --------
    pub reading: BTreeMap<&'static str, ReadingSpine>,
    pub indexes: BTreeMap<RelationId, BiIndex>,
    /// M-C: the symmetric sibling of `indexes` -- closes the "Symmetric
    /// relations: skeleton serves none yet" gap `explore.rs`'s own
    /// `raw_neighbors` has documented since M-A (a disclosed, standing gap,
    /// not a law; M-B's own report named it explicitly for
    /// `temporal-adjacency`). Built the SAME way (one `BiIndex::
    /// build_symmetric` pass per inhabited `SymRelationId`, from the row
    /// tables below) -- `catechism-link` is the first relation to actually
    /// populate it.
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
