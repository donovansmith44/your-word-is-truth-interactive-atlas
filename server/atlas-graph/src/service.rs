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
    /// Reverse index: TextUnit node id -> its position in the bible
    /// reading spine. `GraphQuery` (the port) has no such lookup (only
    /// forward `reading_window`); building this once, alongside the
    /// graph, is what lets `chapter_span`/`position_of` below resolve
    /// "start from this ref" without scanning the whole spine per call.
    bible_position: HashMap<AnyNodeId, usize>,
    /// CORP-2a (decision 8): the Concord-corpus sibling of
    /// `bible_position` above -- SAME shape, over the "concord" reading
    /// spine (empty when no Concord data was built, e.g. every fixture
    /// that doesn't supply a `ConcordBundle` -- an honestly empty
    /// lookup, never a placeholder).
    concord_position: HashMap<AnyNodeId, usize>,
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
    /// M-C2: the same "companion enumeration the generic port doesn't
    /// model" class as `era_ids`/`polity_ids` above -- `handlers::
    /// narratives`' own full listing and `legacy::atlas_data_overlay`'s
    /// own reconstruction both need "every node of kind K," never a
    /// per-position query. `narrative_ids` is `graph.nodes`'s own
    /// alphabetical-by-id order (unmodified -- confirmed to already match
    /// `data/curated/narratives/`'s own sorted-by-filename compiled order,
    /// since a narrative's filename stem IS its id).
    pub narrative_ids: Vec<AnyNodeId>,
    pub event_ids: Vec<AnyNodeId>,
    pub place_ids: Vec<AnyNodeId>,
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
    pub cross_refs_by_from: HashMap<String, Vec<atlas_core::data::CrossRef>>,
    /// M-C2 FIX (definitive surface list, requirement 1 -- a gap this
    /// batch's own first pass at `handlers::verse`/`handlers::xrefs` left
    /// behind): dot-ref -> that verse's own KJV text, the SAME content
    /// `legacy::verses_from_graph` builds for `atlas_data_overlay`'s
    /// `AtlasData.verses` -- computed ONCE here instead, off this
    /// service's own just-published snapshot, so the two named handlers'
    /// cross-ref PREVIEW text (the one remaining `data.verses.get(...)`
    /// read each had) can go through a real graph query instead of
    /// `AtlasData`, exactly like the cross-ref ROWS themselves already do
    /// via `cross_refs_by_from` above. `atlas_data_overlay` now reuses
    /// this field (`gs.verse_text.clone()`) rather than recomputing the
    /// identical map a second time -- one source, not two. `HashMap`, not
    /// `BTreeMap`, for the same reason as `cross_refs_by_from`: only ever
    /// `.get()`'d by key, never iterated or serialized, so iteration order
    /// carries no determinism concern.
    pub verse_text: HashMap<String, String>,
    /// M-D3 (owner ruling U5, "in-text person and place name links,
    /// mentions-attested ONLY"): FROM-verse dot-ref -> every PERSON the
    /// graph's own `mentions` relation attests at that locus, `(id,
    /// display label)` pairs in row-insertion order (the SAME "no
    /// re-sort, wire order is canon order" discipline `EdgeSectionRegistry.
    /// Mentions`'s own client-side doc comment already establishes for
    /// this exact relation). Precomputed once here, the SAME "O(1)
    /// per-verse lookup for a whole-chapter fetch" treatment
    /// `heading_index`/`cross_refs_by_from` above already get, rather than
    /// a per-verse graph query inside the `chapter` handler's own hot
    /// loop. Read straight off `Graph.mentions` (the SAME raw table
    /// `cross_refs_by_from` above reads `Graph.cross_refs` from, BEFORE
    /// publish, since this is built in `assemble` alongside those
    /// companions) filtered to `MentionedEntity::Person` -- `MentionedEntity::
    /// Place` rows are deliberately NOT folded in here: `VerseOut.places`
    /// already has its own, separate, EARLIER-established source
    /// (`AtlasData::places_for_verse`, alias-resolved via
    /// `resolve_display_name` in the handler) that this field does not
    /// replace or duplicate -- EXTEND-ONLY discipline, a place mention's
    /// own existing path is untouched. `HashMap`, not `BTreeMap`, for the
    /// same reason as `cross_refs_by_from`/`verse_text`: only ever
    /// `.get()`'d by key inside the `chapter` handler, never iterated or
    /// serialized as a whole.
    pub persons_by_verse: HashMap<String, Vec<(String, String)>>,
    /// TRAV-1 (controller decision 2, "the graph serves it... one path"):
    /// prior/following-in-time -- DIRECTION read directly off the honest
    /// `earlier`/`later` ends of the graph's own `temporal_adjacency` rows,
    /// never re-derived from a position index; DOMAIN (which ids are dated
    /// at all) seeded from `chronology.chrono.order` so a dated id with no
    /// real neighbor on either side (the atlas's own true first/last event,
    /// or the degenerate single-dated-event case) still gets a `Some((None,
    /// None))` entry rather than going missing outright (`assemble`'s own
    /// comment has the fuller "two different questions" argument). The SAME
    /// "companion the generic port doesn't model" class as
    /// `narrative_legs`/`cross_refs_by_from` above, built once here from
    /// the raw, pre-store `graph` the same way they are. RETIRES
    /// `event_world::Chronology`'s own former `temporal_neighbors` field
    /// (index-arithmetic-derived, over `chrono.order`, a second
    /// representation of this identical fact -- see that struct's own
    /// retirement doc comment): this field is the ONE surviving path.
    /// `HashMap`, not `BTreeMap` -- same "only ever `.get()`'d by key,
    /// never iterated/serialized" reasoning as `cross_refs_by_from`/
    /// `verse_text`/`persons_by_verse` above.
    pub temporal_neighbors: HashMap<String, (Option<String>, Option<String>)>,
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
    /// from`/`verse_text`/`persons_by_verse` above.
    pub red_letter_spans: HashMap<String, Vec<(usize, usize)>>,
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
        Ok(Self::assemble(graph, stats, event_world_stats, Chronology::from_derivation(chrono), red_letter_spans))
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
        Ok(Self::assemble(graph, stats, event_world_stats, Chronology::from_derivation(chrono), HashMap::new()))
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
        Ok(Self::assemble(graph, stats, event_world_stats, chronology, red_letter_spans))
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
    fn assemble(graph: Graph, stats: BuildStats, event_world_stats: EventWorldStats, chronology: Chronology, red_letter_spans: HashMap<String, Vec<(usize, usize)>>) -> Self {
        let bible_position = graph
            .reading
            .get(crate::kjv_adapter::BIBLE_CORPUS)
            .map(|spine| spine.order.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect())
            .unwrap_or_default();
        // CORP-2a: the SAME one-time reverse-index build as `bible_position`
        // above, over the "concord" spine.
        let concord_position = graph
            .reading
            .get(crate::concord_adapter::CONCORD_CORPUS)
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
        // M-C2: the same one-time node-table scan, for the three kinds
        // `handlers::narratives`/`legacy::atlas_data_overlay` need to
        // enumerate. `graph.nodes` is a `BTreeMap<AnyNodeId, _>`, so this
        // is already alphabetical-by-id order (confirmed to match
        // `data/curated/narratives/`'s own sorted-by-filename compiled
        // order for `narrative_ids` -- no separate sort needed here,
        // unlike `era_ids` above).
        let narrative_ids: Vec<AnyNodeId> = graph.nodes.keys().filter(|id| id.kind == atlas_graph_types::id::NodeKind::Narrative).cloned().collect();
        let event_ids: Vec<AnyNodeId> = graph.nodes.keys().filter(|id| id.kind == atlas_graph_types::id::NodeKind::Event).cloned().collect();
        let place_ids: Vec<AnyNodeId> = graph.nodes.keys().filter(|id| id.kind == atlas_graph_types::id::NodeKind::Place).cloned().collect();
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
        // M-C2 (requirement 2): the SAME treatment for the cites relation's
        // own span data -- see this struct's own `cross_refs_by_from` doc
        // comment.
        let mut cross_refs_by_from: HashMap<String, Vec<atlas_core::data::CrossRef>> = HashMap::new();
        for row in &graph.cross_refs {
            let Some(key) = crate::legacy::locus_dot_ref(&row.from) else { continue };
            cross_refs_by_from.entry(key).or_default().push(atlas_core::data::CrossRef { target: row.target_display.clone(), votes: row.votes as i32 });
        }
        // M-D3 (owner ruling U5): the SAME treatment for the mentions
        // relation's own PERSON rows -- see this struct's own
        // `persons_by_verse` doc comment. `AnyNodeId::erase` (PersonId ->
        // AnyNodeId) is the standard "look this typed id up in the raw
        // node map" step every other id-carrying row in this crate already
        // uses; a mentions row naming a person id absent from `graph.nodes`
        // is a data-integrity impossibility the law-check stage already
        // guards elsewhere in this pipeline, but this loop still skips it
        // defensively (`?`) rather than panicking on a network handler's
        // own eventual caller.
        //
        // PG-1a WIRE SEAM (batch-pg1a-brief.md decision 6): the `let-else`
        // match against `MentionedEntity::Person` below ALREADY excludes
        // `MentionedEntity::PeopleGroup` rows by construction (it was
        // written for the two-variant `PlaceOrPerson` era and never
        // widened) -- the nine reclassified Gen-10 gentilics (PG-1a) LOSE
        // their in-text `VerseOut.persons` link here, verified and
        // disclosed rather than silently true: this is the U5-rebinding
        // seam for the READING surface specifically (`graph_handlers::
        // node_edges`'s own filter is the seam for the GENERIC entity-list
        // surface) -- a future PeopleGroup-aware client widens this arm
        // (and adds its own `PeopleGroupRefOut`/`VerseOut.people_groups`),
        // not this batch.
        let mut persons_by_verse: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for row in &graph.mentions {
            let atlas_graph_types::edge::MentionedEntity::Person(person_id) = &row.entity else { continue };
            let Some(key) = crate::legacy::locus_dot_ref(&row.locus) else { continue };
            let Some(label) = graph.nodes.get(&person_id.erase()).and_then(|n| match &n.payload {
                atlas_graph_types::node::NodePayload::Person { label, .. } => Some(label.clone()),
                _ => None,
            }) else {
                continue;
            };
            persons_by_verse.entry(key).or_default().push((person_id.0.clone(), label));
        }
        // TRAV-1: the SAME one-time, pre-store `graph` scan as the
        // companions above. Two DIFFERENT questions, deliberately kept
        // separate: WHICH ids are genuinely part of the timeline at all
        // (a plain DOMAIN/membership fact -- seeded from `chronology.chrono.
        // order`, the same `Vec<String>` `populate_temporal_adjacency`
        // itself was built from, so this is not a second derivation of
        // ORDER, just a re-read of the one that already exists) vs. WHICH
        // direction each neighbor sits in (read directly off
        // `graph.temporal_adjacency`'s own honest `earlier`/`later` row
        // ends, never re-derived from a position index -- see
        // `event_world::Chronology`'s own doc comment). Seeding first
        // matters at the atlas's own true first/last dated event AND at
        // the (rare, but real -- a live-caught HOTFIX-4 regression fixture)
        // single-dated-event case: `windows(2)` on that id's own
        // neighborhood yields no ROW at all, but the id is still honestly
        // DATED, so `timeline` must still be `Some` (both directions
        // `None`), never omitted outright the way a truly general-kind/
        // undated/unknown id is -- `handlers::narrative_event_positions`'s
        // own `.get(id)` presence check is what draws that line.
        let mut temporal_neighbors: HashMap<String, (Option<String>, Option<String>)> =
            chronology.chrono.order.iter().map(|id| (id.clone(), (None, None))).collect();
        for row in &graph.temporal_adjacency {
            temporal_neighbors.entry(row.earlier.0.clone()).or_insert((None, None)).1 = Some(row.later.0.clone());
            temporal_neighbors.entry(row.later.0.clone()).or_insert((None, None)).0 = Some(row.earlier.0.clone());
        }
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
        // M-C2 FIX (this struct's own `verse_text` doc comment): needs a
        // snapshot (the `GraphQuery` trait, not raw `Graph` field access),
        // so computed here, after publish/open, unlike the `&graph`-based
        // companions above.
        let verse_text = crate::legacy::verses_from_graph(&snapshot);
        GraphService {
            snapshot,
            bible_position,
            concord_position,
            stats,
            chronology,
            event_world_stats,
            era_ids,
            polity_ids,
            narrative_ids,
            event_ids,
            place_ids,
            narrative_legs,
            heading_index,
            cross_refs_by_from,
            verse_text,
            persons_by_verse,
            temporal_neighbors,
            red_letter_spans,
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

    /// CORP-2a (decision 8): the Concord-corpus sibling of `position_of`
    /// above — resolves a `(part, article, paragraph)` ref into the
    /// "concord" spine's own starting index.
    pub fn concord_position_of(&self, part: u8, article: u16, paragraph: u16) -> Option<usize> {
        self.concord_position.get(&crate::concord_adapter::text_unit_id(part, article, paragraph)).copied()
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
