use std::collections::HashMap;
use std::sync::Arc;

use atlas_core::data::{demo_fixture, AtlasData, Canon, Event, EventWitness, Person, Place, Polity, PolityEra};
use atlas_core::time::TimeRange;
use axum::body::Body;
use axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn graph_fixture_for(data: &AtlasData) -> Arc<atlas_graph::GraphService> {
    let xrefs_tsv = xrefs_tsv_from(data);
    Arc::new(
        atlas_graph::GraphService::from_canon_and_verses_with_eras(&data.canon, &data.verses, &xrefs_tsv, data, &data.eras)
            .expect("fixture graph must build from this AtlasData's own canon+verses+cross_refs"),
    )
}

fn xrefs_tsv_from(data: &AtlasData) -> String {
    let mut out = String::from("From Verse\tTo Verse\tVotes\t#comment\n");
    let mut froms: Vec<&String> = data.cross_refs.keys().collect();
    froms.sort();
    for from in froms {
        for cr in &data.cross_refs[from] {
            let to = match cr.target.rsplit_once('-') {
                Some((left, last_verse)) if last_verse.chars().all(|c| c.is_ascii_digit()) => {
                    match left.rsplit_once('.') {
                        Some((prefix, _first_verse)) => format!("{left}-{prefix}.{last_verse}"),
                        None => cr.target.clone(),
                    }
                }
                _ => cr.target.clone(),
            };
            out.push_str(&format!("{from}\t{to}\t{}\n", cr.votes));
        }
    }
    out
}

fn graph_fixture() -> Arc<atlas_graph::GraphService> {
    graph_fixture_for(&demo_fixture())
}

fn app() -> axum::Router {
    let data = demo_fixture();
    let graph = graph_fixture_for(&data);
    atlas_contract::app::build(Arc::new(data), graph, None)
}

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

