//! Router construction, kept separate from `main.rs` so tests can build the
//! exact same `Router` the real binary serves (via `tower::ServiceExt::oneshot`)
//! without binding a socket.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use atlas_core::data::AtlasData;
use atlas_core::sources::SourcesDocument;
use atlas_graph::GraphService;

use crate::{graph_handlers, handlers};

/// Batch M-A (fix round 1, C1): the two pieces of server state every
/// handler now draws from -- the pre-existing `AtlasData` (places/events/
/// headings/etc., still the source for everything the graph doesn't
/// materialize yet) and `GraphService` (this batch's own thin wrapper
/// around the owner-approved `atlas_graph_types::store` port --
/// `MemStore`/`MemSnapshot`, implementation #1). Every actual graph QUERY
/// a handler performs goes through `atlas_graph_types::store::GraphQuery`'s
/// own trait methods on an opened snapshot (`graph.snapshot()` -- see
/// `graph_handlers.rs`/`handlers::chapter`), never a raw `Graph` field --
/// `GraphService` itself is held concretely here only because
/// `GraphStore`'s own associated type (`type Snapshot: GraphSnapshot`,
/// owner-authored) is not `dyn`-safe, so there is no single erased handle
/// type to store both the port AND this batch's own reading-spine
/// position index under; `window.rs`'s own query logic is what's generic
/// over the port (`&dyn GraphQuery`), which is the half of the seam a
/// future backend (M-C's serialized artifact, or a database) actually
/// needs to slot into with zero change here. `FromRef` lets EVERY
/// pre-existing handler keep its own `State<Arc<AtlasData>>` extractor
/// completely unchanged -- axum resolves it from this composed state
/// automatically -- while the new graph handlers extract
/// `State<Arc<GraphService>>` the same way. Neither state type needs to
/// know about the other.
/// Batch S addition: `sources` (the Sources page's own single source of
/// truth, `data/compiled/sources.json` -- see `atlas_core::sources`'s own
/// doc comment) is a THIRD, independent piece of state, held the same
/// `Arc`-and-`FromRef` way `data`/`graph` already are. It is deliberately
/// its own field, never folded into `AtlasData`: this content has nothing
/// to do with the Explorable Graph, and batch-s-brief.md's own
/// finalization block requires this batch's own additions stay outside
/// the graph pipeline so `graph.bin`/`data/exports/` stay byte-untouched.
#[derive(Clone)]
pub struct AppState {
    pub data: Arc<AtlasData>,
    pub graph: Arc<GraphService>,
    pub sources: Arc<SourcesDocument>,
}

impl FromRef<AppState> for Arc<AtlasData> {
    fn from_ref(s: &AppState) -> Self {
        s.data.clone()
    }
}

impl FromRef<AppState> for Arc<GraphService> {
    fn from_ref(s: &AppState) -> Self {
        s.graph.clone()
    }
}

impl FromRef<AppState> for Arc<SourcesDocument> {
    fn from_ref(s: &AppState) -> Self {
        s.sources.clone()
    }
}

