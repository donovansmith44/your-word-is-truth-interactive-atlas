#![allow(dead_code)]
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use atlas_core::data::{AtlasData, Canon, Era};
use atlas_core::sources::SourcesDocument;
use atlas_etl::brainfuel::BrainFuelCorpus;
use atlas_etl::concord::{ConcordCorpus, ScOverlapRow};
use atlas_etl::kretzmann::KretzmannCorpus;
use atlas_etl::lexicon::LexiconCorpus;
use atlas_etl::red_letter::RedLetterCorpus;
use atlas_graph::concord_adapter::ConcordBundle;
use atlas_graph::pipeline::{self, BuildCtx};
use atlas_graph::sqlite::snapshot::SqliteSnapshot;
use atlas_graph::sqlite::source::{CommittedZstdSource, SectionLayout};
use atlas_graph::{BuildStats, ChronologyDerivation, EventWorldStats, GraphService};
use atlas_graph_types::graph::Graph;

pub const MAPS: usize = 10;
pub const CORPUS_ROOTS: usize = 2;
/// The Book of Concord's citations of Scripture that lie on its words and name verses the Bible holds.
pub const CONCORD_CITATIONS: usize = 1_246;

/// A from-raw build always reads the KJV, its cross references, the atlas, brain-fuel and the
/// Concord; these are the corpora a suite adds to that.
#[derive(Clone, Copy)]
pub struct OptionalCorpora {
    pub kretzmann: bool,
    pub red_letter: bool,
}

/// The real graph built from `data/raw` over the atlas's own eras, with its indexes built.
pub fn indexed_raw_graph(optional: OptionalCorpora) -> Graph {
    let (mut graph, ..) = RawSources::read(optional).build_graph(&real_atlas().eras);
    graph.build_indexes();
    graph
}

pub struct RawSources {
    kjv_json: String,
    xrefs_tsv: String,
    brainfuel: BrainFuelCorpus,
    concord: ConcordBundle,
    kretzmann: Option<KretzmannCorpus>,
    red_letter: Option<RedLetterCorpus>,
}

impl RawSources {
    pub fn read(optional: OptionalCorpora) -> RawSources {
        let kjv_json = kjv_json();
        let (_, kjv_verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");
        let brainfuel = brainfuel_corpus();
        let kretzmann = optional.kretzmann.then(|| kretzmann_corpus(&kjv_verses));
        let red_letter = optional.red_letter.then(|| red_letter_corpus(&atlas_etl::brainfuel::restore_kjv_case(&brainfuel, &kjv_verses).0));
        RawSources { kjv_json, xrefs_tsv: cross_references_tsv(), brainfuel, concord: concord_bundle(), kretzmann, red_letter }
    }

    pub fn build_graph(&self, eras: &[Era]) -> (Graph, BuildStats, EventWorldStats, ChronologyDerivation) {
        atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(
            &self.kjv_json,
            &self.xrefs_tsv,
            real_atlas(),
            eras,
            Some(&self.brainfuel),
            Some(&self.concord),
            self.kretzmann.as_ref(),
            self.red_letter.as_ref(),
        )
        .expect("the real committed sources must build")
    }

    pub fn build_service(&self, eras: &[Era]) -> GraphService {
        GraphService::from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(
            &self.kjv_json,
            &self.xrefs_tsv,
            real_atlas(),
            eras,
            Some(&self.brainfuel),
            Some(&self.concord),
            self.kretzmann.as_ref(),
            self.red_letter.as_ref(),
        )
        .expect("the real committed sources must build")
    }
}

/// The raw inputs of the full pipeline over the real atlas; `run` borrows them for the context it builds.
pub struct PipelineInputs {
    pub kjv_json: String,
    pub xrefs_tsv: String,
    pub canon: Canon,
    pub verses: HashMap<String, String>,
}

impl PipelineInputs {
    pub fn read() -> PipelineInputs {
        let kjv_json = kjv_json();
        let (canon, verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");
        PipelineInputs { kjv_json, xrefs_tsv: cross_references_tsv(), canon, verses }
    }

    pub fn run(&self) -> BuildCtx<'_> {
        let mut ctx = BuildCtx::new(&self.canon, &self.verses, Some(&self.kjv_json), &self.xrefs_tsv, real_atlas());
        pipeline::run_pipeline(&mut ctx, &pipeline::pipeline()).expect("the real committed sources must build cleanly through the full pipeline");
        ctx
    }
}

/// The real graph from the KJV, its cross references and the atlas, with no other corpus.
pub fn kjv_and_atlas_build(eras: &[Era]) -> (Graph, BuildStats, EventWorldStats, ChronologyDerivation) {
    atlas_graph::build::build_graph_from_sources_with_eras(&kjv_json(), &cross_references_tsv(), real_atlas(), eras).expect("the real committed sources must build")
}

/// Compiled once per test binary; the compile is the slowest step most suites share.
pub fn real_atlas() -> &'static AtlasData {
    static CACHED: OnceLock<AtlasData> = OnceLock::new();
    CACHED.get_or_init(compile_real_atlas)
}

