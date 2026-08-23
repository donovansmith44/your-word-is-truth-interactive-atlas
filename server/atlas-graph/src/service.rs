//! `GraphService`: the one built-graph handle atlas-server holds (behind
//! an `Arc`, built once at startup).
//!
//! FIX ROUND 1 (C1): the GraphStore port -- `GraphQuery`/`GraphSnapshot`/
//! `GraphStore`/`GraphPublisher` -- is now the OWNER-APPROVED shape in
//! `atlas_graph_types::store` (commit `ab75c8b`, "GraphQuery — the shared
//! interface"). This crate does NOT define, and must never define, a
//! competing/parallel store or query trait (a Critical finding against an
//! earlier draft of this file that did exactly that). `GraphService` is
//! purely an ADAPTER-SIDE convenience wrapper around the owner's own
//! `MemStore`/`MemSnapshot` (implementation #1 of the port) plus the one
//! small piece of bookkeeping the port itself doesn't model: a
//! reading-spine reverse index (`GraphQuery` has no "ref -> spine
//! position" lookup, only forward `reading_window`). Every actual GRAPH
//! QUERY this crate or atlas-server performs goes through
//! `atlas_graph_types::store::GraphQuery`'s own trait methods
//! (`node`/`edges`/`edge_summary`/`reading_window`) -- see `window.rs`,
//! which is generic over `&dyn GraphQuery` and touches nothing else.
//!
//! FIDELITY (design doc P3) is enforced unconditionally, as part of
//! `from_sources`/`build` -- a `GraphService` built from real raw KJV JSON
//! cannot exist without having already passed `fidelity::check_kjv_fidelity`
//! (checked on the raw `Graph`, BEFORE it is ever published to the store --
//! "proof at the boundary" is the boundary between raw source bytes and
//! the compiled graph, which is exactly the pre-publish moment; publishing
//! moves the `Graph` by value, and `GraphQuery` has no "enumerate every
//! node" operation, so a full bijection check isn't expressible through
//! the port alone after that point in any case). BATCH M-C: the check
//! itself now runs INSIDE `build::build_graph_from_sources`, as the
//! compiler pipeline's own LAW-CHECK stage (`pipeline.rs`) -- this
//! module's own `from_sources` no longer calls it a second time
//! explicitly; the guarantee is unchanged (a `GraphService` still cannot
//! exist without the check having passed), only WHERE it runs moved.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;

use atlas_core::data::{AtlasData, Canon};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::AnyNodeId;
use atlas_graph_types::store::{GraphPublisher, GraphQuery, GraphStore, GraphVersion, MemSnapshot, MemStore};

use crate::artifact;
use crate::build::{self, BuildStats};
use crate::event_world::{Chronology, EventWorldStats};

pub struct GraphService {
    snapshot: MemSnapshot,
    /// Reverse index: TextUnit node id -> its position in the bible
    /// reading spine. `GraphQuery` (the port) has no such lookup (only
    /// forward `reading_window`); building this once, alongside the
    /// graph, is what lets `chapter_span`/`position_of` below resolve
    /// "start from this ref" without scanning the whole spine per call.
    bible_position: HashMap<AnyNodeId, usize>,
    pub stats: BuildStats,
    /// Batch M-B (narrowed at M-C, renamed `EventWorld` -> `Chronology`):
    /// the chronology companion index -- same status as `bible_position`
    /// above (the generic `GraphQuery` port does not model
    /// `temporal-adjacency`; see `event_world::Chronology`'s own doc
    /// comment for why it's disclosed, not silent, and for the M-C
    /// retirement of this struct's own former narrative-positions half,
    /// now served by the generic port's `EdgeMeta::Narrative`).
    pub chronology: Chronology,
    pub event_world_stats: EventWorldStats,
    /// M-C (map migration, controller decision 7): every Era/Polity node's
    /// own id, in ascending id order -- `GraphQuery`'s port surface is
    /// deliberately minimal (derive/node-lookup/edges/reading-window, no
    /// "list every node of kind K" operation), so a LISTING endpoint
    /// (`/api/eras`, `/api/polities`) needs a companion enumeration, same
    /// status as `bible_position` above. Built once, from the graph's own
    /// node table (a cheap one-time scan; era/polity counts stay in the
    /// tens) -- every actual FIELD each listed era/polity carries still
    /// comes from that node's own payload via a real `GraphQuery::node`
    /// call at request time, never cached here.
    pub era_ids: Vec<AnyNodeId>,
    pub polity_ids: Vec<AnyNodeId>,
}

