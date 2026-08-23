//! Endpoint tests against `atlas_core::data::demo_fixture()` via
//! `tower::ServiceExt::oneshot` — no real server binds a socket here.

use std::collections::HashMap;
use std::sync::Arc;

use atlas_core::data::{demo_fixture, AtlasData, Canon, Event, EventWitness, Polity, PolityEra};
use atlas_core::time::TimeRange;
use axum::body::Body;
use axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

// Batch M-A: `/api/chapter` now sources its verse TEXT from the graph (see
// handlers::chapter's own doc comment) while places/headings stay on
// `AtlasData` -- so a test fixture's graph service MUST agree with whatever
// `AtlasData` the same `app()` call uses about which chapters exist, or the
// chapter endpoint silently comes back empty (a real drift bug this exact
// helper hit once, in this batch's own development). `graph_fixture_for`
// derives the graph from `data`'s OWN `canon`/`verses` fields
// (`atlas_graph::GraphService::from_canon_and_verses`) so there is exactly
// ONE source of truth per test, never two independently hand-authored
// fixtures that can drift apart. No cross-references in this shared
// fixture (`""` xrefs source, valid input -- an empty table, not an
// error) -- tests that need real `cites` edges build their own richer
// graph (see `tests/graph_api.rs`, `tests/graph_equivalence.rs`).
// `GraphService` wraps the owner-approved `atlas_graph_types::store` port
// (fix round 1, C1) -- see `app.rs`'s own doc comment for why it's held
// concretely (not `Arc<dyn ...>`) in `AppState`.
// Batch M-C: `/api/eras`/`/api/polities` now serve from the graph's own
// Era/Polity nodes (era_adapter/polity_adapter), not `AtlasData.eras`/
// `.polities` directly -- `from_canon_and_verses_with_eras` carries
// `data.eras` through so a fixture's own eras (`demo_fixture()`'s two
// real entries) actually reach the graph, same "one source of truth per
// test" reasoning this helper's own doc comment above already states for
// canon/verses. `data.polities` needs no such threading -- `polity_adapter`
// already reads `AtlasData.polities` directly (that source stands, per
// the deletion inventory), which `graph_fixture_for`'s own `data: &AtlasData`
// parameter already supplies.
fn graph_fixture_for(data: &AtlasData) -> Arc<atlas_graph::GraphService> {
    Arc::new(
        atlas_graph::GraphService::from_canon_and_verses_with_eras(&data.canon, &data.verses, "From Verse\tTo Verse\tVotes\t#comment\n", data, &data.eras)
            .expect("fixture graph must build from this AtlasData's own canon+verses"),
    )
}

fn graph_fixture() -> Arc<atlas_graph::GraphService> {
    graph_fixture_for(&demo_fixture())
}

