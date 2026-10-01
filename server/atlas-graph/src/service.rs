//! `GraphService`: the one built-graph handle the server holds, built once at startup. It defines no
//! store or query trait of its own -- every query goes through `atlas_graph_types::store::GraphQuery` --
//! and adds only the companions that port does not model, such as a reading-spine reverse lookup.

use std::collections::{BTreeMap, HashMap};
use std::ops::RangeInclusive;
use std::path::Path;
use std::sync::Arc;

use anyhow::Context;

use atlas_core::data::{AtlasData, Canon, CrossRef};
use atlas_core::refs::ScriptureRef;
use atlas_core::sources::SourcesDocument;
use atlas_graph_types::edge::EdgeId;
use atlas_graph_types::adjacency::{EdgePage, EdgePageWithNodes, EdgeQuery, EdgeSummary, NodePage};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{AnyNodeId, NodeKind, Pid, Position};
use atlas_graph_types::node::Node;
use atlas_graph_types::store::{GraphPublisher, GraphQuery, GraphSnapshot, GraphStore, GraphVersion, MemSnapshot, MemStore, RowRef};
use atlas_graph_types::text::{ConcordRef, TextRef, VerseRef};

use crate::citations::CitationSpan;
use crate::mention_spans::MentionSpan;
use crate::sections::Section;
use crate::sqlite::snapshot::SqliteSnapshot;
use crate::sqlite::SqliteError;
use crate::sqlite::source::sibling_dir;

/// The port handle a `GraphService` serves through: the in-memory store's snapshot or the committed
/// sections. Every arm delegates the whole port, so a handler never sees which one it holds, and either
/// is cheap to clone.
#[derive(Clone)]
pub enum Snap {
    Mem(MemSnapshot),
    Sqlite(Arc<SqliteSnapshot>),
}

impl std::fmt::Debug for Snap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Snap::Mem(s) => f.debug_tuple("Mem").field(&s.version()).finish(),
            Snap::Sqlite(s) => f.debug_tuple("Sqlite").field(&**s).finish(),
        }
    }
}

macro_rules! delegate {
    ($self:expr, $s:ident => $body:expr) => {
        match $self {
            Snap::Mem($s) => $body,
            Snap::Sqlite($s) => $body,
        }
    };
}

impl GraphQuery for Snap {
    fn node(&self, id: &AnyNodeId) -> Option<Node> {
        delegate!(self, s => s.node(id))
    }
    fn derive(&self, pid: &Pid) -> Option<Vec<u8>> {
        delegate!(self, s => s.derive(pid))
    }
    fn edge_summary(&self, p: &Position) -> EdgeSummary {
        delegate!(self, s => s.edge_summary(p))
    }
    fn edges(&self, p: &Position, q: &EdgeQuery) -> EdgePage {
        delegate!(self, s => s.edges(p, q))
    }
    fn reading_window(&self, corpus: &'static str, start: usize, n: usize) -> Vec<AnyNodeId> {
        delegate!(self, s => s.reading_window(corpus, start, n))
    }
    fn nodes_of_kind(&self, kind: NodeKind, cursor: Option<usize>, limit: usize) -> NodePage {
        delegate!(self, s => s.nodes_of_kind(kind, cursor, limit))
    }
    fn nodes(&self, ids: &[AnyNodeId]) -> Vec<Option<Node>> {
        delegate!(self, s => s.nodes(ids))
    }
    fn edges_with_nodes(&self, p: &Position, q: &EdgeQuery) -> EdgePageWithNodes {
        delegate!(self, s => s.edges_with_nodes(p, q))
    }
    fn row_provenance(&self, e: &EdgeId) -> Option<RowRef> {
        delegate!(self, s => s.row_provenance(e))
    }
    fn rows_behind(&self, e: &EdgeId) -> Vec<RowRef> {
        delegate!(self, s => s.rows_behind(e))
    }
    fn position_of(&self, corpus: &'static str, id: &AnyNodeId) -> Option<usize> {
        delegate!(self, s => s.position_of(corpus, id))
    }
}

impl GraphSnapshot for Snap {
    fn version(&self) -> GraphVersion {
        delegate!(self, s => s.version())
    }
}

use crate::build::{self, BuildStats};
use crate::event_world::{Chronology, EventWorldStats};

pub struct GraphService {
    snapshot: Snap,
    pub stats: BuildStats,
    /// The chronology companion: the generic port models no temporal adjacency of its own.
    pub chronology: Chronology,
    pub event_world_stats: EventWorldStats,
    /// narrative id -> its `succession` row's chain, in order: the single source for a narrative's legs,
    /// never duplicated onto the node payload. A narrative with no legs has no entry at all.
    pub narrative_legs: BTreeMap<String, Vec<String>>,
    /// verse -> the one pericope heading that wins there, precomputed once rather than per request.
    pub heading_index: BTreeMap<String, crate::heading::Heading>,
    rows_at_locus: RowsAtLocus,
    /// Optional sections the manifest lists but this deployment lacks; empty on the in-memory arm.
    absent_sections: Vec<Section>,
    /// dot-ref -> the KJV sub-verse char-offset spans. Not derivable from the graph at all -- it needs
    /// the original OSIS alignment -- so `assemble` takes it as an already-resolved parameter.
    pub red_letter_spans: HashMap<String, Vec<(usize, usize)>>,
    /// The per-surface provenance companion: the served graph cannot answer a ROW's provenance itself.
    pub provenance: crate::provenance::ProvenanceIndex,
    /// The map scene's data, materialised once from this service's port. An `OnceLock` for the one thing
    /// `assemble` cannot give it: the two curated-JSON sidecar maps live in `AtlasData`, which this crate
    /// never loads. The server and the CLI prime it at load time, so no request pays the materialisation.
    scene_source: std::sync::OnceLock<crate::scene_source::GraphSceneSource>,
}