/// The longest KJV chapter (Psalm 119) has 176 verses; this probe width is
/// a comfortable, documented margin -- see `chapter_span` below.
const MAX_CHAPTER_SPAN_PROBE: usize = 200;

impl GraphService {
    /// The real KJV/xrefs raw-source path -- the FIDELITY LAW is enforced
    /// unconditionally, as the pipeline's own LAW-CHECK stage
    /// (`build::build_graph_from_sources` -> `pipeline::run_pipeline`;
    /// see `pipeline.rs`'s own module doc comment, controller decision 3).
    /// `atlas`: Batch M-B's own event-world source (see `event_world`'s
    /// own module doc comment).
    pub fn from_sources(kjv_json: &str, xrefs_tsv: &str, atlas: &AtlasData) -> anyhow::Result<Self> {
        Self::from_sources_with_eras(kjv_json, xrefs_tsv, atlas, &[])
    }

    /// M-C: the richer form real startup (and the artifact compile step)
    /// use -- `eras` is `era_adapter.rs`'s own pre-parsed source. See
    /// `build::build_graph_from_sources_with_eras`'s own doc comment.
    pub fn from_sources_with_eras(kjv_json: &str, xrefs_tsv: &str, atlas: &AtlasData, eras: &[atlas_core::data::Era]) -> anyhow::Result<Self> {
        let (graph, stats, event_world_stats, chrono) = build::build_graph_from_sources_with_eras(kjv_json, xrefs_tsv, atlas, eras)?;
        Ok(Self::assemble(graph, stats, event_world_stats, Chronology::from_derivation(chrono)))
    }

    /// Test-fixture path: builds from an already-parsed `(Canon, verses)`
    /// pair (see `build::build_graph_from_canon_and_verses`'s own doc
    /// comment) -- no raw source BYTES exist to re-derive "expected" from,
    /// so the fidelity law is not applicable here (there is nothing
    /// independent to check the already-typed input against).
    pub fn from_canon_and_verses(canon: &Canon, verses: &HashMap<String, String>, xrefs_tsv: &str, atlas: &AtlasData) -> anyhow::Result<Self> {
        Self::from_canon_and_verses_with_eras(canon, verses, xrefs_tsv, atlas, &[])
    }

    /// M-C: the eras-carrying form of `from_canon_and_verses` -- lets a
    /// fixture that already has real `Era` data (e.g. `AtlasData::
    /// demo_fixture`'s own `.eras`) build a graph whose `/api/eras` view
    /// has something real to serve, without a round trip through raw KJV
    /// JSON text just to reach the eras-carrying constructor.
    pub fn from_canon_and_verses_with_eras(canon: &Canon, verses: &HashMap<String, String>, xrefs_tsv: &str, atlas: &AtlasData, eras: &[atlas_core::data::Era]) -> anyhow::Result<Self> {
        let (graph, stats, event_world_stats, chrono) = build::build_graph_from_canon_and_verses_with_eras(canon, verses, xrefs_tsv, atlas, eras)?;
        Ok(Self::assemble(graph, stats, event_world_stats, Chronology::from_derivation(chrono)))
    }

