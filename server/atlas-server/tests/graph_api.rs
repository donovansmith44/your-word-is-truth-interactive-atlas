//! HTTP-level tests for Batch M-A's own new surfaces -- the two generic
//! graph endpoints and the text-window endpoint -- built over the REAL
//! compiled/raw data (not a fixture), via `tower::ServiceExt::oneshot`
//! (mirrors `tests/api.rs`'s own no-socket-bound pattern).
//!
//! `bijection_witness_over_http_...` is the M-A acceptance set's own
//! "forward cites entry and inverse cited-by entry carry the SAME EdgeId
//! (JHN.3.16-class sample, read from the wire)" requirement, exercised
//! against John 3:16's real cross-references (178 real rows in the
//! committed raw data at authoring time).

use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use atlas_core::data::AtlasData;
use atlas_graph::GraphService;

fn real_app() -> axum::Router {
    let compiled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled");
    let raw = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let data = AtlasData::load(&compiled).expect("data/compiled/*.json must exist").finish();
    // GraphService::build runs the FIDELITY LAW unconditionally as part of
    // construction (fix round 1) -- reaching this line already proves it
    // passed on the real committed KJV source.
    let graph = GraphService::build(&raw, &data).expect("data/raw/{kjv.json,xrefs/cross_references.txt} must exist and satisfy the fidelity law");
    atlas_server::app::build(Arc::new(data), Arc::new(graph), None)
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, serde_json::Value, axum::http::HeaderMap) {
    let response = app.clone().oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() { serde_json::Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, json, headers)
}

#[tokio::test]
async fn text_window_single_verse_matches_the_compiled_verse_map() {
    let app = real_app();
    let (st, body, _headers) = get(&app, "/api/text?ref=JHN.3.16").await;
    assert_eq!(st, 200);
    let units = body["units"].as_array().unwrap();
    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["ref"], "JHN.3.16");
    assert!(units[0]["text"].as_str().unwrap().contains("For God so loved the world"));
    assert_eq!(units[0]["text"].as_str().unwrap(), "For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life.");
    assert_eq!(body["next"], "JHN.3.17");
}

#[tokio::test]
async fn text_window_n_and_dir_walk_onward_and_backward() {
    let app = real_app();

    let (st, onward, _) = get(&app, "/api/text?ref=JHN.3.16&n=3&dir=onward").await;
    assert_eq!(st, 200);
    let refs: Vec<&str> = onward["units"].as_array().unwrap().iter().map(|u| u["ref"].as_str().unwrap()).collect();
    assert_eq!(refs, vec!["JHN.3.16", "JHN.3.17", "JHN.3.18"]);

    let (st, backward, _) = get(&app, "/api/text?ref=JHN.3.16&n=3&dir=backward").await;
    assert_eq!(st, 200);
    let refs: Vec<&str> = backward["units"].as_array().unwrap().iter().map(|u| u["ref"].as_str().unwrap()).collect();
    assert_eq!(refs, vec!["JHN.3.14", "JHN.3.15", "JHN.3.16"], "a backward window ENDS at ref, in ascending reading order");
}

#[tokio::test]
async fn text_window_scope_chapter_returns_exactly_that_chapters_units() {
    let app = real_app();
    let (st, body, _) = get(&app, "/api/text?ref=JHN.3&scope=chapter").await;
    assert_eq!(st, 200);
    let units = body["units"].as_array().unwrap();
    assert_eq!(units.len(), 36, "John 3 has 36 verses in the real KJV text");
    assert_eq!(units[0]["ref"], "JHN.3.1");
    assert_eq!(units[35]["ref"], "JHN.3.36");
    assert_eq!(body["next"], "JHN.4.1");

    // A verse-shaped ref also resolves to its own whole chapter under
    // scope=chapter (only the (book, chapter) pair is used).
    let (st2, body2, _) = get(&app, "/api/text?ref=JHN.3.16&scope=chapter").await;
    assert_eq!(st2, 200);
    assert_eq!(body2["units"], body["units"], "JHN.3 and JHN.3.16 under scope=chapter must resolve to the identical chapter window");
}

/// Fix round 1, I1: `scope=chapter&dir=backward` used to silently serve
/// the WRONG chapter's tail (`chapter_span`'s own `start` is always the
/// requested chapter's own verse 1, and applying backward resolution to
/// that as if it were a window's END walks into the PRECEDING chapter).
/// Now rejected outright as a clean 400 -- both paths (accept without
/// `dir`, reject with `dir=backward`) tested here; the accept path is
/// ALSO the same one `text_window_scope_chapter_returns_exactly_that_chapters_units`
/// above already covers, so this test is specifically about the rejection
/// and about `dir=onward` staying accepted (the honest, meaningful case
/// when `scope=chapter` -- onward is the only direction a chapter window
/// could ever mean, so it stays a 200, not also rejected).
#[tokio::test]
async fn text_window_scope_chapter_rejects_dir_backward_but_accepts_dir_onward() {
    let app = real_app();

    let (st, body, _) = get(&app, "/api/text?ref=JHN.3&scope=chapter&dir=backward").await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(body["error"]["code"], "bad_dir", "{body}");

    let (st2, body2, _) = get(&app, "/api/text?ref=JHN.3&scope=chapter&dir=onward").await;
    assert_eq!(st2, 200);
    let units = body2["units"].as_array().unwrap();
    assert_eq!(units.len(), 36, "dir=onward must still serve the ordinary, correct chapter window");
    assert_eq!(units[0]["ref"], "JHN.3.1");
}

