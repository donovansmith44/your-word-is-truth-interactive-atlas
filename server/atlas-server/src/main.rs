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
}

fn parse_args(args: &[String]) -> Result<Args> {
    let mut data_dir: Option<PathBuf> = None;
    let mut static_dir: Option<PathBuf> = None;
    let mut port: u16 = 8000;

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
            other => bail!("unrecognized argument: {other}"),
        }
        i += 1;
    }

    let data_dir = data_dir.context("--data-dir is required, e.g. --data-dir ../data/compiled")?;
    Ok(Args { data_dir, static_dir, port })
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
    let raw_dir = args.data_dir.parent().map(|p| p.join("raw")).unwrap_or_else(|| PathBuf::from("../data/raw"));
    // FAIL-LOUD FIDELITY GATE (design doc P3): `GraphService::build` runs
    // the KJV adapter's own bijection + reconstruction boundary law
    // unconditionally as part of construction (see `atlas_graph::service`'s
    // own doc comment) -- a violation refuses construction entirely, so
    // reaching the `println!` below already proves the gate passed. This
    // graph is published to the owner-approved `atlas_graph_types::store`
    // port (`GraphPublisher::publish`) before being wrapped here; every
    // downstream consumer (app::build, every handler, the window/text
    // path) queries it as `atlas_graph_types::store::GraphQuery`, never a
    // raw `Graph` field (fix round 1, C1).
    let graph = GraphService::build(&raw_dir)
        .with_context(|| format!("building the explorable graph from {} (kjv.json + xrefs/cross_references.txt)", raw_dir.display()))?;

    println!(
        "atlas-graph: {} KJV text units, {} cites edges ({} negative-vote rows dropped, disclosed), graph version {}",
        graph.stats.kjv_verses,
        graph.stats.cites_rows,
        graph.stats.cites_dropped_negative_votes,
        atlas_graph::version_hex(graph.version())
    );
    let graph = Arc::new(graph);

    let app = atlas_server::app::build(data, graph, args.static_dir);

    let addr = format!("0.0.0.0:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.with_context(|| format!("binding {addr}"))?;
    println!("atlas-server listening on http://{addr}");
    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}
