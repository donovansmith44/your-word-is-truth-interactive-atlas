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
use atlas_graph::sqlite::source::{sibling_dir, SectionLayout};
use atlas_graph::GraphService;

struct Args {
    data_dir: PathBuf,
    static_dir: Option<PathBuf>,
    port: u16,
    /// Rebuilds the graph in memory from `data/raw/` and the curated files rather
    /// than opening the compiled artifact, for iterating on curated data without
    /// re-running the compile step.
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
    // parse_args takes a plain argument slice so that it stays testable without a
    // process boundary, which leaves argv[0] to skip here.
    let raw: Vec<String> = env::args().skip(1).collect();
    let args = parse_args(&raw)?;

    let load_start = std::time::Instant::now();
    let (graph, data) = if args.build_from_raw {
        let raw_dir = SectionLayout::under(&args.data_dir).raw_dir();
        let curated_dir = sibling_dir(&args.data_dir, "curated");
        println!("atlas-graph: --build-from-raw -- building in memory from {}", raw_dir.display());
        let data = atlas_etl::compile::compile(&raw_dir, &curated_dir).with_context(|| format!("compiling {} + {}", raw_dir.display(), curated_dir.display()))?.data;
        // `GraphService::build` runs the KJV fidelity law as part of construction and
        // refuses to construct on a violation, so reaching the line after it is
        // already proof that the law held.
        let graph = GraphService::build(&raw_dir, &data)
            .with_context(|| format!("building the explorable graph from {} (kjv.json + xrefs/cross_references.txt)", raw_dir.display()))?;
        // Primes the graph-backed scene source, which the artifact path primes inside
        // `atlas_contract::load`: without this the first scene, chapter or narrative
        // request on this path would pay the whole materialisation inline.
        graph.scene_source(&data);
        (graph, data)
    } else {
        // Through `atlas_contract::load`, which the pact recorder calls too, so the
        // recorded evidence the contract gate runs against cannot drift from what
        // this binary actually serves.
        atlas_contract::load::load_graph_and_data(&args.data_dir)?
    };
    let data = Arc::new(data);
    let load_elapsed = load_start.elapsed();
    println!("atlas-graph: {} complete in {load_elapsed:?}", if args.build_from_raw { "from-raw build" } else { "sections open" });

    println!(
        "atlas-graph: {} KJV text units, {} cites edges ({} negative-vote rows dropped), graph version {}",
        graph.stats.kjv_verses,
        graph.stats.cites_rows,
        graph.stats.cites_dropped_negative_votes,
        atlas_graph::version_hex(graph.version())
    );
    println!(
        "atlas-graph: {} events ({} dated), {} narratives ({} succession rows), {} anchors, {} attests rows, {} located-at rows, {} dated-by rows",
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

    let sources = atlas_contract::load::load_sources(&args.data_dir)?;
    println!("atlas-server: {} source categories, {} sources loaded from the core section under {}", sources.categories.len(), sources.sources.len(), args.data_dir.display());

    // The one door to a serving router, taken by both startup paths and by the pact
    // recorder, so a new piece of state cannot be wired in for one and missed for
    // the others.
    let app = atlas_contract::load::LoadedAtlas { data, graph, sources: Arc::new(sources) }
        .into_router(args.static_dir);

    let addr = format!("0.0.0.0:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.with_context(|| format!("binding {addr}"))?;
    println!("atlas-server listening on http://{addr}");
    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}