#[tokio::test]
async fn scene_time_query_extraction_never_leaks_axum_rejection_body() {
    let app = app();
    for bad in ["/api/scene?from=x&to=y", "/api/scene", "/api/scene?to=5", "/api/scene?from=&to="] {
        let (st, body) = call(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_window", "{bad}: {body}");
        assert!(body["error"]["message"].is_string(), "{bad}: {body}");
    }
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
    assert_eq!(books[0]["chapters"], serde_json::json!([31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 18, 0, 0, 0, 0, 0, 0, 0, 0, 0, 19]));
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
async fn verse_chapter_event_and_404() {
    let app = app();

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
    assert_eq!(verses[0]["places"], serde_json::json!([]));
    assert_eq!(verses[0]["persons"], serde_json::json!([]));

    let (st, body) = call(&app, "/api/chapter/NOPE.1").await;
    assert_eq!(st, 400);
    assert_eq!(body["error"]["code"], "bad_ref");

    let (st, body) = call(&app, "/api/chapter/JOS").await;
    assert_eq!(st, 400);
    assert_eq!(body["error"]["code"], "bad_ref");

    let (st, body) = call(&app, "/api/chapter/JOS.2").await;
    assert_eq!(st, 200);
    assert_eq!(body["verses"].as_array().unwrap().len(), 0);

    let (st, _body) = call(&app, "/api/chapter/JOS.1?translation=kjv").await;
    assert_eq!(st, 200);

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

    let catechism = body["catechism"].as_array().unwrap();
    assert_eq!(catechism.len(), 1, "{body}");
    assert_eq!(catechism[0]["id"], "demo-item-1");
    assert_eq!(catechism[0]["name"], "Demo Catechism Item");
    assert!(catechism[0].get("question").is_none(), "{body}");

    let (st, body) = call(&app, "/api/verse/JOS.6.21").await;
    assert_eq!(st, 200);
    let catechism = body["catechism"].as_array().unwrap();
    assert_eq!(catechism.len(), 1, "{body}");
    assert_eq!(catechism[0]["id"], "demo-item-1");
    assert_eq!(catechism[0]["question"], "Demo Question");

    let (st, body) = call(&app, "/api/verse/JOS.6.24").await;
    assert_eq!(st, 200);
    assert_eq!(body["catechism"], serde_json::json!([]));

    let (st, body) = call(&app, "/api/verse/NOPE.1.1").await;
    assert_eq!(st, 400);
    assert_eq!(body["error"]["code"], "bad_ref");

    let (st, body) = call(&app, "/api/verse/JOS.6").await;
    assert_eq!(st, 400);
    assert_eq!(body["error"]["code"], "bad_ref");

    let (st, body) = call(&app, "/api/verse/GEN.1.1").await;
    assert_eq!(st, 404);
    assert_eq!(body["error"]["code"], "not_found");

    let (st, body) = call(&app, "/api/event/e3").await;
    assert_eq!(st, 200);
    assert_eq!(body["id"], "e3");
    assert_eq!(body["title"], "Jericho falls");
    assert_eq!(body["when"]["from_year"], -1405);
    assert_eq!(body["places"], serde_json::json!([{"id": "jericho", "name": "Jericho", "node": {"id": "Place:jericho", "kind": "Place", "label": "Jericho"}}]));
    let witnesses = body["witnesses"].as_array().unwrap();
    assert_eq!(witnesses.len(), 1, "{body}");
    assert_eq!(witnesses[0]["book"], "JOS");
    assert!(body.get("robertson_section").is_none(), "{body}");

    let (st, body) = call(&app, "/api/event/does-not-exist").await;
    assert_eq!(st, 404);
    assert_eq!(body["error"]["code"], "not_found");

}

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
            kind: atlas_core::data::EventKind::General,
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
    let app = atlas_contract::app::build(Arc::new(data), graph, None);

    let (st, body) = call(&app, "/api/event/g1").await;
    assert_eq!(st, 200);
    assert_eq!(body["kind"], "general");
    assert!(body.get("when").is_none(), "general-kind passage must not carry a `when` key at all: {body}");
    assert_eq!(body["places"], serde_json::json!([]));

    let (st, body) = call(&app, "/api/event/e1").await;
    assert_eq!(st, 200);
    assert_eq!(body["kind"], "event");
    assert_eq!(body["when"]["from_year"], -1406);

    let (st, body) = call(&app, "/api/narrative/event/g1").await;
    assert_eq!(st, 200);
    assert_eq!(body["narrative"], serde_json::json!([]));
    assert!(body.get("timeline").is_none(), "a general-kind passage must carry no `timeline` key at all: {body}");

    let (st, body) = call(&app, "/api/narrative/event/e1").await;
    assert_eq!(st, 200);
    assert!(body.get("timeline").is_some(), "e1 is dated -- `timeline` must be present: {body}");
    assert!(body["timeline"].get("prior").is_none());
    assert!(body["timeline"].get("following").is_none());
}

#[tokio::test]
async fn general_kind_event_places_never_resolve_a_spurious_period_name() {
    let mut data = demo_fixture();
    data.events.push(Event {
        id: "g-hebron".into(),
        label: "A general-kind passage mentioning Hebron".into(),
        when: TimeRange::undated(),
        places: vec!["hebron".into()],
        verses: vec![],
        kind: atlas_core::data::EventKind::General,
        ..Default::default()
    });
    let data = data.finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_contract::app::build(Arc::new(data), graph, None);

    let (st, body) = call(&app, "/api/event/g-hebron").await;
    assert_eq!(st, 200);
    assert_eq!(body["kind"], "general");
    assert!(body.get("when").is_none(), "general-kind passage must not carry a `when` key: {body}");
    assert_eq!(
        body["places"],
        serde_json::json!([{"id": "hebron", "name": "Hebron", "node": {"id": "Place:hebron", "kind": "Place", "label": "Hebron"}}]),
        "must resolve the plain default, NOT the curated period name \"Kirjath-arba\" that undated()'s [-4004,100] span would spuriously intersect: {body}"
    );

    data_hebron_period_name_still_resolves_for_a_real_event_kind_window().await;
}

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
    let app = atlas_contract::app::build(Arc::new(data), graph, None);

    let (st, body) = call(&app, "/api/event/e-hebron-period").await;
    assert_eq!(st, 200);
    assert_eq!(body["kind"], "event");
    assert_eq!(body["when"]["from_year"], -2500);
    assert_eq!(body["places"], serde_json::json!([{"id": "hebron", "name": "Kirjath-arba", "node": {"id": "Place:hebron", "kind": "Place", "label": "Hebron"}}]), "{body}");
}

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
    let app = atlas_contract::app::build(Arc::new(data), graph, None);

    let (st, body) = call(&app, "/api/event/a1").await;
    assert_eq!(st, 200);
    assert_eq!(body["acts_section"], "Acts pericope (this project's own sectioning): Acts 2:14-41");

    let (st, body) = call(&app, "/api/event/a2").await;
    assert_eq!(st, 200);
    assert!(body.get("acts_section").is_none(), "acts_section must be omitted, not null, when absent: {body}");
}