    /// M-C (controller decision 4): loads from a SERIALIZED ARTIFACT --
    /// GraphStore implementation #2 -- instead of building from raw
    /// sources. No `AtlasData`/raw KJV bytes touched at all: the artifact
    /// carries every row/node table plus the chronology companion's own
    /// fields (`artifact.rs`'s own module doc comment for exactly what
    /// "logical artifact" means here). The KJV fidelity law does not run
    /// here (there are no raw bytes on this path to independently re-check
    /// against -- fidelity was already proven once, at COMPILE time, by
    /// the compile step that produced this file, via the SAME
    /// `assert_answers_match` admission `tests/artifact_conformance.rs`
    /// exercises); this is the "proof at the boundary, once" the whole
    /// design insists on (P3), not a weakening of it.
    pub fn from_artifact(path: &Path) -> anyhow::Result<Self> {
        let dump = artifact::read_file(path).map_err(|e| anyhow::anyhow!("{e}"))?;
        let (mut graph, stats, event_world_stats, chronology) = artifact::to_service_parts(dump).map_err(|e| anyhow::anyhow!("{e}"))?;
        graph.build_indexes();
        crate::event_world::add_justified_by(&mut graph);
        Ok(Self::assemble(graph, stats, event_world_stats, chronology))
    }

    /// Reads `raw_dir/kjv.json` and `raw_dir/xrefs/cross_references.txt`
    /// and builds from them, plus the event world from `atlas` — the only
    /// filesystem-touching function in this crate. M-C: also reads
    /// `data/curated/eras.toml` (`raw_dir`'s own sibling `curated/`,
    /// matching every real invocation's own `data/` parent layout, the
    /// SAME derivation `atlas-server/src/main.rs` already uses for
    /// `raw_dir` itself) via `atlas_etl::curated::parse_eras` -- the SAME
    /// parser the pre-M-C `eras.json` compilation path used.
    pub fn build(raw_dir: &Path, atlas: &AtlasData) -> anyhow::Result<Self> {
        let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json"))
            .with_context(|| format!("reading {}", raw_dir.join("kjv.json").display()))?;
        let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt"))
            .with_context(|| format!("reading {}", raw_dir.join("xrefs/cross_references.txt").display()))?;
        let eras = load_eras(raw_dir)?;
        Self::from_sources_with_eras(&kjv_json, &xrefs_tsv, atlas, &eras)
    }

    /// M-C: takes an ALREADY-BUILT `Chronology` rather than `&AtlasData` --
    /// the artifact-load path (`from_artifact`) has no `AtlasData` at all
    /// (the whole point of loading from bytes instead of rebuilding), so
    /// every caller now builds its own `Chronology` however it can (from
    /// `AtlasData`, via `Chronology::build`, on the from-sources paths; from
    /// the artifact's own serialized fields, via `artifact::to_service_parts`,
    /// on the from-artifact path) and hands the finished value in here.
    fn assemble(graph: Graph, stats: BuildStats, event_world_stats: EventWorldStats, chronology: Chronology) -> Self {
        let bible_position = graph
            .reading
            .get(crate::kjv_adapter::BIBLE_CORPUS)
            .map(|spine| spine.order.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect())
            .unwrap_or_default();
        // M-C (map migration): the era_ids/polity_ids companion
        // enumeration (this struct's own doc comment) -- a one-time scan
        // over the node table's own kind tag, before the graph moves into
        // the store below. `era_ids` is sorted by `from_year` (chronological,
        // ascending) rather than left in the node table's own id-alphabetical
        // order (`BTreeMap<AnyNodeId, _>`'s own iteration order): the
        // pre-M-C `/api/eras` wire response's own order was the curated
        // TOML's file order, itself chronological (verified: the real
        // `eras.toml` lists primeval/patriarchs/egypt-exodus/... in
        // strictly ascending `from_year` order) -- sorting by `from_year`
        // reproduces that exact order without needing to thread the
        // original parse order through this far. `polity_ids` needs no
        // such care: the handler re-sorts its own results explicitly
        // (by polity id, then era `from`), unchanged from before this batch.
        let mut era_nodes: Vec<(i32, AnyNodeId)> = graph
            .nodes
            .iter()
            .filter_map(|(id, n)| match &n.payload {
                atlas_graph_types::node::NodePayload::Era { from_year, .. } if id.kind == atlas_graph_types::id::NodeKind::Era => Some((*from_year, id.clone())),
                _ => None,
            })
            .collect();
        era_nodes.sort_by_key(|(from_year, id)| (*from_year, id.raw.clone()));
        let era_ids: Vec<AnyNodeId> = era_nodes.into_iter().map(|(_, id)| id).collect();
        let polity_ids: Vec<AnyNodeId> = graph.nodes.keys().filter(|id| id.kind == atlas_graph_types::id::NodeKind::Polity).cloned().collect();
        // GraphPublisher::publish (design doc §9a): the compiler
        // publishes; serving never writes. One publish, at startup; M-A
        // never calls it again (no hot-reload exists yet) -- MemStore's
        // own version-history retention (it keeps every published
        // version, not just the latest) is what would make a future
        // republish safe for any reader still holding an older snapshot,
        // without this crate needing to do anything extra to earn that.
        let mut store = MemStore::default();
        let version = store.publish(graph);
        let snapshot = store.open(version).expect("the version just published must always be open-able");
        GraphService { snapshot, bible_position, stats, chronology, event_world_stats, era_ids, polity_ids }
    }

