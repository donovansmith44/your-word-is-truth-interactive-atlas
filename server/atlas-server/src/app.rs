//! Router construction, kept separate from `main.rs` so tests can build the
//! exact same `Router` the real binary serves (via `tower::ServiceExt::oneshot`)
//! without binding a socket.

use std::path::PathBuf;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use atlas_core::data::AtlasData;

use crate::handlers;

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
pub fn build(data: Arc<AtlasData>, static_dir: Option<PathBuf>) -> Router {
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
        .route("/api/eras", get(handlers::eras))
        .route("/api/polities", get(handlers::polities))
        .route("/api/landmarks", get(handlers::landmarks))
        .route("/api/land-mask", get(handlers::land_mask))
        .with_state(data);

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
