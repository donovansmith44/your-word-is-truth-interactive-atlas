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

use std::collections::{BTreeMap, HashMap};
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
    /// M-C2: narrative id -> its own `succession` row's `chain`, in order
    /// -- the single source `handlers::narratives`/`legacy::
    /// narrative_from_node` read for `legs`, never duplicated onto the
    /// `Narrative` node's own payload (see `NodePayload::Narrative`'s own
    /// doc comment). A narrative with zero legs has no entry at all
    /// (lawful, honest absence -- `event_world::populate_nodes_and_
    /// direct_rows` never emits a `Succession` row for one).
    pub narrative_legs: BTreeMap<String, Vec<String>>,
    /// M-C2 (requirement 1, the decisive-title law re-homed as a graph
    /// query): verse -> the one pericope heading that wins there, per
    /// `heading::build_heading_index`'s own 3-tier (+ determinism) rule --
    /// precomputed once here (not per-request), the same "O(1) per-verse
    /// lookup for a whole-chapter fetch" reasoning `bible_position` itself
    /// already established.
    pub heading_index: BTreeMap<String, crate::heading::HeadingEntry>,
    /// M-C2 (requirement 2, unblocking `aggregate_span_xrefs`): FROM-verse
    /// dot-ref -> every `cites` row it authors, in the SAME
    /// `atlas_core::data::CrossRef { target, votes }` shape
    /// `atlas_core::xrefs::aggregate_span_xrefs` already takes (`target` is
    /// the row's own `target_display` -- the ORIGINAL citation string,
    /// never a re-synthesized one) -- lets `handlers::xrefs`/`handlers::
    /// verse` call that EXISTING, already-tested aggregation function
    /// completely unchanged (its own signature is `&HashMap<...>`, hence
    /// `HashMap` here, not `BTreeMap` -- this companion is ONLY ever
    /// read via per-key `.get()`, never iterated as a whole, so its own
    /// iteration order is irrelevant and carries no determinism concern,
    /// unlike a value that gets serialized). The port's own `edges()`
    /// cannot serve this (an `EdgeEntry` carries only the target's FIRST
    /// verse + `EdgeMeta::Votes`, not the row's own `to_last`/
    /// `target_display`) -- same "port doesn't model this access shape"
    /// class as `narrative_legs` above.
    ///
    /// DB-3 (plan judgment call 4): NOT retired with the other companions
    /// -- `target_display` (the original citation string) is a `cross_refs`
    /// ROW field the port does not expose, and the spec's replacement is a
    /// `kjv.cross_refs` seek the server can only make once it reads the
    /// SQLite sections (DB-4).
    pub cross_refs_by_from: HashMap<String, Vec<atlas_core::data::CrossRef>>,
    /// RED-1 (decision 4, "the heading-index precedent"): dot-ref -> the
    /// KJV sub-verse span table's own char-offset ranges for that verse --
    /// the SAME "precomputed once here, O(1) per-verse lookup" treatment
    /// `heading_index`/`persons_by_verse` above already get, EXCEPT this
    /// one companion is NOT derivable from the graph at all (it needs the
    /// original OSIS alignment, which the graph itself never carries --
    /// `red_letter_spans.rs`'s own module doc comment: "compiled-data-side,
    /// never the graph artifact") -- `assemble` below takes it as an
    /// ALREADY-RESOLVED parameter instead of computing it from `&graph`,
    /// the one companion field with that shape. `HashMap`, not `BTreeMap`
    /// -- same "only ever `.get()`'d by key" reasoning as `cross_refs_by_
    /// from`/`persons_by_verse` above.
    pub red_letter_spans: HashMap<String, Vec<(usize, usize)>>,
    /// Batch PROV-1 (owner order 1, "one thing we definitely need for
    /// EVERY PIECE OF DATA is the source from which it came"): the SAME
    /// "companion the generic port doesn't model" class as
    /// `narrative_legs`/`temporal_neighbors` above, built once here from
    /// the raw, pre-store `graph` exactly the way they are. It exists
    /// because the served graph genuinely cannot answer this: rows carry
    /// `ProvenanceId`, `BiIndex`/`EdgeMeta` do not, and `MemSnapshot`'s
    /// own `graph` field is private -- see `crate::provenance`'s own module
    /// header for the full constraint, and for why widening `EdgeMeta`
    /// (a frozen-contract change, and a string on every one of ~344k
    /// `cites` index entries) is the wrong shape for it.
    pub provenance: crate::provenance::ProvenanceIndex,
    /// OVERLAY-1 Task 5: the map scene's own data, materialised once from
    /// THIS service's port (`crate::scene_source::GraphSceneSource`) --
    /// what `handlers::scene_time`/`scene_scripture` compose against now
    /// that `legacy::atlas_data_overlay` is gone. Held here rather than as
    /// a fourth `AppState` member deliberately: all EIGHTEEN existing
    /// callers of `app::build`/`app::build_with_sources` (the test
    /// fixtures, the AQC example exporter, the pact recorder) keep their
    /// exact signatures, `main.rs`'s own `--build-from-raw` dev fallback
    /// needs no second construction path, and the two scene handlers reach
    /// it through the `State<Arc<GraphService>>` extractor they can already
    /// ask for.
    ///
    /// `OnceLock`, not a plain field, for the one thing it genuinely needs
    /// that `assemble` cannot give it: the two CURATED-JSON sidecar maps
    /// `scene.rs` reads (`place_history`, `place_name_aliases`) live in
    /// `AtlasData`, which this crate never loads. `scene_source(&data)`
    /// below builds it on first call and hands back the same borrow
    /// forever after. The real server and the CLI PRIME it at load time
    /// (`atlas_server::load::load_graph_and_data`,
    /// `atlas_cli::load::load`) so no request ever pays the
    /// materialisation; every other caller (fixtures, benches, the CLI's
    /// own commands) gets it built on first use, which is why not one of
    /// those eighteen sites had to change.
    scene_source: std::sync::OnceLock<crate::scene_source::GraphSceneSource>,
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
        Self::from_sources_with_eras_and_brainfuel(kjv_json, xrefs_tsv, atlas, eras, None)
    }

    /// CORP-1a: the richest raw-source constructor -- see
    /// `build::build_graph_from_sources_with_eras_and_brainfuel`'s own doc
    /// comment. `from_sources_with_eras` above delegates here with `None`,
    /// unchanged behavior for every existing caller.
    pub fn from_sources_with_eras_and_brainfuel(
        kjv_json: &str,
        xrefs_tsv: &str,
        atlas: &AtlasData,
        eras: &[atlas_core::data::Era],
        brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>,
    ) -> anyhow::Result<Self> {
        Self::from_sources_with_eras_and_brainfuel_and_concord(kjv_json, xrefs_tsv, atlas, eras, brainfuel, None)
    }

    /// CORP-2a: the richest raw-source constructor yet -- see `build::
    /// build_graph_from_sources_with_eras_and_brainfuel_and_concord`'s own
    /// doc comment. `from_sources_with_eras_and_brainfuel` above delegates
    /// here with `None`, unchanged behavior for every existing caller.
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

    /// KRETZ-1: the richest raw-source constructor yet -- see `build::
    /// build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_
    /// kretzmann`'s own doc comment. `from_sources_with_eras_and_brainfuel_
    /// and_concord` above delegates here with `None`, unchanged behavior
    /// for every existing caller.
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

    /// RED-1: the richest raw-source constructor yet -- see `build::
    /// build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_
    /// kretzmann_and_red_letter`'s own doc comment.
    /// `from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann`
    /// above delegates here with `None`, unchanged behavior for every
    /// existing caller. `red_letter_spans` (the `GraphService` companion,
    /// this struct's own field doc comment) is derived here, straight off
    /// the SAME `red_letter` corpus, against the SAME restored verses
    /// `build::...` computes internally for the graph itself -- recomputed
    /// once more here rather than threaded out of that call (a small,
    /// disclosed, one-time-at-startup duplication, the SAME "recompute a
    /// cheap pure derivation a second time for a second purpose" class
    /// `bins/compile_graph.rs`'s own `case_restoration`-for-the-startup-log
    /// recompute already establishes).
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
        // RED-1: this path never carries a real red-letter corpus (no raw
        // source bytes at all on this test-fixture path) -- an honestly
        // empty span table, the SAME "absent == empty" treatment every
        // other companion here gets.
        Ok(Self::assemble(graph, stats, event_world_stats, Chronology::from_derivation(chrono), HashMap::new(), None))
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
        // NODE1-ROWS-1 (fix round 1): container membership/succession are
        // artifact-serialized rows now -- `build_indexes` above lowers
        // them like every other row family; no post-index derivation step
        // exists any more, so from-artifact and from-sources agree by
        // construction.
        // RED-1: the KJV sub-verse span table's own sibling file --
        // `<data_dir>/red-letter-spans.json`, the SAME "disclosed
        // convention: same directory every other compiled file already
        // lives in" `main.rs`'s own `--build-from-raw` doc comment already
        // establishes for `graph.bin` itself. `None` (the file doesn't
        // exist -- an older `data/compiled/` snapshot, or a test fixture
        // directory) is an honestly empty span table, never an error on
        // this path (`red_letter_spans::read_file`'s own doc comment).
        let spans_path = path.parent().map(|p| p.join("red-letter-spans.json")).unwrap_or_else(|| std::path::PathBuf::from("red-letter-spans.json"));
        let red_letter_spans: HashMap<String, Vec<(usize, usize)>> = crate::red_letter_spans::read_file(&spans_path)?.unwrap_or_default().into_iter().collect();
        // DB-4b: the sidecars beside `graph.bin` (`None` for a fixture
        // directory without `canon.json` -- graph-derived tables only).
        let data_dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| std::path::PathBuf::from("."));
        let sidecars = crate::sqlite::sidecars::Sidecars::load(&data_dir)?;
        Ok(Self::assemble(graph, stats, event_world_stats, chronology, red_letter_spans, sidecars.as_ref()))
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
        // CORP-1a: `data/raw/brain-fuel-bible/` (vendored, gitignored --
        // see data/raw/README.md) is read here too, GRACEFULLY: a caller
        // passing a synthetic/fixture `raw_dir` with no such subdirectory
        // (several existing tests do exactly this) gets an honestly empty
        // corpus, not an error -- only a REAL, present vendored tree is
        // ever parsed, and parsing IT is still fail-loud (a malformed
        // vendored file is a real bug, never silently skipped).
        let brainfuel = load_brainfuel(raw_dir)?;
        // CORP-2a: `data/raw/concord/*.html` + `data/curated/
        // concord-sc-overlap.toml` -- the SAME graceful-absence treatment
        // `load_brainfuel` above already gets (a fixture `raw_dir` with no
        // such subdirectory is an honestly empty build, never an error;
        // real, present vendored files are parsed fail-loud).
        let concord = load_concord(raw_dir)?;
        // KRETZ-1: `data/raw/kretzmann/{slug}/{chapter}.html` -- the SAME
        // graceful-absence treatment `load_concord`/`load_brainfuel` above
        // already get. Fix round 1: also threads `kjv_json` through (already
        // in scope above) -- the OVER-EXCISION GUARD's own real canonical
        // source (`kretzmann::read_all`'s own doc comment).
        let kretzmann = load_kretzmann(raw_dir, &kjv_json)?;
        // RED-1: `data/raw/red-letter/eng-kjv.osis.xml` -- the SAME
        // graceful-absence treatment `load_kretzmann`/`load_concord`/
        // `load_brainfuel` above already get. Aligned against RESTORED
        // text (`load_red_letter`'s own doc comment) -- the span-alignment
        // law runs against the graph's own restored casing, never the raw
        // parse.
        let red_letter = load_red_letter(raw_dir, &kjv_json, brainfuel.as_ref())?;
        Self::from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(&kjv_json, &xrefs_tsv, atlas, &eras, brainfuel.as_ref(), concord.as_ref(), kretzmann.as_ref(), red_letter.as_ref())
    }

    /// M-C: takes an ALREADY-BUILT `Chronology` rather than `&AtlasData` --
    /// the artifact-load path (`from_artifact`) has no `AtlasData` at all
    /// (the whole point of loading from bytes instead of rebuilding), so
    /// every caller now builds its own `Chronology` however it can (from
    /// `AtlasData`, via `Chronology::build`, on the from-sources paths; from
    /// the artifact's own serialized fields, via `artifact::to_service_parts`,
    /// on the from-artifact path) and hands the finished value in here.
    ///
    /// DB-4b: `sidecars` -- the compiled directory's own `AtlasData` +
    /// `sources.json` (`from_artifact` loads them beside `graph.bin`) --
    /// fold into the graph's `extra_tables` here, before `publish`, so this
    /// service's version IS `manifest.toml`'s root. The from-sources
    /// constructors pass `None`: graph-derived tables only, a root that is
    /// DISCLOSED as not the manifest's (the `--build-from-raw` dev fallback
    /// and the fixture constructors). `heading_index` is built twice on the
    /// way (once for the service, once inside `Extras::graph_derived`):
    /// 1,711 events, milliseconds, one function, no drift.
    fn assemble(
        mut graph: Graph,
        stats: BuildStats,
        event_world_stats: EventWorldStats,
        chronology: Chronology,
        red_letter_spans: HashMap<String, Vec<(usize, usize)>>,
        sidecars: Option<&crate::sqlite::sidecars::Sidecars>,
    ) -> Self {
        let mut narrative_legs: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for row in &graph.succession {
            narrative_legs.insert(row.narrative.0.clone(), row.chain.iter().map(|e| e.0.clone()).collect());
        }
        // M-C2 (requirement 1): the decisive-title law's own precomputed
        // index -- built here, over the pre-store `graph`, the SAME
        // "cheap, once, at assemble time" treatment every other companion
        // above already gets. M-D3 (owner ruling R1 propagation):
        // `chronology.chrono.resolved` is ALREADY fully built by this point
        // (the `chronology` parameter, assembled by every caller before
        // `assemble` runs) -- passed in so `build_heading_index` can source
        // chronology tie-breaking from the timeline's one authority instead
        // of the payload's now-removed `from_year`/`order_key` mirror.
        let heading_index = crate::heading::build_heading_index(&graph, &chronology.chrono.resolved);
        // DB-4b: the non-graph section tables ride the graph into the root
        // (graph-types `Graph::extra_tables`), computed from the same values
        // the section writer folds (`sqlite::extras`).
        let mut extras = crate::sqlite::extras::Extras::graph_derived(&graph, &chronology.chrono.resolved, &red_letter_spans)
            .expect("assemble: the graph's projections encode");
        if let Some(sc) = sidecars {
            extras.extend(crate::sqlite::sidecars::fold_sidecars(&sc.atlas, &sc.sources).expect("assemble: the sidecars fold"));
        }
        extras.attach(&mut graph);
        // M-C2 (requirement 2): the SAME treatment for the cites relation's
        // own span data -- see this struct's own `cross_refs_by_from` doc
        // comment.
        let mut cross_refs_by_from: HashMap<String, Vec<atlas_core::data::CrossRef>> = HashMap::new();
        for row in &graph.cross_refs {
            let Some(key) = crate::legacy::locus_dot_ref(&row.from) else { continue };
            cross_refs_by_from.entry(key).or_default().push(atlas_core::data::CrossRef { target: row.target_display.clone(), votes: row.votes as i32 });
        }
        // GraphPublisher::publish (design doc §9a): the compiler
        // publishes; serving never writes. One publish, at startup; M-A
        // never calls it again (no hot-reload exists yet) -- MemStore's
        // own version-history retention (it keeps every published
        // version, not just the latest) is what would make a future
        // republish safe for any reader still holding an older snapshot,
        // without this crate needing to do anything extra to earn that.
        // Batch PROV-1: the LAST pre-store scan -- `graph` moves into the
        // store on the very next line, and this index is the only way any
        // handler afterward can read a ROW's provenance at all (see
        // `crate::provenance`'s own module header).
        let provenance = crate::provenance::ProvenanceIndex::build(&graph);
        let mut store = MemStore::default();
        let version = store.publish(graph);
        let snapshot = store.open(version).expect("the version just published must always be open-able");
        GraphService {
            snapshot,
            stats,
            chronology,
            event_world_stats,
            narrative_legs,
            heading_index,
            cross_refs_by_from,
            red_letter_spans,
            provenance,
            scene_source: std::sync::OnceLock::new(),
        }
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
    /// DB-3 (spec 4): every node id of one kind, in id (byte) order -- the
    /// port's `nodes_of_kind`, drained. Replaces the six per-kind id lists
    /// this struct used to precompute; `era_ids`' chronological order is
    /// the eras handler's own concern now (it sorts the payloads it fetches).
    pub fn ids_of_kind(&self, kind: atlas_graph_types::id::NodeKind) -> Vec<AnyNodeId> {
        let mut out: Vec<AnyNodeId> = Vec::new();
        let mut cursor = None;
        loop {
            let page = self.snapshot.nodes_of_kind(kind, cursor, 4096);
            out.extend(page.ids);
            match page.next {
                Some(c) => cursor = Some(c),
                None => break out,
            }
        }
    }

    /// DB-3: the distinct, sorted provenance of every row behind the edges
    /// of one kind at one position -- `rows_behind` over one drained page
    /// walk. A synthesised edge contributes nothing.
    fn provenance_over(&self, p: &atlas_graph_types::id::Position, kind: atlas_graph_types::edge::EdgeKind) -> Vec<String> {
        use atlas_graph_types::explore::EdgeQuery;
        let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut cursor = None;
        loop {
            let page = self.snapshot.edges(p, &EdgeQuery { kind, cursor, limit: 256 });
            for e in &page.entries {
                // `rows_behind`, not `row_provenance`: two rows minting one
                // id (the leper lesson) both count.
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

    /// DB-3: the distinct provenance of the `Attests` rows for ONE event
    /// (the "PARALLEL ACCOUNTS" section's own sources) -- the retired
    /// `ProvenanceIndex::attests_for_event`, through the port.
    pub fn attests_provenance(&self, event_raw: &str) -> Vec<String> {
        use atlas_graph_types::edge::{at, Direction, EdgeKind, RelationId};
        self.provenance_over(&at(&atlas_graph_types::id::EventId::new(event_raw).erase()), EdgeKind::Directed(RelationId::Attests, Direction::Forward))
    }

    /// DB-3: the distinct provenance of the `Mentions` rows naming ONE event
    /// (a mention lowers with the event as OBJECT, so the inverse reading at
    /// the event lists exactly those rows) -- the retired
    /// `event_mentions_for_event`, through the port.
    pub fn event_mentions_provenance(&self, event_raw: &str) -> Vec<String> {
        use atlas_graph_types::edge::{at, Direction, EdgeKind, RelationId};
        self.provenance_over(&at(&atlas_graph_types::id::EventId::new(event_raw).erase()), EdgeKind::Directed(RelationId::Mentions, Direction::Inverse))
    }

    /// DB-3: the provenance of the ONE `Analogue` row joining two events,
    /// from either end -- the retired `analogue_for_pair`, through the port.
    pub fn analogue_provenance(&self, a_raw: &str, b_raw: &str) -> Option<String> {
        use atlas_graph_types::edge::{at, EdgeKind, SymRelationId};
        use atlas_graph_types::explore::EdgeQuery;
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

    /// DB-3: prior/following-in-time for ONE event -- the retired
    /// `temporal_neighbors` companion, through the port. `None` unless the
    /// event is in the chronology's order at all (the DOMAIN fact the old
    /// map seeded from `chronology.chrono.order`); adjacency from the
    /// `temporal-adjacency` edges; direction from that same order (the
    /// neighbour earlier in it is `prior`).
    pub fn temporal_neighbors_of(&self, event_raw: &str) -> Option<(Option<String>, Option<String>)> {
        use atlas_graph_types::edge::{at, EdgeKind, SymRelationId};
        use atlas_graph_types::explore::EdgeQuery;
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

    pub fn snapshot(&self) -> MemSnapshot {
        self.snapshot.clone()
    }

    /// `id`'s own position in the bible reading spine, if it has one --
    /// resolves a ref into a window's own starting index. NOT part of the
    /// generic port (see this struct's own doc comment) -- an adapter-side
    /// companion, same status as `chapter_span` below.
    pub fn position_of(&self, book: u8, chapter: u16, verse: u16) -> Option<usize> {
        // DB-3: the port's own `position_of` (spec 4); the reverse index
        // this used to read (`bible_position`) is retired.
        self.snapshot.position_of(crate::kjv_adapter::BIBLE_CORPUS, &crate::kjv_adapter::verse_node_id(book, chapter, verse))
    }

    /// CORP-2a (decision 8): the Concord-corpus sibling of `position_of`
    /// above — resolves a `(part, article, paragraph)` ref into the
    /// "concord" spine's own starting index.
    pub fn concord_position_of(&self, part: u8, article: u16, paragraph: u16) -> Option<usize> {
        self.snapshot.position_of(crate::concord_adapter::CONCORD_CORPUS, &crate::concord_adapter::text_unit_id(part, article, paragraph))
    }

    /// DB-3 (spec 4): every PERSON the `mentions` relation attests at one
    /// verse, `(id, display label)` in mentions-row order -- the retired
    /// `persons_by_verse` companion, answered through the port
    /// (`edges_with_nodes` over `mentions`, forward, at the verse). Only
    /// `Person` targets: the PG-1a seam (a PeopleGroup mention is a
    /// different wire field, not this one) is preserved by kind.
    pub fn persons_at_verse(&self, book: u8, chapter: u16, verse: u16) -> Vec<(String, String)> {
        use atlas_graph_types::edge::{at, Direction, EdgeKind, RelationId};
        use atlas_graph_types::explore::EdgeQuery;
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
                None => break out,
            }
        }
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

    /// OVERLAY-1 Task 2 ("one KJV in memory"): reads one verse's own KJV
    /// text ON DEMAND, straight off this service's own published snapshot
    /// -- the SAME `kjv_adapter::verse_node_id` + `window::render` pair
    /// `legacy::verses_from_graph` used to build the whole-spine
    /// `verse_text` companion this batch retires (Task 5 then deleted
    /// `legacy::atlas_data_overlay` itself, the third of the "three copies
    /// of the same KJV text in memory" this call replaces).
    /// One node lookup -- microseconds, not a whole-spine walk -- and no
    /// caching: every former reader of `verse_text`/`AtlasData.verses`
    /// calls this instead, per verse, at request time.
    pub fn verse_text_of(&self, r: &atlas_graph_types::text::VerseRef) -> Option<String> {
        let id = crate::kjv_adapter::verse_node_id(r.book, r.chapter, r.verse);
        crate::window::render(&self.snapshot(), &id)
    }

    /// OVERLAY-1 Task 5: the map scene's own data source, built ONCE from
    /// this service's port and cached -- see the `scene_source` field's own
    /// doc comment for why it lives here and why it is an `OnceLock`.
    ///
    /// `sidecars` is only ever read for the two curated-JSON maps
    /// `GraphSceneSource::build` copies (`place_history`,
    /// `place_name_aliases`); a bare, un-`finish()`ed `AtlasData::load(..)`
    /// is a valid argument, and none of that struct's compile-time
    /// `events`/`places`/`narratives` fields is touched. It is IGNORED on
    /// every call after the first -- the source is immutable once built, so
    /// a caller that passes a different `AtlasData` later does not rebuild
    /// it. In this codebase there is exactly one `AtlasData` per process,
    /// loaded before this is ever called, so that never arises; the eager
    /// priming at load time is what keeps it that way by construction.
    pub fn scene_source(&self, sidecars: &AtlasData) -> &crate::scene_source::GraphSceneSource {
        self.scene_source.get_or_init(|| crate::scene_source::GraphSceneSource::build(self, sidecars))
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

/// CORP-1a: `raw_dir/brain-fuel-bible/` -- `None` (not an error) when that
/// directory simply doesn't exist (see `GraphService::build`'s own doc
/// comment); `Some(Err(..))` propagated fail-loud when it exists but is
/// malformed.
fn load_brainfuel(raw_dir: &Path) -> anyhow::Result<Option<atlas_etl::brainfuel::BrainFuelCorpus>> {
    let root = raw_dir.join("brain-fuel-bible");
    if !root.is_dir() {
        return Ok(None);
    }
    atlas_etl::brainfuel::read_all(&root).map(Some).with_context(|| format!("reading vendored brain-fuel data from {}", root.display()))
}

/// CORP-2a: `raw_dir/concord/` -- `None` (not an error) when that
/// directory simply doesn't exist (`load_brainfuel`'s own doc comment,
/// same treatment); `Some(Err(..))` propagated fail-loud when it exists
/// but is malformed. Also reads the curated SC-overlap alignment
/// (`raw_dir`'s own sibling `curated/concord-sc-overlap.toml`, the SAME
/// derivation `load_eras` above uses for `curated/eras.toml`) -- bundled
/// together (`concord_adapter::ConcordBundle`'s own doc comment) since a
/// caller with real Concord HTML but no curated overlap file (or vice
/// versa) would be a real, if unlikely, misconfiguration worth failing
/// loud on rather than silently serving a half-built corpus.
fn load_concord(raw_dir: &Path) -> anyhow::Result<Option<crate::concord_adapter::ConcordBundle>> {
    let root = raw_dir.join("concord");
    if !root.is_dir() {
        return Ok(None);
    }
    let corpus = atlas_etl::concord::read_all(&root).with_context(|| format!("reading vendored Concord data from {}", root.display()))?;
    let curated_dir = raw_dir.parent().map(|p| p.join("curated")).unwrap_or_else(|| Path::new("../data/curated").to_path_buf());
    let overlap_path = curated_dir.join("concord-sc-overlap.toml");
    let overlap_text = std::fs::read_to_string(&overlap_path).with_context(|| format!("reading {}", overlap_path.display()))?;
    let sc_overlap = atlas_etl::concord::parse_sc_overlap(&overlap_text).with_context(|| format!("parsing {}", overlap_path.display()))?;
    Ok(Some(crate::concord_adapter::ConcordBundle { corpus, sc_overlap }))
}

/// KRETZ-1: `raw_dir/kretzmann/` -- `None` (not an error) when that
/// directory simply doesn't exist (`load_concord`'s own doc comment, same
/// treatment); `Some(Err(..))` propagated fail-loud when it exists but is
/// malformed (e.g. a missing page -- `atlas_etl::kretzmann::read_all`'s own
/// per-file `Result`). Fix round 1: `kjv_json` (the SAME raw string
/// `GraphService::build` already reads) is parsed here into the dot-ref
/// verse map `read_all`'s own OVER-EXCISION GUARD requires -- UN-restored
/// text is sufficient (word-content comparison only, no KJV-CASE dependency,
/// `read_all`'s own doc comment), so no brainfuel coupling is needed here.
fn load_kretzmann(raw_dir: &Path, kjv_json: &str) -> anyhow::Result<Option<atlas_etl::kretzmann::KretzmannCorpus>> {
    let root = raw_dir.join("kretzmann");
    if !root.is_dir() {
        return Ok(None);
    }
    let (_, kjv_verses) = atlas_etl::kjv::parse(kjv_json).context("parsing kjv.json for the Kretzmann over-excision guard")?;
    atlas_etl::kretzmann::read_all(&root, &kjv_verses).map(Some).with_context(|| format!("reading vendored Kretzmann data from {}", root.display()))
}

/// RED-1: `raw_dir/red-letter/` -- `None` (not an error) when that
/// directory simply doesn't exist (`load_kretzmann`'s own doc comment,
/// same treatment); `Some(Err(..))` propagated fail-loud when it exists
/// but is malformed. UNLIKE `load_kretzmann` (word-content comparison
/// only), this alignment is CASE-SENSITIVE-FIRST (the GAZ-1 law), so it
/// reads against RESTORED text -- `brainfuel`, when present, applies the
/// SAME KJV-CASE/KJV-CASE-2 restoration `build.rs` itself applies before
/// building the graph's own TextUnit nodes; a caller with no real
/// brainfuel data (most test fixtures) gets the raw, unrestored parse
/// instead -- an honest, if less precise, alignment target on that path,
/// never a panic.
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

    /// DB-3: a synthetic graph for the per-edge provenance laws that moved
    /// here from `provenance.rs` (the retired `ProvenanceIndex` per-event
    /// maps) -- rows only; the events need no node to be edge subjects.
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

    /// THE LEPER LESSON (moved from provenance.rs at DB-3): an event whose
    /// accounts come from TWO sources must report both, never one --
    /// collapsing this to a single value is exactly how a hand-repaired
    /// row ends up wearing an imported source's clothes. Synthetic on
    /// purpose: the REAL `attests` table is single-sourced today.
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
        // An event with no accounts renders no affordance, not a blank one.
        assert!(svc.attests_provenance("e3").is_empty());
        // The family view still sees the set (this one stays on ProvenanceIndex).
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
        // The family view still sees BOTH -- the per-event view is a
        // filter, never a redefinition of the family.
        assert_eq!(svc.provenance.by_family(crate::provenance::family::MENTIONS), vec!["event-mentions".to_string(), "theographic-people".to_string()]);
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

    /// OVERLAY-1 Task 2 ("one KJV in memory"): `verse_text_of` reads one
    /// verse's own text on demand, straight off the published graph -- no
    /// `verse_text` companion map is built at all any more. Proves it
    /// against the SAME primitive the now-deleted whole-spine builder
    /// (`legacy::verses_from_graph`) used per entry: `window::render` over
    /// the `kjv_adapter::verse_node_id` for that verse.
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
