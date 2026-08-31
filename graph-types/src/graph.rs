//! The graph: per-relation typed tables (the tables ARE the indexes),
//! per-corpus reading spines, bidirectional indexes built in one pass.

use std::collections::BTreeMap;

use crate::edge::{
    at, Attests, BiIndex, CatechismLink, CommentsOn, Confesses, Contains, Corresponds, CrossRef,
    SpokenAt, SpokenBy,
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
    /// SVEB-1: the THIRD `Contains<C>` instantiation -- Svebilius' own
    /// section containers over its units. Same shape as the two above.
    pub contains_svebilius: Vec<Contains<crate::text::SvebiliusTag>>,
    pub attests: Vec<Attests>,
    pub succession: Vec<Succession>,
    pub dated_by: Vec<DatedBy>,
    pub located_at: Vec<LocatedAt>,
    pub fulfills: Vec<Fulfills>,
    pub typology: Vec<Typology>,
    pub named_after: Vec<NamedAfter>,
    pub catechism: Vec<CatechismLink>,
    /// PARTS-1: citations attached to a chief PART rather than an item,
    /// plus the structural part -> item membership. Both lower into the
    /// same symmetric `catechism-link` relation as `catechism` above.
    pub catechism_part: Vec<crate::edge::CatechismPartLink>,
    pub catechism_membership: Vec<crate::edge::CatechismMembership>,
    /// KRETZ-1: verse-anchored commentary targets (annotation shape;
    /// the units themselves are CommentaryItem nodes).
    pub comments_on: Vec<CommentsOn>,
    /// RED-1: direct speech — speaker + site (annotation edges over loci).
    pub spoken_by: Vec<SpokenBy>,
    pub spoken_at: Vec<SpokenAt>,
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

/// SVEB-1: the same mapping `text_node` performs, exposed for tests that
/// need to name a locus's own node id (see `tests.rs`'s own
/// `topic_rows_do_not_multiply_the_structural_catechism_edges`).
#[cfg(test)]
pub fn text_node_for_test(l: &TextLocus) -> AnyNodeId {
    text_node(l)
}

fn text_node(kind_hint: &TextLocus) -> AnyNodeId {
    // Skeleton mapping from a locus to its unit node id.
    let raw = match &kind_hint.at {
        TextRef::Bible(v) => format!("bible/{}.{}.{}", v.book, v.chapter, v.verse),
        TextRef::Concord(c) => format!("concord/{}.{}.{}", c.part, c.article, c.paragraph),
        TextRef::Svebilius(v) => format!("svebilius/{}.{}", v.section, v.unit),
    };
    AnyNodeId { kind: NodeKind::TextUnit, raw }
}

impl Graph {
    /// Build every bidirectional index from the row tables — one pass
    /// per relation; both directions are projections of the same rows.
    /// Also builds the pid index (derive() as lookup).
    ///
    /// PERF-2b (profile-guided; see the batch report's own load-time
    /// investigation): row-table lowering (the loops below, building
    /// `pairs`/`sym_pairs`) stays exactly as it always was -- ONE
    /// sequential pass per relation, unchanged. What changed is what
    /// happens AFTER: turning those rows into `pid_index`/`indexes`/
    /// `symmetric_indexes` was measured to be the dominant cost of the
    /// whole artifact-load path (`pid_index`, one hash per NODE, and the
    /// relation lowering below, one hash per EDGE OCCURRENCE -- `cites`
    /// alone ~344k rows on the real committed graph) -- and every one of
    /// those hashes is a pure function of its own input, read-only, with
    /// no relation's rows depending on any other's and no node depending
    /// on any edge. That's safe, unsafe-free data parallelism
    /// (`std::thread::scope` -- no shared mutable state crosses a thread
    /// boundary, no algorithm change: the exact same `Node::pid`/
    /// `BiIndex::build`/`build_symmetric` calls the sequential version
    /// made, just spread across worker threads and merged back), not a
    /// hash/format rewrite -- measured ~2.6x for `build_indexes` alone
    /// (~1.9x for the whole `GraphService::from_artifact` load path it
    /// dominates) on this 16-core machine (see `server/BENCHMARKS.md`'s
    /// own `artifact_load` section for the full before/after).
    pub fn build_indexes(&mut self) {
        use crate::id::ContentAddressed;

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
        // SVEB-1: the third instantiation, lowered identically.
        for row in &self.contains_svebilius {
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
        // KRETZ-1 (the PRE-AUTHORIZED exception, standing since CORP-2a:
        // "activating the declared-but-never-wired comments_on field in
        // build_indexes by mechanically mirroring an existing sibling
        // loop"): `comments_on` is shaped exactly like `attests` above --
        // one node-typed field (`item`/`event`) plus one `BibleLocusRange`
        // field (`on`/`attestation`) -- so this mirrors that loop verbatim,
        // renamed. The range's own FIRST verse is the edge endpoint (the
        // SAME "full range stays on the row for display" precedent
        // `fulfills`/`typology` below also follow) -- a multi-verse
        // CommentaryItem (a pericope/chapter-intro unit) is reachable from
        // its range's first verse today; full multi-verse popover surfacing
        // is deferred with the rest of the client-side POPOVER-LAW-1 work
        // (decision 7).
        for row in &self.comments_on {
            let item = at(&row.item.erase());
            let tl: TextLocus = row.on.from.clone().into();
            pairs.entry(R::CommentsOn).or_default().push((item, at(&text_node(&tl)), M::None));
        }
        // RED-1 (the pre-authorized exception, standing since KRETZ-1's own
        // `comments_on` precedent immediately above): `spoken_by`/
        // `spoken_at` were declared on this struct at BASE but never
        // lowered into `pairs` until this batch. Both mirror `attests`'s
        // own shape (one `BibleLocusRange` field + one node-typed field),
        // but with the OPPOSITE polarity: `attests` puts the NODE (event)
        // first and the TEXT second; here the TEXT (this range's own FIRST
        // verse -- the SAME "full range stays on the row, first verse is
        // the edge endpoint" precedent `comments_on`/`attests` already
        // establish) is the SUBJECT and the node (speaker/place) is the
        // OBJECT -- the polarity each relation's own FORWARD LABEL decides
        // (this function's own comments_on doc comment note), never a
        // fixed node-type-first rule: "spoken-by"/"spoken-at" both read
        // naturally as "[this verse] spoken-by/-at [X]".
        for row in &self.spoken_by {
            let tl: TextLocus = row.locus.from.clone().into();
            pairs.entry(R::SpokenBy).or_default().push((at(&text_node(&tl)), at(&row.speaker.erase()), M::None));
        }
        for row in &self.spoken_at {
            let tl: TextLocus = row.locus.from.clone().into();
            pairs.entry(R::SpokenAt).or_default().push((at(&text_node(&tl)), at(&row.place.erase()), M::None));
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
        // SVEB-1: ROW multiplicity is not EDGE multiplicity, and this is
        // where the two part company.
        //
        // Widening the dedup key to (locus, item, topic) was the whole
        // point -- it recovers the 740 citations the old (locus, item) key
        // dropped. But it means one verse cited under three topics now
        // produces THREE rows, and lowering each row blindly would emit
        // that verse <-> item pair three times over. Both structural pairs
        // therefore get their own seen-set:
        //
        //   verse <-> item   exactly one per (verse, item), as before this
        //                    batch -- three topics citing ROM.1.22 do not
        //                    make three links between ROM.1.22 and the
        //                    First Commandment.
        //   item  <-> topic  exactly one per (item, topic) -- a topic
        //                    gathering five verses is still one topic.
        //   verse <-> topic  one per citation, NOT deduped: each is a
        //                    genuinely distinct verse under that topic.
        //
        // Both were caught by reading the real neighbour list rather than
        // by any guard: the counts looked plausible either way.
        let mut locus_item_seen: std::collections::BTreeSet<(String, String)> =
            std::collections::BTreeSet::new();
        let mut item_topic_seen: std::collections::BTreeSet<(String, String)> =
            std::collections::BTreeSet::new();
        for row in &self.catechism {
            let locus = at(&text_node(&row.locus));
            let item = at(&row.item.erase());
            if locus_item_seen.insert((text_node(&row.locus).raw.clone(), row.item.0.clone())) {
                sym_pairs.entry(S::CatechismLink).or_default().push((locus.clone(), item.clone(), M::None));
            }
            // SVEB-1: a topic-attributed citation indexes TWO further
            // pairs, which is what makes a topic traversable at all --
            // verse <-> topic answers "which topics does this verse
            // serve", item <-> topic answers "what does this item
            // gather". Both ride the SAME symmetric relation as the
            // locus <-> item pair above: `catechism-link` means "joined
            // to the catechism", and a topic is part of the catechism.
            // The locus <-> item pair is emitted UNCONDITIONALLY above,
            // exactly as before, so nothing that already traversed this
            // relation changes shape.
            if let Some(topic) = &row.topic {
                let topic_pos = at(&topic.erase());
                // Verse <-> topic: one per citation, each a distinct verse.
                sym_pairs.entry(S::CatechismLink).or_default().push((locus, topic_pos.clone(), M::None));
                // Item <-> topic: structural, so exactly one.
                if item_topic_seen.insert((row.item.0.clone(), topic.0.clone())) {
                    sym_pairs.entry(S::CatechismLink).or_default().push((item, topic_pos, M::None));
                }
            }
        }

        // PARTS-1: the part-level rows, lowered by the SAME three laws --
        // verse<->part once per (verse,part), part<->topic once per
        // (part,topic), verse<->topic once per citation. Copying the
        // discipline rather than the code because getting this wrong is
        // exactly the bug the item rows already had once.
        let mut locus_part_seen: std::collections::BTreeSet<(String, String)> = std::collections::BTreeSet::new();
        let mut part_topic_seen: std::collections::BTreeSet<(String, String)> = std::collections::BTreeSet::new();
        for row in &self.catechism_part {
            let locus = at(&text_node(&row.locus));
            let part = at(&row.part.erase());
            if locus_part_seen.insert((text_node(&row.locus).raw.clone(), row.part.0.clone())) {
                sym_pairs.entry(S::CatechismLink).or_default().push((locus.clone(), part.clone(), M::None));
            }
            if let Some(topic) = &row.topic {
                let topic_pos = at(&topic.erase());
                sym_pairs.entry(S::CatechismLink).or_default().push((locus, topic_pos.clone(), M::None));
                if part_topic_seen.insert((row.part.0.clone(), topic.0.clone())) {
                    sym_pairs.entry(S::CatechismLink).or_default().push((part, topic_pos, M::None));
                }
            }
        }

        // PARTS-1: part <-> item membership -- one row each already, so no
        // seen-set is needed; the ETL emits exactly one per item.
        for row in &self.catechism_membership {
            sym_pairs.entry(S::CatechismLink).or_default().push((
                at(&row.part.erase()),
                at(&row.item.erase()),
                M::None,
            ));
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

        // PERF-2b: the parallel pass -- see this function's own doc
        // comment. Chunk sizes are sized off `available_parallelism`
        // (falling back to 4 if the platform can't report it), NOT a flat
        // row constant: `pid_index` (one chunk per node-heavy pass) and
        // the edge relations (one chunk pool sized off the TOTAL row count
        // across every relation, so `cites` alone -- by far the largest --
        // is split into roughly as many pieces as there are cores, and the
        // many small relations don't each independently multiply the
        // thread count) are sized so BOTH pools land near, not far under
        // or wildly over, the core count -- measured (see the batch
        // report): a flat 50k-row constant gave `pid_index` only 2 chunks
        // on this 92k-node graph (barely any speedup, ~307ms of a ~344ms
        // sequential baseline) while starving it of CPU share against 20
        // concurrently-running edge chunks; sizing both pools off the same
        // core count fixed that. Every chunk is `BiIndex::build`/
        // `build_symmetric` (or, for `pid_index`, `Node::pid`) run over a
        // SLICE -- the identical function the pre-parallel code called
        // over the WHOLE table, just given less of it -- so per-item
        // computation is bit-for-bit unchanged; only the merge below is
        // new code.
        let n_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);

        // `pid_index`'s own chunks: read-only over `self.nodes`, touches
        // none of the relation row tables the directed/symmetric chunks
        // below read -- safe to run fully concurrently with them. Computed
        // OUTSIDE the scope below (not just outside its closure body) so
        // it outlives every spawned thread, satisfying `thread::scope`'s
        // own borrow requirement.
        let node_refs: Vec<(&AnyNodeId, &Node)> = self.nodes.iter().collect();
        let node_chunk_size = node_refs.len().div_ceil(n_threads).max(1);

        // The edge-relation chunk size: total rows across every relation,
        // divided by the core count -- `cites` (by far the largest single
        // relation) ends up split into roughly `n_threads` pieces on its
        // own; every smaller relation gets fewer (often just one).
        let total_edge_rows: usize = pairs.values().map(|v| v.len()).sum::<usize>() + sym_pairs.values().map(|v| v.len()).sum::<usize>();
        let edge_chunk_size = total_edge_rows.div_ceil(n_threads).max(1);

        let (pid_index, indexes, symmetric_indexes) = std::thread::scope(|scope| {
            let pid_handles: Vec<_> = node_refs
                .chunks(node_chunk_size)
                .map(|chunk| scope.spawn(move || chunk.iter().map(|(_, n)| (n.pid(), n.id.clone())).collect::<BTreeMap<_, _>>()))
                .collect();

            // Directed-relation chunks. Pushed relation by relation, and
            // WITHIN a relation strictly in original row order -- the
            // order this vector (and so `directed_handles`, and so the
            // merge loop below, which joins/processes handles in vector
            // order regardless of which thread the OS finishes first) is
            // built in is EXACTLY the order the pre-parallel sequential
            // pass would have visited these same rows in.
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

            // The symmetric sibling -- same splitting rule (today's tables
            // are far smaller than `edge_chunk_size`, so this is one chunk each
            // in practice, but the rule stays uniform for future growth).
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

            // pid_index: key-unique (one pid per node), so chunk merge
            // order carries no meaning -- a plain union.
            let pid_index: BTreeMap<crate::id::Pid, AnyNodeId> = pid_handles.into_iter().flat_map(|h| h.join().expect("pid-index worker panicked").into_iter()).collect();

            // Directed indexes: merge each relation's own chunks back
            // together IN ORDER (`Vec::append`, never a re-sort) -- for
            // any `Position` key that chunk i and chunk i+1 both touch,
            // chunk i's edges land first in `fwd`/`inv`'s Vec, exactly
            // reproducing the single sequential pass's own per-key order
            // (proven for a large, multi-chunk relation by
            // `parallel_build_indexes_matches_sequential_over_a_large_relation`
            // below, and by the standing full suite, including
            // `scene_byte_identity.rs`'s pinned response hashes, staying
            // green through this batch).
            let mut indexes: BTreeMap<R, BiIndex> = BTreeMap::new();
            for h in directed_handles {
                let (rel, partial) = h.join().expect("index-build worker panicked");
                let entry = indexes.entry(rel).or_default();
                for (k, mut v) in partial.fwd {
                    entry.fwd.entry(k).or_default().append(&mut v);
                }
                for (k, mut v) in partial.inv {
                    entry.inv.entry(k).or_default().append(&mut v);
                }
            }

            let mut symmetric_indexes: BTreeMap<S, BiIndex> = BTreeMap::new();
            for h in sym_handles {
                let (rel, partial) = h.join().expect("symmetric index-build worker panicked");
                let entry = symmetric_indexes.entry(rel).or_default();
                for (k, mut v) in partial.fwd {
                    entry.fwd.entry(k).or_default().append(&mut v);
                }
                // `.inv` is always empty for a symmetric `BiIndex`
                // (`build_symmetric`'s own doc comment) -- merged anyway,
                // for free, rather than assumed.
                for (k, mut v) in partial.inv {
                    entry.inv.entry(k).or_default().append(&mut v);
                }
            }

            (pid_index, indexes, symmetric_indexes)
        });

        self.pid_index = pid_index;
        self.indexes = indexes;
        self.symmetric_indexes = symmetric_indexes;
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

    /// PERF-2b / PERF-m2 (batch-finalp2-brief.md ticket 4 -- strengthening
    /// this SAME test, test-file-only, per that ticket's own disclosed
    /// carve-out: "the standing veto is on the TYPES, not this crate's own
    /// tests"; origin: PERF-2b review finding 1, LOW, "the crate equality
    /// test is synthetic toy-data and re-implements the merge inline rather
    /// than calling production `Graph::build_indexes`"):
    /// `Graph::build_indexes`'s own chunk-then-merge parallelism (this
    /// file's own `build_indexes` doc comment) is only as correct as the
    /// claim that splitting a relation's rows into contiguous chunks,
    /// building a `BiIndex` over each chunk separately, and merging the
    /// chunks back IN ORDER (`Vec::append`, never a re-sort) reproduces
    /// EXACTLY what `BiIndex::build` over the WHOLE, unchunked slice would
    /// have produced -- including PER-KEY EDGE ORDER for a position that
    /// spans more than one chunk.
    ///
    /// STRENGTHENED (was: raw `Position` tuples fed straight to
    /// `BiIndex::build`, chunked and merged by a hand-rolled 3-chunk loop
    /// living entirely in this test -- a real, disclosed gap: the merge
    /// algorithm under test was a SEPARATE, inline REIMPLEMENTATION of
    /// `build_indexes`'s own logic, so a real bug in the PRODUCTION
    /// function's own chunking/merge could exist while this test stayed
    /// green, having never called it). NOW: real, STRUCTURED domain rows
    /// (`Attests`, populated via `Graph.attests` -- the actual authored row
    /// table, not a bypass of it) drive the REAL, production
    /// `g.build_indexes()` entry point directly -- real `std::thread::
    /// scope`, real chunk count off THIS machine's own `std::thread::
    /// available_parallelism()`, the exact call every real
    /// `from_sources`/`from_artifact` build path makes -- and the result
    /// (`g.indexes`) is compared against an independently-computed
    /// sequential reference (`BiIndex::build` over the SAME row->pair
    /// mapping `build_indexes`'s own `attests` loop uses, computed once,
    /// unchunked, here in the test). 300 rows, all sharing one hub
    /// `EventId` (every row's own SOURCE position) -- comfortably exceeds
    /// this machine's own core count, so a position spanning more than one
    /// chunk (the case that actually exercises cross-chunk merge order) is
    /// exercised on any real machine, not just a hardcoded chunk count.
    #[test]
    fn parallel_build_indexes_matches_sequential_over_a_large_relation() {
        use crate::edge::Justification;
        use crate::explore::EdgeMeta as M;
        use crate::id::EventId;
        use crate::text::{BibleLocusRange, VerseRef};

        let hub = EventId::new("hub-event".to_string());
        let mut g = Graph::default();
        for i in 0..300u16 {
            let v = VerseRef { book: 0, chapter: 1, verse: i + 1 };
            let attestation = BibleLocusRange::new(Locus::whole(v.clone()), Locus::whole(v)).expect("a single-verse range must construct");
            g.attests.push(Attests { event: hub.clone(), attestation, provenance: ProvenanceId::from("test"), justification: Justification::default() });
        }

        // The SAME row->pair mapping `build_indexes`'s own `attests` loop
        // uses (this file's own `for row in &self.attests` block above),
        // computed independently here so the sequential reference below is
        // built from the real rows, not a second synthetic dataset.
        let pairs: Vec<(Position, Position, M)> = g
            .attests
            .iter()
            .map(|row| {
                let tl: TextLocus = row.attestation.from.clone().into();
                (at(&row.event.clone().erase()), at(&text_node(&tl)), M::None)
            })
            .collect();
        assert_eq!(pairs.len(), 300, "sanity: every real Attests row must lower to exactly one pair");

        // Sequential reference: `BiIndex::build` over the WHOLE, unchunked
        // pair list -- exactly what the pre-PERF-2b code always did.
        let sequential = crate::edge::BiIndex::build(RelationId::Attests, &pairs);

        // THE REAL THING: production `build_indexes()`, called directly --
        // real thread::scope, real chunk count off this machine's own core
        // count, the identical entry point every real build path calls.
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

    /// KRETZ-1: the pre-authorized exception's own proof -- `comments_on`
    /// was declared on this struct at BASE but never lowered into `pairs`
    /// until this batch. Mirrors `attests`'s own shape/test exactly (one
    /// node-typed field + one `BibleLocusRange` field, range's first verse
    /// is the edge endpoint): a CommentaryItem's own forward "comments-on"
    /// frontier reaches its target verse, and that verse's own inverse
    /// "commented-on-by" frontier reaches back, under the SAME EdgeId.
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

    /// RED-1: the SAME pre-authorized-exception shape as `comments_on`
    /// immediately above -- `spoken_by`/`spoken_at` were declared on this
    /// struct at BASE but never lowered into `pairs` until this batch.
    /// OPPOSITE polarity from `comments_on`/`attests` (this function's own
    /// RED-1 doc comment): the TEXT is the subject here, the node
    /// (speaker/place) is the object -- "[verse] spoken-by/-at [X]" reads
    /// naturally either way.
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
        g.nodes.insert(jesus_id.erase(), Node { id: jesus_id.erase(), payload: NodePayload::Person { label: "Jesus".into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None }, provenance: "test".into() });
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
}
