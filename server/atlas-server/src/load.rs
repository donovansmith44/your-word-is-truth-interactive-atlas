//! THE ONE ASSEMBLY PATH — how a `data_dir` on disk becomes a serving
//! `Router`, in exactly one place.
//!
//! # Why this module exists (batch CDC-1, fix round 1, review finding C-3)
//!
//! `contracts/pacts/` is the recorded evidence the contract gate runs every
//! expectation against, and the argument that makes it evidence rather than
//! self-agreement is that it is *regenerated from the real provider* in the
//! same gate. That argument holds only while the recorder assembles the app
//! the way the real server assembles it.
//!
//! CDC-1 got that wrong twice, and both times the suite stayed **green**:
//!
//!   * the recorder built with `SourcesDocument::default()`, so
//!     `GET /api/sources` recorded an empty registry while the real server
//!     served 5 categories / 18 sources / 26 provenances;
//!   * the recorder ran the `--build-from-raw` dev fallback instead of the
//!     default path, so `AtlasData::finish()` never ran, the derived index
//!     `verse_to_catechism` was empty, and `GET /api/catechism/{sref}`
//!     answered `[]` **for every reference** — a whole endpoint dark behind
//!     a passing suite.
//!
//! The first was caught by luck (a projection demanded an array and got an
//! object). The second was caught only when the recorder was moved onto the
//! real path for unrelated reasons. The fix shipped at the time was a
//! comment saying "any change to `main.rs`'s startup must be mirrored
//! here" — a convention, and this project's whole discipline is that
//! conventions of that shape do not hold.
//!
//! So the load sequence lives here, `main.rs` and
//! `tests/contract_pact.rs` both call it, and there is no second copy to
//! drift from. Divergence is not *detected*, it is **unrepresentable** —
//! the same move `contract_pact.rs`'s own `node_kind_manifest!` makes for
//! node kinds, applied to the thing the gate's credibility actually rests
//! on.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Router;

use atlas_core::data::AtlasData;
use atlas_core::sources::SourcesDocument;
use atlas_graph::GraphService;

/// Everything a serving `Router` is built from. Held as one value so there
/// is one thing to pass around and one place a fourth ingredient would have
/// to be added — a new `AppState` member cannot be wired into the server
/// while quietly missing from the recorder, because they receive the same
/// struct from the same constructor.
pub struct LoadedAtlas {
    pub data: Arc<AtlasData>,
    pub graph: Arc<GraphService>,
    pub sources: Arc<SourcesDocument>,
}

impl LoadedAtlas {
    /// For the `--build-from-raw` dev fallback, which builds its graph and
    /// data differently but must still reach the Router through this one
    /// door.
    pub fn new(data: AtlasData, graph: GraphService, sources: SourcesDocument) -> Self {
        LoadedAtlas { data: Arc::new(data), graph: Arc::new(graph), sources: Arc::new(sources) }
    }

    /// THE ONLY CALL SITE of `app::build_with_sources` outside tests that
    /// deliberately exercise a partial app. Keeping it single is what makes
    /// "the recorder builds what the server builds" a fact about the code
    /// rather than a promise in a comment.
    pub fn into_router(self, static_dir: Option<PathBuf>) -> Router {
        crate::app::build_with_sources(self.data, self.graph, self.sources, static_dir)
    }
}

/// The compiled source registry (`<data_dir>/sources.json`).
///
/// Its own independent piece of state, never folded into `AtlasData` or the
/// graph (see `app::AppState`'s doc comment). A missing or unparseable file
/// fails loud, the same "never silently serve stale/absent data" discipline
/// applied to the sections.
pub fn load_sources(data_dir: &Path) -> Result<SourcesDocument> {
    // DB-4c: from core's `source_*`/`provenance_entry` tables (the fold of
    // `sources.json`), never the JSON. One-worker open, sources only.
    let (_, _, sources) = load_all(data_dir)?;
    Ok(sources)
}

/// DB-4c: THE served path in one call -- the sections opened once,
/// `AtlasData` finished, the scene source primed. `load_graph_and_data` and
/// `load_sources` are the same call minus one output each.
pub fn load_all(data_dir: &Path) -> Result<(GraphService, AtlasData, SourcesDocument)> {
    let (graph, data, sources) = GraphService::from_sections(data_dir).with_context(|| {
        format!(
            "opening the committed sections under {} (run atlas-graph-compile first, or pass --build-from-raw for the dev fallback)",
            data_dir.display()
        )
    })?;
    let data = data.finish();
    graph.scene_source(&data);
    Ok((graph, data, sources))
}

/// The DEFAULT startup path: load the serialized graph artifact, load the
/// compiled JSON sidecars, `finish()` the sidecar-derived indexes, and build
/// the graph-backed scene source.
///
/// The `finish()` here is not a detail. Without it every index derived from
/// the compiled files is empty, and endpoints that read one answer as though
/// they had no data -- silently, with a 200. That was fidelity bug 2. What
/// it still derives on this path is the SIDECAR half (the catechism trio,
/// the heading reference index, the merge tables): `AtlasData.events`/
/// `.places`/`.narratives` are never populated on any serving path any more,
/// so every derivation built from them is a no-op over empty collections.
///
/// OVERLAY-1 Task 5: `legacy::atlas_data_overlay` is GONE. The scene's data
/// is `GraphSceneSource`, materialised from the graph port and primed here,
/// eagerly, so no request ever pays for it (see
/// `GraphService::scene_source`). That is the batch's whole point: the
/// events/places/narratives the map composes from exist ONCE, in that
/// object, instead of once there and once again inside `AtlasData`.
pub fn load_graph_and_data(data_dir: &Path) -> Result<(GraphService, AtlasData)> {
    // DB-4c: the sections, not `graph.bin` + the JSON sidecars (spec §8
    // row 5). `finish()` and the scene priming are unchanged in role.
    let (graph, data, _sources) = load_all(data_dir)?;
    Ok((graph, data))
}

/// The whole default path, graph through sources, as one call.
///
/// This is what the pact recorder uses; `main.rs` uses the two halves so it
/// can print its startup statistics between them, and then reaches the
/// Router through the same `into_router`.
pub fn load_from_data_dir(data_dir: &Path) -> Result<LoadedAtlas> {
    let (graph, data, sources) = load_all(data_dir)?;
    Ok(LoadedAtlas::new(data, graph, sources))
}