fn app() -> axum::Router {
    let data = demo_fixture();
    let graph = graph_fixture_for(&data);
    atlas_server::app::build(Arc::new(data), graph, None)
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
    // Batch R requirement 5 (place-in-verse hover -> marker blink): `places`
    // is ALWAYS present (never an omitted key), empty here since none of
    // JOS.1's own fixture verses are in any place's `verse_links`. Real
    // resolution (a verse that DOES have a linked place) is covered by
    // atlas-core's own `places_for_verse` unit test -- see data.rs -- since
    // exercising it here would mean growing demo_fixture()'s own GEN canon
    // entry (currently a deliberately tiny `chapters: vec![31]`, asserted
    // verbatim by health_books_eras_narratives_shapes above) just to reach a
    // chapter 13 the chapter handler can serve at all.
    assert_eq!(verses[0]["places"], serde_json::json!([]));

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

    // Batch T requirement 3 ("verse popover: event membership replaces
    // prev/next"): `VerseDetailOut.narrative_positions` is RETIRED --
    // JOS.6.20's own chronological PRIOR (e2)/FOLLOWING (e4) now live
    // entirely on the EVENT node (`GET /api/narrative/event/e3`, see
    // `narrative_event_positions_endpoint`), never on the verse response.
    // The verse's own "EVENT" membership (naming e3, "Jericho falls," as
    // explorable) is exactly the PRE-EXISTING `events` field already
    // asserted above (`events[0]`) -- no new wire field needed for that
    // half, so there is nothing further to assert here.

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

    // Batch F: JOS.6.20 ALSO carries demo_fixture()'s own small catechism
    // citation ("demo-item-1") -- proving cross_refs and catechism coexist
    // on the same verse detail response without either disturbing the other
    // (see demo_fixture()'s own comment for why this exact verse was
    // deliberately reused rather than an unrelated one). This is an
    // ITEM-level citation (demo-item-1's own `verses`) -- no `question` key
    // at all (omitted, not null, per CatechismRefOut's own
    // skip_serializing_if convention).
    let catechism = body["catechism"].as_array().unwrap();
    assert_eq!(catechism.len(), 1, "{body}");
    assert_eq!(catechism[0]["id"], "demo-item-1");
    assert_eq!(catechism[0]["name"], "Demo Catechism Item");
    assert!(catechism[0].get("question").is_none(), "{body}");

    // Batch F2: JOS.6.21 carries a QUESTION-level citation instead (demo-item-1's
    // own `questions[0]`, see demo_fixture()'s own comment) -- same item id,
    // but `question` is now present, naming the question title.
    let (st, body) = call(&app, "/api/verse/JOS.6.21").await;
    assert_eq!(st, 200);
    let catechism = body["catechism"].as_array().unwrap();
    assert_eq!(catechism.len(), 1, "{body}");
    assert_eq!(catechism[0]["id"], "demo-item-1");
    assert_eq!(catechism[0]["question"], "Demo Question");

    // A verse with zero catechism citations of EITHER kind still carries the
    // key, empty (always-an-array wire convention, same as `places` on
    // ChapterOut/VerseOut).
    let (st, body) = call(&app, "/api/verse/JOS.6.24").await;
    assert_eq!(st, 200);
    assert_eq!(body["catechism"], serde_json::json!([]));

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

    // --- /api/event (Batch T requirement 4: the EVENT node's own rich fetch) ---
    let (st, body) = call(&app, "/api/event/e3").await;
    assert_eq!(st, 200);
    assert_eq!(body["id"], "e3");
    assert_eq!(body["title"], "Jericho falls");
    assert_eq!(body["when"]["from_year"], -1405);
    assert_eq!(body["places"], serde_json::json!([{"id": "jericho", "name": "Jericho"}]));
    // No curated `witnesses` for e3 (demo_fixture's own Task-3-era events
    // predate this batch) -- the single-implicit-witness synthesis
    // (`scene::witnesses_for`) still resolves exactly ONE witness, grouped
    // from e3's own `verses` (all JOS.6, one book) -- requirement 4's own
    // "single-witness events show the one passage" case.
    let witnesses = body["witnesses"].as_array().unwrap();
    assert_eq!(witnesses.len(), 1, "{body}");
    assert_eq!(witnesses[0]["book"], "JOS");
    assert!(body.get("robertson_section").is_none(), "{body}");

    let (st, body) = call(&app, "/api/event/does-not-exist").await;
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

/// Batch T2 (general-kind PASSAGEs -- requirement 2's own "otherwise it
/// ships general-kind... title + witnesses, still explorable, still
/// headed in the reader"): `GET /api/event/{id}` must never claim a
/// fabricated date/place for a `kind == "general"` passage -- `when` is
/// OMITTED entirely (not null; matches this app's own "absent key, not a
/// null value" conditional-presence convention, e.g. `robertson_section`/
/// `ref_note` already documented this way in CONTRACT.md) and `places` is
/// an empty array, while an ordinary `kind == "event"` passage keeps
/// carrying `when` exactly as before.
#[tokio::test]
async fn event_endpoint_omits_when_for_general_kind_passages() {
    let mut verses = HashMap::new();
    verses.insert("LUK.1.1".to_string(), "Forasmuch as many have taken in hand...".to_string());
    let events = vec![
        Event {
            id: "g1".into(),
            label: "Luke's preface".into(),
            when: TimeRange::undated(),
            places: vec![],
            verses: vec!["LUK.1.1".into()],
            kind: "general".into(),
            robertson_section: Some("Robertson (1922) §1".into()),
            ..Default::default()
        },
        Event {
            id: "e1".into(),
            label: "An ordinary event-kind passage".into(),
            when: TimeRange::new(-1406, -1406).unwrap(),
            places: vec![],
            verses: vec![],
            ..Default::default()
        },
    ];
    let data = AtlasData::new(Canon { books: vec![] }, vec![], events, vec![], vec![], vec![], verses, HashMap::new()).finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_server::app::build(Arc::new(data), graph, None);

    let (st, body) = call(&app, "/api/event/g1").await;
    assert_eq!(st, 200);
    assert_eq!(body["kind"], "general");
    assert!(body.get("when").is_none(), "general-kind passage must not carry a `when` key at all: {body}");
    assert_eq!(body["places"], serde_json::json!([]));

    let (st, body) = call(&app, "/api/event/e1").await;
    assert_eq!(st, 200);
    assert_eq!(body["kind"], "event");
    assert_eq!(body["when"]["from_year"], -1406);

    // Batch HOTFIX-4 requirement 2 ("general-kind containers: NOT part of
    // time traversal... fabricating one is forbidden"): g1's own
    // `/api/narrative/event/{id}` response carries NO `timeline` key at
    // all (omitted, not null/empty) -- it is a leg of no narrative either,
    // so `narrative` is an empty array, same "no results" shape as any
    // other narrative-less event.
    let (st, body) = call(&app, "/api/narrative/event/g1").await;
    assert_eq!(st, 200);
    assert_eq!(body["narrative"], serde_json::json!([]));
    assert!(body.get("timeline").is_none(), "a general-kind passage must carry no `timeline` key at all: {body}");

    // e1 (real, dated, but ALSO a leg of no narrative here) still gets a
    // real timeline position -- the true first AND last dated event in
    // this tiny two-event fixture, so BOTH prior and following are absent
    // (conditional presence, not a disabled stub), but the `timeline` KEY
    // itself is present (unlike g1 above) -- proving the two "absent"
    // shapes (whole-field-omitted vs. both-sides-omitted) are genuinely
    // different and both correctly realized.
    let (st, body) = call(&app, "/api/narrative/event/e1").await;
    assert_eq!(st, 200);
    assert!(body.get("timeline").is_some(), "e1 is dated -- `timeline` must be present: {body}");
    assert!(body["timeline"].get("prior").is_none());
    assert!(body["timeline"].get("following").is_none());
}

/// Fix round 1 (I-1): a `kind == "general"` passage's own `event-places` row
/// must resolve the alias/default tier, NEVER a spurious curated period
/// name -- `e.when` is `TimeRange::undated()` for a general-kind passage
/// (the whole atlas span, [-4004,100]), which trivially intersects
/// `demo_fixture()`'s own curated `hebron` history (`Kirjath-arba`,
/// [-4004,-2001] -- entirely inside `undated()`'s span) if it's ever passed
/// through as a real window. Pre-fix, this test's own "g-hebron" event
/// would have shown "Kirjath-arba" here (a fabricated period claim for a
/// passage that structurally has no date at all); post-fix it must show the
/// plain default "Hebron" -- hebron carries no curated KJV alias in this
/// fixture, so the resolution falls all the way to the bottom tier.
#[tokio::test]
async fn general_kind_event_places_never_resolve_a_spurious_period_name() {
    let mut data = demo_fixture();
    data.events.push(Event {
        id: "g-hebron".into(),
        label: "A general-kind passage mentioning Hebron".into(),
        when: TimeRange::undated(),
        places: vec!["hebron".into()],
        verses: vec![],
        kind: "general".into(),
        ..Default::default()
    });
    let data = data.finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_server::app::build(Arc::new(data), graph, None);

    let (st, body) = call(&app, "/api/event/g-hebron").await;
    assert_eq!(st, 200);
    assert_eq!(body["kind"], "general");
    assert!(body.get("when").is_none(), "general-kind passage must not carry a `when` key: {body}");
    assert_eq!(
        body["places"],
        serde_json::json!([{"id": "hebron", "name": "Hebron"}]),
        "must resolve the plain default, NOT the curated period name \"Kirjath-arba\" that undated()'s [-4004,100] span would spuriously intersect: {body}"
    );

    // Control: the SAME place, on a REAL event-kind passage whose own date
    // genuinely falls inside the curated range, still correctly resolves
    // the period name -- proving the fix is a kind-gate, not a blanket
    // "never resolve period names on event-places" regression.
    data_hebron_period_name_still_resolves_for_a_real_event_kind_window().await;
}

/// Helper for the test above: confirms the kind-gate fix didn't collaterally
/// break the case it must keep working -- a genuine `kind == "event"`
/// passage dated inside a curated period-name range still resolves that
/// name on its own `event-places` row.
async fn data_hebron_period_name_still_resolves_for_a_real_event_kind_window() {
    let mut data = demo_fixture();
    data.events.push(Event {
        id: "e-hebron-period".into(),
        label: "A real event-kind passage, dated inside Kirjath-arba's range".into(),
        when: TimeRange::new(-2500, -2500).unwrap(),
        places: vec!["hebron".into()],
        verses: vec![],
        ..Default::default()
    });
    let data = data.finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_server::app::build(Arc::new(data), graph, None);

    let (st, body) = call(&app, "/api/event/e-hebron-period").await;
    assert_eq!(st, 200);
    assert_eq!(body["kind"], "event");
    assert_eq!(body["when"]["from_year"], -2500);
    assert_eq!(body["places"], serde_json::json!([{"id": "hebron", "name": "Kirjath-arba"}]), "{body}");
}

/// Batch T2 (Acts provenance): `acts_section` is Acts's own sibling
/// provenance field to `robertson_section` (owner's own ambiguity ruling --
/// "acts sections get their own provenance key, NOT robertson_section").
/// `GET /api/event/{id}` must carry it, omitted (not null) when absent,
/// exactly like `robertson_section` already is.
#[tokio::test]
async fn event_endpoint_carries_acts_section_when_present() {
    let events = vec![
        Event {
            id: "a1".into(),
            label: "Peter preaches at Pentecost".into(),
            when: TimeRange::new(30, 30).unwrap(),
            places: vec![],
            verses: vec![],
            acts_section: Some("Acts pericope (this project's own sectioning): Acts 2:14-41".into()),
            ..Default::default()
        },
        Event { id: "a2".into(), label: "No Acts provenance".into(), when: TimeRange::new(30, 30).unwrap(), places: vec![], verses: vec![], ..Default::default() },
    ];
    let data = AtlasData::new(Canon { books: vec![] }, vec![], events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_server::app::build(Arc::new(data), graph, None);

    let (st, body) = call(&app, "/api/event/a1").await;
    assert_eq!(st, 200);
    assert_eq!(body["acts_section"], "Acts pericope (this project's own sectioning): Acts 2:14-41");

    let (st, body) = call(&app, "/api/event/a2").await;
    assert_eq!(st, 200);
    assert!(body.get("acts_section").is_none(), "acts_section must be omitted, not null, when absent: {body}");
}

/// Batch W3 (fix round 1, Minor-2, batch-w3-review.md): `kjv_superscription`
/// is the KJV's own literal-citation sibling to `robertson_section`/
/// `acts_section`/`atlas_section` (see `atlas_core::data::Event::
/// kjv_superscription`'s own doc comment). `GET /api/event/{id}` must carry
/// it, omitted (not null) when absent, exactly like its three siblings --
/// mirrors `event_endpoint_carries_acts_section_when_present` immediately
/// above, the same dedicated wire-level regression its three siblings
/// already have (or, for `robertson_section`/`atlas_section`, are only ever
/// incidentally exercised without their own dedicated positive-value
/// assertion -- this test closes that same gap for `kjv_superscription`
/// specifically, per the review's own finding).
#[tokio::test]
async fn event_endpoint_carries_kjv_superscription_when_present() {
    let events = vec![
        Event {
            id: "k1".into(),
            label: "A Psalm of David, when he fled from Absalom his son.".into(),
            when: TimeRange::undated(),
            places: vec![],
            verses: vec![],
            kind: "general".into(),
            kjv_superscription: Some("PSA.3.1, the psalm's own KJV superscription, quoted verbatim".into()),
            ..Default::default()
        },
        Event { id: "k2".into(), label: "No KJV-superscription provenance".into(), when: TimeRange::undated(), places: vec![], verses: vec![], kind: "general".into(), ..Default::default() },
    ];
    let data = AtlasData::new(Canon { books: vec![] }, vec![], events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_server::app::build(Arc::new(data), graph, None);

    let (st, body) = call(&app, "/api/event/k1").await;
    assert_eq!(st, 200);
    assert_eq!(body["kjv_superscription"], "PSA.3.1, the psalm's own KJV superscription, quoted verbatim");

    let (st, body) = call(&app, "/api/event/k2").await;
    assert_eq!(st, 200);
    assert!(body.get("kjv_superscription").is_none(), "kjv_superscription must be omitted, not null, when absent: {body}");
}

/// Batch W1 requirement 1b's own MODEL GENERALIZATION ("W1 implements
/// before authoring at scale... a general-kind container may carry witness
/// rows and its popover shows PARALLEL ACCOUNTS identically"). Proves the
/// wire mechanism end-to-end for a `kind == "general"` passage with TWO
/// witnesses -- the exact shape a reader's own EVENT popover renders as
/// "PARALLEL ACCOUNTS" (`EventWitnessesSection`, client-side, keyed only
/// off `witnesses.len() >= 2`, never off `kind`). Nothing in this
/// codebase's own witness-resolution path (`scene::witnesses_for`,
/// `handlers::event`) branches on `kind` at all -- this test is the
/// live, wire-level proof of that, not merely an inspection of the source.
#[tokio::test]
async fn event_endpoint_general_kind_with_multiple_witnesses_shows_parallel_accounts() {
    let mut verses = HashMap::new();
    verses.insert("EXO.20.1".to_string(), "And God spake all these words, saying,".to_string());
    verses.insert("DEU.5.6".to_string(), "I am the LORD thy God...".to_string());
    let events = vec![Event {
        id: "g_general_witnessed".into(),
        label: "A general-kind passage with parallel accounts".into(),
        when: TimeRange::undated(),
        places: vec![],
        verses: vec!["EXO.20.1".into()],
        kind: "general".into(),
        atlas_section: Some("test fixture".into()),
        witnesses: vec![
            EventWitness {
                book: "EXO".into(),
                translations: HashMap::from([("kjv".to_string(), vec!["EXO.20.1".to_string()])]),
                ref_note: None,
                robertson_section: None,
            },
            EventWitness {
                book: "DEU".into(),
                translations: HashMap::from([("kjv".to_string(), vec!["DEU.5.6".to_string()])]),
                ref_note: None,
                robertson_section: None,
            },
        ],
        ..Default::default()
    }];
    let data = AtlasData::new(Canon { books: vec![] }, vec![], events, vec![], vec![], vec![], verses, HashMap::new()).finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_server::app::build(Arc::new(data), graph, None);

    let (st, body) = call(&app, "/api/event/g_general_witnessed").await;
    assert_eq!(st, 200);
    assert_eq!(body["kind"], "general");
    assert!(body.get("when").is_none(), "general-kind passage must not carry a `when` key, even with witnesses: {body}");
    let witnesses = body["witnesses"].as_array().expect("witnesses array");
    assert_eq!(witnesses.len(), 2, "a general-kind passage's own witnesses must resolve identically to an event-kind one's: {body}");
    let books: Vec<&str> = witnesses.iter().map(|w| w["book"].as_str().unwrap()).collect();
    assert!(books.contains(&"EXO") && books.contains(&"DEU"), "both witness books must resolve: {body}");
}

/// Batch N ("narratives as first-class graph structure"), retired
/// verse-keyed half per Batch T requirement 3 ("verse popover: event
/// membership replaces prev/next" -- `VerseDetailOut.narrative_positions`
/// is GONE; see `verse_chapter_place_and_404`'s own updated assertions for
/// the verse popover's new "EVENT" membership, which reads the
/// pre-existing `events` field instead). The event-id-keyed endpoint itself
/// (`GET /api/narrative/event/{id}`) is UNCHANGED plumbing -- still the
/// EVENT node's own PRIOR/FOLLOWING source (`GET /api/event/{id}` is a
/// separate, richer fetch for the rest of that node's own content) -- this
/// test now exercises it alone, end to end through the real HTTP handler.
#[tokio::test]
async fn narrative_event_positions_endpoint() {
    let app = app();

    // e2 ("Jericho besieged") is a leg of BOTH conquest (mid-chain:
    // prior=e1, following=e3) AND patriarchs-demo (its OWN only leg:
    // neither) -- both positions come back from ONE event-id-keyed call,
    // "an event in multiple narratives returns all positions."
    let (st, body) = call(&app, "/api/narrative/event/e2").await;
    assert_eq!(st, 200);
    let positions = body["narrative"].as_array().unwrap().clone();
    assert_eq!(positions.len(), 2, "{body}");

    let conquest = positions.iter().find(|p| p["narrative_id"] == "conquest").expect("e2 is a conquest leg");
    assert_eq!(conquest["event_id"], "e2");
    assert_eq!(conquest["event_label"], "Jericho besieged");
    assert_eq!(conquest["prior"]["id"], "e1");
    assert_eq!(conquest["prior"]["label"], "Camp at Gilgal");
    assert_eq!(conquest["following"]["id"], "e3");

    let patriarchs = positions.iter().find(|p| p["narrative_id"] == "patriarchs-demo").expect("e2 is patriarchs-demo's own leg");
    assert_eq!(patriarchs["narrative_name"], "Patriarchs (demo)");
    // Conditional presence, no disabled stub: patriarchs-demo's ONLY leg has
    // neither a prior nor a following -- the keys are ABSENT, not null.
    assert!(patriarchs.get("prior").is_none(), "{patriarchs}");
    assert!(patriarchs.get("following").is_none(), "{patriarchs}");

    // Batch HOTFIX-4 requirement 1: e2's own GLOBAL TIMELINE position
    // (independent of, alongside, the narrative rows above) -- demo_fixture's
    // own chronological order is e5, e1, e2, e3, e4, so e2's own timeline
    // neighbors happen to coincide with its conquest-narrative ones here,
    // but come from the SEPARATE `timeline` field, not `narrative`.
    assert_eq!(body["timeline"]["prior"]["id"], "e1");
    assert_eq!(body["timeline"]["following"]["id"], "e3");

    // A real event that is a leg of NO narrative -- `narrative` is an empty
    // array (the "no results" case), not a 404 (the identifier itself is
    // real) -- but Batch HOTFIX-4 requirement 1's own point: it STILL gets a
    // real `timeline` position (e5 is demo_fixture's own chronologically
    // EARLIEST event -- no prior, following = e1), proving traversal no
    // longer depends on narrative membership at all.
    let (st, body) = call(&app, "/api/narrative/event/e5").await;
    assert_eq!(st, 200);
    assert_eq!(body["narrative"], serde_json::json!([]));
    assert!(body["timeline"].get("prior").is_none(), "e5 is the fixture's true first dated event -- no prior: {body}");
    assert_eq!(body["timeline"]["following"]["id"], "e1");

    // An id naming no real event at all -- 404 not_found (the "bad
    // identifier" case), same precedent as /api/place/{id} and
    // /api/catechism/item/{id}.
    let (st, body) = call(&app, "/api/narrative/event/does-not-exist").await;
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

/// Batch G1: `GET /api/xrefs/{sref}` end to end, against `demo_fixture()`'s
/// own JOS.6.20/JOS.6.21 cross-ref entries (see data.rs's own comment on the
/// JOS.6.21 addition for exactly what this proves and why).
#[tokio::test]
async fn xrefs_span_aggregation_ok_and_bad_ref() {
    let app = app();

    // A single-verse span is just that one verse's own list, minus nothing
    // (JOS.1.3/GEN.13.18 aren't self-targets of a 1-verse span) -- same
    // shape/order /api/verse/JOS.6.20's own cross_refs already proves.
    let (st, body) = call(&app, "/api/xrefs/JOS.6.20").await;
    assert_eq!(st, 200);
    let xrefs = body.as_array().unwrap();
    assert_eq!(xrefs.len(), 3);
    assert_eq!(xrefs[0]["target"], "JOS.6.20-21");
    assert_eq!(xrefs[0]["votes"], 9);

    // The real span-aggregation case: JOS.6.20-21 unions both member
    // verses' own lists. JOS.6.20-21 (self, exactly the span) and JOS.6.20
    // (self, a member verse) are both dropped; JOS.1.3 is cited by BOTH
    // members and comes back SUMMED (5 + 4 = 9), not just the first hit;
    // GEN.13.18 survives untouched at 2. Sorted votes desc -> JOS.1.3 (9)
    // before GEN.13.18 (2).
    let (st, body) = call(&app, "/api/xrefs/JOS.6.20-21").await;
    assert_eq!(st, 200);
    let xrefs = body.as_array().unwrap();
    assert_eq!(xrefs.len(), 2, "{body}");
    assert_eq!(xrefs[0]["target"], "JOS.1.3");
    assert_eq!(xrefs[0]["votes"], 9);
    assert!(xrefs[0]["preview"].as_str().unwrap().contains("sole of your foot"));
    assert_eq!(xrefs[1]["target"], "GEN.13.18");
    assert_eq!(xrefs[1]["votes"], 2);

    // A structurally valid vref with no cross-refs recorded at all (never
    // itself a `From` key in the fixture) -- 200, gracefully empty, not 404
    // (ruling-3 policy: unlike /api/verse/{vref}, this endpoint never 404s).
    let (st, body) = call(&app, "/api/xrefs/GEN.13.18").await;
    assert_eq!(st, 200);
    assert_eq!(body, serde_json::json!([]));

    // Structurally invalid srefs -> 400 bad_ref, same typed error as every
    // other ref-shaped endpoint.
    for bad in ["/api/xrefs/NOPE.1.1", "/api/xrefs/GEN.0.1", "/api/xrefs/gen..1", "/api/xrefs/JOS.6.31-21"] {
        let (st, body) = call(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_ref", "{bad}: {body}");
    }

    // Book/Chapter-shaped refs are structurally valid ScriptureRefs but not
    // one of this endpoint's two accepted shapes (Verse/Passage) -- also
    // bad_ref (handlers::xrefs's own doc comment explains why).
    for bad in ["/api/xrefs/JOS", "/api/xrefs/JOS.6"] {
        let (st, body) = call(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_ref", "{bad}: {body}");
    }
}

/// Batch F ("the small catechism"): `GET /api/catechism/{sref}` and
/// `GET /api/catechism/item/{id}` end to end, against `demo_fixture()`'s own
/// "demo-item-1" (citing JOS.6.20, `where_written` present, `text` absent --
/// see that fixture's own comment). The rich multi-item union/dedup/order
/// behavior is already covered directly against
/// `atlas_core::catechism::items_for_span` (server/atlas-core/src/catechism.rs's
/// own unit tests) and `AtlasData::catechism_items_for_span`
/// (server/atlas-core/src/data.rs's own `catechism_tests` module) -- this
/// test's job, mirroring `xrefs_span_aggregation_ok_and_bad_ref`'s own
/// division of labor, is proving the HTTP plumbing (routing, ref-shape
/// gating, 404 vs. gracefully-empty) end to end, not re-proving the pure
/// aggregation logic a third time.
#[tokio::test]
async fn catechism_span_and_item_endpoints() {
    let app = app();

    // Single-verse span: JOS.6.20 cites demo-item-1.
    let (st, body) = call(&app, "/api/catechism/JOS.6.20").await;
    assert_eq!(st, 200);
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 1, "{body}");
    assert_eq!(items[0]["id"], "demo-item-1");
    assert_eq!(items[0]["name"], "Demo Catechism Item");

    // Passage span aggregation: JOS.6.20-21 unions member verses 20 (cites
    // demo-item-1 at the ITEM level, no question) and 21 (cites demo-item-1
    // AGAIN, but via a Batch F2 QUESTION-level citation, "Demo Question") --
    // the union surfaces BOTH rows (dedup is by (id, question), never id
    // alone -- see items_for_span's own doc comment), exactly the
    // "span/passage selections aggregate citing items the way xrefs already
    // aggregate" requirement, now extended to question granularity.
    let (st, body) = call(&app, "/api/catechism/JOS.6.20-21").await;
    assert_eq!(st, 200);
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 2, "{body}");
    assert_eq!(items[0]["id"], "demo-item-1");
    assert!(items[0].get("question").is_none(), "{body}");
    assert_eq!(items[1]["id"], "demo-item-1");
    assert_eq!(items[1]["question"], "Demo Question");

    // A structurally valid span with zero citing items -- 200, gracefully
    // empty, never a 404 (ruling-3 policy, same as /api/xrefs/{sref}).
    let (st, body) = call(&app, "/api/catechism/JOS.1.1").await;
    assert_eq!(st, 200);
    assert_eq!(body, serde_json::json!([]));

    // Structurally invalid srefs, and Book/Chapter-shaped refs (not one of
    // the two accepted Verse/Passage shapes) -> 400 bad_ref, same typed
    // error every other ref-shaped endpoint uses.
    for bad in ["/api/catechism/NOPE.1.1", "/api/catechism/gen..1", "/api/catechism/JOS", "/api/catechism/JOS.6"] {
        let (st, body) = call(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_ref", "{bad}: {body}");
    }

    // --- /api/catechism/item/{id} ------------------------------------------
    let (st, body) = call(&app, "/api/catechism/item/demo-item-1").await;
    assert_eq!(st, 200);
    assert_eq!(body["id"], "demo-item-1");
    assert_eq!(body["name"], "Demo Catechism Item");
    assert_eq!(body["part_title"], "Demo Part");
    // `text` is None on this fixture item (mirrors a real Baptism/Confession/
    // Sacrament-of-the-Altar item's own shape) -- omitted from the wire
    // entirely (skip_serializing_if), not present-as-null.
    assert!(body.get("text").is_none(), "{body}");
    assert_eq!(body["explanation_heading"], "What does this mean?");
    assert_eq!(body["explanation"], "Demo item explanation.");
    assert_eq!(body["where_written"], "Demo where-written text.");
    // Batch F2: THE SCRIPTURES now lists BOTH the item-level verse (JOS.6.20,
    // no `question`) and the question-level one (JOS.6.21, `question` =
    // "Demo Question") -- item-level first, per CatechismItemOut's own doc
    // comment ("items keep their F-batch embedded-citation links too,"
    // listed as the primary source).
    let verses = body["verses"].as_array().unwrap();
    assert_eq!(verses.len(), 2, "{body}");
    assert_eq!(verses[0]["vref"], "JOS.6.20");
    assert!(verses[0]["text"].as_str().unwrap().contains("wall fell down flat"));
    assert!(verses[0].get("question").is_none(), "{body}");
    assert_eq!(verses[1]["vref"], "JOS.6.21");
    assert_eq!(verses[1]["question"], "Demo Question");

    // Unknown item id -> 404 not_found, same exact-identifier precedent
    // `/api/place/{id}` already set.
    let (st, body) = call(&app, "/api/catechism/item/does-not-exist").await;
    assert_eq!(st, 404);
    assert_eq!(body["error"]["code"], "not_found");
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
                PolityEra { name: "Egypt".into(), from: -2100, to: -1200, ref_note: "fixture".into(), rings: vec![square_ring()], transition: None, fall: None },
                PolityEra { name: "Ptolemaic Egypt".into(), from: -331, to: -30, ref_note: "fixture".into(), rings: vec![square_ring()], transition: None, fall: None },
            ],
        },
        Polity {
            id: "judah".into(),
            color_key: 9,
            eras: vec![PolityEra { name: "Kingdom of Judah".into(), from: -900, to: -600, ref_note: "fixture".into(), rings: vec![square_ring()], transition: None, fall: None }],
        },
        Polity {
            id: "rome".into(),
            color_key: 5,
            eras: vec![PolityEra { name: "Roman Empire".into(), from: -30, to: 100, ref_note: "fixture".into(), rings: vec![square_ring()], transition: None, fall: None }],
        },
    ];
    let data: AtlasData = data.finish();
    let graph = graph_fixture_for(&data);
    atlas_server::app::build(Arc::new(data), graph, None)
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

/// Batch M requirement 1: `transition`/`fall` are OMITTED (not null) when a
/// curator honestly left an era boundary uneventful, and present with their
/// full shape (`event`/`verses`/`ref_note`) when authored -- the wire-level
/// half of "an uneventful boundary stays visible but gets the minimal
/// popover" (the CLIENT side of conditional presence lives in
/// `client.Tests`/Playwright; this only proves the server's own JSON never
/// emits a `null` placeholder either way).
#[tokio::test]
async fn polities_transition_and_fall_conditional_presence_on_the_wire() {
    let mut data = demo_fixture();
    data.polities = vec![Polity {
        id: "delta-test".into(),
        color_key: 0,
        eras: vec![
            // First era: no transition curated (an honest omission) --
            // must be ABSENT from the JSON, not `"transition": null`.
            PolityEra {
                name: "Rises Quietly".into(),
                from: -1000,
                to: -700,
                ref_note: "fixture".into(),
                rings: vec![square_ring()],
                transition: None,
                fall: None,
            },
            // Second (final) era: BOTH a transition (from the first era)
            // AND a fall (its own end) are curated.
            PolityEra {
                name: "Falls Dramatically".into(),
                from: -699,
                to: -600,
                ref_note: "fixture".into(),
                rings: vec![square_ring()],
                transition: Some(atlas_core::data::PolityDelta {
                    event: "Test event: the change happens".into(),
                    verses: vec!["GEN.1.1".into()],
                    ref_note: "fixture ref_note".into(),
                    for_era_from: -699,
                }),
                fall: Some(atlas_core::data::PolityDelta {
                    event: "Test event: the fall happens".into(),
                    verses: vec![],
                    ref_note: "fixture fall ref_note".into(),
                    for_era_from: -699,
                }),
            },
        ],
    }];
    let data: AtlasData = data.finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_server::app::build(Arc::new(data), graph, None);

    let (st, body) = call(&app, "/api/polities?from=-1000&to=-600").await;
    assert_eq!(st, 200);
    let polities = body["polities"].as_array().unwrap();
    assert_eq!(polities.len(), 2, "{polities:?}");

    let quiet = &polities[0];
    assert_eq!(quiet["name"], "Rises Quietly");
    assert!(!quiet.as_object().unwrap().contains_key("transition"), "expected NO transition key at all (omitted, not null): {quiet:?}");
    assert!(!quiet.as_object().unwrap().contains_key("fall"), "expected NO fall key at all (omitted, not null): {quiet:?}");

    let dramatic = &polities[1];
    assert_eq!(dramatic["name"], "Falls Dramatically");
    assert_eq!(dramatic["transition"]["event"], "Test event: the change happens");
    assert_eq!(dramatic["transition"]["verses"], serde_json::json!(["GEN.1.1"]));
    assert_eq!(dramatic["transition"]["ref_note"], "fixture ref_note");
    assert_eq!(dramatic["fall"]["event"], "Test event: the fall happens");
    assert_eq!(dramatic["fall"]["verses"], serde_json::json!([]));
    assert_eq!(dramatic["fall"]["ref_note"], "fixture fall ref_note");
}

#[tokio::test]
async fn landmarks_empty_fixture_list() {
    let app = app();
    let (st, body) = call(&app, "/api/landmarks").await;
    assert_eq!(st, 200);
    assert_eq!(body, serde_json::json!([]));
}

// Batch R requirement 1 ("borders become part of the plate"): GET
// /api/land-mask, same "empty on the fixture, real shape checked by the
// ETL's own validate::run_land_mask + curated::parse_land_mask tests"
// treatment as landmarks_empty_fixture_list above -- demo_fixture() carries
// no real coastline geometry (no server test needs one), so this just pins
// the wire shape (`{"rings": [...]}`, always present, empty here).
#[tokio::test]
async fn land_mask_empty_fixture_shape() {
    let app = app();
    let (st, body) = call(&app, "/api/land-mask").await;
    assert_eq!(st, 200);
    assert_eq!(body, serde_json::json!({ "rings": [] }));
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

    let app = atlas_server::app::build(Arc::new(demo_fixture()), graph_fixture(), Some(dir.clone()));

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
