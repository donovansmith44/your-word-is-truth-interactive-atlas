//! Endpoint tests against `atlas_core::data::demo_fixture()` via
//! `tower::ServiceExt::oneshot` — no real server binds a socket here.

use std::sync::Arc;

use atlas_core::data::{demo_fixture, AtlasData, Polity, PolityEra};
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

/// Batch E: `/api/place/{id}?from=&to=` end to end, against
/// `demo_fixture()`'s own small curated "hebron" history (Kirjath-arba ->
/// Hebron at -2001/-2000, an `established` claim at -2000, a `patriarchs`-
/// era blurb) -- exercises the SAME resolution `atlas_core::history`'s own
/// unit/property tests cover, but through the real HTTP handler, proving
/// the query-param plumbing (parse, `TimeRange::new`, `resolve_display_name`/
/// `resolve_blurb`) is wired correctly end to end.
#[tokio::test]
async fn place_history_resolves_by_window_and_is_deterministic() {
    let app = app();

    // A place with no curated history at all -> `history` is entirely
    // absent from the response, window or not.
    let (st, body) = call(&app, "/api/place/jericho?from=-1406&to=-1405").await;
    assert_eq!(st, 200);
    assert!(body.get("history").is_none(), "{body}");

    // No window at all: history is still present (established is
    // window-independent) but with the default name and no blurb.
    let (st, body) = call(&app, "/api/place/hebron").await;
    assert_eq!(st, 200);
    assert_eq!(body["history"]["display_name"], "Hebron");
    assert!(body["history"]["blurb"].is_null(), "{body}");
    assert_eq!(body["history"]["established"]["when"]["from_year"], -2000);
    assert_eq!(body["history"]["established"]["verses"], serde_json::json!(["GEN.23.19"]));
    assert_eq!(body["history"]["established"]["note"], "traditional");
    assert!(body["history"]["destroyed"].is_null());

    // NAME-1: a window fully inside the curated "Kirjath-arba" range.
    let (st, body) = call(&app, "/api/place/hebron?from=-2500&to=-2200").await;
    assert_eq!(st, 200);
    assert_eq!(body["history"]["display_name"], "Kirjath-arba");

    // NAME-1: the exact boundary years pinned exhaustively -- -2001 is the
    // curated range's own last "Kirjath-arba" year, -2000 is the first
    // "Hebron" (default-fallback) year.
    let (st, body) = call(&app, "/api/place/hebron?from=-2001&to=-2001").await;
    assert_eq!(body["history"]["display_name"], "Kirjath-arba");
    let (st2, body2) = call(&app, "/api/place/hebron?from=-2000&to=-2000").await;
    assert_eq!(st, 200);
    assert_eq!(st2, 200);
    assert_eq!(body2["history"]["display_name"], "Hebron");

    // BLURB-1: a window inside the curated "era" blurb range shows it;
    // a window outside every curated blurb range shows none. established
    // is window-independent -- still -2000 under BOTH windows, even the
    // one that resolves a totally different display_name/blurb.
    let (st, blurb_hit) = call(&app, "/api/place/hebron?from=-2100&to=-2000").await;
    assert_eq!(st, 200);
    assert_eq!(blurb_hit["history"]["blurb"], "Abraham buried Sarah in the cave of Machpelah here.");
    assert_eq!(blurb_hit["history"]["established"]["when"]["from_year"], -2000);

    let (st, blurb_miss) = call(&app, "/api/place/hebron?from=1&to=50").await;
    assert_eq!(st, 200);
    assert!(blurb_miss["history"]["blurb"].is_null(), "{blurb_miss}");
    assert_eq!(blurb_miss["history"]["established"]["when"]["from_year"], -2000);

    // Determinism: repeating the exact same request yields the exact same body.
    let (_, again) = call(&app, "/api/place/hebron?from=-2100&to=-2000").await;
    let (_, repeat) = call(&app, "/api/place/hebron?from=-2100&to=-2000").await;
    assert_eq!(again, repeat);
}