#[tokio::test]
async fn event_endpoint_carries_kjv_superscription_when_present() {
    let events = vec![
        Event {
            id: "k1".into(),
            label: "A Psalm of David, when he fled from Absalom his son.".into(),
            when: TimeRange::undated(),
            places: vec![],
            verses: vec![],
            kind: atlas_core::data::EventKind::General,
            kjv_superscription: Some("PSA.3.1, the psalm's own KJV superscription, quoted verbatim".into()),
            ..Default::default()
        },
        Event { id: "k2".into(), label: "No KJV-superscription provenance".into(), when: TimeRange::undated(), places: vec![], verses: vec![], kind: atlas_core::data::EventKind::General, ..Default::default() },
    ];
    let data = AtlasData::new(Canon { books: vec![] }, vec![], events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_contract::app::build(Arc::new(data), graph, None);

    let (st, body) = call(&app, "/api/event/k1").await;
    assert_eq!(st, 200);
    assert_eq!(body["kjv_superscription"], "PSA.3.1, the psalm's own KJV superscription, quoted verbatim");

    let (st, body) = call(&app, "/api/event/k2").await;
    assert_eq!(st, 200);
    assert!(body.get("kjv_superscription").is_none(), "kjv_superscription must be omitted, not null, when absent: {body}");
}

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
        kind: atlas_core::data::EventKind::General,
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
    let app = atlas_contract::app::build(Arc::new(data), graph, None);

    let (st, body) = call(&app, "/api/event/g_general_witnessed").await;
    assert_eq!(st, 200);
    assert_eq!(body["kind"], "general");
    assert!(body.get("when").is_none(), "general-kind passage must not carry a `when` key, even with witnesses: {body}");
    let witnesses = body["witnesses"].as_array().expect("witnesses array");
    assert_eq!(witnesses.len(), 2, "a general-kind passage's own witnesses must resolve identically to an event-kind one's: {body}");
    let books: Vec<&str> = witnesses.iter().map(|w| w["book"].as_str().unwrap()).collect();
    assert!(books.contains(&"EXO") && books.contains(&"DEU"), "both witness books must resolve: {body}");
}

#[tokio::test]
async fn narrative_event_positions_endpoint() {
    let app = app();

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
    assert!(patriarchs.get("prior").is_none(), "{patriarchs}");
    assert!(patriarchs.get("following").is_none(), "{patriarchs}");

    assert_eq!(body["timeline"]["prior"]["id"], "e1");
    assert_eq!(body["timeline"]["following"]["id"], "e3");

    let (st, body) = call(&app, "/api/narrative/event/e5").await;
    assert_eq!(st, 200);
    assert_eq!(body["narrative"], serde_json::json!([]));
    assert!(body["timeline"].get("prior").is_none(), "e5 is the fixture's true first dated event -- no prior: {body}");
    assert_eq!(body["timeline"]["following"]["id"], "e1");

    let (st, body) = call(&app, "/api/narrative/event/does-not-exist").await;
    assert_eq!(st, 404);
    assert_eq!(body["error"]["code"], "not_found");
}