enum RowsAtLocus {
    InMemory(MemRowsAtLocus),
    Sections(Arc<SqliteSnapshot>),
}

struct MemRowsAtLocus {
    cross_refs: HashMap<String, Vec<CrossRef>>,
    mention_spans: BTreeMap<VerseRef, Vec<MentionSpan>>,
    citation_spans: BTreeMap<ConcordRef, Vec<CitationSpan>>,
}

impl MemRowsAtLocus {
    fn of(graph: &Graph) -> MemRowsAtLocus {
        let mut rows = MemRowsAtLocus { cross_refs: HashMap::new(), mention_spans: BTreeMap::new(), citation_spans: BTreeMap::new() };
        for row in &graph.cross_refs {
            match &row.from.at {
                TextRef::Bible(verse) => rows
                    .cross_refs
                    .entry(crate::kjv_adapter::dot_ref(verse.book, verse.chapter, verse.verse))
                    .or_default()
                    .push(CrossRef { target: row.target_display.clone(), votes: row.votes as i32 }),
                TextRef::Concord(paragraph) => rows.citation_spans.entry(paragraph.clone()).or_default().extend(CitationSpan::of(row)),
            }
        }
        for row in &graph.mentions {
            if let TextRef::Bible(verse) = &row.locus.at {
                rows.mention_spans.entry(verse.clone()).or_default().extend(MentionSpan::of(row));
            }
        }
        rows
    }
}

fn held_in<U: Ord + Clone, S: Clone>(held: &BTreeMap<U, Vec<S>>, units: &RangeInclusive<U>) -> BTreeMap<U, Vec<S>> {
    held.range(units.clone()).filter(|(_, spans)| !spans.is_empty()).map(|(unit, spans)| (unit.clone(), spans.clone())).collect()
}

/// The longest KJV chapter has 176 verses, so this probe width is a comfortable margin.
const MAX_CHAPTER_SPAN_PROBE: usize = 200;

impl GraphService {
    pub fn from_sources(kjv_json: &str, xrefs_tsv: &str, atlas: &AtlasData) -> anyhow::Result<Self> {
        Self::from_sources_with_eras(kjv_json, xrefs_tsv, atlas, &[])
    }

    pub fn from_sources_with_eras(kjv_json: &str, xrefs_tsv: &str, atlas: &AtlasData, eras: &[atlas_core::data::Era]) -> anyhow::Result<Self> {
        Self::from_sources_with_eras_and_brainfuel(kjv_json, xrefs_tsv, atlas, eras, None)
    }

