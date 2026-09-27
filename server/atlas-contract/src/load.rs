use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Router;

use atlas_core::data::AtlasData;
use atlas_core::sources::SourcesDocument;
use atlas_graph::GraphService;

/// Everything a serving `Router` is built from, held as one value: a fourth
/// ingredient cannot be wired into the server while missing from the pact
/// recorder, because both receive this struct from the same constructor.
pub struct LoadedAtlas {
    pub data: Arc<AtlasData>,
    pub graph: Arc<GraphService>,
    pub sources: Arc<SourcesDocument>,
}

impl LoadedAtlas {
    pub fn new(data: AtlasData, graph: GraphService, sources: SourcesDocument) -> Self {
        LoadedAtlas { data: Arc::new(data), graph: Arc::new(graph), sources: Arc::new(sources) }
    }

    pub fn into_router(self, static_dir: Option<PathBuf>) -> Router {
        crate::app::build_with_sources(self.data, self.graph, self.sources, static_dir)
    }
}

pub fn load_sources(data_dir: &Path) -> Result<SourcesDocument> {
    let (_, _, sources) = load_all(data_dir)?;
    Ok(sources)
}

pub fn load_all(data_dir: &Path) -> Result<(GraphService, AtlasData, SourcesDocument)> {
    let (graph, data, sources) = GraphService::from_sections(data_dir).with_context(|| {
        format!(
            "opening the committed sections under {} (run atlas-graph-compile first, or pass --build-from-raw for the dev fallback)",
            data_dir.display()
        )
    })?;
    // Without `finish()` every sidecar-derived index is empty and the endpoints
    // that read one answer 200 as though the atlas held no data.
    let data = data.finish();
    graph.scene_source(&data);
    Ok((graph, data, sources))
}

pub fn load_graph_and_data(data_dir: &Path) -> Result<(GraphService, AtlasData)> {
    let (graph, data, _sources) = load_all(data_dir)?;
    Ok((graph, data))
}

pub fn load_from_data_dir(data_dir: &Path) -> Result<LoadedAtlas> {
    let (graph, data, sources) = load_all(data_dir)?;
    Ok(LoadedAtlas::new(data, graph, sources))
}