#[tokio::test]
async fn polities_empty_fixture_shape_and_errors() {
    let app = app();

    let (st, body) = call(&app, "/api/polities?from=-1450&to=-1400").await;
    assert_eq!(st, 200);
    assert_eq!(body["polities"], serde_json::json!([]));

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

    let (st, body) = call(&app, "/api/polities?from=-50000&to=50000").await;
    assert_eq!(st, 200);
    assert_eq!(body["polities"], serde_json::json!([]));
}

#[tokio::test]
async fn xrefs_span_aggregation_ok_and_bad_ref() {
    let app = app();

    let (st, body) = call(&app, "/api/xrefs/JOS.6.20").await;
    assert_eq!(st, 200);
    let xrefs = body.as_array().unwrap();
    assert_eq!(xrefs.len(), 3);
    assert_eq!(xrefs[0]["target"], "JOS.6.20-21");
    assert_eq!(xrefs[0]["votes"], 9);

    let (st, body) = call(&app, "/api/xrefs/JOS.6.20-21").await;
    assert_eq!(st, 200);
    let xrefs = body.as_array().unwrap();
    assert_eq!(xrefs.len(), 2, "{body}");
    assert_eq!(xrefs[0]["target"], "JOS.1.3");
    assert_eq!(xrefs[0]["votes"], 9);
    assert!(xrefs[0]["preview"].as_str().unwrap().contains("sole of your foot"));
    assert_eq!(xrefs[1]["target"], "GEN.13.18");
    assert_eq!(xrefs[1]["votes"], 2);

    let (st, body) = call(&app, "/api/xrefs/GEN.13.18").await;
    assert_eq!(st, 200);
    assert_eq!(body, serde_json::json!([]));

    for bad in ["/api/xrefs/NOPE.1.1", "/api/xrefs/GEN.0.1", "/api/xrefs/gen..1", "/api/xrefs/JOS.6.31-21"] {
        let (st, body) = call(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_ref", "{bad}: {body}");
    }

    for bad in ["/api/xrefs/JOS", "/api/xrefs/JOS.6"] {
        let (st, body) = call(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_ref", "{bad}: {body}");
    }
}

#[tokio::test]
async fn catechism_span_and_item_endpoints() {
    let app = app();

    let (st, body) = call(&app, "/api/catechism/JOS.6.20").await;
    assert_eq!(st, 200);
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 1, "{body}");
    assert_eq!(items[0]["id"], "demo-item-1");
    assert_eq!(items[0]["name"], "Demo Catechism Item");
    assert!(items[0]["provenance"].is_array(), "a catechism row must carry its own provenance field: {body}");

    let (st, body) = call(&app, "/api/catechism/JOS.6.20-21").await;
    assert_eq!(st, 200);
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 2, "{body}");
    assert_eq!(items[0]["id"], "demo-item-1");
    assert!(items[0].get("question").is_none(), "{body}");
    assert_eq!(items[1]["id"], "demo-item-1");
    assert_eq!(items[1]["question"], "Demo Question");

    let (st, body) = call(&app, "/api/catechism/JOS.1.1").await;
    assert_eq!(st, 200);
    assert_eq!(body, serde_json::json!([]));

    for bad in ["/api/catechism/NOPE.1.1", "/api/catechism/gen..1", "/api/catechism/JOS", "/api/catechism/JOS.6"] {
        let (st, body) = call(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_ref", "{bad}: {body}");
    }

    let (st, body) = call(&app, "/api/catechism/item/demo-item-1").await;
    assert_eq!(st, 200);
    assert_eq!(body["id"], "demo-item-1");
    assert_eq!(body["name"], "Demo Catechism Item");
    assert_eq!(body["part_title"], "Demo Part");
    assert!(body.get("text").is_none(), "{body}");
    assert_eq!(body["explanation_heading"], "What does this mean?");
    assert_eq!(body["explanation"], "Demo item explanation.");
    assert_eq!(body["where_written"], "Demo where-written text.");
    let verses = body["verses"].as_array().unwrap();
    assert_eq!(verses.len(), 2, "{body}");
    assert_eq!(verses[0]["vref"], "JOS.6.20");
    assert!(verses[0]["text"].as_str().unwrap().contains("wall fell down flat"));
    assert!(verses[0].get("question").is_none(), "{body}");
    assert_eq!(verses[1]["vref"], "JOS.6.21");
    assert_eq!(verses[1]["question"], "Demo Question");

    let (st, body) = call(&app, "/api/catechism/item/does-not-exist").await;
    assert_eq!(st, 404);
    assert_eq!(body["error"]["code"], "not_found");
}

const YEAR_ZERO: i32 = 0;

#[tokio::test]
async fn a_person_whose_record_holds_a_year_zero_is_refused_as_this_atlases_own_defect() {
    // Arrange
    let mut data = demo_fixture();
    data.people.push(Person { id: "nobody_0".into(), name: "Nobody".into(), birth_year: Some(YEAR_ZERO), verse_links: vec!["GEN.1.1".into()], ..Default::default() });
    let data: AtlasData = data.finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_contract::app::build(Arc::new(data), graph, None);

    // Act
    let answer = call(&app, "/api/node/Person:nobody_0").await;

    // Assert
    assert_eq!(
        answer,
        (StatusCode::INTERNAL_SERVER_ERROR, serde_json::json!({ "error": { "code": "internal", "message": "nobody_0 records a year zero" } }))
    );
}

#[tokio::test]
async fn a_book_whose_record_dates_its_writing_at_one_end_only_is_refused_as_this_atlases_own_defect() {
    // Arrange
    let mut data = demo_fixture();
    let joshua = data.books_meta.iter_mut().find(|meta| meta.book == "JOS").expect("the demo atlas records Joshua");
    joshua.write_to = None;
    let data: AtlasData = data.finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_contract::app::build(Arc::new(data), graph, None);

    // Act
    let answer = call(&app, "/api/node/Container:bible-book-JOS").await;

    // Assert
    assert_eq!(
        answer,
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::json!({ "error": { "code": "internal", "message": "the writing date JOS records is refused: a span records one of its ends and not the other" } })
        )
    );
}

