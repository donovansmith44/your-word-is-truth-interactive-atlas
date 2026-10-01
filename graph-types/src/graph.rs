//! The graph: per-relation typed tables (the tables ARE the indexes),
//! per-corpus reading spines, bidirectional indexes built in one pass.

use std::collections::BTreeMap;

use crate::edge::{
    at, Analogue, Attests, Authored, BiIndex, CanonSuccession, CatechismLink, CommentsOn, Confesses, ContainerContent, Contains, Corresponds, CrossRef,
    SpokenAt, SpokenBy,
    Fulfills,
    LocatedAt, MapSuccession, Mentions, NamedAfter, Namesake, Occurs, ParentOf, Participates, Spouses, Quotes, RelationId, Brethren,
    Shown, Succession, TemporalAdjacency, Typology,
};
use crate::chrono::DatedBy;
use crate::id::{AnyNodeId, NodeKind, Position};
use crate::node::Node;
use crate::text::{BibleTag, Corpus, TextLocus, TextRef};

/// A total order per corpus, window-queried rather than paged as a relation.
#[derive(Debug, Default)]
pub struct ReadingSpine {
    pub order: Vec<AnyNodeId>,
}

#[derive(Debug, Default)]
pub struct Graph {
    pub nodes: BTreeMap<AnyNodeId, Node>,

    // Authored.
    pub contains_bible: Vec<Contains<BibleTag>>,
    pub contains_concord: Vec<Contains<crate::text::ConcordTag>>,
    pub attests: Vec<Attests>,
    pub succession: Vec<Succession>,
    /// Pairwise canon chapter and book steps: the second row family lowering into the same
    /// succession relation.
    pub canon_succession: Vec<CanonSuccession>,
    pub dated_by: Vec<DatedBy>,
    pub located_at: Vec<LocatedAt>,
    pub fulfills: Vec<Fulfills>,
    pub typology: Vec<Typology>,
    pub named_after: Vec<NamedAfter>,
    pub catechism: Vec<CatechismLink>,
    /// Verse-anchored commentary targets; the commentary's own prose is a node payload.
    pub comments_on: Vec<CommentsOn>,
    pub spoken_by: Vec<SpokenBy>,
    pub spoken_at: Vec<SpokenAt>,
    // Imported.
    pub mentions: Vec<Mentions>,
    pub cross_refs: Vec<CrossRef>,
    pub quotes: Vec<Quotes>,
    pub confesses: Vec<Confesses>,
    pub corresponds_bible: Vec<Corresponds<BibleTag>>,
    /// Derived at compile time from consecutive pairs of the temporal order, so it sits with
    /// the imported rows: the pipeline authors it, never a curator.
    pub temporal_adjacency: Vec<TemporalAdjacency>,
    /// Distinct events whose accounts are similar in form or content, never two accounts of
    /// one event. Authored, never derived: a similarity metric mistook one leper healing for
    /// another, which is why no metric mints these.
    pub analogue: Vec<Analogue>,
    /// One row per aligned original-language token, in canonical reading order, which is what
    /// makes a concordance page read in canon order by construction and nothing else.
    pub occurs: Vec<Occurs>,
    /// One row per (parent, child) pair.
    pub parent_of: Vec<ParentOf>,
    pub spouses: Vec<Spouses>,
    pub participates: Vec<Participates>,
    pub authored: Vec<Authored>,
    pub shown: Vec<Shown>,
    pub map_succession: Vec<MapSuccession>,
    pub brethren: Vec<Brethren>,

    // Built, never authored.
    pub reading: BTreeMap<&'static str, ReadingSpine>,
    /// The canonical bodies of the section tables that are not derived from the nodes, rows
    /// and spines here. The compiler supplies them and the artifact load attaches them again
    /// from the same files, so the version root covers them on both sides.
    pub extra_tables: BTreeMap<&'static str, Vec<Vec<u8>>>,
    pub indexes: BTreeMap<RelationId, BiIndex>,
    /// Built exactly as `indexes` is, one pass per inhabited symmetric relation.
    pub symmetric_indexes: BTreeMap<crate::edge::SymRelationId, BiIndex>,
    /// Built alongside the other indexes so resolving a pid is a lookup, not a scan.
    pub pid_index: BTreeMap<crate::id::Pid, AnyNodeId>,
    /// Sorted by `(hash, row_ord)` for binary search. EVERY row is kept: two rows sharing one
    /// (relation, subject, object) mint one id, and both sources must stay reachable behind it.
    pub edge_rows: Vec<EdgeRow>,
    /// Resolving a unit's position is a lookup instead of a scan of the spine.
    pub spine_index: BTreeMap<&'static str, BTreeMap<AnyNodeId, usize>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EdgeRow {
    pub hash: crate::id::ContentHash,
    pub family: crate::canon::RowFamily,
    pub row_ord: u32,
}

/// `None` for any other string: the id grammar belongs to whoever mints ids, not to this.
pub fn edge_hash(e: &crate::edge::EdgeId) -> Option<crate::id::ContentHash> {
    let (_, hex) = e.0.split_once(':')?;
    crate::id::ContentHash::from_hex(hex)
}

/// Which relation a row family lowers into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeRel {
    Directed(RelationId),
    Symmetric(crate::edge::SymRelationId),
}

/// `row_ord` is the row's position in its family's vector. A set-valued row -- one naming N
/// loci, or a chain of N events -- yields N entries all carrying the same `row_ord`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowEdge {
    pub family: crate::canon::RowFamily,
    pub row_ord: usize,
    pub rel: EdgeRel,
    pub subject: Position,
    pub object: Position,
    pub meta: crate::adjacency::EdgeMeta,
}

fn text_node(kind_hint: &TextLocus) -> AnyNodeId {
    let raw = match &kind_hint.at {
        TextRef::Bible(v) => format!("bible/{}.{}.{}", v.book, v.chapter, v.verse),
        TextRef::Concord(c) => format!("concord/{}.{}.{}", c.part, c.article, c.paragraph),
    };
    AnyNodeId { kind: NodeKind::TextUnit, raw }
}

