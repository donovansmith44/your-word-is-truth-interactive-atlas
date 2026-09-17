//! DB-2a: every node and every row in the graph the compile binary ships
//! round-trips through the canonical encoding, and the encoding is
//! deterministic across builds.
//!
//! This is the COVERAGE proof for `graph-types`'s `canon` module: the
//! golden vectors in `graph-types/tests/canon_vectors.rs` and
//! `canon_row_vectors.rs` pin the BYTES of
//! hand-built specimens, and this file proves those same encoders total
//! over the real committed corpus -- ~92k nodes and every row of all 21
//! families, every `Option`, every enum variant the data actually
//! inhabits, every string the KJV/Concord/Kretzmann text carries.
//!
//! Real-data cost, disclosed: the first three tests load the committed
//! `data/compiled/graph.bin` once (shared through a `OnceLock`, the same
//! decode + `build_indexes` + `add_justified_by` sequence
//! `GraphService::from_artifact` performs); the determinism test builds
//! the graph TWICE from the real raw+curated sources, the way
//! `determinism.rs` does. Minutes, not seconds -- expected, and the
//! reason the per-family counts are printed rather than merely asserted.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

use atlas_graph_types::canon::ids::{parse_position, position_str};
use atlas_graph_types::canon::{encode_row_in_family, Canon, RowFamily};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::Position;
use atlas_graph_types::node::Node;

// --------------------------------------------------------------- the graph

/// The REAL graph the compile binary ships, read off the committed
/// artifact and indexed exactly as `GraphService::from_artifact` indexes
/// it (`to_service_parts` -> `build_indexes` -> `add_justified_by`).
/// Loaded ONCE for the whole binary: the three tests below all walk the
/// same ~92k nodes, and paying the 100MB decode three times would buy
/// nothing.
fn committed_graph() -> &'static Graph {
    static CACHED: OnceLock<Graph> = OnceLock::new();
    CACHED.get_or_init(|| {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled/graph.bin");
        let dump = atlas_graph::artifact::read_file(&path).expect(
            "data/compiled/graph.bin must exist -- run `cargo run -p atlas-graph --bin atlas-graph-compile` from server/ first",
        );
        let (mut graph, _stats, _ews, _chronology) =
            atlas_graph::artifact::to_service_parts(dump).expect("to_service_parts must succeed");
        graph.build_indexes();
        atlas_graph::event_world::add_justified_by(&mut graph);
        graph
    })
}

// ------------------------------------------------------------------ nodes

#[test]
fn every_node_round_trips_and_re_encodes_identically() {
    let graph = committed_graph();
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut count = 0usize;

    for (id, n) in &graph.nodes {
        let bytes = n.encode();
        let back = Node::decode(&bytes)
            .unwrap_or_else(|e| panic!("node {id:?} failed to decode its own bytes: {e}"));
        // The round-trip law (R6): byte fixed point AND decoded equality.
        assert_eq!(back.encode(), bytes, "node {id:?} is not a byte fixed point");
        assert_eq!(&back.id, id, "node {id:?} lost its id");
        assert_eq!(back.provenance, n.provenance, "node {id:?} lost its provenance");
        assert_eq!(
            format!("{:?}", back.payload),
            format!("{:?}", n.payload),
            "node {id:?} lost payload content"
        );
        *by_kind.entry(format!("{:?}", id.kind)).or_default() += 1;
        count += 1;
    }

    println!("DB-2a NODE CANON: {count} nodes round-tripped");
    for (kind, n) in &by_kind {
        println!("  node {kind}: {n}");
    }
    assert!(count > 90_000, "the committed graph must carry ~92k nodes, found {count}");
}

// ------------------------------------------------------------------- rows

/// One family's proof: every row encodes, decodes, and re-encodes to the
/// SAME bytes, both bare and wrapped in its family (the form edge ids and
/// section logical hashes hash, spec 3.4).
fn round_trip_family<T: Canon>(rows: &[T], family: RowFamily) -> usize {
    for (i, row) in rows.iter().enumerate() {
        let value = row.to_value();
        let bytes = row.encode();
        let back = T::decode(&bytes).unwrap_or_else(|e| {
            panic!("{} row {i} failed to decode its own bytes: {e}", family.name())
        });
        assert_eq!(back.to_value(), value, "{} row {i} lost content", family.name());
        assert_eq!(back.encode(), bytes, "{} row {i} is not a byte fixed point", family.name());
        assert_eq!(
            encode_row_in_family(family, back.to_value()),
            encode_row_in_family(family, value),
            "{} row {i} is not a byte fixed point inside its family",
            family.name()
        );
    }
    rows.len()
}

