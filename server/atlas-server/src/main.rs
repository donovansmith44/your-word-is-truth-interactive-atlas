//! atlas-server binary: hand-parsed CLI, loads compiled data, serves the
//! API (and optionally the published client) over HTTP.
//!
//! ```text
//! atlas-server --data-dir ../data/compiled [--static-dir <path>] [--port 8000]
//! ```

use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use atlas_core::data::AtlasData;
use atlas_graph::GraphService;

struct Args {
    data_dir: PathBuf,
    static_dir: Option<PathBuf>,
    port: u16,
    /// M-C (controller decision 4): the startup build retires -- the
    /// DEFAULT path loads the serialized graph artifact
    /// (`<data_dir>/graph.bin`, disclosed convention: same directory
    /// every other compiled file already lives in). `--build-from-raw`
    /// is the dev fallback flag the brief's own wording calls for
    /// (disclosed): rebuilds in memory from `data/raw/` + curated eras,
    /// exactly the M-A/M-B startup path, for iterating on curated data
    /// without re-running the compile step.
    build_from_raw: bool,
}

fn parse_args(args: &[String]) -> Result<Args> {
    let mut data_dir: Option<PathBuf> = None;
    let mut static_dir: Option<PathBuf> = None;
    let mut port: u16 = 8000;
    let mut build_from_raw = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--data-dir" => {
                i += 1;
                let v = args.get(i).context("--data-dir requires a value")?;
                data_dir = Some(PathBuf::from(v));
            }
            "--static-dir" => {
                i += 1;
                let v = args.get(i).context("--static-dir requires a value")?;
                static_dir = Some(PathBuf::from(v));
            }
            "--port" => {
                i += 1;
                let v = args.get(i).context("--port requires a value")?;
                port = v.parse().with_context(|| format!("--port value '{v}' is not a valid port number"))?;
            }
            "--build-from-raw" => {
                build_from_raw = true;
            }
            other => bail!("unrecognized argument: {other}"),
        }
        i += 1;
    }

    let data_dir = data_dir.context("--data-dir is required, e.g. --data-dir ../data/compiled")?;
    Ok(Args { data_dir, static_dir, port, build_from_raw })
}

#[tokio::main]
async fn main() -> Result<()> {
    // Skip argv[0] (the executable path) before handing off to parse_args,
    // which is written to take a plain argument slice so it stays testable
    // without a process boundary.
    let raw: Vec<String> = env::args().skip(1).collect();
    let args = parse_args(&raw)?;

    let data = AtlasData::load(&args.data_dir)
        .with_context(|| format!("loading compiled data from {}", args.data_dir.display()))?
        .finish();
    let data = Arc::new(data);

    // Batch M-A (controller ruling 2): the Explorable Graph is built IN
    // MEMORY at startup from the same raw sources atlas-etl reads --
    // `data/raw/`, always the sibling of `--data-dir`'s own `data/compiled`
    // under one `data/` parent (true of every real invocation of this
    // binary, including start-api.ps1's own `--data-dir ../data/compiled`;
    // deriving it this way avoids a second CLI flag for M-A's own pragmatic
    // scope). 31,102 KJV verses plus ~344k raw cross-reference rows is
    // trivial startup work.
    // Batch M-C (controller decision 4): the startup BUILD retires -- the
    // default path LOADS the serialized graph artifact
    // (`<data_dir>/graph.bin`, produced by the `atlas-graph-compile` bin,
    // see `bin/compile_graph.rs`'s own doc comment), start-to-listening
    // <=3s release, a committed law (`tests/artifact_conformance.rs`
    // proves it in CI). `--build-from-raw` is the disclosed dev fallback:
    // rebuilds in memory from `data/raw/` + curated eras, the M-A/M-B
    // startup path, unconditionally still enforcing the KJV fidelity law
    // (design doc P3) since raw bytes are actually in hand on this path.
    let load_start = std::time::Instant::now();
    let graph = if args.build_from_raw {
        let raw_dir = args.data_dir.parent().map(|p| p.join("raw")).unwrap_or_else(|| PathBuf::from("../data/raw"));
        println!("atlas-graph: --build-from-raw -- building in memory from {} (dev fallback, disclosed)", raw_dir.display());
        // FAIL-LOUD FIDELITY GATE (design doc P3): `GraphService::build`
        // runs the KJV adapter's own bijection + reconstruction boundary
        // law unconditionally as part of construction (see
        // `atlas_graph::service`'s own doc comment) -- a violation
        // refuses construction entirely, so reaching the `println!` below
        // already proves the gate passed.
        // Batch M-B: the event world (events/attestations/narratives/
        // anchors/chronology) is built from the SAME already-loaded
        // `data` -- see `atlas_graph::event_world`'s own module doc
        // comment for why this adapter reads `AtlasData` rather than
        // re-parsing `data/curated/`.
        GraphService::build(&raw_dir, &data)
            .with_context(|| format!("building the explorable graph from {} (kjv.json + xrefs/cross_references.txt)", raw_dir.display()))?
    } else {
        let artifact_path = args.data_dir.join("graph.bin");
        GraphService::from_artifact(&artifact_path)
            .with_context(|| format!("loading the serialized graph artifact from {} (run atlas-graph-compile first, or pass --build-from-raw for the dev fallback)", artifact_path.display()))?
    };
    let load_elapsed = load_start.elapsed();
    println!("atlas-graph: {} load complete in {load_elapsed:?}", if args.build_from_raw { "from-raw build" } else { "artifact load" });

    // Published to the owner-approved `atlas_graph_types::store` port
    // (`GraphPublisher::publish`) before being wrapped here, on either
    // path; every downstream consumer (app::build, every handler, the
    // window/text path) queries it as `atlas_graph_types::store::
    // GraphQuery`, never a raw `Graph` field (fix round 1, C1).
    println!(
        "atlas-graph: {} KJV text units, {} cites edges ({} negative-vote rows dropped, disclosed), graph version {}",
        graph.stats.kjv_verses,
        graph.stats.cites_rows,
        graph.stats.cites_dropped_negative_votes,
        atlas_graph::version_hex(graph.version())
    );
    println!(
        "atlas-graph (M-B event world): {} events ({} dated), {} narratives ({} succession rows), {} anchors, {} attests rows, {} located-at rows, {} dated-by rows",
        graph.event_world_stats.events,
        graph.event_world_stats.dated_events,
        graph.event_world_stats.narratives,
        graph.event_world_stats.succession_rows,
        graph.event_world_stats.anchors,
        graph.event_world_stats.attests_rows,
        graph.event_world_stats.located_at_rows,
        graph.event_world_stats.dated_by_rows,
    );
    let graph = Arc::new(graph);

    let app = atlas_server::app::build(data, graph, args.static_dir);

    let addr = format!("0.0.0.0:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.with_context(|| format!("binding {addr}"))?;
    println!("atlas-server listening on http://{addr}");
    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}