impl Graph {
    /// The one row-to-edge lowering, and its order is load-bearing: index order is cursor
    /// order is pinned response bytes, so a new family is appended LAST and no earlier entry
    /// moves. This is not `RowFamily::ALL` order.
    pub fn row_edges(&self) -> Vec<RowEdge> {
        use crate::canon::RowFamily;
        use crate::edge::SymRelationId as S;
        use crate::adjacency::EdgeMeta as M;
        use RelationId as R;

        fn push_edge(out: &mut Vec<RowEdge>, family: RowFamily, row_ord: usize, rel: EdgeRel, (subject, object, meta): (Position, Position, M)) {
            out.push(RowEdge { family, row_ord, rel, subject, object, meta });
        }

        let mut out: Vec<RowEdge> = Vec::new();
        for (i, row) in self.contains_bible.iter().enumerate() {
            let c = at(&row.container.erase());
            match &row.content {
                ContainerContent::Loci(set) => {
                    for l in &set.0 {
                        let tl: TextLocus = l.clone().into();
                        push_edge(&mut out, RowFamily::ContainsBible, i, EdgeRel::Directed(R::Contains), (c.clone(), at(&text_node(&tl)), M::None));
                    }
                }
                ContainerContent::Container(child) => {
                    push_edge(&mut out, RowFamily::ContainsBible, i, EdgeRel::Directed(R::Contains), (c, at(&child.erase()), M::None));
                }
            }
        }
        for (i, row) in self.contains_concord.iter().enumerate() {
            let c = at(&row.container.erase());
            match &row.content {
                ContainerContent::Loci(set) => {
                    for l in &set.0 {
                        let tl: TextLocus = l.clone().into();
                        push_edge(&mut out, RowFamily::ContainsConcord, i, EdgeRel::Directed(R::Contains), (c.clone(), at(&text_node(&tl)), M::None));
                    }
                }
                ContainerContent::Container(child) => {
                    push_edge(&mut out, RowFamily::ContainsConcord, i, EdgeRel::Directed(R::Contains), (c, at(&child.erase()), M::None));
                }
            }
        }
        for (i, row) in self.attests.iter().enumerate() {
            let e = at(&row.event.erase());
            let tl: TextLocus = row.attestation.from.clone().into();
            push_edge(&mut out, RowFamily::Attests, i, EdgeRel::Directed(R::Attests), (e, at(&text_node(&tl)), M::None));
        }
        for (i, row) in self.succession.iter().enumerate() {
            for w in row.chain.windows(2) {
                push_edge(&mut out, RowFamily::Succession, i, EdgeRel::Directed(R::Succession), (
                    at(&w[0].erase()),
                    at(&w[1].erase()),
                    M::Narrative(row.narrative.clone()),
                ));
            }
        }
        // No narrative meta: the canon order is itself the chain, so a canon step belongs to
        // no narrative.
        for (i, row) in self.canon_succession.iter().enumerate() {
            push_edge(&mut out, RowFamily::CanonSuccession, i, EdgeRel::Directed(R::Succession), (
                at(&row.prior.erase()),
                at(&row.next.erase()),
                M::None,
            ));
        }
        for (i, row) in self.dated_by.iter().enumerate() {
            let e = at(&row.event.erase());
            let t = match row.placement.target() {
                crate::chrono::ChronoTarget::Anchor(a) => at(&a.erase()),
                crate::chrono::ChronoTarget::Prior(p) => at(&p.erase()),
                crate::chrono::ChronoTarget::Era(er) => at(&er.erase()),
            };
            push_edge(&mut out, RowFamily::DatedBy, i, EdgeRel::Directed(R::DatedBy), (e, t, M::None));
        }
        for (i, row) in self.comments_on.iter().enumerate() {
            let item = at(&row.item.erase());
            let tl: TextLocus = row.on.from.clone().into();
            push_edge(&mut out, RowFamily::CommentsOn, i, EdgeRel::Directed(R::CommentsOn), (item, at(&text_node(&tl)), M::None));
        }
        for (i, row) in self.spoken_by.iter().enumerate() {
            let tl: TextLocus = row.locus.from.clone().into();
            push_edge(&mut out, RowFamily::SpokenBy, i, EdgeRel::Directed(R::SpokenBy), (at(&text_node(&tl)), at(&row.speaker.erase()), M::None));
        }
        for (i, row) in self.spoken_at.iter().enumerate() {
            let tl: TextLocus = row.locus.from.clone().into();
            push_edge(&mut out, RowFamily::SpokenAt, i, EdgeRel::Directed(R::SpokenAt), (at(&text_node(&tl)), at(&row.place.erase()), M::None));
        }
        for (i, row) in self.located_at.iter().enumerate() {
            push_edge(&mut out, RowFamily::LocatedAt, i, EdgeRel::Directed(R::LocatedAt), (
                at(&row.event.erase()),
                at(&row.place.erase()),
                M::None,
            ));
        }
        for (i, row) in self.named_after.iter().enumerate() {
            let s = match &row.namesake {
                Namesake::PeopleGroup(g) => at(&g.erase()),
                Namesake::Place(p) => at(&p.erase()),
                Namesake::Polity(p) => at(&p.erase()),
            };
            push_edge(&mut out, RowFamily::NamedAfter, i, EdgeRel::Directed(R::NamedAfter), (s, at(&row.eponym.erase()), M::None));
        }
        for (i, row) in self.mentions.iter().enumerate() {
            let s = at(&text_node(&row.locus));
            push_edge(&mut out, RowFamily::Mentions, i, EdgeRel::Directed(R::Mentions), (s, at(&row.entity.node_id()), M::None));
        }
        for (i, row) in self.cross_refs.iter().enumerate() {
            push_edge(&mut out, RowFamily::CrossRefs, i, EdgeRel::Directed(R::Cites), (
                at(&text_node(&row.from)),
                at(&text_node(&row.to)),
                M::Votes(row.votes),
            ));
        }
        for (i, row) in self.quotes.iter().enumerate() {
            let s = at(&text_node(&row.quoting));
            let tl: TextLocus = row.quoted.from.clone().into();
            push_edge(&mut out, RowFamily::Quotes, i, EdgeRel::Directed(R::Quotes), (s, at(&text_node(&tl)), M::None));
        }
        for (i, row) in self.confesses.iter().enumerate() {
            let s: TextLocus = row.confessing.clone().into();
            let o: TextLocus = row.confessed.from.clone().into();
            push_edge(&mut out, RowFamily::Confesses, i, EdgeRel::Directed(R::Confesses), (
                at(&text_node(&s)),
                at(&text_node(&o)),
                M::None,
            ));
        }
        for (i, row) in self.fulfills.iter().enumerate() {
            let s: TextLocus = row.prophecy.from.clone().into();
            let o: TextLocus = row.fulfillment.from.clone().into();
            push_edge(&mut out, RowFamily::Fulfills, i, EdgeRel::Directed(R::Fulfillment), (
                at(&text_node(&s)),
                at(&text_node(&o)),
                M::None,
            ));
        }
        for (i, row) in self.typology.iter().enumerate() {
            let s: TextLocus = row.type_passage.from.clone().into();
            let o: TextLocus = row.antitype_passage.from.clone().into();
            push_edge(&mut out, RowFamily::Typology, i, EdgeRel::Directed(R::Typology), (
                at(&text_node(&s)),
                at(&text_node(&o)),
                M::None,
            ));
        }

        for (i, row) in self.catechism.iter().enumerate() {
            let locus = at(&text_node(&row.locus));
            let item = at(&row.item.erase());
            push_edge(&mut out, RowFamily::Catechism, i, EdgeRel::Symmetric(S::CatechismLink), (locus, item, M::None));
        }
        for (i, row) in self.temporal_adjacency.iter().enumerate() {
            push_edge(&mut out, RowFamily::TemporalAdjacency, i, EdgeRel::Symmetric(S::TemporalAdjacency), (
                at(&row.earlier.erase()),
                at(&row.later.erase()),
                M::None,
            ));
        }
        for (i, row) in self.analogue.iter().enumerate() {
            push_edge(&mut out, RowFamily::Analogue, i, EdgeRel::Symmetric(S::Analogue), (
                at(&row.a.erase()),
                at(&row.b.erase()),
                M::None,
            ));
        }
        // A word locus lowers to its VERSE node, as every sub-verse locus does; the token
        // itself lives in the row.
        for (i, row) in self.occurs.iter().enumerate() {
            push_edge(&mut out, RowFamily::Occurs, i, EdgeRel::Directed(RelationId::Occurs), (
                at(&row.entry.erase()),
                at(&text_node(&row.locus)),
                M::None,
            ));
        }
        for (i, row) in self.parent_of.iter().enumerate() {
            push_edge(&mut out, RowFamily::ParentOf, i, EdgeRel::Directed(RelationId::ParentOf), (
                at(&row.parent.erase()),
                at(&row.child.erase()),
                M::Parentage(row.parentage),
            ));
        }
        for (i, row) in self.spouses.iter().enumerate() {
            push_edge(&mut out, RowFamily::Spouses, i, EdgeRel::Symmetric(S::Spouses), (
                at(&row.a.erase()),
                at(&row.b.erase()),
                M::None,
            ));
        }
        for (i, row) in self.brethren.iter().enumerate() {
            push_edge(&mut out, RowFamily::Brethren, i, EdgeRel::Symmetric(S::Brethren), (
                at(&row.a.erase()),
                at(&row.b.erase()),
                M::None,
            ));
        }
        for (i, row) in self.participates.iter().enumerate() {
            push_edge(&mut out, RowFamily::Participates, i, EdgeRel::Directed(RelationId::Participates), (
                at(&row.person.erase()),
                at(&row.event.erase()),
                M::None,
            ));
        }
        for (i, row) in self.authored.iter().enumerate() {
            push_edge(&mut out, RowFamily::Authored, i, EdgeRel::Directed(RelationId::AuthoredBy), (
                at(&row.book.erase()),
                at(&row.person.erase()),
                M::None,
            ));
        }
        for (i, row) in self.shown.iter().enumerate() {
            push_edge(&mut out, RowFamily::Shown, i, EdgeRel::Directed(RelationId::Shows), (
                at(&row.map.erase()),
                at(&row.node),
                M::None,
            ));
        }
        for (i, row) in self.map_succession.iter().enumerate() {
            push_edge(&mut out, RowFamily::MapSuccession, i, EdgeRel::Directed(RelationId::Succession), (
                at(&row.prior.erase()),
                at(&row.next.erase()),
                M::None,
            ));
        }
        out
    }