#[test]
fn every_row_of_every_family_round_trips() {
    let g = committed_graph();
    let mut counts: Vec<(RowFamily, usize)> = Vec::new();

    macro_rules! fam {
        ($field:ident, $family:expr) => {
            counts.push(($family, round_trip_family(&g.$field, $family)))
        };
    }

    fam!(contains_bible, RowFamily::ContainsBible);
    fam!(contains_concord, RowFamily::ContainsConcord);
    fam!(attests, RowFamily::Attests);
    fam!(succession, RowFamily::Succession);
    fam!(canon_succession, RowFamily::CanonSuccession);
    fam!(dated_by, RowFamily::DatedBy);
    fam!(located_at, RowFamily::LocatedAt);
    fam!(fulfills, RowFamily::Fulfills);
    fam!(typology, RowFamily::Typology);
    fam!(named_after, RowFamily::NamedAfter);
    fam!(catechism, RowFamily::Catechism);
    fam!(comments_on, RowFamily::CommentsOn);
    fam!(spoken_by, RowFamily::SpokenBy);
    fam!(spoken_at, RowFamily::SpokenAt);
    fam!(mentions, RowFamily::Mentions);
    fam!(cross_refs, RowFamily::CrossRefs);
    fam!(quotes, RowFamily::Quotes);
    fam!(confesses, RowFamily::Confesses);
    fam!(corresponds_bible, RowFamily::CorrespondsBible);
    fam!(temporal_adjacency, RowFamily::TemporalAdjacency);
    fam!(analogue, RowFamily::Analogue);

    let total: usize = counts.iter().map(|(_, n)| *n).sum();
    println!("DB-2a ROW CANON: {total} rows round-tripped across {} families", counts.len());
    for (family, n) in &counts {
        println!("  [{}] {}: {n}", family.ordinal(), family.name());
    }

    // Every family in the closed enum is covered, in ordinal order --
    // a new family cannot be added without joining this proof.
    assert_eq!(counts.len(), RowFamily::ALL.len(), "every row family must be walked");
    let walked: Vec<RowFamily> = counts.iter().map(|(f, _)| *f).collect();
    assert_eq!(walked, RowFamily::ALL.to_vec(), "families must be walked in ordinal order");

    // The three uninhabited families (artifact.rs refuses to dump a
    // non-empty one) are still real encoders: an empty table round-trips
    // vacuously, and the golden vectors pin their bytes from hand-built
    // rows.
    let empty = [RowFamily::Quotes, RowFamily::Confesses, RowFamily::CorrespondsBible];
    for family in empty {
        let (_, n) = counts.iter().find(|(f, _)| *f == family).expect("family walked");
        assert_eq!(*n, 0, "{} is uninhabited in the shipped graph", family.name());
    }
    assert!(total > 300_000, "the committed graph must carry ~400k rows, found {total}");
}

// --------------------------------------------------------------- positions

#[test]
fn canonical_id_strings_parse_back_for_every_node_and_every_index_position() {
    let g = committed_graph();
    let mut count = 0usize;

    fn check(p: &Position, count: &mut usize) {
        let s = position_str(p);
        let back = parse_position(&s, "$.position")
            .unwrap_or_else(|e| panic!("position `{s}` failed to parse back: {e}"));
        assert_eq!(&back, p, "position `{s}` did not round-trip");
        *count += 1;
    }

    for id in g.nodes.keys() {
        check(&Position::Node(id.clone()), &mut count);
    }
    // Every index key is a Position -- node ends AND edge ends (the
    // edges-as-positions law: JustifiedBy subjects are edge positions).
    // A directed relation's objects are the inverse map's own keys, so
    // walking both maps' keys walks both ends of every entry.
    for ix in g.indexes.values() {
        for p in ix.fwd.keys().chain(ix.inv.keys()) {
            check(p, &mut count);
        }
    }
    for ix in g.symmetric_indexes.values() {
        for p in ix.fwd.keys().chain(ix.inv.keys()) {
            check(p, &mut count);
        }
    }

    println!("DB-2a POSITION CANON: {count} position strings round-tripped");
    assert!(count > 90_000, "every node is a position at minimum, found {count}");
}

// ------------------------------------------------------------ determinism

/// The real sources, read fresh. Mirrors `determinism.rs`'s own
/// `real_sources` (two INDEPENDENT reads + ETL compiles prove the whole
/// pipeline, not just the graph-build half) and, like
/// `artifact_conformance.rs`'s KRETZ-m3 note, threads Concord, Kretzmann
/// AND red-letter in as well -- the full sequence `atlas-graph-compile`
/// itself performs. Anything less and this proof would never see
/// `contains_concord`, `catechism`, `comments_on`, `spoken_by` or
/// `spoken_at` rows, so five of the eighteen inhabited families would sit
/// outside the determinism law.
struct RealSources {
    kjv_json: String,
    xrefs_tsv: String,
    atlas: atlas_core::data::AtlasData,
    eras: Vec<atlas_core::data::Era>,
    brainfuel: atlas_etl::brainfuel::BrainFuelCorpus,
    concord: atlas_graph::concord_adapter::ConcordBundle,
    kretzmann: atlas_etl::kretzmann::KretzmannCorpus,
    red_letter: atlas_etl::red_letter::RedLetterCorpus,
}

