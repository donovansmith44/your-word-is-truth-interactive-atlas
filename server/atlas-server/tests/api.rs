//! Endpoint tests against `atlas_core::data::demo_fixture()` via
//! `tower::ServiceExt::oneshot` — no real server binds a socket here.

use std::sync::Arc;

use atlas_core::data::demo_fixture;
use axum::body::Body;
use axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn app() -> axum::Router {
    atlas_server::app::build(Arc::new(demo_fixture()), None)
}

/// Fetches `uri` from `app` and parses the response body as JSON. Only for
/// endpoints that actually return JSON — `/health` returns plain text and is
/// fetched with its own ad hoc oneshot call in `health_books_eras_narratives_shapes`.
async fn call(app: &axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    (status, json)
}

#[tokio::test]
async fn scene_time_ok_and_errors() {
    let app = app();
    let (st, body) = call(&app, "/api/scene?from=-1406&to=-1405").await;
    assert_eq!(st, 200);
    assert_eq!(body["mode"], "time");
    assert!(body["places"].as_array().unwrap().iter().any(|p| p["id"] == "jericho"));
    for bad in ["/api/scene?from=0&to=5", "/api/scene?from=5&to=-5", "/api/scene?from=1"] {
        let (st, body) = call(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_window");
    }
}

/// Extends the verbatim brief test above with the two cases the self-review
/// explicitly calls out: a fully-unparseable pair of values, and both params
/// missing entirely. Both must still yield the typed `bad_window` body, not
/// axum's default extractor-rejection body — proving `Query<HashMap<String,
/// String>>` (which cannot itself reject on these inputs) is doing the job.
#[tokio::test]
async fn scene_time_query_extraction_never_leaks_axum_rejection_body() {
    let app = app();
    for bad in ["/api/scene?from=x&to=y", "/api/scene", "/api/scene?to=5", "/api/scene?from=&to="] {
        let (st, body) = call(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_window", "{bad}: {body}");
        assert!(body["error"]["message"].is_string(), "{bad}: {body}");
    }
    // Ruling 2: out-of-span years are NOT errors, only zero/inverted/missing/unparseable are.
    let (st, body) = call(&app, "/api/scene?from=-50000&to=50000").await;
    assert_eq!(st, 200);
    assert_eq!(body["mode"], "time");
}

#[tokio::test]
async fn scene_scripture_ok_and_bad_ref() {
    let app = app();
    let (st, body) = call(&app, "/api/scene/scripture?ref=GEN.13.18").await;
    assert_eq!(st, 200);
    assert_eq!(body["mode"], "scripture");
    assert_eq!(body["ref"], "GEN.13.18");
    assert!(body["places"].as_array().unwrap().iter().any(|p| p["id"] == "hebron"));

    // Structurally invalid refs (ruling 3: these MUST be 400 bad_ref) plus a
    // missing `ref` param entirely (ruling 1: same treatment as `from`/`to`).
    for bad in [
        "/api/scene/scripture?ref=NOPE",
        "/api/scene/scripture?ref=GEN.0.1",
        "/api/scene/scripture?ref=gen..1",
        "/api/scene/scripture?ref=GEN.1.9-2",
        "/api/scene/scripture",
    ] {
        let (st, body) = call(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_ref", "{bad}: {body}");
    }

    // Ruling 3 policy choice (documented in handlers.rs): a structurally
    // valid ref whose chapter number is out of range for the loaded canon
    // does NOT error — it composes a (here, empty) scene, same as an
    // out-of-span time window. Chapter 99 of Genesis matches no event/place
    // verse link in the fixture, so this must come back 200 with no places.
    let (st, body) = call(&app, "/api/scene/scripture?ref=GEN.99").await;
    assert_eq!(st, 200);
    assert_eq!(body["mode"], "scripture");
    assert_eq!(body["places"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn health_books_eras_narratives_shapes() {
    let app = app();

    let response =
        app.clone().oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&bytes[..], b"ok");

    let (st, body) = call(&app, "/api/books").await;
    assert_eq!(st, 200);
    let books = body.as_array().unwrap();
    assert_eq!(books.len(), 2);
    assert_eq!(books[0]["code"], "GEN");
    assert_eq!(books[0]["name"], "Genesis");
    assert_eq!(books[0]["chapters"], serde_json::json!([31]));
    assert_eq!(books[1]["code"], "JOS");

    let (st, body) = call(&app, "/api/eras").await;
    assert_eq!(st, 200);
    let eras = body.as_array().unwrap();
    assert_eq!(eras.len(), 2);
    assert_eq!(eras[0]["id"], "patriarchs");
    assert_eq!(eras[0]["from_year"], -2166);
    assert_eq!(eras[0]["to_year"], -1877);

    let (st, body) = call(&app, "/api/narratives").await;
    assert_eq!(st, 200);
    let narratives = body.as_array().unwrap();
    assert_eq!(narratives.len(), 2);
    let conquest = narratives.iter().find(|n| n["id"] == "conquest").expect("conquest narrative present");
    assert_eq!(conquest["name"], "The Conquest");
    assert_eq!(conquest["color"], "#7C3AED");
    assert_eq!(conquest["legs"], serde_json::json!(["e1", "e2", "e3", "e4"]));
}

#[tokio::test]
async fn verse_chapter_place_and_404() {
    let app = app();

    // --- /api/chapter -----------------------------------------------------
    let (st, body) = call(&app, "/api/chapter/JOS.1").await;
    assert_eq!(st, 200);
    assert_eq!(body["ref"], "JOS.1");
    assert_eq!(body["book"], "Joshua");
    assert_eq!(body["chapter"], 1);
    let verses = body["verses"].as_array().unwrap();
    assert_eq!(verses.len(), 3);
    assert_eq!(verses[0]["verse"], 1);
    assert_eq!(verses[2]["verse"], 3);
    assert!(verses[0]["text"].as_str().unwrap().contains("Moses"));

    // Structurally invalid cref -> 400 bad_ref.
    let (st, body) = call(&app, "/api/chapter/NOPE.1").await;
    assert_eq!(st, 400);
    assert_eq!(body["error"]["code"], "bad_ref");

    // Wrong-shape cref (a book ref, not a chapter ref) -> 400 bad_ref.
    let (st, body) = call(&app, "/api/chapter/JOS").await;
    assert_eq!(st, 400);
    assert_eq!(body["error"]["code"], "bad_ref");

    // Ruling-3 policy: valid shape, chapter number out of range for this
    // fixture's canon (only chapter 1 of Joshua is known) -> 200, empty list.
    let (st, body) = call(&app, "/api/chapter/JOS.2").await;
    assert_eq!(st, 200);
    assert_eq!(body["verses"].as_array().unwrap().len(), 0);

    // `?translation=kjv` must be silently ignored (ruling 5), not 400.
    let (st, _body) = call(&app, "/api/chapter/JOS.1?translation=kjv").await;
    assert_eq!(st, 200);

    // --- /api/verse ---------------------------------------------------------
    let (st, body) = call(&app, "/api/verse/JOS.6.20").await;
    assert_eq!(st, 200);
    assert_eq!(body["ref"], "JOS.6.20");
    assert!(body["text"].as_str().unwrap().contains("wall fell down flat"));
    assert_eq!(body["book_meta"]["author"], "Joshua");
    assert_eq!(body["book_meta"]["write_place"], "gilgal");
    assert_eq!(body["book_meta"]["write_from"], -1400);
    assert_eq!(body["book_meta"]["write_to"], -1370);

    let events = body["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["id"], "e3");
    assert_eq!(events[0]["label"], "Jericho falls");
    assert_eq!(events[0]["when"]["from_year"], -1405);
    assert_eq!(events[0]["places"], serde_json::json!(["jericho"]));
    assert!(events[0]["verse_groups"].as_array().unwrap().iter().any(|g| g["book"] == "JOS" && g["chapter"] == 6));

    let cross_refs = body["cross_refs"].as_array().unwrap();
    assert_eq!(cross_refs.len(), 3);
    assert_eq!(cross_refs[0]["target"], "JOS.6.20-21");
    assert_eq!(cross_refs[0]["votes"], 9);
    assert!(cross_refs[0]["preview"].as_str().unwrap().contains("wall fell down flat"));
    assert_eq!(cross_refs[1]["target"], "JOS.1.3");
    assert_eq!(cross_refs[1]["votes"], 5);
    assert!(cross_refs[1]["preview"].as_str().unwrap().contains("sole of your foot"));
    assert_eq!(cross_refs[2]["target"], "GEN.13.18");
    assert!(cross_refs[2]["preview"].as_str().unwrap().contains("Hebron"));

    // Structurally invalid vref -> 400 bad_ref.
    let (st, body) = call(&app, "/api/verse/NOPE.1.1").await;
    assert_eq!(st, 400);
    assert_eq!(body["error"]["code"], "bad_ref");

    // Wrong-shape vref (a chapter ref, not a verse ref) -> 400 bad_ref.
    let (st, body) = call(&app, "/api/verse/JOS.6").await;
    assert_eq!(st, 400);
    assert_eq!(body["error"]["code"], "bad_ref");

    // Structurally valid vref whose text doesn't exist in this fixture -> 404.
    let (st, body) = call(&app, "/api/verse/GEN.1.1").await;
    assert_eq!(st, 404);
    assert_eq!(body["error"]["code"], "not_found");

    // --- /api/place -----------------------------------------------------
    let (st, body) = call(&app, "/api/place/jericho").await;
    assert_eq!(st, 200);
    assert_eq!(body["id"], "jericho");
    assert_eq!(body["name"], "Jericho");
    let events = body["events"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["id"], "e2"); // -1406
    assert_eq!(events[1]["id"], "e3"); // -1405, so sorted after e2
    assert!(events[0]["when"]["from_year"].as_i64().unwrap() <= events[1]["when"]["from_year"].as_i64().unwrap());

    let (st, body) = call(&app, "/api/place/does-not-exist").await;
    assert_eq!(st, 404);
    assert_eq!(body["error"]["code"], "not_found");
}

/// `app::build`'s `static_dir` branch: API routes still win over the static
/// fallback, an existing file is served as itself, and an unmatched path
/// (a Blazor client-side route like `/world`) falls back to `index.html`'s
/// content (tower-http's `not_found_service` SPA idiom — the status on that
/// fallback response is 404, which is fine: the client-side router only
/// needs the HTML body, not the status). Also asserts the permissive-CORS
/// layer reaches BOTH an `/api`-routed response and a fallback-served
/// response: `Router::layer` only wraps routes/fallback that exist at the
/// time it's called, so if `.layer(CorsLayer::permissive())` were ever
/// reordered to run before `.fallback_service(...)` is attached, the
/// fallback branch would silently lose its CORS headers while `/api/*`
/// still had them — this pins the correct (CORS-applied-last) ordering.
#[tokio::test]
async fn static_dir_serves_files_api_still_wins_and_falls_back_to_index_for_spa_routes() {
    let dir = std::env::temp_dir().join(format!("atlas-server-static-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<html>shell</html>").unwrap();
    std::fs::write(dir.join("app.css"), "body{color:red}").unwrap();

    let app = atlas_server::app::build(Arc::new(demo_fixture()), Some(dir.clone()));

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/health").header("origin", "http://example.com").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN).map(|v| v.to_str().unwrap()),
        Some("*"),
        "an /api route response must carry permissive CORS headers"
    );
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&bytes[..], b"ok", "an API route must win over the static fallback");

    let response = app.clone().oneshot(Request::builder().uri("/app.css").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&bytes[..], b"body{color:red}");

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/world").header("origin", "http://example.com").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN).map(|v| v.to_str().unwrap()),
        Some("*"),
        "a fallback-served (SPA-route) response must ALSO carry permissive CORS headers, \
         not just direct /api routes — this is the case the layer-ordering bug broke"
    );
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&bytes[..], b"<html>shell</html>", "unmatched client-side routes must fall back to index.html");

    std::fs::remove_dir_all(&dir).ok();
}