/// A fresh compile, for the suites whose law is that two independent builds agree.
pub fn compile_real_atlas() -> AtlasData {
    atlas_etl::compile::compile(&raw_dir(), &curated_dir())
        .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify")
        .data
}

pub fn kjv_json() -> String {
    std::fs::read_to_string(raw_dir().join("kjv.json")).expect("data/raw/kjv.json must exist")
}

pub fn cross_references_tsv() -> String {
    std::fs::read_to_string(raw_dir().join("xrefs").join("cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist")
}

pub fn brainfuel_corpus() -> BrainFuelCorpus {
    atlas_etl::brainfuel::read_all(&brainfuel_dir()).expect("data/raw/brain-fuel-bible must exist -- run the CORP-1a vendoring step first")
}

pub fn lexicon_corpus() -> LexiconCorpus {
    atlas_etl::lexicon::read_all(&brainfuel_dir()).expect("the vendored lexicon + morphology")
}

fn brainfuel_dir() -> PathBuf {
    raw_dir().join("brain-fuel-bible")
}

pub fn concord_bundle() -> ConcordBundle {
    ConcordBundle { corpus: concord_corpus(), sc_overlap: sc_overlap() }
}

pub fn concord_corpus() -> ConcordCorpus {
    atlas_etl::concord::read_all(&raw_dir().join("concord"), &curated_dir()).expect("data/raw/concord + data/curated/concord-titles.toml must read -- run data/fetch-raw.ps1 first")
}

pub fn sc_overlap() -> Vec<ScOverlapRow> {
    let text = std::fs::read_to_string(curated_dir().join("concord-sc-overlap.toml")).expect("data/curated/concord-sc-overlap.toml must exist");
    atlas_etl::concord::parse_sc_overlap(&text).expect("concord-sc-overlap.toml must parse")
}

pub fn kretzmann_corpus(kjv_verses: &HashMap<String, String>) -> KretzmannCorpus {
    atlas_etl::kretzmann::read_all(&raw_dir().join("kretzmann"), kjv_verses).expect("data/raw/kretzmann must exist -- run data/fetch-raw.ps1 first")
}

/// `restored_verses` is the KJV text after brain-fuel's case restoration, which the red-letter spans align against.
pub fn red_letter_corpus(restored_verses: &HashMap<String, String>) -> RedLetterCorpus {
    atlas_etl::red_letter::read_all(&raw_dir().join("red-letter"), restored_verses).expect("data/raw/red-letter must exist -- run data/fetch-raw.ps1 first")
}

pub fn sources_registry() -> SourcesDocument {
    let path = compiled_dir().join("sources.json");
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()));
    serde_json::from_str(&json).expect("data/compiled/sources.json must parse as a SourcesDocument")
}

pub fn committed_graph() -> &'static Graph {
    static CACHED: OnceLock<Graph> = OnceLock::new();
    CACHED.get_or_init(|| atlas_graph::sqlite::reload::committed_graph(&compiled_dir()).expect("the committed sections read back (run atlas-graph-compile first)").0)
}

pub fn committed_service() -> &'static GraphService {
    static CACHED: OnceLock<GraphService> = OnceLock::new();
    CACHED.get_or_init(|| GraphService::from_sections(&compiled_dir()).expect("data/compiled/manifest.toml + sections/ must exist and open -- run atlas-graph-compile first").0)
}

pub fn committed_sections() -> &'static SqliteSnapshot {
    static CACHED: OnceLock<SqliteSnapshot> = OnceLock::new();
    CACHED.get_or_init(|| {
        let layout = SectionLayout::under(&compiled_dir());
        SqliteSnapshot::open(&layout.manifest_path(), &CommittedZstdSource { layout }).expect("the committed sections open")
    })
}

pub fn raw_dir() -> PathBuf {
    data_dir().join("raw")
}

pub fn curated_dir() -> PathBuf {
    data_dir().join("curated")
}

pub fn compiled_dir() -> PathBuf {
    data_dir().join("compiled")
}

pub fn data_dir() -> PathBuf {
    repo_dir().join("data")
}

pub fn contract_vectors_dir() -> PathBuf {
    repo_dir().join("contracts").join("atlas-query-contract").join("vectors")
}

pub fn graph_types_src_dir() -> PathBuf {
    repo_dir().join("graph-types").join("src")
}

fn repo_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
