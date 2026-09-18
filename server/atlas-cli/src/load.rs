//! Startup: open the committed sections (`GraphService::from_sections`,
//! DB-4c; DB-5 retired `graph.bin` and the compiled JSON sidecars), the
//! SAME path `atlas-server/src/main.rs`'s default (non `--build-from-raw`)
//! branch already uses -- see that file's own doc comment for the full
//! reasoning (OVERLAY-1 Task 5 retired the overlay that used to
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

/// Opens the sections under `data_dir` (manifest.toml + sections/). Every
/// failure on this path is `data_load_failed` (CONTRACT.md) -- this runs
/// before any command's own logic, so nothing downstream can distinguish
/// "graph missing" from "graph corrupt" from "a JSON file missing"; the
/// message always names the exact path this process looked at and the
/// exact underlying I/O/parse error, plus the fixed remedy (`--data-dir`,
/// or re-run the compile step).
pub fn load(data_dir: &Path) -> Result<Loaded, CliError> {
    // DB-4c: the committed sections (manifest.toml + sections/*.sqlite.zst),
    // never graph.bin or the JSON sidecars -- the same
    // `GraphService::from_sections` the server starts from.
    let (graph, data, _sources) = GraphService::from_sections(data_dir).map_err(|e| {
        CliError::data_load_failed(
            format!("could not open the sections under {}", data_dir.display()),
            e.to_string(),
            "run 'cargo run -p atlas-graph --bin atlas-graph-compile' from server/ first, or pass --data-dir to point at a directory that has manifest.toml and sections/",
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