    /// So no caller re-spells the directed and symmetric id functions.
    pub fn edge_id_of(e: &RowEdge) -> crate::edge::EdgeId {
        match e.rel {
            EdgeRel::Directed(r) => crate::edge::entry_id(r, &e.subject, &e.object),
            EdgeRel::Symmetric(s) => crate::edge::entry_id_symmetric(s, &e.subject, &e.object),
        }
    }

    /// In `(family, row_ord)` order; empty for an unknown or synthesised id.
    pub fn rows_of_edge(&self, e: &crate::edge::EdgeId) -> &[EdgeRow] {
        let Some(h) = edge_hash(e) else { return &[] };
        let start = self.edge_rows.partition_point(|r| r.hash < h);
        let end = start + self.edge_rows[start..].partition_point(|r| r.hash == h);
        &self.edge_rows[start..end]
    }

    pub fn edge_row(&self, e: &crate::edge::EdgeId) -> Option<EdgeRow> {
        self.rows_of_edge(e).first().copied()
    }

    pub fn row_provenance_of(&self, family: crate::canon::RowFamily, row_ord: usize) -> Option<&str> {
        use crate::canon::RowFamily as F;
        match family {
            F::ContainsBible => self.contains_bible.get(row_ord).map(|r| r.provenance.as_str()),
            F::ContainsConcord => self.contains_concord.get(row_ord).map(|r| r.provenance.as_str()),
            F::Attests => self.attests.get(row_ord).map(|r| r.provenance.as_str()),
            F::Succession => self.succession.get(row_ord).map(|r| r.provenance.as_str()),
            F::CanonSuccession => self.canon_succession.get(row_ord).map(|r| r.provenance.as_str()),
            F::DatedBy => self.dated_by.get(row_ord).map(|r| r.provenance.as_str()),
            F::LocatedAt => self.located_at.get(row_ord).map(|r| r.provenance.as_str()),
            F::Fulfills => self.fulfills.get(row_ord).map(|r| r.provenance.as_str()),
            F::Typology => self.typology.get(row_ord).map(|r| r.provenance.as_str()),
            F::NamedAfter => self.named_after.get(row_ord).map(|r| r.provenance.as_str()),
            F::Catechism => self.catechism.get(row_ord).map(|r| r.provenance.as_str()),
            F::CommentsOn => self.comments_on.get(row_ord).map(|r| r.provenance.as_str()),
            F::SpokenBy => self.spoken_by.get(row_ord).map(|r| r.provenance.as_str()),
            F::SpokenAt => self.spoken_at.get(row_ord).map(|r| r.provenance.as_str()),
            F::Mentions => self.mentions.get(row_ord).map(|r| r.provenance.as_str()),
            F::CrossRefs => self.cross_refs.get(row_ord).map(|r| r.provenance.as_str()),
            F::Quotes => self.quotes.get(row_ord).map(|r| r.provenance.as_str()),
            F::Confesses => self.confesses.get(row_ord).map(|r| r.provenance.as_str()),
            F::CorrespondsBible => self.corresponds_bible.get(row_ord).map(|r| r.provenance.as_str()),
            F::TemporalAdjacency => self.temporal_adjacency.get(row_ord).map(|r| r.provenance.as_str()),
            F::Analogue => self.analogue.get(row_ord).map(|r| r.provenance.as_str()),
            F::Occurs => self.occurs.get(row_ord).map(|r| r.provenance.as_str()),
            F::ParentOf => self.parent_of.get(row_ord).map(|r| r.provenance.as_str()),
            F::Spouses => self.spouses.get(row_ord).map(|r| r.provenance.as_str()),
            F::Participates => self.participates.get(row_ord).map(|r| r.provenance.as_str()),
            F::Authored => self.authored.get(row_ord).map(|r| r.provenance.as_str()),
            F::Shown => self.shown.get(row_ord).map(|r| r.provenance.as_str()),
            F::MapSuccession => self.map_succession.get(row_ord).map(|r| r.provenance.as_str()),
            F::Brethren => self.brethren.get(row_ord).map(|r| r.provenance.as_str()),
        }
    }

