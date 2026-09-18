//! DB-2b's gate (spec §6.2) over the COMMITTED graph: write the four
//! sections, open them through `SqliteSnapshot`, and prove (a) every port
//! answer over the full position inventory equals the in-memory graph's,
//! (b) each section's logical hash recomputed from its tables equals the
//! one computed from the partition and stamped in the manifest, (c) two
//! writes are identical in every logical hash and in the root.
//!
//! Wall-clock gate: `#[ignore]`d and run serialized by
//! `scripts/timing-gates.sh` (gate 9), the CONTENTION-1 mechanism --
//! never in the parallel workspace run.

use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;

use atlas_graph::sqlite::logical::{logical_dump_of_db, logical_hash};
use atlas_graph::sqlite::manifest::read_manifest;
use atlas_graph::sqlite::open_read_only;
use atlas_graph::sqlite::snapshot::SqliteSnapshot;
use atlas_graph::sqlite::extras::Extras;
use atlas_graph::sqlite::source::{CommittedZstdSource, SectionLayout};
use atlas_graph::sqlite::writer::write_sections;

fn layout_under(dir: &Path) -> SectionLayout {
    SectionLayout { compiled_dir: dir.join("compiled"), cache_dir: dir.join("cache").join("sections") }
}
fn open_written(dir: &Path) -> Result<SqliteSnapshot, atlas_graph::sqlite::SqliteError> {
    let layout = layout_under(dir);
    SqliteSnapshot::open(&layout.manifest_path(), &CommittedZstdSource { layout })
}
use atlas_graph_types::graph::Graph;
use atlas_graph_types::store::{assert_answers_match, GraphSnapshot};

fn data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled")
}

/// Loaded ONCE for the binary. DB-5: the committed SECTIONS read back
/// (`sqlite::reload`) -- extras already attached -- plus the typed extras
/// the writer needs, recomputed from the ETL's in-memory `AtlasData`
/// (raw + curated) and `sources.json`, exactly as the compile folds them.
fn committed_graph() -> &'static (Graph, Extras) {
    static CACHED: OnceLock<(Graph, Extras)> = OnceLock::new();
    CACHED.get_or_init(|| {
        let (graph, snap) = atlas_graph::sqlite::reload::committed_graph(&data_dir()).expect("the committed sections read back");
        let (chrono, red_letter) = snap.with_conn(|c| Ok((atlas_graph::sqlite::serve::load_chronology(c)?, atlas_graph::sqlite::serve::load_red_letter_spans(c)?))).expect("companions");
        let data = data_dir().parent().unwrap().to_path_buf();
        let atlas = atlas_etl::compile::compile(&data.join("raw"), &data.join("curated")).expect("the ETL compiles").data;
        let sources: atlas_core::sources::SourcesDocument = serde_json::from_str(&std::fs::read_to_string(data_dir().join("sources.json")).unwrap()).unwrap();
        let extras = atlas_graph::sqlite::extras::compute(&graph, &chrono, &red_letter, &atlas, &sources).expect("the fold");
        (graph, extras)
    })
}

/// Ceiling: the FIRST measured run x 2, rounded up to the next 30 s, written
/// beside the measurement in `scripts/timing-gates.sh`'s DB-2b note and in
/// `server/BENCHMARKS.md`. Never loosened afterward.
/// Measured 2026-09-17 (debug build, serialized): write 46.6 s + dump
/// re-derivation 18.1 s + assert_answers_match 159.7 s + second write =
/// 271.5 s total -> ceiling 570 s.
/// RE-DERIVED at DB-4b (2026-09-17), the same rule over a wider gate: the
/// write now includes zstd-19 of ~356 MB (four threads) and the extra
/// tables, the open goes through `CommittedZstdSource`. Measured
/// standalone: write 104.4 s + dump re-derivation 4.2 s +
/// assert_answers_match 255.5 s + second write = 471.3 s -> ceiling 960 s
/// (x2, rounded up to 30 s). Not loosened afterward.
const CEILING_SECS: u64 = 960;

#[test]
#[ignore = "wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)"]
fn the_full_real_graph_is_admitted_over_the_sqlite_backend_and_the_logical_hashes_agree() {
    let (g, extras) = committed_graph();
    let dir = std::env::temp_dir().join("db2b-real-sections");
    let _ = std::fs::remove_dir_all(&dir);
    let t0 = Instant::now();
    let (m1, written) = write_sections(g, extras, "test", &layout_under(&dir)).expect("write");
    let write_secs = t0.elapsed().as_secs_f64();
    for w in &written {
        println!(
            "DB-4b SECTION {:?}: {} nodes, {} rows, {} extra rows, {} edges, {} -> {} bytes, logical {}, in {:?}",
            w.section, w.node_count, w.row_count, w.extra_row_count, w.edge_count, w.uncompressed_bytes, w.bytes, w.logical, w.elapsed
        );
    }
    let t1 = Instant::now();
    for w in &written {
        let conn = open_read_only(&w.path).unwrap();
        assert_eq!(
            logical_hash(&logical_dump_of_db(&conn, w.section).unwrap()),
            w.logical,
            "{:?}: the file's tables do not re-derive the partition's hash",
            w.section
        );
    }
    let dump_secs = t1.elapsed().as_secs_f64();
    let snap = open_written(&dir).expect("open");
    assert_eq!(snap.version().0, atlas_graph_types::sections::version_root(g), "one root: the snapshot, the manifest and the in-memory graph agree");
    let t2 = Instant::now();
    assert_answers_match(&snap, g);
    let admit_secs = t2.elapsed().as_secs_f64();
    let dir2 = std::env::temp_dir().join("db2b-real-sections-2");
    let _ = std::fs::remove_dir_all(&dir2);
    let (m2, _) = write_sections(g, extras, "test", &layout_under(&dir2)).expect("write 2");
    assert_eq!(m1.root, m2.root, "determinism: two writes, one root");
    assert_eq!(
        m1.sections.iter().map(|s| &s.logical).collect::<Vec<_>>(),
        m2.sections.iter().map(|s| &s.logical).collect::<Vec<_>>(),
        "determinism: every logical hash"
    );
    assert_eq!(read_manifest(&layout_under(&dir).manifest_path()).unwrap().root, m1.root);
    // DB-4b: the COMMITTED manifest is what this graph + these sidecars produce
    let committed = read_manifest(&data_dir().join("manifest.toml")).expect("data/compiled/manifest.toml is committed");
    assert_eq!(committed.root, m1.root, "data/compiled/manifest.toml's root is this graph's (recompile if the sidecars or the graph moved)");
    assert_eq!(
        committed.sections.iter().map(|s| (&s.name, &s.logical)).collect::<Vec<_>>(),
        m1.sections.iter().map(|s| (&s.name, &s.logical)).collect::<Vec<_>>()
    );
    for w in &written {
        assert!(w.bytes <= atlas_graph::sqlite::blob::BLOB_CEILING, "{:?}: {} bytes over the ceiling", w.section, w.bytes);
    }
    let total = t0.elapsed().as_secs_f64();
    println!(
        "DB-4b GATE: write {write_secs:.1}s (incl. zstd-19), dump-recompute {dump_secs:.1}s, assert_answers_match {admit_secs:.1}s, total {total:.1}s (ceiling {CEILING_SECS}s)"
    );
    assert!(total <= CEILING_SECS as f64, "DB-4b gate {total:.1}s exceeds ceiling {CEILING_SECS}s");
}