    /// The version this service published at construction (M-A: the only
    /// version there will ever be, for this process's lifetime).
    pub fn version(&self) -> GraphVersion {
        atlas_graph_types::store::GraphSnapshot::version(&self.snapshot)
    }

    /// Opens THE PORT's own snapshot handle -- `MemSnapshot` implements
    /// `atlas_graph_types::store::GraphQuery`, so every actual graph
    /// query goes through its trait methods from here on, never through a
    /// direct `Graph` field reach. Cheap to clone (an `Arc<Graph>` inside).
    pub fn snapshot(&self) -> MemSnapshot {
        self.snapshot.clone()
    }

    /// `id`'s own position in the bible reading spine, if it has one --
    /// resolves a ref into a window's own starting index. NOT part of the
    /// generic port (see this struct's own doc comment) -- an adapter-side
    /// companion, same status as `chapter_span` below.
    pub fn position_of(&self, book: u8, chapter: u16, verse: u16) -> Option<usize> {
        self.bible_position.get(&crate::kjv_adapter::verse_node_id(book, chapter, verse)).copied()
    }

    /// The (start, n) window covering exactly one chapter -- `scope=chapter`'s
    /// own "N derived server-side" (M-A brief requirement 4): still just an
    /// input to `window::window` (the SAME generic, port-only primitive
    /// every other window call uses), not a separate query mechanism.
    /// Derived by probing the port's own `reading_window` for a generous
    /// upper bound starting at the chapter's first verse, then trimming to
    /// the contiguous run that actually shares (book, chapter).
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
}

/// `raw_dir`'s sibling `curated/eras.toml`, parsed via
/// `atlas_etl::curated::parse_eras` -- the one other filesystem read this
/// crate performs, alongside `GraphService::build`'s own KJV/xrefs reads.
fn load_eras(raw_dir: &Path) -> anyhow::Result<Vec<atlas_core::data::Era>> {
    let curated_dir = raw_dir.parent().map(|p| p.join("curated")).unwrap_or_else(|| Path::new("../data/curated").to_path_buf());
    let path = curated_dir.join("eras.toml");
    let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    atlas_etl::curated::parse_eras(&text).with_context(|| format!("parsing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::store::GraphSnapshot as _;

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

    #[test]
    fn chapter_span_covers_exactly_that_chapters_verses_and_no_more() {
        let svc = service();
        let (start, n) = svc.chapter_span(0, 1).unwrap();
        assert_eq!(n, 3, "Genesis 1 has 3 verses in this fixture");
        let ids = svc.snapshot().reading_window(crate::kjv_adapter::BIBLE_CORPUS, start, n);
        let decoded: Vec<_> = ids.iter().map(|id| crate::kjv_adapter::decode_text_unit(id).unwrap()).collect();
        assert_eq!(decoded, vec![(0, 1, 1), (0, 1, 2), (0, 1, 3)], "must not spill into chapter 2");
    }
}