#[tokio::test]
async fn polities_empty_fixture_shape_and_errors() {
    let app = app();

    // demo_fixture() has no compiled polities -- a valid window still 200s,
    // with an empty `polities` array (mirrors scene_time's "out-of-span is
    // not an error" spirit one level up: "no polity data at all" is a valid
    // state, not a failure).
    let (st, body) = call(&app, "/api/polities?from=-1450&to=-1400").await;
    assert_eq!(st, 200);
    assert_eq!(body["polities"], serde_json::json!([]));

    // Same bad_window rulings as /api/scene: missing/unparseable/zero/inverted.
    for bad in [
        "/api/polities?from=0&to=5",
        "/api/polities?from=5&to=-5",
        "/api/polities?from=1",
        "/api/polities",
        "/api/polities?from=x&to=y",
        "/api/polities?from=&to=",
    ] {
        let (st, body) = call(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_window", "{bad}: {body}");
    }

    // Ruling 2 parity: a structurally valid but wildly out-of-span window
    // is NOT an error, same as /api/scene.
    let (st, body) = call(&app, "/api/polities?from=-50000&to=50000").await;
    assert_eq!(st, 200);
    assert_eq!(body["polities"], serde_json::json!([]));
}

fn square_ring() -> Vec<(f64, f64)> {
    vec![(10.0, 10.0), (10.0, 11.0), (11.0, 11.0), (11.0, 10.0), (10.0, 10.0)]
}

/// A tiny `AtlasData` carrying real, hand-built polities -- `demo_fixture()`
/// plus an overwritten `polities` field, `.finish()`d again (idempotent per
/// its own doc comment) so the derived indexes stay consistent. THREE
/// polities: `egypt` with TWO non-overlapping eras (renamed across them,
/// "Egypt" -> "Ptolemaic Egypt", mirroring the real curated data -- the
/// exact shape `color_key`'s own "stable across a rename" guarantee needs a
/// live test for), `judah` (fix round 1, M1: a third id, so the "all
/// distinct" assertion below has more than two rows to actually check), and
/// `rome` with one era, so ordering ("by id then from") has more than one
/// id to actually sort.
///
/// Fix round 1 (M1): color_key here is still hand-set on the fixture
/// directly (3/9/5) rather than routed through the real
/// `atlas_etl::polities::assign_color_keys` -- this test suite (atlas-
/// server) builds its own `AtlasData` fixtures straight from `atlas_core`
/// structs and has no ETL dependency to call that function with. The actual
/// collision-free ALGORITHM is unit-tested directly where it lives
/// (`server/atlas-etl/src/polities.rs`'s own
/// `assign_color_keys_is_collision_free_for_the_real_curated_roster` and
/// its sibling tests). What THIS test proves, honestly, is the wire-level
/// half of the property: `/api/polities` faithfully passes through
/// whatever `color_key` each polity already carries -- stably across a
/// single polity's own eras, and distinctly across different polities'
/// own ids -- without ever re-deriving or colliding them itself.
fn app_with_test_polities() -> axum::Router {
    let mut data = demo_fixture();
    data.polities = vec![
        Polity {
            id: "egypt".into(),
            color_key: 3,
            eras: vec![
                PolityEra { name: "Egypt".into(), from: -2100, to: -1200, ref_note: "fixture".into(), rings: vec![square_ring()] },
                PolityEra { name: "Ptolemaic Egypt".into(), from: -331, to: -30, ref_note: "fixture".into(), rings: vec![square_ring()] },
            ],
        },
        Polity {
            id: "judah".into(),
            color_key: 9,
            eras: vec![PolityEra { name: "Kingdom of Judah".into(), from: -900, to: -600, ref_note: "fixture".into(), rings: vec![square_ring()] }],
        },
        Polity {
            id: "rome".into(),
            color_key: 5,
            eras: vec![PolityEra { name: "Roman Empire".into(), from: -30, to: 100, ref_note: "fixture".into(), rings: vec![square_ring()] }],
        },
    ];
    let data: AtlasData = data.finish();
    atlas_server::app::build(Arc::new(data), None)
}

#[tokio::test]
async fn polities_intersection_ordering_and_color_key_stability() {
    let app = app_with_test_polities();

    // A window inside egypt's FIRST era only -- exactly one row; neither
    // judah (era starts at -900, well after this window ends) nor rome
    // (whose only era starts at -30) appears.
    let (st, body) = call(&app, "/api/polities?from=-2000&to=-1900").await;
    assert_eq!(st, 200);
    let polities = body["polities"].as_array().unwrap();
    assert_eq!(polities.len(), 1, "{polities:?}");
    assert_eq!(polities[0]["id"], "egypt");
    assert_eq!(polities[0]["name"], "Egypt");
    assert_eq!(polities[0]["color_key"], 3);

    // A window spanning BOTH of egypt's eras plus judah's and rome's own --
    // four rows, deterministically ordered "by id then from": egypt's OLDER
    // era first, egypt's newer era second, judah third ("judah" sorts
    // between "egypt" and "rome"), rome last.
    let (st, body) = call(&app, "/api/polities?from=-1500&to=50").await;
    assert_eq!(st, 200);
    let polities = body["polities"].as_array().unwrap();
    assert_eq!(polities.len(), 4, "{polities:?}");
    assert_eq!(polities[0]["id"], "egypt");
    assert_eq!(polities[0]["name"], "Egypt");
    assert_eq!(polities[0]["from"], -2100);
    assert_eq!(polities[1]["id"], "egypt");
    assert_eq!(polities[1]["name"], "Ptolemaic Egypt");
    assert_eq!(polities[1]["from"], -331);
    assert_eq!(polities[2]["id"], "judah");
    assert_eq!(polities[2]["from"], -900);
    assert_eq!(polities[3]["id"], "rome");

    // color_key stability across eras of the SAME polity: both egypt rows
    // above carry the SAME color_key (3) despite their different era NAMES
    // -- the exact property the batch brief asks for ("hash the polity ID,
    // not the era name").
    assert_eq!(polities[0]["color_key"], 3);
    assert_eq!(polities[1]["color_key"], 3);
    assert_eq!(polities[0]["color_key"], polities[1]["color_key"]);

    // Fix round 1 (M1 -- review finding B2: "update the color-stability
    // property test to also assert all-distinct"). Among the DIFFERENT
    // polities visible together in this same window, every color_key is
    // pairwise distinct -- egypt/judah/rome never share a tint. This is the
    // wire-level half of the "no two polities share a tint" property --
    // see app_with_test_polities's own doc comment for where the actual
    // collision-free ASSIGNMENT algorithm is unit-tested instead. Grouped
    // by (id, color_key) first so repeated era rows for the SAME id (egypt
    // appears twice above) count once, not twice.
    let distinct_id_key_pairs: std::collections::HashSet<(&str, i64)> =
        polities.iter().map(|p| (p["id"].as_str().unwrap(), p["color_key"].as_i64().unwrap())).collect();
    let distinct_keys: std::collections::HashSet<i64> = distinct_id_key_pairs.iter().map(|&(_, k)| k).collect();
    assert_eq!(
        distinct_id_key_pairs.len(),
        distinct_keys.len(),
        "expected every DIFFERENT polity id in this response to carry a DISTINCT color_key; got {polities:?}"
    );

    // A window touching none of the three polities -- empty, not an error.
    let (st, body) = call(&app, "/api/polities?from=-4004&to=-2500").await;
    assert_eq!(st, 200);
    assert_eq!(body["polities"], serde_json::json!([]));

    // Rings travel through byte-identical to how they're stored -- [lat, lon]
    // pairs, never transposed.
    let (_, body) = call(&app, "/api/polities?from=-2000&to=-1900").await;
    assert_eq!(body["polities"][0]["rings"][0][0], serde_json::json!([10.0, 10.0]));
}

#[tokio::test]
async fn landmarks_empty_fixture_list() {
    let app = app();
    let (st, body) = call(&app, "/api/landmarks").await;
    assert_eq!(st, 200);
    assert_eq!(body, serde_json::json!([]));
}

/// `app::build`'s `static_dir` branch: API routes still win over the static
/// fallback, an existing file is served as itself, and an unmatched path
/// (a Blazor client-side route like `/world` or `/read/EXO/14`) falls back
/// to `index.html`'s content WITH a 200 status — tower-http's
/// `not_found_service` SPA idiom natively tags that fallback response 404
/// (correct from `ServeDir`'s point of view, wrong from the SPA's: a deep
/// link to a real client-side route is not an error), and `app::build`'s
/// `.map_response` rewrite is what corrects it; this test pins that fix so
/// it can't silently regress. Also asserts the permissive-CORS layer
/// reaches BOTH an `/api`-routed response and a fallback-served response:
/// `Router::layer` only wraps routes/fallback that exist at the time it's
/// called, so if `.layer(CorsLayer::permissive())` were ever reordered to
/// run before `.fallback_service(...)` is attached, the fallback branch
/// would silently lose its CORS headers while `/api/*` still had them —
/// this pins the correct (CORS-applied-last) ordering.
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

    for deep_link in ["/world", "/read/EXO/14"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(deep_link).header("origin", "http://example.com").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "a SPA deep link ({deep_link}) must fall back to index.html with a 200, not tower-http's native 404 — \
             a client (or curl) that checks status instead of sniffing the body must not see a false failure"
        );
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN).map(|v| v.to_str().unwrap()),
            Some("*"),
            "a fallback-served (SPA-route) response ({deep_link}) must ALSO carry permissive CORS headers, \
             not just direct /api routes — this is the case the layer-ordering bug broke"
        );
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            &bytes[..],
            b"<html>shell</html>",
            "unmatched client-side route {deep_link} must fall back to index.html's body"
        );
    }

    std::fs::remove_dir_all(&dir).ok();
}