    /// One pass per relation: both directions are projections of the same rows.
    /// The work after lowering is parallel because every hash is a pure function of its own
    /// input, no relation's rows depend on another's, and no node depends on any edge.
    pub fn build_indexes(&mut self) {
        use crate::id::ContentAddressed;

        use RelationId as R;
        use crate::edge::SymRelationId as S;

        use crate::adjacency::EdgeMeta as M;
        let mut pairs: BTreeMap<RelationId, Vec<(Position, Position, M)>> = BTreeMap::new();
        let mut sym_pairs: BTreeMap<S, Vec<(Position, Position, M)>> = BTreeMap::new();
        let edges = self.row_edges();
        let mut edge_rows: Vec<EdgeRow> = {
            let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
            let chunk = edges.len().div_ceil(n).max(1);
            std::thread::scope(|scope| {
                let handles: Vec<_> = edges
                    .chunks(chunk)
                    .map(|c| {
                        scope.spawn(move || {
                            c.iter()
                                .filter_map(|e| edge_hash(&Graph::edge_id_of(e)).map(|hash| EdgeRow { hash, family: e.family, row_ord: e.row_ord as u32 }))
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect();
                handles.into_iter().flat_map(|h| h.join().expect("an edge-row chunk never panics")).collect()
            })
        };
        edge_rows.sort_unstable_by_key(|r| (r.hash, r.family, r.row_ord));
        edge_rows.dedup();
        self.edge_rows = edge_rows;
        for e in edges {
            match e.rel {
                EdgeRel::Directed(r) => pairs.entry(r).or_default().push((e.subject, e.object, e.meta)),
                EdgeRel::Symmetric(s) => sym_pairs.entry(s).or_default().push((e.subject, e.object, e.meta)),
            }
        }
        self.spine_index = self
            .reading
            .iter()
            .map(|(corpus, spine)| (*corpus, spine.order.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect()))
            .collect();

        let n_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);

        // Read-only over the nodes, touching none of the row tables the edge chunks read, and
        // computed outside the scope below so it outlives every spawned thread.
        let node_refs: Vec<(&AnyNodeId, &Node)> = self.nodes.iter().collect();
        let node_chunk_size = node_refs.len().div_ceil(n_threads).max(1);

        // Sized off the total row count, so the largest relation splits into roughly one
        // chunk per thread and every smaller one gets fewer.
        let total_edge_rows: usize = pairs.values().map(|v| v.len()).sum::<usize>() + sym_pairs.values().map(|v| v.len()).sum::<usize>();
        let edge_chunk_size = total_edge_rows.div_ceil(n_threads).max(1);

        let (pid_index, indexes, symmetric_indexes) = std::thread::scope(|scope| {
            let pid_handles: Vec<_> = node_refs
                .chunks(node_chunk_size)
                .map(|chunk| scope.spawn(move || chunk.iter().map(|(_, n)| (n.pid(), n.id.clone())).collect::<BTreeMap<_, _>>()))
                .collect();

            // Pushed relation by relation and, within a relation, strictly in row order:
            // exactly the order a single sequential pass would have visited them, whichever
            // thread finishes first.
            let mut directed_chunks: Vec<(R, &[(Position, Position, M)])> = Vec::new();
            for (rel, ps) in &pairs {
                if ps.len() <= edge_chunk_size {
                    directed_chunks.push((*rel, ps.as_slice()));
                } else {
                    for c in ps.chunks(edge_chunk_size) {
                        directed_chunks.push((*rel, c));
                    }
                }
            }
            let directed_handles: Vec<_> = directed_chunks.into_iter().map(|(rel, ps)| scope.spawn(move || (rel, BiIndex::build(rel, ps)))).collect();

            let mut sym_chunks: Vec<(S, &[(Position, Position, M)])> = Vec::new();
            for (rel, ps) in &sym_pairs {
                if ps.len() <= edge_chunk_size {
                    sym_chunks.push((*rel, ps.as_slice()));
                } else {
                    for c in ps.chunks(edge_chunk_size) {
                        sym_chunks.push((*rel, c));
                    }
                }
            }
            let sym_handles: Vec<_> = sym_chunks.into_iter().map(|(rel, ps)| scope.spawn(move || (rel, BiIndex::build_symmetric(rel, ps)))).collect();

            // One pid per node, so merge order carries no meaning here: a plain union.
            let pid_index: BTreeMap<crate::id::Pid, AnyNodeId> = pid_handles.into_iter().flat_map(|h| h.join().expect("pid-index worker panicked").into_iter()).collect();

            // Merged in order, by appending and never re-sorting, so a key touched by two
            // chunks keeps the per-key order the sequential pass produced.
            let mut indexes: BTreeMap<R, BiIndex> = BTreeMap::new();
            for h in directed_handles {
                let (rel, partial) = h.join().expect("index-build worker panicked");
                let entry = indexes.entry(rel).or_default();
                for (k, v) in partial.fwd {
                    entry.fwd.entry(k).or_default().append(v);
                }
                for (k, v) in partial.inv {
                    entry.inv.entry(k).or_default().append(v);
                }
            }

            let mut symmetric_indexes: BTreeMap<S, BiIndex> = BTreeMap::new();
            for h in sym_handles {
                let (rel, partial) = h.join().expect("symmetric index-build worker panicked");
                let entry = symmetric_indexes.entry(rel).or_default();
                for (k, v) in partial.fwd {
                    entry.fwd.entry(k).or_default().append(v);
                }
                // Always empty for a symmetric index -- merged anyway rather than assumed.
                for (k, v) in partial.inv {
                    entry.inv.entry(k).or_default().append(v);
                }
            }

            (pid_index, indexes, symmetric_indexes)
        });

        self.pid_index = pid_index;
        self.indexes = indexes;
        self.symmetric_indexes = symmetric_indexes;
    }

    /// The reader's only primitive: any partition into windows concatenates to the same
    /// sequence.
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

pub fn corpus_key<C: Corpus>() -> &'static str {
    C::ID
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge::{Contains, Direction, EdgeKind, RelationId};
    use crate::adjacency::{EdgeQuery, Adjacent, PositionRef};
    use crate::id::{ContainerNodeId, NodeKind};
    use crate::ingest::ProvenanceId;
    use crate::node::{Node, NodePayload};
    use crate::text::{ConcordRef, ConcordTag, Locus, LocusSet, TranslationId};
    use std::collections::BTreeSet;

    #[test]
    fn parallel_build_indexes_matches_sequential_over_a_large_relation() {
        use crate::edge::Justification;
        use crate::adjacency::EdgeMeta as M;
        use crate::id::EventId;
        use crate::text::{BibleLocusRange, VerseRef};

        let hub = EventId::new("hub-event".to_string());
        let mut g = Graph::default();
        for i in 0..300u16 {
            let v = VerseRef { book: 0, chapter: 1, verse: i + 1 };
            let attestation = BibleLocusRange::new(Locus::whole(v.clone()), Locus::whole(v)).expect("a single-verse range must construct");
            g.attests.push(Attests { event: hub.clone(), attestation, provenance: ProvenanceId::from("test"), justification: Justification::default() });
        }

        let pairs: Vec<(Position, Position, M)> = g
            .attests
            .iter()
            .map(|row| {
                let tl: TextLocus = row.attestation.from.clone().into();
                (at(&row.event.clone().erase()), at(&text_node(&tl)), M::None)
            })
            .collect();
        assert_eq!(pairs.len(), 300, "sanity: every real Attests row must lower to exactly one pair");

        let sequential = crate::edge::BiIndex::build(RelationId::Attests, &pairs);

        g.build_indexes();
        let merged = g.indexes.get(&RelationId::Attests).expect("build_indexes must populate a real index for a relation with real rows");

        assert_eq!(merged.fwd.len(), sequential.fwd.len());
        assert_eq!(merged.inv.len(), sequential.inv.len());
        for (k, v) in &sequential.fwd {
            assert_eq!(merged.fwd.get(k), Some(v), "fwd entry for {k:?} must match the sequential build, IN ORDER");
        }
        for (k, v) in &sequential.inv {
            assert_eq!(merged.inv.get(k), Some(v), "inv entry for {k:?} must match the sequential build, IN ORDER");
        }
    }

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
        g.contains_concord.push(Contains { container: container_id.clone(), content: ContainerContent::Loci(LocusSet(content)), provenance: ProvenanceId::from("test"), justification: Default::default() });

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

    #[test]
    fn container_child_and_canon_succession_rows_lower_into_the_existing_indexes_both_ways() {
        let mut g = Graph::default();
        let book = ContainerNodeId::new("bible-book-GEN");
        let ch1 = ContainerNodeId::new("bible-chapter-GEN-1");
        let ch2 = ContainerNodeId::new("bible-chapter-GEN-2");
        for c in [&book, &ch1, &ch2] {
            g.nodes.insert(
                c.erase(),
                Node { id: c.erase(), payload: NodePayload::Container { title: "t".into() }, provenance: "test".into() },
            );
        }
        g.contains_bible.push(Contains {
            container: book.clone(),
            content: ContainerContent::Container(ch1.clone()),
            provenance: ProvenanceId::from("test"),
            justification: Default::default(),
        });
        g.contains_bible.push(Contains {
            container: book.clone(),
            content: ContainerContent::Container(ch2.clone()),
            provenance: ProvenanceId::from("test"),
            justification: Default::default(),
        });
        g.canon_succession.push(crate::edge::CanonSuccession {
            prior: ch1.clone(),
            next: ch2.clone(),
            provenance: ProvenanceId::from("test"),
            justification: Default::default(),
        });

        g.build_indexes();

        let forward = EdgeKind::Directed(RelationId::Contains, Direction::Forward);
        let page = PositionRef(crate::id::Position::Node(book.erase())).edges(&g, &EdgeQuery { kind: forward, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 2, "the book's own forward 'contains' frontier lists both chapter containers");
        let inverse = EdgeKind::Directed(RelationId::Contains, Direction::Inverse);
        let back = PositionRef(crate::id::Position::Node(ch1.erase())).edges(&g, &EdgeQuery { kind: inverse, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 1, "a chapter is a member of exactly its book");
        assert_eq!(back.entries[0].node, crate::id::Position::Node(book.erase()));
        let fwd_entry = page.entries.iter().find(|e| e.node == crate::id::Position::Node(ch1.erase())).expect("book page lists chapter 1");
        assert_eq!(fwd_entry.edge, back.entries[0].edge, "the SAME edge id, from either end -- the bijection witness");

        let follows = EdgeKind::Directed(RelationId::Succession, Direction::Forward);
        let succ = PositionRef(crate::id::Position::Node(ch1.erase())).edges(&g, &EdgeQuery { kind: follows, cursor: None, limit: 10 });
        assert_eq!(succ.entries.len(), 1);
        assert_eq!(succ.entries[0].node, crate::id::Position::Node(ch2.erase()));
        assert_eq!(succ.entries[0].meta, crate::adjacency::EdgeMeta::None, "a canon step carries no narrative annotation");
        let precedes = EdgeKind::Directed(RelationId::Succession, Direction::Inverse);
        let prev = PositionRef(crate::id::Position::Node(ch2.erase())).edges(&g, &EdgeQuery { kind: precedes, cursor: None, limit: 10 });
        assert_eq!(prev.entries.len(), 1);
        assert_eq!(prev.entries[0].node, crate::id::Position::Node(ch1.erase()));
        assert_eq!(prev.entries[0].edge, succ.entries[0].edge);
    }

    #[test]
    fn comments_on_rows_lower_into_the_directed_index_both_ways() {
        use crate::edge::CommentsOn;
        use crate::id::CommentaryItemId;
        use crate::text::{BibleLocusRange, VerseRef};

        let mut g = Graph::default();
        let verse_id = crate::id::AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/0.1.2".into() };
        g.nodes.insert(
            verse_id.clone(),
            Node {
                id: verse_id.clone(),
                payload: NodePayload::TextUnit { corpus: "bible", renderings: [(TranslationId("kjv".into()), "text".into())].into_iter().collect() },
                provenance: "test".into(),
            },
        );
        let item_id = CommentaryItemId::new("kretzmann/0.1.0");
        g.nodes.insert(
            item_id.erase(),
            Node { id: item_id.erase(), payload: NodePayload::CommentaryItem { work: crate::id::SourceId::new("kretzmann-popular-commentary"), heading: None, text: "prose".into() }, provenance: "test".into() },
        );
        let range = BibleLocusRange::new(Locus::whole(VerseRef { book: 0, chapter: 1, verse: 2 }), Locus::whole(VerseRef { book: 0, chapter: 1, verse: 2 })).unwrap();
        g.comments_on.push(CommentsOn { item: item_id.clone(), on: range, provenance: ProvenanceId::from("test"), justification: Default::default() });

        g.build_indexes();

        let forward = EdgeKind::Directed(RelationId::CommentsOn, Direction::Forward);
        let page = PositionRef(crate::id::Position::Node(item_id.erase())).edges(&g, &EdgeQuery { kind: forward, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 1, "the CommentaryItem's own forward 'comments-on' frontier reaches its verse");
        assert_eq!(page.entries[0].node, crate::id::Position::Node(verse_id.clone()));

        let inverse = EdgeKind::Directed(RelationId::CommentsOn, Direction::Inverse);
        let back = PositionRef(crate::id::Position::Node(verse_id)).edges(&g, &EdgeQuery { kind: inverse, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 1, "the verse's own inverse 'commented-on-by' frontier lists the CommentaryItem back");
        assert_eq!(back.entries[0].edge, page.entries[0].edge, "the SAME edge id, from either end -- the bijection witness");
    }

    #[test]
    fn spoken_by_and_spoken_at_rows_lower_into_the_directed_index_both_ways() {
        use crate::edge::{SpokenAt, SpokenBy};
        use crate::id::{PersonId, PlaceId};
        use crate::text::{BibleLocusRange, VerseRef};

        let mut g = Graph::default();
        let verse_id = crate::id::AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/39.4.19".into() };
        g.nodes.insert(
            verse_id.clone(),
            Node {
                id: verse_id.clone(),
                payload: NodePayload::TextUnit { corpus: "bible", renderings: [(TranslationId("kjv".into()), "Follow me".into())].into_iter().collect() },
                provenance: "test".into(),
            },
        );
        let jesus_id = PersonId::new("jesus_905");
        g.nodes.insert(jesus_id.erase(), Node { id: jesus_id.erase(), payload: NodePayload::Person { label: "Jesus".into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None, first_year: None, last_year: None, eternal: false, eternal_grounds: vec![] }, provenance: "test".into() });
        let place_id = PlaceId::new("sea-of-galilee");
        g.nodes.insert(place_id.erase(), Node { id: place_id.erase(), payload: NodePayload::Place { canonical: "Sea of Galilee".into(), lat: 0.0, lon: 0.0, aliases: vec![], description: None }, provenance: "test".into() });

        let range = BibleLocusRange::new(Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 }), Locus::whole(VerseRef { book: 39, chapter: 4, verse: 19 })).unwrap();
        g.spoken_by.push(SpokenBy { locus: range.clone(), speaker: jesus_id.clone(), provenance: ProvenanceId::from("test"), justification: Default::default() });
        g.spoken_at.push(SpokenAt { locus: range, place: place_id.clone(), provenance: ProvenanceId::from("test"), justification: Default::default() });

        g.build_indexes();

        let forward_by = EdgeKind::Directed(RelationId::SpokenBy, Direction::Forward);
        let page = PositionRef(crate::id::Position::Node(verse_id.clone())).edges(&g, &EdgeQuery { kind: forward_by, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 1, "the verse's own forward 'spoken-by' frontier reaches Jesus");
        assert_eq!(page.entries[0].node, crate::id::Position::Node(jesus_id.erase()));

        let inverse_by = EdgeKind::Directed(RelationId::SpokenBy, Direction::Inverse);
        let back = PositionRef(crate::id::Position::Node(jesus_id.erase())).edges(&g, &EdgeQuery { kind: inverse_by, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 1, "Jesus's own inverse 'speech-of' frontier lists the verse back");
        assert_eq!(back.entries[0].edge, page.entries[0].edge, "the SAME edge id, from either end -- the bijection witness");

        let forward_at = EdgeKind::Directed(RelationId::SpokenAt, Direction::Forward);
        let at_page = PositionRef(crate::id::Position::Node(verse_id.clone())).edges(&g, &EdgeQuery { kind: forward_at, cursor: None, limit: 10 });
        assert_eq!(at_page.entries.len(), 1, "the verse's own forward 'spoken-at' frontier reaches the place");
        assert_eq!(at_page.entries[0].node, crate::id::Position::Node(place_id.erase()));

        let inverse_at = EdgeKind::Directed(RelationId::SpokenAt, Direction::Inverse);
        let at_back = PositionRef(crate::id::Position::Node(place_id.erase())).edges(&g, &EdgeQuery { kind: inverse_at, cursor: None, limit: 10 });
        assert_eq!(at_back.entries.len(), 1, "the place's own inverse 'site-of-speech' frontier lists the verse back");
        assert_eq!(at_back.entries[0].edge, at_page.entries[0].edge, "the SAME edge id, from either end -- the bijection witness");
    }

    #[test]
    fn analogue_rows_and_event_mentions_lower_into_the_existing_indexes_both_ways() {
        use crate::edge::{Analogue, Mentions, MentionedEntity, SymRelationId};
        use crate::id::EventId;

        let mut g = Graph::default();
        let a = EventId::new("rob_leper_healed");
        let b = EventId::new("mat_leper_healed");
        for e in [&a, &b] {
            g.nodes.insert(
                e.erase(),
                Node {
                    id: e.erase(),
                    payload: NodePayload::Event { label: e.0.clone(), kind: "event".into(), verses: vec![], witnesses: vec![], robertson_section: None, acts_section: None, atlas_section: None, kjv_superscription: None, ref_note: None },
                    provenance: "test".into(),
                },
            );
        }
        let verse_id = crate::id::AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/41.1.27".into() };
        g.nodes.insert(
            verse_id.clone(),
            Node { id: verse_id.clone(), payload: NodePayload::TextUnit { corpus: "bible", renderings: [(TranslationId("kjv".into()), "To a virgin espoused to a man".into())].into_iter().collect() }, provenance: "test".into() },
        );
        let espousal = EventId::new("theo-249");
        g.nodes.insert(
            espousal.erase(),
            Node { id: espousal.erase(), payload: NodePayload::Event { label: "Espousal of Mary".into(), kind: "event".into(), verses: vec![], witnesses: vec![], robertson_section: None, acts_section: None, atlas_section: None, kjv_superscription: None, ref_note: None }, provenance: "test".into() },
        );

        g.analogue.push(Analogue { a: a.clone(), b: b.clone(), provenance: ProvenanceId::from("test") });
        g.mentions.push(Mentions {
            locus: TextLocus { at: TextRef::Bible(crate::text::VerseRef { book: 41, chapter: 1, verse: 27 }), span: None },
            entity: MentionedEntity::Event(espousal.clone()),
            provenance: ProvenanceId::from("test"),
        });

        g.build_indexes();

        let sym = EdgeKind::Symmetric(SymRelationId::Analogue);
        let from_a = PositionRef(crate::id::Position::Node(a.erase())).edges(&g, &EdgeQuery { kind: sym, cursor: None, limit: 10 });
        assert_eq!(from_a.entries.len(), 1, "the first event's own 'analogous-to' frontier reaches the second");
        assert_eq!(from_a.entries[0].node, crate::id::Position::Node(b.erase()));
        let from_b = PositionRef(crate::id::Position::Node(b.erase())).edges(&g, &EdgeQuery { kind: sym, cursor: None, limit: 10 });
        assert_eq!(from_b.entries.len(), 1, "and the second's reaches the first -- symmetric, no direction to get backwards");
        assert_eq!(from_b.entries[0].node, crate::id::Position::Node(a.erase()));
        assert_eq!(from_a.entries[0].edge, from_b.entries[0].edge, "the SAME edge id from either end -- the symmetric bijection witness");

        let fwd = EdgeKind::Directed(RelationId::Mentions, Direction::Forward);
        let verse_side = PositionRef(crate::id::Position::Node(verse_id.clone())).edges(&g, &EdgeQuery { kind: fwd, cursor: None, limit: 10 });
        assert_eq!(verse_side.entries.len(), 1, "the verse's own forward 'mentions' frontier reaches the event");
        assert_eq!(verse_side.entries[0].node, crate::id::Position::Node(espousal.erase()));
        let inv = EdgeKind::Directed(RelationId::Mentions, Direction::Inverse);
        let event_side = PositionRef(crate::id::Position::Node(espousal.erase())).edges(&g, &EdgeQuery { kind: inv, cursor: None, limit: 10 });
        assert_eq!(event_side.entries.len(), 1, "the EVENT's own inverse 'mentioned-in' frontier lists the verse back -- L3's mention-only frontier");
        assert_eq!(event_side.entries[0].node, crate::id::Position::Node(verse_id));
        assert_eq!(event_side.entries[0].edge, verse_side.entries[0].edge, "the SAME edge id, from either end");
    }

    #[test]
    fn a_parent_of_row_carries_its_parentage_on_both_ends() {
        // Arrange
        let mut g = Graph::default();
        let (father, son) = (crate::id::PersonId::new("god_1324"), crate::id::PersonId::new("jesus_905"));
        g.parent_of.push(crate::edge::ParentOf {
            parent: father.clone(),
            child: son.clone(),
            parentage: crate::edge::Parentage::Eternal,
            provenance: ProvenanceId::from("test"),
            justification: Default::default(),
        });
        g.build_indexes();
        let query = |kind| EdgeQuery { kind, cursor: None, limit: 10 };
        // Act
        let children = PositionRef(crate::id::Position::Node(father.erase())).edges(&g, &query(EdgeKind::Directed(RelationId::ParentOf, Direction::Forward)));
        let parents = PositionRef(crate::id::Position::Node(son.erase())).edges(&g, &query(EdgeKind::Directed(RelationId::ParentOf, Direction::Inverse)));
        // Assert
        assert_eq!(
            (children.entries.iter().map(|e| e.meta.clone()).collect::<Vec<_>>(), parents.entries.iter().map(|e| e.meta.clone()).collect::<Vec<_>>()),
            (vec![crate::adjacency::EdgeMeta::Parentage(crate::edge::Parentage::Eternal)], vec![crate::adjacency::EdgeMeta::Parentage(crate::edge::Parentage::Eternal)])
        );
    }
}

#[cfg(test)]
mod row_edge_laws {
    use super::*;
    use crate::canon::RowFamily;
    use crate::edge::{Analogue, Attests, EdgeId, Justification, LocatedAt, Succession};
    use crate::id::{EventId, NarrativeId, PlaceId};
    use crate::text::{Locus, LocusRange, VerseRef};

    fn g() -> Graph {
        let mut g = Graph::default();
        let e1 = EventId::new("e1");
        let e2 = EventId::new("e2");
        let p = PlaceId::new("p");
        g.located_at.push(LocatedAt { event: e1.clone(), place: p.clone(), provenance: "prov".into(), justification: Justification::default() });
        g.located_at.push(LocatedAt { event: e2.clone(), place: p.clone(), provenance: "prov".into(), justification: Justification::default() });
        g.succession.push(Succession::new(NarrativeId::new("n"), vec![e1.clone(), e2.clone()], "prov".into(), Justification::default()).unwrap());
        g.analogue.push(Analogue { a: e1.clone(), b: e2.clone(), provenance: "prov".into() });
        let from = Locus::<BibleTag> { unit: VerseRef { book: 1, chapter: 1, verse: 1 }, span: None };
        let to = Locus::<BibleTag> { unit: VerseRef { book: 1, chapter: 1, verse: 3 }, span: None };
        g.attests.push(Attests { event: e1, attestation: LocusRange::new(from, to).unwrap(), provenance: "prov".into(), justification: Justification::default() });
        g
    }

    #[test]
    fn an_occurs_row_lowers_to_one_entry_to_verse_edge_appended_last() {
        use crate::edge::{Occurs, RelationId};
        use crate::graph::EdgeRel;
        use crate::id::{AnyNodeId, LexiconEntryId, NodeKind};
        use crate::text::{TextLocus, TextRef, TokenSpan, TranslationId};
        let mut g = g();
        let before = g.row_edges().len();
        g.occurs.push(Occurs {
            entry: LexiconEntryId::new("G3056"),
            locus: TextLocus { at: TextRef::Bible(VerseRef { book: 42, chapter: 1, verse: 1 }), span: Some(TokenSpan::new(TranslationId("greek_textus_receptus".into()), 7, 7).unwrap()) },
            provenance: "stepbible-tagnt".into(),
        });
        let edges = g.row_edges();
        assert_eq!(edges.len(), before + 1);
        let e = edges.last().unwrap();
        assert_eq!(e.family, crate::canon::RowFamily::Occurs);
        assert_eq!(e.row_ord, 0);
        assert_eq!(e.rel, EdgeRel::Directed(RelationId::Occurs));
        assert_eq!(e.subject, crate::edge::at(&AnyNodeId { kind: NodeKind::LexiconEntry, raw: "G3056".into() }));
        assert_eq!(e.object, crate::edge::at(&AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/42.1.1".into() }), "the word locus lowers to its VERSE node; the token stays in the row");
        assert_eq!(g.row_provenance_of(crate::canon::RowFamily::Occurs, 0), Some("stepbible-tagnt"));
    }

    #[test]
    fn every_row_edge_names_its_row_and_its_family_relation() {
        let g = g();
        let edges = g.row_edges();
        assert_eq!(edges.len(), 2 + 1 + 1 + 1, "2 located_at + 1 succession step + 1 analogue + 1 attests");
        for e in &edges {
            assert_eq!(e.rel, e.family.relation(), "{:?}", e.family);
        }
        let located: Vec<_> = edges.iter().filter(|e| e.family == RowFamily::LocatedAt).map(|e| e.row_ord).collect();
        assert_eq!(located, vec![0, 1]);
        let attests_subject = edges.iter().find(|e| e.family == RowFamily::Attests).unwrap();
        assert_eq!(crate::canon::ids::position_str(&attests_subject.object), "n:TextUnit:bible/1.1.1", "the range's FIRST verse is the endpoint");
    }

    #[test]
    fn build_indexes_is_exactly_the_row_edges_placed_each_edge_once_in_first_row_order() {
        let mut g = g();
        g.build_indexes();
        let mut from_rows: BTreeMap<(EdgeRel, Position), Vec<EdgeId>> = BTreeMap::new();
        let mut place = |rel: EdgeRel, subject: &Position, id: &EdgeId| {
            let placed = from_rows.entry((rel, subject.clone())).or_default();
            if !placed.contains(id) {
                placed.push(id.clone());
            }
        };
        for e in g.row_edges() {
            let id = Graph::edge_id_of(&e);
            place(e.rel, &e.subject, &id);
            match e.rel {
                EdgeRel::Directed(_) => {}
                EdgeRel::Symmetric(_) => place(e.rel, &e.object, &id),
            }
        }
        for (rel, ix) in &g.indexes {
            for (subject, adjacency) in &ix.fwd {
                let ids: Vec<EdgeId> = adjacency.edges().map(|e| e.edge.clone()).collect();
                assert_eq!(ids, from_rows[&(EdgeRel::Directed(*rel), subject.clone())], "fwd order at {subject:?}");
            }
        }
        for (rel, ix) in &g.symmetric_indexes {
            for (subject, adjacency) in &ix.fwd {
                let ids: Vec<EdgeId> = adjacency.edges().map(|e| e.edge.clone()).collect();
                assert_eq!(ids, from_rows[&(EdgeRel::Symmetric(*rel), subject.clone())], "sym order at {subject:?}");
            }
        }
    }

    #[test]
    fn relation_map_is_total_over_all_21_families() {
        for f in RowFamily::ALL {
            let _ = f.relation();
        }
    }
}