#[tokio::test]
async fn a_place_that_shares_a_catechism_items_id_carries_no_catechism_prose() {
    // Arrange
    let mut data = demo_fixture();
    data.places.push(Place { id: "demo-item-1".into(), name: "Demo Item".into(), lat: 31.5, lon: 35.5, verse_links: vec!["JOS.1.1".into()] });
    let data: AtlasData = data.finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_contract::app::build(Arc::new(data), graph, None);

    // Act
    let (status, record) = call(&app, "/api/node/Place:demo-item-1").await;

    // Assert
    assert_eq!(
        (status, record.clone()),
        (
            StatusCode::OK,
            serde_json::json!({
                "id": "Place:demo-item-1",
                "kind": "Place",
                "label": "Demo Item",
                "provenance": "curated-places",
                "edge_summary": [{ "kind": "mentioned-in", "count": 1 }],
                "version": record["version"],
                "place": { "lat": 31.5, "lon": 35.5, "display_name": "Demo Item" },
            })
        )
    );
}

#[tokio::test]
async fn a_place_a_chapter_names_carries_the_node_it_opens_on() {
    // Arrange
    let mut data = demo_fixture();
    data.places.push(Place { id: "kadesh".into(), name: "Kadesh".into(), lat: 30.6, lon: 34.4, verse_links: vec!["JOS.1.1".into()] });
    let data: AtlasData = data.finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_contract::app::build(Arc::new(data), graph, None);

    // Act
    let (status, chapter) = call(&app, "/api/chapter/JOS.1").await;

    // Assert
    assert_eq!(
        (status, chapter["verses"][0]["places"].clone()),
        (
            StatusCode::OK,
            serde_json::json!([{ "id": "kadesh", "name": "Kadesh", "node": { "id": "Place:kadesh", "kind": "Place", "label": "Kadesh" } }])
        )
    );
}

