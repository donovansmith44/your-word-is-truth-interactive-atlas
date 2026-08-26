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
use atlas_core::sources::SourcesDocument;
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
    //
    // M-C2: `AtlasData` construction now depends on WHICH graph path ran,
    // reordered so the graph loads/builds FIRST:
    // - `--build-from-raw`: needs a real, validated `AtlasData` as its own
    //   event-world/place/polity/catechism adapter source (unchanged
    //   reasoning from M-A/M-B/M-C) -- built via `atlas_etl::compile::
    //   compile` (a real raw+curated compile; the SAME orchestration
    //   `atlas-etl`'s own binary runs), since `AtlasData::load`'s own five
    //   retiring-file reads (places/events/narratives/verses-kjv/
    //   cross-refs.json, this batch's own deletion target) return empty.
    // - default (artifact load): stays fast -- `AtlasData::load` still
    //   reads the TEN surviving compiled files (canon/books-meta/
    //   chronology-anchors/book-narration-windows/polities/landmarks/
    //   place-history/place-names-kjv/land-mask/catechism, all untouched
    //   by this batch), then `atlas_graph::legacy::atlas_data_overlay`
    //   reconstructs the five retiring fields DIRECTLY FROM THE
    //   ALREADY-LOADED GRAPH (no raw/curated re-parsing at all) -- fast,
    //   in-memory, which is what keeps the artifact LOAD-TIME ceiling
    //   (<=3s, a committed law) meaningful for this server's own real
    //   total startup, not just the graph's own isolated load step. Every
    //   surface not yet migrated onto the graph this batch (scene.rs's map
    //   composition, `handlers::chapter`'s place-mention half,
    //   `handlers::catechism_item`'s proof-verse text, `narrative_event_
    //   positions`'s residual `adjacent_event` calls) keeps working on
    //   this reconstructed `AtlasData`, unchanged.
    let load_start = std::time::Instant::now();
    let (graph, data) = if args.build_from_raw {
        let raw_dir = args.data_dir.parent().map(|p| p.join("raw")).unwrap_or_else(|| PathBuf::from("../data/raw"));
        let curated_dir = args.data_dir.parent().map(|p| p.join("curated")).unwrap_or_else(|| PathBuf::from("../data/curated"));
        println!("atlas-graph: --build-from-raw -- building in memory from {} (dev fallback, disclosed)", raw_dir.display());
        let data = atlas_etl::compile::compile(&raw_dir, &curated_dir).with_context(|| format!("compiling {} + {}", raw_dir.display(), curated_dir.display()))?.data;
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
        let graph = GraphService::build(&raw_dir, &data)
            .with_context(|| format!("building the explorable graph from {} (kjv.json + xrefs/cross_references.txt)", raw_dir.display()))?;
        (graph, data)
    } else {
        let artifact_path = args.data_dir.join("graph.bin");
        let graph = GraphService::from_artifact(&artifact_path)
            .with_context(|| format!("loading the serialized graph artifact from {} (run atlas-graph-compile first, or pass --build-from-raw for the dev fallback)", artifact_path.display()))?;
        let mut data = AtlasData::load(&args.data_dir)
            .with_context(|| format!("loading compiled data from {}", args.data_dir.display()))?;
        let overlay = atlas_graph::legacy::atlas_data_overlay(&graph);
        data.events = overlay.events;
        data.places = overlay.places;
        data.narratives = overlay.narratives;
        data.verses = overlay.verses;
        let data = data.finish();
        (graph, data)
    };
    let data = Arc::new(data);
    let load_elapsed = load_start.elapsed();
    // M-C2 (folded M-C review M-2): was "{} load complete" with the
    // artifact-path branch string ALREADY ending in "load" ("artifact
    // load"), producing "artifact load load complete" -- a doubled word.
    // Dropping "load" from the format string (not the branch strings)
    // reads correctly for both: "artifact load complete" / "from-raw build
    // complete".
    println!("atlas-graph: {} complete in {load_elapsed:?}", if args.build_from_raw { "from-raw build" } else { "artifact load" });

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

    // Batch S: the Sources page's own single source of truth --
    // `data/compiled/sources.json`, a sibling of every other compiled
    // file already read above, but deliberately loaded as its OWN
    // independent piece of state (never folded into `AtlasData`/the
    // graph -- see `app::AppState`'s own doc comment). A missing or
    // unparseable file fails loud at startup, the same "never silently
    // serve stale/absent data" discipline this binary already applies to
    // `graph.bin`/the compiled JSON files above -- run `cargo run -p
    // atlas-etl --bin gen_sources` (from `server/`) to (re)generate it.
    let sources_path = args.data_dir.join("sources.json");
    let sources_json = std::fs::read_to_string(&sources_path)
        .with_context(|| format!("reading {} (run `cargo run -p atlas-etl --bin gen_sources` from server/ first)", sources_path.display()))?;
    let sources: SourcesDocument = serde_json::from_str(&sources_json).with_context(|| format!("parsing {}", sources_path.display()))?;
    println!("atlas-server: {} source categories, {} sources loaded from {}", sources.categories.len(), sources.sources.len(), sources_path.display());
    let sources = Arc::new(sources);

    let app = atlas_server::app::build_with_sources(data, graph, sources, args.static_dir);

    let addr = format!("0.0.0.0:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.with_context(|| format!("binding {addr}"))?;
    println!("atlas-server listening on http://{addr}");
    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}
