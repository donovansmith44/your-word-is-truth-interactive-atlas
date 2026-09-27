//! Startup: open the committed sections. This crate never builds from raw sources and never
//! touches HTTP.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use atlas_core::data::AtlasData;
use atlas_graph::GraphService;

use crate::error::CliError;

pub struct Loaded {
    pub graph: Arc<GraphService>,
    pub data: Arc<AtlasData>,
}

/// Every failure here is `data_load_failed`: this runs before any command's own logic, so
/// nothing downstream could tell a missing graph from a corrupt one. The message always
/// names the exact path looked at and the underlying error.
pub fn load(data_dir: &Path) -> Result<Loaded, CliError> {
    let (graph, data, _sources) = GraphService::from_sections(data_dir).map_err(|e| {
        CliError::data_load_failed(
            format!("could not open the sections under {}", data_dir.display()),
            e.to_string(),
            "run 'cargo run -p atlas-graph --bin atlas-graph-compile' from server/ first, or pass --data-dir to point at a directory that has manifest.toml and sections/",
        )
    })?;
    let data = data.finish();
    // Priming the scene source here keeps its cost inside the load step, which is the step
    // that reports a failure as `data_load_failed`.
    graph.scene_source(&data);

    Ok(Loaded { graph: Arc::new(graph), data: Arc::new(data) })
}

pub fn default_data_dir() -> PathBuf {
    PathBuf::from("../data/compiled")
}