fn real_sources() -> RealSources {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let raw_dir = data_dir.join("raw");
    let curated_dir = data_dir.join("curated");

    let kjv_json =
        std::fs::read_to_string(raw_dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt"))
        .expect("data/raw/xrefs/cross_references.txt must exist");
    let atlas = atlas_etl::compile::compile(&raw_dir, &curated_dir)
        .expect("data/raw + data/curated must compile")
        .data;
    let eras = atlas.eras.clone();
    let brainfuel = atlas_etl::brainfuel::read_all(&raw_dir.join("brain-fuel-bible"))
        .expect("data/raw/brain-fuel-bible must exist");
    let concord_corpus = atlas_etl::concord::read_all(&raw_dir.join("concord"))
        .expect("data/raw/concord must exist");
    let sc_overlap_text = std::fs::read_to_string(curated_dir.join("concord-sc-overlap.toml"))
        .expect("data/curated/concord-sc-overlap.toml must exist");
    let sc_overlap = atlas_etl::concord::parse_sc_overlap(&sc_overlap_text)
        .expect("concord-sc-overlap.toml must parse");
    let (_, kjv_verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");
    let kretzmann = atlas_etl::kretzmann::read_all(&raw_dir.join("kretzmann"), &kjv_verses)
        .expect("data/raw/kretzmann must exist");
    // The red-letter span alignment runs against the graph's own
    // KJV-CASE-restored text, never the raw parse -- the same
    // recomputation `compile_graph.rs` performs for the same reason.
    let (restored_verses, _case_report) =
        atlas_etl::brainfuel::restore_kjv_case(&brainfuel, &atlas.verses);
    let red_letter = atlas_etl::red_letter::read_all(&raw_dir.join("red-letter"), &restored_verses)
        .expect("data/raw/red-letter must exist");

    RealSources {
        kjv_json,
        xrefs_tsv,
        atlas,
        eras,
        brainfuel,
        concord: atlas_graph::concord_adapter::ConcordBundle {
            corpus: concord_corpus,
            sc_overlap,
        },
        kretzmann,
        red_letter,
    }
}

fn build(s: &RealSources) -> Graph {
    atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(
        &s.kjv_json,
        &s.xrefs_tsv,
        &s.atlas,
        &s.eras,
        Some(&s.brainfuel),
        Some(&s.concord),
        Some(&s.kretzmann),
        Some(&s.red_letter),
    )
    .expect("the real committed sources must build")
    .0
}

fn compare_rows<T: Canon>(a: &[T], b: &[T], family: RowFamily) -> usize {
    assert_eq!(a.len(), b.len(), "{} row count differs between builds", family.name());
    for (i, (ra, rb)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(
            encode_row_in_family(family, ra.to_value()),
            encode_row_in_family(family, rb.to_value()),
            "{} row {i} encoded differently in two independent builds",
            family.name()
        );
    }
    a.len()
}

#[test]
fn encoding_is_deterministic_across_two_independent_builds() {
    // Streamed pairwise, not concatenated into two multi-hundred-MB
    // buffers: comparing item by item in BTreeMap/Vec order is the SAME
    // byte equality the concatenation would assert, and it also pins the
    // key order itself.
    let a = build(&real_sources());
    let b = build(&real_sources());

    assert_eq!(a.nodes.len(), b.nodes.len(), "node count differs between builds");
    let mut node_bytes = 0usize;
    for ((ida, na), (idb, nb)) in a.nodes.iter().zip(b.nodes.iter()) {
        assert_eq!(ida, idb, "node key order differs between builds");
        let ba = na.encode();
        let bb = nb.encode();
        assert_eq!(ba, bb, "node {ida:?} encoded differently in two independent builds");
        node_bytes += ba.len();
    }

    let mut rows = 0usize;
    macro_rules! fam {
        ($field:ident, $family:expr) => {
            rows += compare_rows(&a.$field, &b.$field, $family)
        };
    }
    fam!(contains_bible, RowFamily::ContainsBible);
    fam!(contains_concord, RowFamily::ContainsConcord);
    fam!(attests, RowFamily::Attests);
    fam!(succession, RowFamily::Succession);
    fam!(canon_succession, RowFamily::CanonSuccession);
    fam!(dated_by, RowFamily::DatedBy);
    fam!(located_at, RowFamily::LocatedAt);
    fam!(fulfills, RowFamily::Fulfills);
    fam!(typology, RowFamily::Typology);
    fam!(named_after, RowFamily::NamedAfter);
    fam!(catechism, RowFamily::Catechism);
    fam!(comments_on, RowFamily::CommentsOn);
    fam!(spoken_by, RowFamily::SpokenBy);
    fam!(spoken_at, RowFamily::SpokenAt);
    fam!(mentions, RowFamily::Mentions);
    fam!(cross_refs, RowFamily::CrossRefs);
    fam!(quotes, RowFamily::Quotes);
    fam!(confesses, RowFamily::Confesses);
    fam!(corresponds_bible, RowFamily::CorrespondsBible);
    fam!(temporal_adjacency, RowFamily::TemporalAdjacency);
    fam!(analogue, RowFamily::Analogue);

    println!(
        "DB-2a DETERMINISM: {} nodes ({node_bytes} canon bytes) + {rows} rows byte-identical across two independent builds",
        a.nodes.len()
    );
}