#[tokio::test]
async fn text_window_etag_round_trips_via_if_none_match() {
    let app = real_app();
    let (st, _body, headers) = get(&app, "/api/text?ref=GEN.1.1").await;
    assert_eq!(st, 200);
    let etag = headers.get(header::ETAG).expect("ETag header must be present").to_str().unwrap().to_string();
    assert!(etag.starts_with('"') && etag.ends_with('"'), "ETag must be a quoted opaque string: {etag}");

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/api/text?ref=GEN.1.1").header(header::IF_NONE_MATCH, &etag).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert!(bytes.is_empty(), "a 304 must carry no body");
}

#[tokio::test]
async fn text_window_bad_ref_and_missing_ref_are_400() {
    let app = real_app();
    for bad in ["/api/text?ref=NOPE.1.1", "/api/text", "/api/text?ref="] {
        let (st, body, _) = get(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_ref", "{bad}: {body}");
    }
}

#[tokio::test]
async fn node_card_returns_id_kind_label_edge_summary_and_version() {
    let app = real_app();
    let (st, body, _) = get(&app, "/api/node/text-unit:JHN.3.16").await;
    assert_eq!(st, 200);
    assert_eq!(body["id"], "text-unit:JHN.3.16");
    assert_eq!(body["kind"], "TextUnit");
    assert_eq!(body["label"], "JHN.3.16");
    assert!(!body["version"].as_str().unwrap().is_empty());
    let summary = body["edge_summary"].as_array().unwrap();
    let cites = summary.iter().find(|e| e["kind"] == "cites").expect("JHN.3.16 must have real cites in the compiled data");
    assert!(cites["count"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn node_card_unknown_id_is_404_malformed_id_is_400() {
    let app = real_app();
    let (st, body, _) = get(&app, "/api/node/text-unit:GEN.999.999").await;
    assert_eq!(st, 404);
    assert_eq!(body["error"]["code"], "not_found");

    let (st2, body2, _) = get(&app, "/api/node/not-a-real-id").await;
    assert_eq!(st2, 400);
    assert_eq!(body2["error"]["code"], "bad_ref");
}

// ---------------------------------------------------------------------
// Batch M-B: the generic endpoints growing to serve Event/Narrative/
// Anchor/Place-stub cards and their frontiers (brief requirement 4).
// `ab_ur` ("Terah's family leaves Ur," the real, committed FIRST leg of
// the real `abraham-migration` narrative -- `data/curated/narratives/
// abraham-migration.toml`'s own `legs = ["ab_ur", "ab_haran", ...]`) is
// used throughout: a real, stable, multi-relation event.
// ---------------------------------------------------------------------

#[tokio::test]
async fn event_card_and_frontiers_are_served_by_the_generic_endpoints() {
    let app = real_app();

    let (st, body, _) = get(&app, "/api/node/Event:ab_ur").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["id"], "Event:ab_ur");
    assert_eq!(body["kind"], "Event");
    assert_eq!(body["label"], "Terah's family leaves Ur");
    let summary: Vec<String> = body["edge_summary"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap().to_string()).collect();
    assert!(summary.contains(&"attested-in".to_string()), "ab_ur must carry a real attested-in frontier: {summary:?}");
    assert!(summary.contains(&"located-at".to_string()), "ab_ur must carry a real located-at frontier: {summary:?}");
    assert!(summary.contains(&"dated-by".to_string()), "ab_ur must carry a real dated-by frontier: {summary:?}");
    assert!(summary.contains(&"follows-in".to_string()), "ab_ur is abraham-migration's own first leg -- it must follow-in to ab_haran: {summary:?}");

    let (st2, edges, _) = get(&app, "/api/node/Event:ab_ur/edges?kind=located-at").await;
    assert_eq!(st2, 200, "{edges}");
    let entries = edges["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["node"]["id"], "Place:ur-1");
    assert_eq!(entries[0]["node"]["kind"], "Place");

    let (st3, followed, _) = get(&app, "/api/node/Event:ab_ur/edges?kind=follows-in").await;
    assert_eq!(st3, 200, "{followed}");
    let followed_entries = followed["entries"].as_array().unwrap();
    assert_eq!(followed_entries.len(), 1);
    assert_eq!(followed_entries[0]["node"]["id"], "Event:ab_haran");
}

#[tokio::test]
async fn narrative_card_and_place_stub_card_are_served_generically() {
    let app = real_app();

    let (st, body, _) = get(&app, "/api/node/Narrative:abraham-migration").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["kind"], "Narrative");
    assert_eq!(body["label"], "Abraham's Migration");

    let (st2, place, _) = get(&app, "/api/node/Place:ur-1").await;
    assert_eq!(st2, 200, "{place}");
    assert_eq!(place["kind"], "Place");
    let summary: Vec<String> = place["edge_summary"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap().to_string()).collect();
    assert!(summary.contains(&"site-of".to_string()), "ur-1 must show its real inverse located-at frontier: {summary:?}");
}

/// An Anchor's own card carries its citation ("why this date?" explorable
/// -- the owner's own design, brief requirement 4). `solomon-crowned`
/// (`data/curated/chronology-anchors.toml`) is a real, bound anchor row.
#[tokio::test]
async fn anchor_card_carries_its_citation_and_dates_frontier() {
    let app = real_app();

    let (st, body, _) = get(&app, "/api/node/Anchor:solomon-crowned").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["kind"], "Anchor");
    let label = body["label"].as_str().unwrap();
    assert!(label.contains("Source:"), "an Anchor's own card label IS its citation: {label}");

    let (st2, dates, _) = get(&app, "/api/node/Anchor:solomon-crowned/edges?kind=dates").await;
    assert_eq!(st2, 200, "{dates}");
    let entries = dates["entries"].as_array().unwrap();
    assert!(!entries.is_empty(), "solomon-crowned must date at least one real event");

    let (st3, justifies, _) = get(&app, "/api/node/Anchor:solomon-crowned/edges?kind=justifies").await;
    assert_eq!(st3, 200, "{justifies}");
    assert!(!justifies["entries"].as_array().unwrap().is_empty(), "an anchor-bound DatedBy row's own justified-by ground must resolve back to this anchor (brief requirement 4)");
}

#[tokio::test]
async fn node_edges_bad_kind_and_missing_kind_are_400() {
    let app = real_app();
    let (st, body, _) = get(&app, "/api/node/text-unit:JHN.3.16/edges").await;
    assert_eq!(st, 400);
    assert_eq!(body["error"]["code"], "bad_kind");

    let (st2, body2, _) = get(&app, "/api/node/text-unit:JHN.3.16/edges?kind=not-a-real-kind").await;
    assert_eq!(st2, 400);
    assert_eq!(body2["error"]["code"], "bad_kind");
}

#[tokio::test]
async fn node_edges_pagination_pages_are_windows_over_the_total() {
    let app = real_app();
    let (_, full, _) = get(&app, "/api/node/text-unit:JHN.3.16/edges?kind=cites&limit=200").await;
    let full_entries = full["entries"].as_array().unwrap();
    assert!(full_entries.len() > 1, "JHN.3.16 has many real cross-references");

    let mut paged: Vec<serde_json::Value> = Vec::new();
    let mut cursor: Option<u64> = None;
    loop {
        let uri = match cursor {
            Some(c) => format!("/api/node/text-unit:JHN.3.16/edges?kind=cites&limit=1&cursor={c}"),
            None => "/api/node/text-unit:JHN.3.16/edges?kind=cites&limit=1".to_string(),
        };
        let (st, page, _) = get(&app, &uri).await;
        assert_eq!(st, 200);
        paged.extend(page["entries"].as_array().unwrap().iter().cloned());
        match page["next"].as_u64() {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    assert_eq!(paged, *full_entries, "limit=1 pages, concatenated, must equal the single wide page");
}

/// THE BIJECTION WITNESS OVER HTTP (M-A acceptance set): a forward `cites`
/// entry and its target's inverse `cited-by` entry carry the SAME EdgeId,
/// read entirely off the wire -- no direct graph access in this test.
/// Paginates the inverse side to completion (`limit=200` per page, cursor
/// loop) rather than assuming a single page holds the match -- some real
/// targets (e.g. a heavily-cited verse) may carry more than 200 citers.
#[tokio::test]
async fn bijection_witness_over_http_cites_and_cited_by_share_the_same_edge_id() {
    let app = real_app();

    let (st, forward_page, _) = get(&app, "/api/node/text-unit:JHN.3.16/edges?kind=cites&limit=1").await;
    assert_eq!(st, 200);
    let entry = &forward_page["entries"][0];
    let edge_id = entry["edge"].as_str().unwrap().to_string();
    let target_id = entry["node"]["id"].as_str().unwrap().to_string();

    let mut cursor: Option<u64> = None;
    let mut found: Option<String> = None;
    loop {
        let uri = match cursor {
            Some(c) => format!("/api/node/{target_id}/edges?kind=cited-by&limit=200&cursor={c}"),
            None => format!("/api/node/{target_id}/edges?kind=cited-by&limit=200"),
        };
        let (st2, inverse_page, _) = get(&app, &uri).await;
        assert_eq!(st2, 200);
        let inverse_entries = inverse_page["entries"].as_array().unwrap();
        if let Some(back) = inverse_entries.iter().find(|e| e["node"]["id"] == "text-unit:JHN.3.16") {
            found = Some(back["edge"].as_str().unwrap().to_string());
            break;
        }
        match inverse_page["next"].as_u64() {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }

    assert_eq!(
        found.as_deref(),
        Some(edge_id.as_str()),
        "the target's own cited-by pages must list JHN.3.16 as a citer, carrying the SAME edge id -- one row, two projections, one id, read from the wire"
    );
}