    pub fn from_sources_with_eras_and_brainfuel(
        kjv_json: &str,
        xrefs_tsv: &str,
        atlas: &AtlasData,
        eras: &[atlas_core::data::Era],
        brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>,
    ) -> anyhow::Result<Self> {
        Self::from_sources_with_eras_and_brainfuel_and_concord(kjv_json, xrefs_tsv, atlas, eras, brainfuel, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_sources_with_eras_and_brainfuel_and_concord(
        kjv_json: &str,
        xrefs_tsv: &str,
        atlas: &AtlasData,
        eras: &[atlas_core::data::Era],
        brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>,
        concord: Option<&crate::concord_adapter::ConcordBundle>,
    ) -> anyhow::Result<Self> {
        Self::from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann(kjv_json, xrefs_tsv, atlas, eras, brainfuel, concord, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann(
        kjv_json: &str,
        xrefs_tsv: &str,
        atlas: &AtlasData,
        eras: &[atlas_core::data::Era],
        brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>,
        concord: Option<&crate::concord_adapter::ConcordBundle>,
        kretzmann: Option<&atlas_etl::kretzmann::KretzmannCorpus>,
    ) -> anyhow::Result<Self> {
        Self::from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(kjv_json, xrefs_tsv, atlas, eras, brainfuel, concord, kretzmann, None)
    }

    /// `red_letter_spans` is derived here off the same corpus and the same restored verses the graph
    /// itself was built from, recomputed once rather than threaded out of that call.
    #[allow(clippy::too_many_arguments)]
    pub fn from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(
        kjv_json: &str,
        xrefs_tsv: &str,
        atlas: &AtlasData,
        eras: &[atlas_core::data::Era],
        brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>,
        concord: Option<&crate::concord_adapter::ConcordBundle>,
        kretzmann: Option<&atlas_etl::kretzmann::KretzmannCorpus>,
        red_letter: Option<&atlas_etl::red_letter::RedLetterCorpus>,
    ) -> anyhow::Result<Self> {
        let (graph, stats, event_world_stats, chrono) =
            build::build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(kjv_json, xrefs_tsv, atlas, eras, brainfuel, concord, kretzmann, red_letter)?;
        let red_letter_spans: HashMap<String, Vec<(usize, usize)>> = match red_letter {
            Some(corpus) => {
                let (_, verses) = atlas_etl::kjv::parse(kjv_json).context("parsing the KJV source (kjv.json) for the red-letter span table")?;
                let restored_verses;
                let verses_ref: &HashMap<String, String> = match brainfuel {
                    Some(bf) => {
                        restored_verses = atlas_etl::brainfuel::restore_kjv_case(bf, &verses).0;
                        &restored_verses
                    }
                    None => &verses,
                };
                crate::red_letter_spans::spans_by_dot_ref(corpus, verses_ref).into_iter().collect()
            }
            None => HashMap::new(),
        };
        Ok(Self::assemble(graph, stats, event_world_stats, Chronology::from_derivation(chrono), red_letter_spans, None))
    }

    pub fn from_canon_and_verses(canon: &Canon, verses: &HashMap<String, String>, xrefs_tsv: &str, atlas: &AtlasData) -> anyhow::Result<Self> {
        Self::from_canon_and_verses_with_eras(canon, verses, xrefs_tsv, atlas, &[])
    }

    pub fn from_canon_and_verses_with_eras(canon: &Canon, verses: &HashMap<String, String>, xrefs_tsv: &str, atlas: &AtlasData, eras: &[atlas_core::data::Era]) -> anyhow::Result<Self> {
        let (graph, stats, event_world_stats, chrono) = build::build_graph_from_canon_and_verses_with_eras(canon, verses, xrefs_tsv, atlas, eras)?;
        Ok(Self::assemble(graph, stats, event_world_stats, Chronology::from_derivation(chrono), HashMap::new(), None))
    }

    /// Reads the raw KJV and cross-reference sources, and the curated eras beside them: the only
    /// filesystem-touching function in this crate.
    pub fn build(raw_dir: &Path, atlas: &AtlasData) -> anyhow::Result<Self> {
        let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json"))
            .with_context(|| format!("reading {}", raw_dir.join("kjv.json").display()))?;
        let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt"))
            .with_context(|| format!("reading {}", raw_dir.join("xrefs/cross_references.txt").display()))?;
        let eras = load_eras(raw_dir)?;
        let brainfuel = load_brainfuel(raw_dir)?;
        let concord = load_concord(raw_dir)?;
        let kretzmann = load_kretzmann(raw_dir, &kjv_json)?;
        let red_letter = load_red_letter(raw_dir, &kjv_json, brainfuel.as_ref())?;
        Self::from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(&kjv_json, &xrefs_tsv, atlas, &eras, brainfuel.as_ref(), concord.as_ref(), kretzmann.as_ref(), red_letter.as_ref())
    }

    /// Takes an ALREADY-BUILT `Chronology`, because the artifact path has no `AtlasData` to re-derive from.
    /// `sidecars`, when present, fold into the graph's extra tables before `publish`, so this service's version
    /// IS the manifest root; `None` publishes a root that is disclosed as not the manifest's.
    fn assemble(
        mut graph: Graph,
        stats: BuildStats,
        event_world_stats: EventWorldStats,
        chronology: Chronology,
        red_letter_spans: HashMap<String, Vec<(usize, usize)>>,
        sidecars: Option<(&AtlasData, &SourcesDocument)>,
    ) -> Self {
        let narrative_legs = narrative_legs_of(&graph);
        let heading_index = crate::heading::build_heading_index(&graph, &chronology.chrono.resolved);
        // The non-graph section tables ride the graph into the version root, computed from the same
        // values the section writer folds.
        let mut extras = crate::sqlite::extras::Extras::graph_derived(&graph, &chronology.chrono, &red_letter_spans)
            .expect("assemble: the graph's projections encode");
        if let Some((atlas, sources)) = sidecars {
            extras.extend(crate::sqlite::sidecars::fold_sidecars(atlas, sources).expect("assemble: the sidecars fold"));
        }
        extras.attach(&mut graph);
        let rows_at_locus = RowsAtLocus::InMemory(MemRowsAtLocus::of(&graph));
        // The compiler publishes and serving never writes: one publish, at startup. This is also the
        // LAST pre-store scan -- `graph` moves into the store on the very next line.
        let provenance = crate::provenance::ProvenanceIndex::build(&graph);
        let mut store = MemStore::default();
        let version = store.publish(graph);
        let snapshot = store.open(version).expect("the version just published must always be open-able");
        GraphService {
            snapshot: Snap::Mem(snapshot),
            stats,
            chronology,
            event_world_stats,
            narrative_legs,
            heading_index,
            rows_at_locus,
            absent_sections: Vec::new(),
            red_letter_spans,
            provenance,
            scene_source: std::sync::OnceLock::new(),
        }
    }

    /// Opens the committed sections, one connection per worker, loading every companion the handlers read. A
    /// manifest whose root does not recompute, a required section missing or failing its transport hash, an unknown
    /// `user_version` or an unwritable cache all refuse before anything is served. The `AtlasData` is un-finished.
    pub fn from_sections(data_dir: &Path) -> anyhow::Result<(GraphService, AtlasData, SourcesDocument)> {
        use crate::sqlite::source::{CommittedZstdSource, SectionLayout};
        let layout = SectionLayout::under(data_dir);
        std::fs::create_dir_all(&layout.cache_dir)
            .map_err(|e| anyhow::anyhow!("cache directory {} is not writable: {e}", layout.cache_dir.display()))?;
        let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).min(8);
        let snap = SqliteSnapshot::open_with_workers(&layout.manifest_path(), &CommittedZstdSource { layout: layout.clone() }, workers)
            .map_err(|e| anyhow::anyhow!("opening the sections at {}: {e}", layout.manifest_path().display()))?;
        for s in snap.absent() {
            eprintln!("atlas: optional section {} absent -- its kinds are uninhabited", s.name());
        }
        let present: Vec<Section> = snap.present().to_vec();
        let absent_sections: Vec<Section> = snap.absent().to_vec();
        let (chrono, heading_index, red_letter_spans, narrative_legs, families, (stats, event_world_stats), (atlas, sources)) = snap
            .with_conn(|c| {
                use crate::sqlite::serve;
                Ok((
                    serve::load_chronology(c)?,
                    serve::load_heading_index(c)?,
                    serve::load_red_letter_spans(c)?,
                    serve::load_narrative_legs(c)?,
                    serve::load_provenance_families(c, &present)?,
                    serve::load_counters(c, &present)?,
                    crate::sqlite::sidecars::unfold(c)?,
                ))
            })
            .map_err(|e| anyhow::anyhow!("loading the serving companions from the sections: {e}"))?;
        let snap = Arc::new(snap);
        let service = GraphService {
            snapshot: Snap::Sqlite(snap.clone()),
            stats,
            chronology: Chronology::from_derivation(chrono),
            event_world_stats,
            narrative_legs,
            heading_index,
            rows_at_locus: RowsAtLocus::Sections(snap),
            absent_sections,
            red_letter_spans,
            provenance: crate::provenance::ProvenanceIndex::from_families(families),
            scene_source: std::sync::OnceLock::new(),
        };
        Ok((service, atlas, sources))
    }

    pub fn absent_sections(&self) -> &[Section] {
        &self.absent_sections
    }

    /// The cross-refs authored by the span's member verses, keyed by dot-ref, each list in row order with
    /// its original `target_display`: exactly the slice the span aggregation reads.
    pub fn cross_refs_for_span(&self, span: &ScriptureRef) -> HashMap<String, Vec<CrossRef>> {
        match &self.rows_at_locus {
            RowsAtLocus::Sections(s) => s.with_conn(|c| crate::sqlite::serve::cross_refs_for_span(c, span)).unwrap_or_default(),
            RowsAtLocus::InMemory(rows) => rows
                .cross_refs
                .iter()
                .filter(|(key, _)| match ScriptureRef::parse(key) {
                    Ok(ScriptureRef::Verse(v)) => match span {
                        ScriptureRef::Book(b) => v.book == *b,
                        ScriptureRef::Chapter { book, chapter } => v.book == *book && v.chapter == *chapter,
                        ScriptureRef::Passage { book, chapter, from_verse, to_verse } => {
                            v.book == *book && v.chapter == *chapter && v.verse >= *from_verse && v.verse <= *to_verse
                        }
                        ScriptureRef::Verse(w) => v == *w,
                    },
                    _ => false,
                })
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }

    pub fn mention_spans_in(&self, verses: &RangeInclusive<VerseRef>) -> Result<BTreeMap<VerseRef, Vec<MentionSpan>>, SqliteError> {
        match &self.rows_at_locus {
            RowsAtLocus::Sections(s) => s.with_conn(|c| crate::sqlite::serve::mention_spans_in(c, verses)),
            RowsAtLocus::InMemory(rows) => Ok(held_in(&rows.mention_spans, verses)),
        }
    }

    pub fn citation_spans_in(&self, paragraphs: &RangeInclusive<ConcordRef>) -> Result<BTreeMap<ConcordRef, Vec<CitationSpan>>, SqliteError> {
        match &self.rows_at_locus {
            RowsAtLocus::Sections(s) => s.with_conn(|c| crate::sqlite::serve::citation_spans_in(c, paragraphs)),
            RowsAtLocus::InMemory(rows) => Ok(held_in(&rows.citation_spans, paragraphs)),
        }
    }

    /// The version this service published at construction -- the only one for this process's lifetime.
    pub fn version(&self) -> GraphVersion {
        atlas_graph_types::store::GraphSnapshot::version(&self.snapshot)
    }

    /// Every node id of one kind, in id byte order, drained from the port.
    pub fn ids_of_kind(&self, kind: NodeKind) -> Vec<AnyNodeId> {
        ids_of_kind(&self.snapshot, kind)
    }

    /// The distinct, sorted provenance of every row behind the edges of one kind at one position. A
    /// synthesised edge contributes nothing.
    fn provenance_over(&self, p: &atlas_graph_types::id::Position, kind: atlas_graph_types::edge::EdgeKind) -> Vec<String> {
        use atlas_graph_types::adjacency::EdgeQuery;
        let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut cursor = None;
        loop {
            let page = self.snapshot.edges(p, &EdgeQuery { kind, cursor, limit: 256 });
            for e in &page.entries {
                // `rows_behind`, not `row_provenance`: where two rows mint one id, both count.
                for r in self.snapshot.rows_behind(&e.edge) {
                    set.insert(r.provenance);
                }
            }
            match page.next {
                Some(c) => cursor = Some(c),
                None => break set.into_iter().collect(),
            }
        }
    }

    /// The distinct provenance of the `Attests` rows of ONE event.
    pub fn attests_provenance(&self, event_raw: &str) -> Vec<String> {
        use atlas_graph_types::edge::{at, Direction, EdgeKind, RelationId};
        self.provenance_over(&at(&atlas_graph_types::id::EventId::new(event_raw).erase()), EdgeKind::Directed(RelationId::Attests, Direction::Forward))
    }

    /// The distinct provenance of the `Mentions` rows naming ONE event: a mention lowers with the event
    /// as OBJECT, so the inverse reading at the event lists exactly those rows.
    pub fn event_mentions_provenance(&self, event_raw: &str) -> Vec<String> {
        use atlas_graph_types::edge::{at, Direction, EdgeKind, RelationId};
        self.provenance_over(&at(&atlas_graph_types::id::EventId::new(event_raw).erase()), EdgeKind::Directed(RelationId::Mentions, Direction::Inverse))
    }

    /// The provenance of the ONE `Analogue` row joining two events, from either end.
    pub fn analogue_provenance(&self, a_raw: &str, b_raw: &str) -> Option<String> {
        use atlas_graph_types::edge::{at, EdgeKind, SymRelationId};
        use atlas_graph_types::adjacency::EdgeQuery;
        let a = at(&atlas_graph_types::id::EventId::new(a_raw).erase());
        let b = at(&atlas_graph_types::id::EventId::new(b_raw).erase());
        let kind = EdgeKind::Symmetric(SymRelationId::Analogue);
        let mut cursor = None;
        loop {
            let page = self.snapshot.edges(&a, &EdgeQuery { kind, cursor, limit: 256 });
            if let Some(e) = page.entries.iter().find(|e| e.node == b) {
                return self.snapshot.row_provenance(&e.edge).map(|r| r.provenance);
            }
            match page.next {
                Some(c) => cursor = Some(c),
                None => return None,
            }
        }
    }

    /// Prior and following in time for ONE event. `None` unless the event is in the chronology's order at
    /// all: adjacency comes from the `temporal-adjacency` edges and direction from that order.
    pub fn temporal_neighbors_of(&self, event_raw: &str) -> Option<(Option<String>, Option<String>)> {
        use atlas_graph_types::edge::{at, EdgeKind, SymRelationId};
        use atlas_graph_types::adjacency::EdgeQuery;
        use atlas_graph_types::id::Position;
        let order = &self.chronology.chrono.order;
        let me = order.iter().position(|x| x == event_raw)?;
        let p = at(&atlas_graph_types::id::EventId::new(event_raw).erase());
        let kind = EdgeKind::Symmetric(SymRelationId::TemporalAdjacency);
        let mut prior: Option<(usize, String)> = None;
        let mut following: Option<(usize, String)> = None;
        let mut cursor = None;
        loop {
            let page = self.snapshot.edges(&p, &EdgeQuery { kind, cursor, limit: 256 });
            for e in &page.entries {
                let Position::Node(n) = &e.node else { continue };
                let Some(idx) = order.iter().position(|x| *x == n.raw) else { continue };
                if idx < me {
                    if prior.as_ref().is_none_or(|(i, _)| idx > *i) {
                        prior = Some((idx, n.raw.clone()));
                    }
                } else if idx > me && following.as_ref().is_none_or(|(i, _)| idx < *i) {
                    following = Some((idx, n.raw.clone()));
                }
            }
            match page.next {
                Some(c) => cursor = Some(c),
                None => break,
            }
        }
        Some((prior.map(|(_, id)| id), following.map(|(_, id)| id)))
    }

    pub fn snapshot(&self) -> Snap {
        self.snapshot.clone()
    }

    /// `id`'s own position in the bible reading spine, which is what resolves a ref into a window's start.
    pub fn position_of(&self, book: u8, chapter: u16, verse: u16) -> Option<usize> {
        self.snapshot.position_of(crate::kjv_adapter::BIBLE_CORPUS, &crate::kjv_adapter::verse_node_id(book, chapter, verse))
    }

    pub fn concord_position_of(&self, part: u8, article: u16, paragraph: u16) -> Option<usize> {
        self.snapshot.position_of(crate::concord_adapter::CONCORD_CORPUS, &crate::concord_adapter::text_unit_id(part, article, paragraph))
    }

    pub fn persons_at_verse(&self, book: u8, chapter: u16, verse: u16) -> Vec<(String, String)> {
        use atlas_graph_types::edge::{at, Direction, EdgeKind, RelationId};
        use atlas_graph_types::adjacency::EdgeQuery;
        use atlas_graph_types::id::NodeKind;
        use atlas_graph_types::node::NodePayload;
        let p = at(&crate::kjv_adapter::verse_node_id(book, chapter, verse));
        let kind = EdgeKind::Directed(RelationId::Mentions, Direction::Forward);
        let mut out = Vec::new();
        let mut cursor = None;
        loop {
            let page = self.snapshot.edges_with_nodes(&p, &EdgeQuery { kind, cursor, limit: 256 });
            for e in page.entries {
                let Some(node) = e.node else { continue };
                if node.id.kind != NodeKind::Person {
                    continue;
                }
                if let NodePayload::Person { label, .. } = node.payload {
                    out.push((node.id.raw.clone(), label));
                }
            }
            match page.next {
                Some(c) => cursor = Some(c),
                None => {
                    out.dedup();
                    break out;
                }
            }
        }
    }

    /// The `(start, n)` window covering exactly one chapter, derived by probing the port's own
    /// `reading_window` for a generous upper bound and trimming to the contiguous run that shares
    /// (book, chapter).
    pub fn chapter_span(&self, book: u8, chapter: u16) -> Option<(usize, usize)> {
        let start = self.position_of(book, chapter, 1)?;
        let probe = self.snapshot.reading_window(crate::kjv_adapter::BIBLE_CORPUS, start, MAX_CHAPTER_SPAN_PROBE);
        let n = probe.iter().take_while(|id| matches!(crate::kjv_adapter::decode_text_unit(id), Some((b, c, _)) if b == book && c == chapter)).count();
        if n == 0 {
            None
        } else {
            Some((start, n))
        }
    }

    /// Reads one verse's KJV text ON DEMAND off the published snapshot: one node lookup rather than a
    /// whole-spine walk, and no caching.
    pub fn verse_text_of(&self, r: &atlas_graph_types::text::VerseRef) -> Option<String> {
        let id = crate::kjv_adapter::verse_node_id(r.book, r.chapter, r.verse);
        crate::window::render(&self.snapshot(), &id)
    }

    /// `sidecars` is read only for the two curated-JSON maps the scene source copies, so a bare,
    /// un-`finish()`ed `AtlasData` is a valid argument. It is IGNORED after the first call: the source is
    /// immutable once built, so passing a different one later does not rebuild it.
    pub fn scene_source(&self, sidecars: &AtlasData) -> &crate::scene_source::GraphSceneSource {
        self.scene_source.get_or_init(|| crate::scene_source::GraphSceneSource::build(self, sidecars))
    }
}

/// Every node id of one kind, in id byte order, drained from any port handle.
pub fn ids_of_kind(q: &impl GraphQuery, kind: NodeKind) -> Vec<AnyNodeId> {
    let mut out: Vec<AnyNodeId> = Vec::new();
    let mut cursor = None;
    loop {
        let page = q.nodes_of_kind(kind, cursor, 4096);
        out.extend(page.ids);
        match page.next {
            Some(c) => cursor = Some(c),
            None => break out,
        }
    }
}

/// narrative id -> its `succession` row's chain, in order: the single source for a narrative's legs,
/// never duplicated onto the node payload. A narrative with no legs has no entry at all.
pub fn narrative_legs_of(graph: &Graph) -> BTreeMap<String, Vec<String>> {
    graph.succession.iter().map(|row| (row.narrative.0.clone(), row.chain.iter().map(|e| e.0.clone()).collect())).collect()
}

/// The curated eras beside `raw_dir`: the one other filesystem read this crate performs.
fn load_eras(raw_dir: &Path) -> anyhow::Result<Vec<atlas_core::data::Era>> {
    let path = sibling_dir(raw_dir, "curated").join("eras.toml");
    let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    atlas_etl::curated::parse_eras(&text).with_context(|| format!("parsing {}", path.display()))
}

/// `None`, not an error, when the vendored directory simply does not exist; fail-loud when it exists but
/// is malformed.
fn load_brainfuel(raw_dir: &Path) -> anyhow::Result<Option<atlas_etl::brainfuel::BrainFuelCorpus>> {
    let root = raw_dir.join("brain-fuel-bible");
    if !root.is_dir() {
        return Ok(None);
    }
    atlas_etl::brainfuel::read_all(&root).map(Some).with_context(|| format!("reading vendored brain-fuel data from {}", root.display()))
}

/// `None`, not an error, when the vendored directory does not exist. The corpus and the curated overlap
/// file are bundled: one present without the other is a misconfiguration worth failing loud on rather
/// than half-building a corpus.
fn load_concord(raw_dir: &Path) -> anyhow::Result<Option<crate::concord_adapter::ConcordBundle>> {
    let root = raw_dir.join("concord");
    if !root.is_dir() {
        return Ok(None);
    }
    let curated_dir = sibling_dir(raw_dir, "curated");
    let corpus = atlas_etl::concord::read_all(&root, &curated_dir).with_context(|| format!("reading vendored Concord data from {}", root.display()))?;
    let overlap_path = curated_dir.join("concord-sc-overlap.toml");
    let overlap_text = std::fs::read_to_string(&overlap_path).with_context(|| format!("reading {}", overlap_path.display()))?;
    let sc_overlap = atlas_etl::concord::parse_sc_overlap(&overlap_text).with_context(|| format!("parsing {}", overlap_path.display()))?;
    Ok(Some(crate::concord_adapter::ConcordBundle { corpus, sc_overlap }))
}

/// `None`, not an error, when the vendored directory does not exist. The KJV text is parsed here into the
/// dot-ref map the over-excision guard needs; unrestored text suffices, since that guard compares word
/// content only.
fn load_kretzmann(raw_dir: &Path, kjv_json: &str) -> anyhow::Result<Option<atlas_etl::kretzmann::KretzmannCorpus>> {
    let root = raw_dir.join("kretzmann");
    if !root.is_dir() {
        return Ok(None);
    }
    let (_, kjv_verses) = atlas_etl::kjv::parse(kjv_json).context("parsing kjv.json for the Kretzmann over-excision guard")?;
    atlas_etl::kretzmann::read_all(&root, &kjv_verses).map(Some).with_context(|| format!("reading vendored Kretzmann data from {}", root.display()))
}

/// `None`, not an error, when the vendored directory does not exist. Unlike the commentary corpus this
/// alignment is case-sensitive first, so it reads against RESTORED text; with no edition corpus to
/// restore from it aligns against the raw parse -- a less precise target, never a panic.
fn load_red_letter(raw_dir: &Path, kjv_json: &str, brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>) -> anyhow::Result<Option<atlas_etl::red_letter::RedLetterCorpus>> {
    let root = raw_dir.join("red-letter");
    if !root.is_dir() {
        return Ok(None);
    }
    let (_, verses) = atlas_etl::kjv::parse(kjv_json).context("parsing kjv.json for the red-letter alignment")?;
    let restored_verses;
    let verses_ref: &std::collections::HashMap<String, String> = match brainfuel {
        Some(bf) => {
            restored_verses = atlas_etl::brainfuel::restore_kjv_case(bf, &verses).0;
            &restored_verses
        }
        None => &verses,
    };
    atlas_etl::red_letter::read_all(&root, verses_ref).map(Some).with_context(|| format!("reading vendored red-letter data from {}", root.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KJV_FIXTURE: &str = r#"{
      "translation": "KJV",
      "books": [
        { "name": "Genesis", "chapters": [
          { "chapter": 1, "verses": [
            { "verse": 1, "text": "In the beginning God created the heaven and the earth." },
            { "verse": 2, "text": "And the earth was without form, and void." },
            { "verse": 3, "text": "And God said, Let there be light: and there was light." }
          ] },
          { "chapter": 2, "verses": [
            { "verse": 1, "text": "Thus the heavens and the earth were finished." }
          ] }
        ] }
      ]
    }"#;
    const NO_XREFS: &str = "From Verse\tTo Verse\tVotes\t#comment\n";

    fn service() -> GraphService {
        GraphService::from_sources(KJV_FIXTURE, NO_XREFS, &crate::event_world::empty_atlas()).unwrap()
    }

    #[test]
    fn from_sources_publishes_and_opens_a_snapshot_at_that_version() {
        let svc = service();
        assert_eq!(svc.snapshot().version(), svc.version(), "the opened snapshot must be exactly the version this service published");
    }

    #[test]
    fn bible_position_resolves_through_the_reverse_index() {
        let svc = service();
        assert_eq!(svc.position_of(0, 1, 2), Some(1));
        assert_eq!(svc.position_of(0, 99, 1), None, "unknown verse position is None, not a panic");
    }

    fn provenance_service(g: Graph) -> GraphService {
        let mut g = g;
        g.build_indexes();
        GraphService::assemble(g, BuildStats::default(), EventWorldStats::default(), Chronology::from_derivation(crate::event_world::ChronologyDerivation::default()), HashMap::new(), None)
    }

    fn prov_range() -> atlas_graph_types::text::BibleLocusRange {
        atlas_graph_types::text::LocusRange::new(
            atlas_graph_types::text::BibleLocus::whole(atlas_graph_types::text::VerseRef { book: 40, chapter: 8, verse: 1 }),
            atlas_graph_types::text::BibleLocus::whole(atlas_graph_types::text::VerseRef { book: 40, chapter: 8, verse: 4 }),
        )
        .expect("from <= to")
    }

    fn prov_locus() -> atlas_graph_types::text::TextLocus {
        atlas_graph_types::text::TextLocus { at: atlas_graph_types::text::TextRef::Bible(atlas_graph_types::text::VerseRef { book: 40, chapter: 8, verse: 2 }), span: None }
    }

    #[test]
    fn an_events_accounts_report_every_source_behind_them_not_just_one() {
        use atlas_graph_types::edge::Attests;
        use atlas_graph_types::id::EventId;
        let mut g = Graph::default();
        g.attests.push(Attests { event: EventId::new("e1"), attestation: prov_range(), provenance: "event-witnesses".into(), justification: Default::default() });
        g.attests.push(Attests { event: EventId::new("e1"), attestation: prov_range(), provenance: "attestation-corrections".into(), justification: Default::default() });
        g.attests.push(Attests { event: EventId::new("e2"), attestation: prov_range(), provenance: "event-witnesses".into(), justification: Default::default() });
        let svc = provenance_service(g);
        assert_eq!(svc.attests_provenance("e1"), vec!["attestation-corrections".to_string(), "event-witnesses".to_string()]);
        assert_eq!(svc.attests_provenance("e2"), vec!["event-witnesses".to_string()]);
        assert!(svc.attests_provenance("e3").is_empty());
        assert_eq!(svc.provenance.by_family(crate::provenance::family::ATTESTS), vec!["attestation-corrections".to_string(), "event-witnesses".to_string()]);
    }

    #[test]
    fn an_analogue_row_resolves_from_either_end_because_the_relation_is_symmetric() {
        use atlas_graph_types::edge::Analogue;
        use atlas_graph_types::id::EventId;
        let mut g = Graph::default();
        g.analogue.push(Analogue { a: EventId::new("mat_leper_healed"), b: EventId::new("rob_leper_healed"), provenance: "curated-analogues".into() });
        let svc = provenance_service(g);
        assert_eq!(svc.analogue_provenance("mat_leper_healed", "rob_leper_healed").as_deref(), Some("curated-analogues"));
        assert_eq!(svc.analogue_provenance("rob_leper_healed", "mat_leper_healed").as_deref(), Some("curated-analogues"));
        assert_eq!(svc.analogue_provenance("mat_leper_healed", "nothing"), None);
    }

    #[test]
    fn only_event_mentions_reach_event_mentions_provenance() {
        use atlas_graph_types::edge::{MentionedEntity, Mentions};
        use atlas_graph_types::id::EventId;
        let mut g = Graph::default();
        g.mentions.push(Mentions { locus: prov_locus(), entity: MentionedEntity::Event(EventId::new("theo-249")), provenance: "event-mentions".into() });
        g.mentions.push(Mentions { locus: prov_locus(), entity: MentionedEntity::Person(atlas_graph_types::id::PersonId::new("joseph_1")), provenance: "theographic-people".into() });
        let svc = provenance_service(g);
        assert_eq!(svc.event_mentions_provenance("theo-249"), vec!["event-mentions".to_string()]);
        assert_eq!(svc.provenance.by_family(crate::provenance::family::MENTIONS), vec!["event-mentions".to_string(), "theographic-people".to_string()]);
    }

    #[test]
    fn a_person_named_twice_in_a_verse_is_listed_there_once() {
        use atlas_graph_types::edge::{MentionedEntity, Mentions};
        use atlas_graph_types::id::PersonId;
        use atlas_graph_types::node::{Node, NodePayload};
        use atlas_graph_types::text::{BibleLocus, TextLocus, VerseRef};
        // Arrange
        let (abram, sarai) = (PersonId::new("abraham_58"), PersonId::new("sarah_1"));
        let mut g = Graph::default();
        for (id, label) in [(&abram, "Abram"), (&sarai, "Sarai")] {
            let payload = NodePayload::Person {
                label: label.into(),
                gender: None,
                birth_year: None,
                death_year: None,
                also_called: vec![],
                description: None,
                first_year: None,
                last_year: None,
                eternal: false,
                eternal_grounds: vec![],
            };
            g.nodes.insert(id.clone().erase(), Node { id: id.clone().erase(), payload, provenance: "test".into() });
        }
        let at_word = |ord: u16| TextLocus::from(BibleLocus { unit: VerseRef { book: 0, chapter: 12, verse: 11 }, span: Some(crate::tokens::span(crate::kjv_adapter::KJV_TRANSLATION, ord, ord).expect("one word")) });
        for (ord, person) in [(0, &abram), (9, &abram), (13, &sarai)] {
            g.mentions.push(Mentions { locus: at_word(ord), entity: MentionedEntity::Person(person.clone()), provenance: "test".into() });
        }
        let svc = provenance_service(g);
        // Act
        let persons = svc.persons_at_verse(0, 12, 11);
        // Assert
        assert_eq!(persons, vec![("abraham_58".to_string(), "Abram".to_string()), ("sarah_1".to_string(), "Sarai".to_string())]);
    }

    #[test]
    fn chapter_span_covers_exactly_that_chapters_verses_and_no_more() {
        let svc = service();
        let (start, n) = svc.chapter_span(0, 1).unwrap();
        assert_eq!(n, 3, "Genesis 1 has 3 verses in this fixture");
        let ids = svc.snapshot().reading_window(crate::kjv_adapter::BIBLE_CORPUS, start, n);
        let decoded: Vec<_> = ids.iter().map(|id| crate::kjv_adapter::decode_text_unit(id).unwrap()).collect();
        assert_eq!(decoded, vec![(0, 1, 1), (0, 1, 2), (0, 1, 3)], "must not spill into chapter 2");
    }

    #[test]
    fn verse_text_of_equals_window_render_for_the_same_verse() {
        let svc = service();
        let id = crate::kjv_adapter::verse_node_id(0, 1, 1);
        let want = crate::window::render(&svc.snapshot(), &id);
        assert_eq!(want.as_deref(), Some("In the beginning God created the heaven and the earth."));
        let got = svc.verse_text_of(&atlas_graph_types::text::VerseRef { book: 0, chapter: 1, verse: 1 });
        assert_eq!(got, want, "verse_text_of must read exactly what window::render gives for the same verse");
    }

    #[test]
    fn verse_text_of_is_none_for_an_out_of_canon_verse() {
        let svc = service();
        assert_eq!(svc.verse_text_of(&atlas_graph_types::text::VerseRef { book: 0, chapter: 99, verse: 1 }), None, "an unknown verse must be a graceful None, not a panic");
    }
}