/// Builds the full application router: the `/api/*` + `/health` JSON API,
/// permissive CORS (this is a local sketch app with no auth/cookies to
/// protect), and — when `static_dir` is given — the published Blazor client
/// served as a fallback for any path the API doesn't claim. The fallback
/// uses `ServeDir::not_found_service(ServeFile::new(index.html))`, tower-http's
/// documented single-page-app idiom: a request for a file that exists (e.g.
/// `app.css`) serves that file; a request for anything else (e.g. the
/// client-side route `/world`) serves `index.html` so client-side routing
/// can take over and deep links work.
///
/// tower-http's `not_found_service` tags that fallback response `404 Not
/// Found` — correct from `ServeDir`'s point of view (the literal path `/world`
/// isn't a file on disk) but wrong from the SPA's: a deep link to a real
/// client-side route is not an error, and a browser/`curl` that checks the
/// status instead of sniffing the body sees a false failure. `.map_response`
/// rewrites that 404 to 200 (mirroring what every other SPA host — nginx
/// `try_files`, Netlify/Vercel rewrites — does for this exact pattern: a
/// path that isn't a real static file is by definition client-side-routable,
/// not missing). This only touches the fallback branch; a request for a file
/// that genuinely exists under `static_dir` (or an `/api/*` route) never
/// carries a 404 in the first place and passes through unchanged.
///
/// `.layer(CorsLayer::permissive())` is applied LAST, after the
/// `static_dir` fallback (if any) has been attached — deliberately, not
/// incidentally. `Router::layer` only wraps whatever routes/fallback exist
/// on the router at the moment it's called; `fallback_service` (called
/// inside the `Some(dir)` arm) replaces the fallback with a brand-new
/// `Route` that bypasses any layer applied earlier. Wrapping first and
/// attaching the fallback second would silently leave every fallback-served
/// response (the client's `index.html`, its static assets, and every
/// SPA-route fallback) without CORS headers while `/api/*` had them —
/// exactly the bug a fix-round review caught here. See the
/// `static_dir_serves_files_api_still_wins_and_falls_back_to_index_for_spa_routes`
/// test in `tests/api.rs`, which asserts the CORS header on both an `/api`
/// response and a fallback-served response so this ordering can't silently
/// regress.
/// Every pre-existing caller (the real binary via `main.rs`, and every
/// test in this crate that doesn't itself need `/api/sources`) keeps
/// calling this exact signature, unchanged -- it delegates to
/// [`build_with_sources`] with an empty [`SourcesDocument`], the same
/// "feature absent, everything else identical" shape this crate already
/// uses for optional state. Only `main.rs`'s own real startup path calls
/// `build_with_sources` directly, with the real loaded
/// `data/compiled/sources.json`.
pub fn build(data: Arc<AtlasData>, graph: Arc<GraphService>, static_dir: Option<PathBuf>) -> Router {
    build_with_sources(data, graph, Arc::new(SourcesDocument::default()), static_dir)
}

pub fn build_with_sources(
    data: Arc<AtlasData>,
    graph: Arc<GraphService>,
    sources: Arc<SourcesDocument>,
    static_dir: Option<PathBuf>,
) -> Router {
    let state = AppState { data, graph, sources };

    let api = Router::new()
        .route("/health", get(handlers::health))
        .route("/api/scene", get(handlers::scene_time))
        .route("/api/scene/scripture", get(handlers::scene_scripture))
        .route("/api/books", get(handlers::books))
        .route("/api/chapter/{cref}", get(handlers::chapter))
        .route("/api/verse/{vref}", get(handlers::verse))
        .route("/api/xrefs/{sref}", get(handlers::xrefs))
        .route("/api/catechism/item/{id}", get(handlers::catechism_item))
        .route("/api/catechism/{sref}", get(handlers::catechism_for_span))
        .route("/api/place/{id}", get(handlers::place))
        .route("/api/narratives", get(handlers::narratives))
        .route("/api/narrative/event/{id}", get(handlers::narrative_event_positions))
        .route("/api/event/{id}", get(handlers::event))
        .route("/api/eras", get(handlers::eras))
        .route("/api/polities", get(handlers::polities))
        .route("/api/landmarks", get(handlers::landmarks))
        .route("/api/land-mask", get(handlers::land_mask))
        // Batch S: the Sources page's own single source of truth --
        // straight off `data/compiled/sources.json`, never a hardcoded
        // duplicate list (requirement 3).
        .route("/api/sources", get(handlers::sources))
        // Batch M-A: the two generic graph endpoints (design doc §5) + the
        // text-window endpoint (design doc §6). Uniform across every
        // node/edge kind the graph carries -- not bespoke per-feature shapes.
        .route("/api/node/{id}", get(graph_handlers::node_card))
        .route("/api/node/{id}/edges", get(graph_handlers::node_edges))
        .route("/api/text", get(graph_handlers::text_window))
        .with_state(state);

    let router = match static_dir {
        Some(dir) => {
            let index = dir.join("index.html");
            let serve_dir = ServeDir::new(dir).not_found_service(ServeFile::new(index));
            // See the doc comment above: rewrite the not-found-service
            // fallback's 404 to 200 so SPA deep links (e.g. `/world`) report
            // success, not a false failure. `ServiceExt::<axum::extract::Request>`
            // pins the request body type explicitly — `ServeDir` implements
            // `Service` generically over the request body, so plain
            // `.map_response(..)` leaves that type ambiguous until something
            // downstream forces it; `fallback_service` never does.
            let serve_dir = tower::ServiceExt::<axum::extract::Request>::map_response(serve_dir, |mut res| {
                if res.status() == StatusCode::NOT_FOUND {
                    *res.status_mut() = StatusCode::OK;
                }
                res
            });
            api.fallback_service(serve_dir)
        }
        None => api,
    };

    router.layer(CorsLayer::permissive())
}