#[tokio::test]
async fn a_place_an_event_names_carries_the_node_it_opens_on() {
    // Arrange
    let app = app();

    // Act
    let (status, event) = call(&app, "/api/event/e3").await;

    // Assert
    assert_eq!(
        (status, event["places"].clone()),
        (
            StatusCode::OK,
            serde_json::json!([{ "id": "jericho", "name": "Jericho", "node": { "id": "Place:jericho", "kind": "Place", "label": "Jericho" } }])
        )
    );
}

#[tokio::test]
async fn an_event_record_carries_every_section_and_note_its_event_records() {
    // Arrange
    let mut data = demo_fixture();
    data.events.push(Event {
        id: "e-sections".into(),
        label: "A demo event cited by every outline".into(),
        when: TimeRange::new(-1000, -1000).unwrap(),
        verses: vec!["JOS.1.1".into()],
        robertson_section: Some("Robertson §1".into()),
        acts_section: Some("Acts §1".into()),
        atlas_section: Some("Atlas §1".into()),
        kjv_superscription: Some("A Psalm of David.".into()),
        ref_note: Some("Dated by the demo atlas alone.".into()),
        ..Default::default()
    });
    let data: AtlasData = data.finish();
    let graph = graph_fixture_for(&data);
    let app = atlas_contract::app::build(Arc::new(data), graph, None);

    // Act
    let (status, record) = call(&app, "/api/node/Event:e-sections").await;

    // Assert
    assert_eq!(
        (status, record["event"].clone()),
        (
            StatusCode::OK,
            serde_json::json!({
                "kind": "event",
                "when": { "from": { "value": -1000, "label": "1000 BC" }, "to": { "value": -1000, "label": "1000 BC" }, "label": "1000 BC" },
                "robertson_section": "Robertson §1",
                "acts_section": "Acts §1",
                "atlas_section": "Atlas §1",
                "kjv_superscription": "A Psalm of David.",
                "ref_note": "Dated by the demo atlas alone.",
            })
        )
    );
}

fn square_ring() -> Vec<(f64, f64)> {
    vec![(10.0, 10.0), (10.0, 11.0), (11.0, 11.0), (11.0, 10.0), (10.0, 10.0)]
}

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
    atlas_contract::app::build(Arc::new(data), graph, None)
}

#[tokio::test]
async fn an_era_carries_its_window_beside_the_integers_the_edge_suite_consumes() {
    // Arrange
    let app = app();

    // Act
    let (status, eras) = call(&app, "/api/eras").await;

    // Assert
    assert_eq!(status, StatusCode::OK, "{eras}");
    assert_eq!(
        eras,
        serde_json::json!([
            {
                "id": "patriarchs",
                "node": { "id": "Era:patriarchs", "kind": "Era", "label": "Patriarchs" },
                "name": "Patriarchs",
                "from_year": -2166,
                "to_year": -1877,
                "window": { "from": { "value": -2166, "label": "2166 BC" }, "to": { "value": -1877, "label": "1877 BC" }, "label": "2166 – 1877 BC" },
            },
            {
                "id": "conquest-judges",
                "node": { "id": "Era:conquest-judges", "kind": "Era", "label": "Conquest & Judges" },
                "name": "Conquest & Judges",
                "from_year": -1406,
                "to_year": -1051,
                "window": { "from": { "value": -1406, "label": "1406 BC" }, "to": { "value": -1051, "label": "1051 BC" }, "label": "1406 – 1051 BC" },
            },
        ])
    );
}

