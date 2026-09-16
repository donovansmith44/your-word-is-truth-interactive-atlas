//! Startup: load the compiled graph artifact + `AtlasData`, the SAME
//! artifact-load path `atlas-server/src/main.rs`'s default (non
//! `--build-from-raw`) branch already uses -- see that file's own doc
//! comment for the full reasoning (`GraphService::from_artifact` reads
//! `<data-dir>/graph.bin`; `AtlasData::load` reads the ten surviving
//! compiled JSON files; OVERLAY-1 Task 5 retired the overlay that used to
//! reconstruct the five deleted `AtlasData` fields on top of that, so the
//! event/place/narrative data `bibex verse` reads now lives in ONE place,
//! `GraphService::scene_source`, materialised straight off the already-
//! loaded graph -- still no raw/curated re-parsing). This crate never builds
//! from raw
//! sources (`--build-from-raw` is a server-only dev fallback, out of
//! scope for a query CLI) and never touches HTTP/axum -- R1's "no server,
//! no HTTP" requirement.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use atlas_core::data::AtlasData;
use atlas_graph::GraphService;

use crate::error::CliError;

pub struct Loaded {
    pub graph: Arc<GraphService>,
    pub data: Arc<AtlasData>,
}

/// Loads `graph.bin` + the compiled JSON files from `data_dir`. Every
/// failure on this path is `data_load_failed` (CONTRACT.md) -- this runs
/// before any command's own logic, so nothing downstream can distinguish
/// "graph missing" from "graph corrupt" from "a JSON file missing"; the
/// message always names the exact path this process looked at and the
/// exact underlying I/O/parse error, plus the fixed remedy (`--data-dir`,
/// or re-run the compile step).
pub fn load(data_dir: &Path) -> Result<Loaded, CliError> {
    let artifact_path = data_dir.join("graph.bin");
    let graph = GraphService::from_artifact(&artifact_path).map_err(|e| {
        CliError::data_load_failed(
            format!("could not load {}", artifact_path.display()),
            e.to_string(),
            "run 'cargo run -p atlas-graph --bin atlas-graph-compile' from server/ first, or pass --data-dir to point at a directory that already has graph.bin",
        )
    })?;

    let data = AtlasData::load(data_dir).map_err(|e| {
        CliError::data_load_failed(
            format!("could not load compiled data from {}", data_dir.display()),
            e.to_string(),
            "pass --data-dir to point at the directory containing canon.json, books-meta.json, and the other compiled JSON files (the sibling of graph.bin)",
        )
    })?;
    let data = data.finish();
    // OVERLAY-1 Task 5: prime the graph-backed scene source, the SAME thing
    // `atlas_server::load::load_graph_and_data` primes -- `bibex verse`'s
    // PLACES/EVENTS/PASSAGES sections read it, and doing it here keeps the
    // cost inside the load step this crate already reports as
    // `data_load_failed` on failure rather than inside a command.
    graph.scene_source(&data);

    Ok(Loaded { graph: Arc::new(graph), data: Arc::new(data) })
}

/// The default `--data-dir`: `../data/compiled`, the same relative layout
/// every other tool in this repo assumes when run from `server/` (see
/// `atlas-server/src/main.rs`'s own doc comment).
pub fn default_data_dir() -> PathBuf {
    PathBuf::from("../data/compiled")
}
