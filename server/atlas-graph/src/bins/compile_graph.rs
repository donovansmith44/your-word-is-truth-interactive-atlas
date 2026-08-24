//! Batch M-C, controller decision 4: THE COMPILE STEP -- "a compile step
//! (extend atlas-etl's entry point or a new atlas-graph bin -- your call,
//! disclosed) produces it [the serialized artifact]." A NEW BIN here, not
//! an extension of atlas-etl's own binary: atlas-etl's `main.rs` already
//! owns the curated-TOML+Theographic parse/merge/validate pipeline that
//! produces `data/compiled/*.json` (this binary's own OWN INPUT, via
//! `AtlasData::load`); the graph-artifact compile step is a SEPARATE
//! concern layered on top (raw KJV/xrefs + curated eras.toml + the
//! already-compiled `AtlasData` -> one graph artifact file), which reads
//! more naturally as its own small tool than as a mode flag inside an
//! already-large ETL binary.
//!
//! ```text
//! atlas-graph-compile --data-dir data/compiled --out data/compiled/graph.bin
//! ```
//! (`--data-dir` names the SAME `data/compiled` every other tool in this
//! workspace uses as the anchor; `raw/`/`curated/` are its siblings under
//! the shared `data/` parent, exactly the derivation `atlas-server`'s own
//! `main.rs` and `GraphService::build` already use.)
//!
//! M-C2: this binary's own INPUT `AtlasData` now comes from
//! `atlas_etl::compile::compile(raw_dir, curated_dir)` (a real raw+curated
//! compile, the same orchestration `atlas-etl`'s own binary runs) rather
//! than `AtlasData::load(&data_dir)` reading `data/compiled/*.json` --
//! five of those files (places/events/narratives/verses-kjv/cross-refs)
//! are the M-C2 deletion event's own target and no longer exist.
//!
//! ADMISSION (design §9a: "implementation #2 passes the same law as #1"):
//! before writing anything, this binary independently re-builds the SAME
//! graph a second time from the identical sources and runs
//! `atlas_graph_types::store::assert_answers_match` between the dump's own
//! round-tripped reconstruction and that second build -- the artifact
//! this binary produces has PASSED the SAME conformance law
//! `tests/artifact_conformance.rs` proves in CI, not merely "should."
//! This is the expensive check (measured: ~3-15s over the full real
//! graph); it happens HERE, once, at compile time -- never at server
//! startup, which is exactly what keeps the LOAD-TIME ceiling (controller
//! decision 6) achievable.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};

fn parse_args(args: &[String]) -> Result<(PathBuf, PathBuf)> {
    let mut data_dir: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--data-dir" => {
                i += 1;
                data_dir = Some(PathBuf::from(args.get(i).context("--data-dir requires a value")?));
            }
            "--out" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).context("--out requires a value")?));
            }
            other => anyhow::bail!("unrecognized argument: {other}"),
        }
        i += 1;
    }
    let data_dir = data_dir.context("--data-dir is required, e.g. --data-dir ../data/compiled")?;
    let out = out.context("--out is required, e.g. --out ../data/compiled/graph.bin")?;
    Ok((data_dir, out))
}