#[tokio::test]
async fn a_polity_carries_its_reign_beside_its_integers() {
    // Arrange
    let app = app_with_test_polities();

    // Act
    let (status, polities) = call(&app, "/api/polities?from=-2000&to=-1900").await;

    // Assert
    assert_eq!(status, StatusCode::OK, "{polities}");
    assert_eq!(
        polities,
        serde_json::json!({
            "polities": [{
                "id": "egypt",
                "node": { "id": "Polity:egypt", "kind": "Polity", "label": "Ptolemaic Egypt" },
                "name": "Egypt",
                "from": -2100,
                "to": -1200,
                "reign": { "from": { "value": -2100, "label": "2100 BC" }, "to": { "value": -1200, "label": "1200 BC" }, "label": "2100 – 1200 BC" },
                "rings": [square_ring()],
                "color_key": 3,
            }],
        })
    );
}

#[tokio::test]
async fn polities_intersection_ordering_and_color_key_stability() {
    let app = app_with_test_polities();

    let (st, body) = call(&app, "/api/polities?from=-2000&to=-1900").await;
    assert_eq!(st, 200);
    let polities = body["polities"].as_array().unwrap();
    assert_eq!(polities.len(), 1, "{polities:?}");
    assert_eq!(polities[0]["id"], "egypt");
    assert_eq!(polities[0]["name"], "Egypt");
    assert_eq!(polities[0]["color_key"], 3);

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

    assert_eq!(polities[0]["color_key"], 3);
    assert_eq!(polities[1]["color_key"], 3);
    assert_eq!(polities[0]["color_key"], polities[1]["color_key"]);

    let distinct_id_key_pairs: std::collections::HashSet<(&str, i64)> =
        polities.iter().map(|p| (p["id"].as_str().unwrap(), p["color_key"].as_i64().unwrap())).collect();
    let distinct_keys: std::collections::HashSet<i64> = distinct_id_key_pairs.iter().map(|&(_, k)| k).collect();
    assert_eq!(
        distinct_id_key_pairs.len(),
        distinct_keys.len(),
        "expected every DIFFERENT polity id in this response to carry a DISTINCT color_key; got {polities:?}"
    );

    let (st, body) = call(&app, "/api/polities?from=-4004&to=-2500").await;
    assert_eq!(st, 200);
    assert_eq!(body["polities"], serde_json::json!([]));

    let (_, body) = call(&app, "/api/polities?from=-2000&to=-1900").await;
    assert_eq!(body["polities"][0]["rings"][0][0], serde_json::json!([10.0, 10.0]));
}

#[tokio::test]
async fn polities_transition_and_fall_conditional_presence_on_the_wire() {
    let mut data = demo_fixture();
    data.polities = vec![Polity {
        id: "delta-test".into(),
        color_key: 0,
        eras: vec![
            PolityEra {
                name: "Rises Quietly".into(),
                from: -1000,
                to: -700,
                ref_note: "fixture".into(),
                rings: vec![square_ring()],
                transition: None,
                fall: None,
            },
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
    let app = atlas_contract::app::build(Arc::new(data), graph, None);

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

#[tokio::test]
async fn land_mask_empty_fixture_shape() {
    let app = app();
    let (st, body) = call(&app, "/api/land-mask").await;
    assert_eq!(st, 200);
    assert_eq!(body, serde_json::json!({ "rings": [] }));
}

#[tokio::test]
async fn static_dir_serves_files_api_still_wins_and_falls_back_to_index_for_spa_routes() {
    let dir = std::env::temp_dir().join(format!("atlas-server-static-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<html>shell</html>").unwrap();
    std::fs::write(dir.join("app.css"), "body{color:red}").unwrap();

    let app = atlas_contract::app::build(Arc::new(demo_fixture()), graph_fixture(), Some(dir.clone()));

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