fn main() -> Result<()> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let (data_dir, out_path) = parse_args(&raw)?;

    let raw_dir = data_dir.parent().map(|p| p.join("raw")).unwrap_or_else(|| Path::new("../data/raw").to_path_buf());
    let curated_dir = data_dir.parent().map(|p| p.join("curated")).unwrap_or_else(|| Path::new("../data/curated").to_path_buf());

    // M-C2: `AtlasData::load(&data_dir)` retired as this binary's own
    // source -- the five files it read (places/events/narratives/
    // verses-kjv/cross-refs.json) are the deletion event's own target
    // (batch-mc2-report.md). `atlas_etl::compile::compile` is the SAME
    // raw+curated orchestration `atlas-etl`'s own binary runs, called here
    // directly instead of round-tripping through `data/compiled/*.json` --
    // it also already carries `data.eras` (compiled once, validated),
    // so the separate `eras.toml` re-parse this binary used to do on its
    // own retires too: one parse, one source, for both.
    let atlas = atlas_etl::compile::compile(&raw_dir, &curated_dir).with_context(|| format!("compiling {} + {}", raw_dir.display(), curated_dir.display()))?.data;
    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).with_context(|| format!("reading {}", raw_dir.join("kjv.json").display()))?;
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt"))
        .with_context(|| format!("reading {}", raw_dir.join("xrefs/cross_references.txt").display()))?;
    let eras = atlas.eras.clone();

    // CORP-1a: the real compile step HARD-REQUIRES the vendored brain-fuel
    // data (unlike GraphService::build's own dev-fallback, which degrades
    // gracefully for a fixture raw_dir) -- a graph.bin compiled without it
    // would silently ship KJV-only text, defeating this batch's own
    // purpose. `data/raw/brain-fuel-bible/` (see data/raw/README.md).
    let brainfuel_root = raw_dir.join("brain-fuel-bible");
    println!("atlas-graph-compile: reading vendored brain-fuel editions from {}...", brainfuel_root.display());
    let brainfuel = atlas_etl::brainfuel::read_all(&brainfuel_root).with_context(|| format!("reading {}", brainfuel_root.display()))?;
    println!(
        "atlas-graph-compile: brain-fuel {} OT + {} NT chapters, {} verse rows, present renderings {:?}, absent markers {:?}, anomalies {}",
        brainfuel.stats.ot_chapters,
        brainfuel.stats.nt_chapters,
        brainfuel.rows.len(),
        brainfuel.stats.per_edition_present,
        brainfuel.stats.per_edition_absent,
        brainfuel.stats.anomalies,
    );

    println!("atlas-graph-compile: building implementation #1 (from raw sources)...");
    let build_start = Instant::now();
    let (graph_a, stats, event_world_stats, chrono) =
        atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel(&kjv_json, &xrefs_tsv, &atlas, &eras, Some(&brainfuel)).context("building the graph from raw sources")?;
    println!(
        "atlas-graph-compile: {} text units, {} cites edges, {} events ({} dated), {} places, {} narratives, {} anchors -- build time {:?}",
        stats.kjv_verses, stats.cites_rows, event_world_stats.events, event_world_stats.dated_events, event_world_stats.places, event_world_stats.narratives, event_world_stats.anchors,
        build_start.elapsed()
    );
    let chronology = atlas_graph::Chronology::from_derivation(chrono);

    println!("atlas-graph-compile: dumping to the artifact DTO form...");
    let dump = atlas_graph::artifact::dump(&graph_a, &chronology, &stats, &event_world_stats).map_err(|e| anyhow::anyhow!("{e}")).context("dumping the built graph")?;

    println!("atlas-graph-compile: ADMISSION -- rebuilding implementation #1 a second time (independent model)...");
    let (mut graph_b, ..) = atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel(&kjv_json, &xrefs_tsv, &atlas, &eras, Some(&brainfuel)).context("building the independent model graph")?;
    graph_b.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut graph_b);

    let mut graph_a_indexed = graph_a;
    graph_a_indexed.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut graph_a_indexed);

    let admit_start = Instant::now();
    atlas_graph_types::store::assert_answers_match(&graph_a_indexed, &graph_b);
    println!("atlas-graph-compile: ADMISSION passed (assert_answers_match, full graph) in {:?}", admit_start.elapsed());

    // Also prove the DUMP ITSELF round-trips losslessly (not just that the
    // pre-dump graph was internally consistent) -- decode what will
    // actually be written and admit THAT against the model too.
    let bytes = atlas_graph::artifact::encode(&dump).map_err(|e| anyhow::anyhow!("{e}"))?;
    let redecoded = atlas_graph::artifact::decode(&bytes).map_err(|e| anyhow::anyhow!("{e}"))?;
    let (mut reconstructed, ..) = atlas_graph::artifact::to_service_parts(redecoded).map_err(|e| anyhow::anyhow!("{e}"))?;
    reconstructed.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut reconstructed);
    atlas_graph_types::store::assert_answers_match(&reconstructed, &graph_b);
    println!("atlas-graph-compile: ADMISSION passed for the encoded bytes themselves (round-trip, not just the pre-dump graph)");

    std::fs::create_dir_all(out_path.parent().unwrap_or_else(|| Path::new("."))).ok();
    std::fs::write(&out_path, &bytes).with_context(|| format!("writing {}", out_path.display()))?;
    println!("atlas-graph-compile: wrote {} ({} bytes)", out_path.display(), bytes.len());

    Ok(())
}
