use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use atlas_contract::error::ApiError;
use atlas_core::data::AtlasData;
use atlas_graph::GraphService;

/// The real corpus and the graph built from it, both shared: the router only ever reads them, and
/// building either one per test cost this file's 41 routers 41 compiles and 41 graph builds.
fn real_atlas() -> (Arc<AtlasData>, Arc<GraphService>) {
    static CACHED: std::sync::OnceLock<(Arc<AtlasData>, Arc<GraphService>)> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let (graph, data) = atlas_contract::load::build_from_raw(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled")).expect("data/raw + data/curated must compile into a graph");
            (Arc::new(data), Arc::new(graph))
        })
        .clone()
}

fn compiled_app() -> axum::Router {
    let (data, graph) = real_atlas();
    atlas_contract::app::build(data, graph, None)
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
    let app = compiled_app();
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
    let app = compiled_app();

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
    let app = compiled_app();
    let (st, body, _) = get(&app, "/api/text?ref=JHN.3&scope=chapter").await;
    assert_eq!(st, 200);
    let units = body["units"].as_array().unwrap();
    assert_eq!(units.len(), 36, "John 3 has 36 verses in the real KJV text");
    assert_eq!(units[0]["ref"], "JHN.3.1");
    assert_eq!(units[35]["ref"], "JHN.3.36");
    assert_eq!(body["next"], "JHN.4.1");

    let (st2, body2, _) = get(&app, "/api/text?ref=JHN.3.16&scope=chapter").await;
    assert_eq!(st2, 200);
    assert_eq!(body2["units"], body["units"], "JHN.3 and JHN.3.16 under scope=chapter must resolve to the identical chapter window");
}

#[tokio::test]
async fn text_window_scope_chapter_rejects_dir_backward_but_accepts_dir_onward() {
    let app = compiled_app();

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
    let app = compiled_app();
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
async fn a_chapter_shaped_ref_without_scope_chapter_is_a_bad_ref() {
    let app = compiled_app();
    let (status, body, _) = get(&app, "/api/text?ref=JHN.3").await;
    assert_eq!(
        (status, body),
        (StatusCode::BAD_REQUEST, serde_json::json!({ "error": { "code": "bad_ref", "message": "invalid scripture reference: 'JHN.3'" } }))
    );
}

#[tokio::test]
async fn a_word_outside_a_query_vocabulary_is_refused_with_that_vocabularys_own_code() {
    let app = compiled_app();
    let mut refused = Vec::new();
    for uri in ["/api/text?ref=JHN.3.16&scope=paragraph", "/api/text?ref=JHN.3.16&dir=sideways", "/api/text?ref=JHN.3.16&corpus=vulgate"] {
        let (status, body, _) = get(&app, uri).await;
        refused.push((status, body));
    }
    assert_eq!(
        refused,
        vec![
            (StatusCode::BAD_REQUEST, refusal(ApiError::unknown_scope("paragraph"))),
            (StatusCode::BAD_REQUEST, refusal(ApiError::unknown_dir("sideways"))),
            (StatusCode::BAD_REQUEST, refusal(ApiError::bad_corpus("vulgate"))),
        ]
    );
}

#[tokio::test]
async fn a_count_that_is_not_a_count_leaves_the_windows_own_default_standing() {
    let app = compiled_app();
    let (status, body, _) = get(&app, "/api/text?ref=JHN.3.16&n=notacount").await;
    let refs: Vec<&str> = body["units"].as_array().unwrap().iter().map(|u| u["ref"].as_str().unwrap()).collect();
    assert_eq!((status, refs), (StatusCode::OK, vec!["JHN.3.16"]));
}

#[tokio::test]
async fn a_conditional_request_whose_query_cannot_be_read_is_refused_rather_than_answered_not_modified() {
    // Arrange
    let app = compiled_app();
    let (_, _, headers) = get(&app, "/api/text?ref=GEN.1.1").await;
    let etag = headers.get(header::ETAG).expect("ETag header must be present").to_str().unwrap().to_string();
    // Act
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/api/text?ref=GEN.1.1&scope=paragraph").header(header::IF_NONE_MATCH, &etag).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body: serde_json::Value = serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    // Assert
    assert_eq!((status, body), (StatusCode::BAD_REQUEST, refusal(ApiError::unknown_scope("paragraph"))));
}

#[tokio::test]
async fn a_frontier_asked_for_at_a_node_that_is_not_there_is_refused_for_the_frontier_first() {
    // Arrange
    let app = compiled_app();
    let absent_node = "/api/node/Person:nonexistent-xyz/edges?kind=not-a-real-kind";
    // Act
    let (status, body, _) = get(&app, absent_node).await;
    // Assert
    assert_eq!((status, body), (StatusCode::BAD_REQUEST, refusal(ApiError::bad_kind("not-a-real-kind"))));
}

/// The refusal as it reaches a caller. Built from the constructor rather than written
/// out, because the words themselves are pinned by the mapping's own table test.
fn refusal(refused: ApiError) -> serde_json::Value {
    serde_json::json!({ "error": { "code": refused.code, "message": refused.message } })
}

#[tokio::test]
async fn text_window_bad_ref_and_missing_ref_are_400() {
    let app = compiled_app();
    for bad in ["/api/text?ref=NOPE.1.1", "/api/text", "/api/text?ref="] {
        let (st, body, _) = get(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_ref", "{bad}: {body}");
    }
}

#[tokio::test]
async fn contents_bible_is_books_then_chapters_and_stops_there() {
    let app = compiled_app();
    let (st, body, _) = get(&app, "/api/contents/bible").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["corpus"], "bible");
    let roots = body["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 66);
    assert_eq!(roots[0]["title"], "Genesis");
    assert_eq!(roots[0]["kind"], "book");
    assert_eq!(roots[0]["group"], "OT");
    assert_eq!(roots[0]["ref"], "GEN.1");
    let gen_ch = roots[0]["children"].as_array().unwrap();
    assert_eq!(gen_ch.len(), 50);
    assert_eq!(gen_ch[0]["kind"], "chapter");
    assert_eq!(gen_ch[0]["title"], "1");
    assert_eq!(gen_ch[0]["ref"], "GEN.1");
    assert_eq!(gen_ch[0]["count"], 31, "Genesis 1 has 31 verses");
    assert_eq!(gen_ch[49]["ref"], "GEN.50");
    assert!(gen_ch[0].get("children").is_none(), "depth stops at chapter");
    assert_eq!(roots[39]["title"], "Matthew");
    assert_eq!(roots[39]["group"], "NT");
    assert_eq!(roots[65]["children"].as_array().unwrap().len(), 22, "Revelation has 22 chapters");
    let total: usize = roots.iter().map(|r| r["children"].as_array().unwrap().len()).sum();
    assert_eq!(total, 1189, "1,189 chapters over the canon");
}

#[tokio::test]
async fn contents_concord_is_documents_then_articles() {
    let app = compiled_app();
    let (st, body, _) = get(&app, "/api/contents/concord").await;
    assert_eq!(st, 200, "{body}");
    let roots = body["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 10, "ten Concord documents");
    let mut prev_part = 0u64;
    for d in roots {
        assert_eq!(d["kind"], "document");
        assert!(d.get("group").is_none(), "no testament grouping for the Concord");
        let arts = d["children"].as_array().unwrap();
        assert!(!arts.is_empty(), "{} has articles", d["title"]);
        for a in arts {
            assert_eq!(a["kind"], "article");
            let sref = a["ref"].as_str().unwrap();
            assert!(sref.starts_with("BoC ") && sref.matches('.').count() == 2, "{sref}");
            assert!(a["count"].as_u64().unwrap() > 0, "{sref} has paragraphs");
        }
        let part: u64 = arts[0]["ref"].as_str().unwrap()["BoC ".len()..].split('.').next().unwrap().parse().unwrap();
        assert!(part >= prev_part, "documents in part order: {} after {prev_part}", part);
        prev_part = part;
        assert_eq!(d["ref"], arts[0]["ref"], "a document navigates to its first article's first paragraph");
    }
    let (st, body, _) = get(&app, "/api/contents/nope").await;
    assert_eq!(st, 404);
    assert_eq!(body["error"]["code"], "not_found");
}

#[tokio::test]
async fn text_window_units_carry_their_edge_summary() {
    let app = compiled_app();
    let (st, body, _) = get(&app, "/api/text?ref=BoC%207.2.1&n=3&corpus=concord").await;
    assert_eq!(st, 200, "{body}");
    let units = body["units"].as_array().expect("units");
    assert!(!units.is_empty());
    for u in units {
        let es = u["edge_summary"].as_array().expect("edge_summary present on every unit");
        assert!(!es.is_empty(), "every Concord paragraph is at least a member of its article: {u}");
        for e in es {
            assert!(e["kind"].is_string() && e["count"].as_u64().unwrap_or(0) > 0, "only inhabited kinds are listed: {e}");
        }
        let kinds: Vec<&str> = es.iter().map(|e| e["kind"].as_str().unwrap()).collect();
        assert!(kinds.contains(&"member-of"), "a paragraph is a member of its article: {kinds:?}");
    }
    let (st, jhn, _) = get(&app, "/api/text?ref=JHN.3.16&n=1").await;
    assert_eq!(st, 200);
    let kinds: Vec<&str> = jhn["units"][0]["edge_summary"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"cites"), "JHN.3.16 cites others: {kinds:?}");
    assert!(kinds.contains(&"catechism-link"), "JHN.3.16 is catechism-linked: {kinds:?}");
    let (_, card, _) = get(&app, "/api/node/text-unit:JHN.3.16").await;
    assert_eq!(jhn["units"][0]["edge_summary"], card["edge_summary"]);
}

#[tokio::test]
async fn text_window_concord_single_paragraph_is_the_real_sc_first_commandment() {
    let app = compiled_app();
    let (st, body, _headers) = get(&app, "/api/text?ref=BoC%207.2.1&corpus=concord").await;
    assert_eq!(st, 200, "{body}");
    let units = body["units"].as_array().unwrap();
    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["ref"], "BoC 7.2.1");
    assert_eq!(
        units[0]["text"].as_str().unwrap(),
        "Thou shalt have no other gods. What does this mean? \u{2013}Answer: We should fear, love, and trust in God above all things.",
        "the real bookofconcord.org-sourced First Commandment paragraph, served through the existing generic endpoint"
    );
    assert_eq!(body["next"], "BoC 7.2.2");
}

#[tokio::test]
async fn text_window_concord_n_and_dir_walk_onward_and_backward_within_augsburg_confession_iv() {
    let app = compiled_app();
    let (st, onward, _) = get(&app, "/api/text?ref=BoC%203.5.1&n=3&dir=onward&corpus=concord").await;
    assert_eq!(st, 200, "{onward}");
    let refs: Vec<&str> = onward["units"].as_array().unwrap().iter().map(|u| u["ref"].as_str().unwrap()).collect();
    assert_eq!(refs, vec!["BoC 3.5.1", "BoC 3.5.2", "BoC 3.5.3"]);

    let (st, backward, _) = get(&app, "/api/text?ref=BoC%203.5.3&n=3&dir=backward&corpus=concord").await;
    assert_eq!(st, 200, "{backward}");
    let refs: Vec<&str> = backward["units"].as_array().unwrap().iter().map(|u| u["ref"].as_str().unwrap()).collect();
    assert_eq!(refs, vec!["BoC 3.5.1", "BoC 3.5.2", "BoC 3.5.3"], "a backward window ENDS at ref, in ascending reading order");
}

#[tokio::test]
async fn text_window_concord_scope_chapter_and_bad_ref_and_unknown_corpus_are_400() {
    let app = compiled_app();
    let (st, body, _) = get(&app, "/api/text?ref=BoC%207.2.1&scope=chapter&corpus=concord").await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(body["error"]["code"], "bad_scope", "a chapter is a Scripture reading, so it is the SCOPE a Concord caller must change: {body}");

    let (st, body, _) = get(&app, "/api/text?ref=JHN.3.16&corpus=concord").await;
    assert_eq!(st, 400, "a Bible-shaped ref under corpus=concord is bad_ref, never silently reinterpreted: {body}");
    assert_eq!(body["error"]["code"], "bad_ref");

    let (st, body, _) = get(&app, "/api/text?ref=BoC%207.2.1&corpus=lxx").await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(body["error"]["code"], "bad_corpus");
}

#[tokio::test]
async fn text_window_bible_default_corpus_is_unchanged_by_the_new_param() {
    let app = compiled_app();
    let (st, body, _) = get(&app, "/api/text?ref=JHN.3.16&corpus=bible").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["units"][0]["text"].as_str().unwrap(), "For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life.");
}

#[tokio::test]
async fn node_card_returns_id_kind_label_edge_summary_and_version() {
    let app = artifact_app();
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
    let app = artifact_app();
    let (st, body, _) = get(&app, "/api/node/text-unit:GEN.999.999").await;
    assert_eq!(st, 404);
    assert_eq!(body["error"]["code"], "not_found");

    let (st2, body2, _) = get(&app, "/api/node/not-a-real-id").await;
    assert_eq!(st2, 400);
    assert_eq!(body2["error"]["code"], "bad_ref");
}

#[tokio::test]
async fn event_card_and_frontiers_are_served_by_the_generic_endpoints() {
    let app = artifact_app();

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
    assert_eq!(entries[0]["neighbour"]["node"]["id"], "Place:ur-1");
    assert_eq!(entries[0]["neighbour"]["node"]["kind"], "Place");

    let (st3, followed, _) = get(&app, "/api/node/Event:ab_ur/edges?kind=follows-in").await;
    assert_eq!(st3, 200, "{followed}");
    let followed_entries = followed["entries"].as_array().unwrap();
    assert_eq!(followed_entries.len(), 1);
    assert_eq!(followed_entries[0]["neighbour"]["node"]["id"], "Event:ab_haran");
}

#[tokio::test]
async fn narrative_card_and_place_stub_card_are_served_generically() {
    let app = artifact_app();

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

#[tokio::test]
async fn anchor_card_carries_its_citation_and_dates_frontier() {
    let app = artifact_app();

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
async fn person_card_and_mentioned_in_frontier_are_served_by_the_generic_endpoints() {
    let app = artifact_app();

    let (st, body, _) = get(&app, "/api/node/Person:aaron_1").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["id"], "Person:aaron_1");
    assert_eq!(body["kind"], "Person");
    assert_eq!(body["label"], "Aaron");
    assert_eq!(body["provenance"], "theographic-people");
    let summary: Vec<serde_json::Value> = body["edge_summary"].as_array().unwrap().clone();
    let mentioned_in = summary.iter().find(|e| e["kind"] == "mentioned-in").expect("aaron_1 must carry a real mentioned-in frontier");
    assert_eq!(mentioned_in["count"], 331, "Aaron's 331 resolved verse links: one edge per verse, however many times the verse names him");

    let (st2, page, _) = get(&app, "/api/node/Person:aaron_1/edges?kind=mentioned-in&limit=3").await;
    assert_eq!(st2, 200, "{page}");
    let entries = page["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0]["neighbour"]["node"]["id"], "text-unit:EXO.4.14");
    assert_eq!(entries[1]["neighbour"]["node"]["id"], "text-unit:EXO.4.27");
    assert_eq!(entries[2]["neighbour"]["node"]["id"], "text-unit:EXO.4.28");
    assert_eq!(page["next"], 3, "a 331-entry frontier at limit=3 must page, not silently truncate");
}

fn committed_rows() -> &'static atlas_graph_types::graph::Graph {
    static ROWS: std::sync::OnceLock<atlas_graph_types::graph::Graph> = std::sync::OnceLock::new();
    ROWS.get_or_init(|| atlas_graph::sqlite::reload::committed_graph(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled")).expect("the committed sections read back").0)
}

fn bible_mention_rows() -> Vec<(atlas_graph_types::text::VerseRef, &'static atlas_graph_types::edge::Mentions)> {
    committed_rows()
        .mentions
        .iter()
        .filter_map(|row| match &row.locus.at {
            atlas_graph_types::text::TextRef::Bible(verse) => Some((verse.clone(), row)),
            atlas_graph_types::text::TextRef::Concord(_) => None,
        })
        .collect()
}

fn the_verse_naming_one_entity_most_often() -> (atlas_graph_types::text::VerseRef, atlas_graph_types::id::AnyNodeId) {
    let mut rows_of_pair: std::collections::BTreeMap<(atlas_graph_types::text::VerseRef, atlas_graph_types::id::AnyNodeId), usize> = std::collections::BTreeMap::new();
    for (verse, row) in bible_mention_rows() {
        *rows_of_pair.entry((verse, row.entity.node_id())).or_default() += 1;
    }
    rows_of_pair.into_iter().max_by_key(|(_, rows)| *rows).map(|(pair, _)| pair).expect("the artifact carries mention rows")
}

fn word_span(verse: &atlas_graph_types::text::VerseRef, words: &atlas_graph_types::text::TokenSpan) -> serde_json::Value {
    let unit = serde_json::json!({ "corpus": "bible", "book": atlas_core::refs::BookId(verse.book).code(), "chapter": verse.chapter, "verse": verse.verse });
    serde_json::json!({ "from": { "unit": unit, "word": words.start }, "to": { "unit": unit, "word": words.end } })
}

fn loci_of(verse: &atlas_graph_types::text::VerseRef, rows: &[&atlas_graph_types::edge::Mentions]) -> serde_json::Value {
    let spans: Vec<serde_json::Value> = rows.iter().filter_map(|row| row.locus.span.as_ref()).map(|words| word_span(verse, words)).collect();
    if spans.is_empty() { serde_json::Value::Null } else { serde_json::Value::Array(spans) }
}

fn wire_id_of(entity: &atlas_graph_types::id::AnyNodeId) -> String {
    atlas_contract::graph_wire::encode_node_id(entity)
}

fn wire_id_of_verse(verse: &atlas_graph_types::text::VerseRef) -> String {
    atlas_contract::graph_wire::encode_node_id(&atlas_graph::kjv_adapter::verse_node_id(verse.book, verse.chapter, verse.verse))
}

fn node_and_loci(entries: &serde_json::Value) -> Vec<serde_json::Value> {
    entries.as_array().unwrap().iter().map(|e| serde_json::json!({ "node": e["neighbour"]["node"]["id"], "loci": e["loci"] })).collect()
}

#[tokio::test]
async fn a_verses_mentions_page_lists_each_entity_once_with_every_occurrence_as_loci() {
    // Arrange
    let app = artifact_app();
    let (verse, _) = the_verse_naming_one_entity_most_often();
    let at_verse: Vec<&atlas_graph_types::edge::Mentions> = bible_mention_rows().into_iter().filter(|(v, _)| *v == verse).map(|(_, row)| row).collect();
    let mut entities: Vec<atlas_graph_types::id::AnyNodeId> = Vec::new();
    for row in &at_verse {
        if !entities.contains(&row.entity.node_id()) && !matches!(row.entity, atlas_graph_types::edge::MentionedEntity::PeopleGroup(_)) {
            entities.push(row.entity.node_id());
        }
    }
    let composed: Vec<serde_json::Value> = entities
        .iter()
        .map(|entity| {
            let rows: Vec<&atlas_graph_types::edge::Mentions> = at_verse.iter().copied().filter(|row| row.entity.node_id() == *entity).collect();
            serde_json::json!({ "node": wire_id_of(entity), "loci": loci_of(&verse, &rows) })
        })
        .collect();

    // Act
    let (status, page, _) = get(&app, &format!("/api/node/{}/edges?kind=mentions", wire_id_of_verse(&verse))).await;

    // Assert
    assert_eq!(
        (status, page["kind"].clone(), node_and_loci(&page["entries"]), page["next"].clone(), at_verse.len() > entities.len()),
        (StatusCode::OK, serde_json::json!("mentions"), composed, serde_json::Value::Null, true),
        "at {}",
        wire_id_of_verse(&verse)
    );
}

#[tokio::test]
async fn a_persons_mentioned_in_page_lists_a_verse_once_and_continues_from_the_next_verses_first_row() {
    // Arrange
    let app = artifact_app();
    let (verse, entity) = the_verse_naming_one_entity_most_often();
    let verses_of_entity: Vec<atlas_graph_types::text::VerseRef> = bible_mention_rows().into_iter().filter(|(_, row)| row.entity.node_id() == entity).map(|(v, _)| v).collect();
    let first_row = verses_of_entity.iter().position(|v| *v == verse).expect("the entity's rows include the verse");
    let rows_at_verse: Vec<&atlas_graph_types::edge::Mentions> = bible_mention_rows().into_iter().filter(|(v, row)| *v == verse && row.entity.node_id() == entity).map(|(_, row)| row).collect();
    let next_edges_first_row = verses_of_entity.iter().enumerate().skip(first_row).find(|(_, v)| **v != verse).map(|(ord, _)| ord);

    // Act
    let (status, page, _) = get(&app, &format!("/api/node/{}/edges?kind=mentioned-in&cursor={first_row}&limit=1", wire_id_of(&entity))).await;

    // Assert
    assert_eq!(
        (status, page["kind"].clone(), node_and_loci(&page["entries"]), page["next"].clone(), rows_at_verse.len() > 1),
        (
            StatusCode::OK,
            serde_json::json!("mentioned-in"),
            vec![serde_json::json!({ "node": wire_id_of_verse(&verse), "loci": loci_of(&verse, &rows_at_verse) })],
            serde_json::json!(next_edges_first_row),
            true,
        ),
        "{} at {}",
        wire_id_of(&entity),
        wire_id_of_verse(&verse)
    );
}

#[tokio::test]
async fn a_persons_card_counts_the_verses_that_mention_him_not_the_times_they_name_him() {
    // Arrange
    let app = artifact_app();
    let mut rows_of_entity: std::collections::BTreeMap<atlas_graph_types::id::AnyNodeId, Vec<atlas_graph_types::text::VerseRef>> = std::collections::BTreeMap::new();
    for (verse, row) in bible_mention_rows() {
        rows_of_entity.entry(row.entity.node_id()).or_default().push(verse);
    }
    let (entity, verses) = rows_of_entity.into_iter().max_by_key(|(_, verses)| verses.len()).expect("the artifact carries mention rows");
    let distinct: std::collections::BTreeSet<&atlas_graph_types::text::VerseRef> = verses.iter().collect();

    // Act
    let (status, card, _) = get(&app, &format!("/api/node/{}", wire_id_of(&entity))).await;
    let mentioned_in = card["edge_summary"].as_array().unwrap().iter().find(|e| e["kind"] == "mentioned-in").cloned();

    // Assert
    assert_eq!(
        (status, mentioned_in, verses.len() > distinct.len()),
        (StatusCode::OK, Some(serde_json::json!({ "kind": "mentioned-in", "count": distinct.len() })), true),
        "{}",
        wire_id_of(&entity)
    );
}

#[tokio::test]
async fn person_card_carries_a_real_easton_description_when_a_match_exists() {
    let app = artifact_app();
    let (st, body, _) = get(&app, "/api/node/Person:aaron_1").await;
    assert_eq!(st, 200, "{body}");
    let description = body["description"].as_str().expect("Aaron must carry a real description over the real compiled data");
    assert!(description.starts_with("The eldest son of Amram"), "must be Easton's own verbatim prose, got: {description}");
}

#[tokio::test]
async fn place_detail_carries_a_real_easton_description_when_a_match_exists() {
    let app = artifact_app();
    let (st, body, _) = get(&app, "/api/place/hebron").await;
    assert_eq!(st, 200, "{body}");
    let description = body["description"].as_str().expect("Hebron must carry a real description over the real compiled data");
    assert!(!description.trim().is_empty());
    assert!(description.contains("Eshcol") || description.contains("Jerusalem"), "must be Easton's own real Hebron prose, got: {description}");
}

#[tokio::test]
async fn commentary_item_card_carries_its_own_real_kretzmann_prose_via_description() {
    let app = artifact_app();
    let (st, body, _) = get(&app, "/api/node/CommentaryItem:kretzmann%2F0.1.0").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["label"], "The Creation of the World.: The Creation of Chaos and Light");
    let description = body["description"].as_str().expect("a real CommentaryItem must carry its own prose over the real compiled data");
    assert!(description.starts_with("In the beginning, cp. John 1, 1"), "must be Kretzmann's own verbatim prose (the lemma already excised, KRETZ-1), got: {description}");
    assert!(description.contains("the heaven"), "must be the FULL unit text, not truncated, got: {description}");
}

#[tokio::test]
async fn node_card_omits_description_for_a_kind_that_never_carries_one() {
    let app = artifact_app();
    let (st, body, _) = get(&app, "/api/node/Era:primeval").await;
    assert_eq!(st, 200, "{body}");
    assert!(body.get("description").is_none(), "an Era card must never carry a description key at all, got: {body}");
}

#[tokio::test]
async fn verse_endpoint_serves_the_real_words_of_christ_span_for_mat_4_19() {
    let app = compiled_app();
    let (st, body, _) = get(&app, "/api/verse/MAT.4.19").await;
    assert_eq!(st, 200, "{body}");

    let text = body["text"].as_str().unwrap();
    assert_eq!(text, "And he saith unto them, Follow me, and I will make you fishers of men.");

    let spans = body["words_of_christ"].as_array().expect("words_of_christ must always be present, even at 0, never an omitted key");
    assert_eq!(spans.len(), 1, "MAT.4.19 must carry exactly one real red-letter span over the compiled data: {spans:?}");
    assert_eq!(spans[0]["start"], 24, "{spans:?}");
    assert_eq!(spans[0]["end"], 70, "{spans:?}");

    let start = spans[0]["start"].as_u64().unwrap() as usize;
    let end = spans[0]["end"].as_u64().unwrap() as usize;
    assert_eq!(&text[start..end], "Follow me, and I will make you fishers of men.");
}

#[tokio::test]
async fn chapter_endpoint_serves_the_same_real_words_of_christ_span_for_mat_4_19() {
    let app = compiled_app();
    let (st, chapter, _) = get(&app, "/api/chapter/MAT.4").await;
    assert_eq!(st, 200);
    let v19 = chapter["verses"].as_array().unwrap().iter().find(|v| v["verse"] == 19).expect("MAT.4.19 must be in the chapter");
    let spans = v19["words_of_christ"].as_array().expect("words_of_christ must always be present, even at 0, never an omitted key");
    assert_eq!(spans.len(), 1, "{spans:?}");
    assert_eq!(spans[0]["start"], 24, "{spans:?}");
    assert_eq!(spans[0]["end"], 70, "{spans:?}");
}

#[tokio::test]
async fn a_verses_mentions_frontier_carries_person_entities_alongside_place() {
    let app = compiled_app();

    let (st, page, _) = get(&app, "/api/node/text-unit:EXO.4.14/edges?kind=mentions").await;
    assert_eq!(st, 200, "{page}");
    let entries = page["entries"].as_array().unwrap();
    let person_labels: Vec<String> = entries.iter().filter(|e| e["neighbour"]["node"]["kind"] == "Person").map(|e| e["neighbour"]["node"]["label"].as_str().unwrap().to_string()).collect();
    assert!(person_labels.contains(&"Aaron".to_string()), "{person_labels:?}");
    assert!(person_labels.contains(&"Moses".to_string()), "{person_labels:?}");
}

#[tokio::test]
async fn peoplegroup_mentions_are_real_in_the_graph_but_filtered_from_the_generic_edges_page() {
    let app = compiled_app();

    let (st, card, _) = get(&app, "/api/node/text-unit:GEN.10.16").await;
    assert_eq!(st, 200, "{card}");
    let summary = card["edge_summary"].as_array().unwrap();
    let mentions_count = summary.iter().find(|e| e["kind"] == "mentions").and_then(|e| e["count"].as_u64()).unwrap_or(0);
    assert!(mentions_count >= 3, "GEN.10.16 must carry >=3 real mentions (Jebusite/Amorite/Girgasite) in the built graph, unfiltered: {summary:?}");

    let (st2, page, _) = get(&app, "/api/node/text-unit:GEN.10.16/edges?kind=mentions").await;
    assert_eq!(st2, 200, "{page}");
    let entries = page["entries"].as_array().unwrap();
    assert!(entries.iter().all(|e| e["neighbour"]["node"]["kind"] != "PeopleGroup"), "no edge-page entry may carry kind=PeopleGroup -- the current client cannot render or re-fetch it: {entries:?}");
}

#[tokio::test]
async fn peoplegroup_node_id_cannot_be_fetched_directly_yet() {
    let app = compiled_app();
    let (st, body, _) = get(&app, "/api/node/PeopleGroup:jebusite_748").await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(body["error"]["code"], "bad_ref");
}

#[tokio::test]
async fn chapter_response_for_a_gentilic_locus_carries_no_peoplegroup_kind_span() {
    let app = compiled_app();
    let (st, chapter, _) = get(&app, "/api/chapter/GEN.10").await;
    assert_eq!(st, 200);
    let v16 = chapter["verses"].as_array().unwrap().iter().find(|v| v["verse"] == 16).expect("GEN.10.16 must be in the chapter");
    let persons = v16["persons"].as_array().expect("persons must always be present, even at 0, never an omitted key");
    let names: Vec<&str> = persons.iter().map(|p| p["name"].as_str().unwrap()).collect();
    for gone in ["Jebusite", "Amorite", "Girgasite"] {
        assert!(!names.contains(&gone), "'{gone}' must NOT appear in GEN.10.16's own persons list any more -- it is a PeopleGroup now: {names:?}");
    }
}

#[tokio::test]
async fn chapter_verse_persons_is_always_present_and_matches_the_generic_mentions_frontier() {
    let app = compiled_app();

    let (st, chapter, _) = get(&app, "/api/chapter/EXO.4").await;
    assert_eq!(st, 200);
    let v14 = chapter["verses"].as_array().unwrap().iter().find(|v| v["verse"] == 14).expect("EXO.4.14 must be in the chapter");
    let persons = v14["persons"].as_array().expect("persons must always be present, even at 0, never an omitted key");
    let chapter_names: std::collections::BTreeSet<String> = persons.iter().map(|p| p["name"].as_str().unwrap().to_string()).collect();
    assert_eq!(chapter_names, std::collections::BTreeSet::from(["Aaron".to_string(), "God".to_string(), "Moses".to_string()]), "{persons:?}");

    let (st2, edges, _) = get(&app, "/api/node/text-unit:EXO.4.14/edges?kind=mentions").await;
    assert_eq!(st2, 200);
    let frontier_names: std::collections::BTreeSet<String> = edges["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["neighbour"]["node"]["kind"] == "Person")
        .map(|e| e["neighbour"]["node"]["label"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(chapter_names, frontier_names, "the chapter view's own persons list must equal the generic mentions frontier's own Person entries for the SAME verse");

    let v7 = chapter["verses"].as_array().unwrap().iter().find(|v| v["verse"] == 7).expect("EXO.4.7 must be in the chapter");
    assert_eq!(v7["persons"], serde_json::json!([]), "{v7:?}");
}

#[tokio::test]
async fn person_card_unknown_id_is_404() {
    let app = artifact_app();
    let (st, body, _) = get(&app, "/api/node/Person:nonexistent-xyz").await;
    assert_eq!(st, 404, "{body}");
}

#[tokio::test]
async fn node_edges_bad_kind_and_missing_kind_are_400() {
    let app = artifact_app();
    let (st, body, _) = get(&app, "/api/node/text-unit:JHN.3.16/edges").await;
    assert_eq!(st, 400);
    assert_eq!(body["error"]["code"], "bad_kind");

    let (st2, body2, _) = get(&app, "/api/node/text-unit:JHN.3.16/edges?kind=not-a-real-kind").await;
    assert_eq!(st2, 400);
    assert_eq!(body2["error"]["code"], "bad_kind");
}

#[tokio::test]
async fn node_edges_pagination_pages_are_windows_over_the_total() {
    let app = artifact_app();
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

#[tokio::test]
async fn bijection_witness_over_http_cites_and_cited_by_share_the_same_edge_id() {
    let app = compiled_app();

    let (st, forward_page, _) = get(&app, "/api/node/text-unit:JHN.3.16/edges?kind=cites&limit=1").await;
    assert_eq!(st, 200);
    let entry = &forward_page["entries"][0];
    let edge_id = entry["edge"].as_str().unwrap().to_string();
    let target_id = entry["neighbour"]["node"]["id"].as_str().unwrap().to_string();

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
        if let Some(back) = inverse_entries.iter().find(|e| e["neighbour"]["node"]["id"] == "text-unit:JHN.3.16") {
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

fn first_verse_of(target: &str) -> String {
    let head = target.split('-').next().unwrap_or(target);
    let parts: Vec<&str> = head.split('.').collect();
    assert!(parts.len() >= 3, "a cross-reference target's own head must be a canonical BOOK.CHAPTER.VERSE ref: {target}");
    format!("{}.{}.{}", parts[0], parts[1], parts[2])
}

#[tokio::test]
async fn chapter_verse_xref_count_is_always_present_and_matches_the_generic_edges_page() {
    let app = compiled_app();

    let (st, chapter, _) = get(&app, "/api/chapter/JHN.3").await;
    assert_eq!(st, 200);
    let v16 = chapter["verses"].as_array().unwrap().iter().find(|v| v["verse"] == 16).expect("JHN.3.16 must be in the chapter");
    let chapter_count = v16["xref_count"].as_u64().expect("xref_count must always be present, even at 0") as usize;
    assert!(chapter_count > 1, "JHN.3.16 must carry real, multiple cross-references in the compiled data: {chapter_count}");

    let (st2, edges, _) = get(&app, "/api/node/text-unit:JHN.3.16/edges?kind=cites&limit=200").await;
    assert_eq!(st2, 200);
    let entries = edges["entries"].as_array().unwrap();
    assert!(entries.len() < 200, "limit=200 must exceed JHN.3.16's own real total, or this test's own page needs widening");
    assert_eq!(chapter_count, entries.len(), "the chapter view's own xref_count must equal the generic edges page's own true count for the SAME verse");

    let (st3, card, _) = get(&app, "/api/node/text-unit:JHN.3.16").await;
    assert_eq!(st3, 200);
    let cites_summary = card["edge_summary"].as_array().unwrap().iter().find(|e| e["kind"] == "cites").expect("JHN.3.16 must summarize a real cites frontier");
    assert_eq!(chapter_count, cites_summary["count"].as_u64().unwrap() as usize, "the chapter view's own xref_count must equal the node card's own edge_summary count");
}

#[tokio::test]
async fn chapter_verse_xref_count_is_always_present_never_omitted_across_a_whole_chapter() {
    let app = compiled_app();

    let (st, chapter, _) = get(&app, "/api/chapter/GEN.1").await;
    assert_eq!(st, 200);
    let verses = chapter["verses"].as_array().unwrap();
    assert!(!verses.is_empty());
    for v in verses {
        let count = v.get("xref_count");
        assert!(count.is_some(), "xref_count must never be omitted, even at 0: verse {}", v["verse"]);
        assert!(count.unwrap().as_u64().is_some(), "xref_count must be a plain non-negative integer: verse {}", v["verse"]);
    }
}

#[tokio::test]
async fn chapter_verse_xref_count_is_zero_not_omitted_for_a_real_verse_with_no_cross_references() {
    let app = compiled_app();

    let candidates = ["GEN.5", "GEN.10", "GEN.36", "NUM.1", "1CH.1", "1CH.2", "EZR.2", "NEH.7"];
    let mut zero_count_found = false;
    'outer: for cref in candidates {
        let (st, chapter, _) = get(&app, &format!("/api/chapter/{cref}")).await;
        if st != StatusCode::OK {
            continue;
        }
        for v in chapter["verses"].as_array().unwrap() {
            if v["xref_count"] == 0 {
                zero_count_found = true;
                let verse_num = v["verse"].as_u64().unwrap();
                let (book, chnum) = cref.split_once('.').unwrap();
                let (_, card, _) = get(&app, &format!("/api/node/text-unit:{book}.{chnum}.{verse_num}")).await;
                let has_cites = card["edge_summary"].as_array().unwrap().iter().any(|e| e["kind"] == "cites");
                assert!(!has_cites, "a zero xref_count verse must have NO cites entry in its own node card's edge_summary");
                break 'outer;
            }
        }
    }
    assert!(zero_count_found, "expected at least one real zero-cross-reference verse among the scanned candidate chapters");
}

#[tokio::test]
async fn generic_cites_edges_are_already_votes_descending_matching_the_bespoke_verse_endpoint() {
    let app = compiled_app();

    let (st, verse, _) = get(&app, "/api/verse/JHN.3.16").await;
    assert_eq!(st, 200);
    let bespoke: Vec<String> = verse["cross_refs"].as_array().unwrap().iter().map(|cr| first_verse_of(cr["target"].as_str().unwrap())).collect();
    assert!(bespoke.len() > 1, "need >1 real cross-references to prove an ORDER, not just a singleton");

    let (st2, edges, _) = get(&app, "/api/node/text-unit:JHN.3.16/edges?kind=cites&limit=200").await;
    assert_eq!(st2, 200);
    let generic: Vec<String> = edges["entries"].as_array().unwrap().iter().map(|e| e["neighbour"]["node"]["id"].as_str().unwrap().trim_start_matches("text-unit:").to_string()).collect();

    assert_eq!(
        generic, bespoke,
        "the generic `cites` edge page must already be votes-descending, position for position matching the bespoke, provably-votes-sorted /api/verse endpoint -- no client-side re-sort should ever be needed"
    );
}

#[tokio::test]
async fn a_fulfillment_edge_is_reachable_via_the_generic_frontier_for_mat_1_22() {
    let app = compiled_app();

    let (st, body, _) = get(&app, "/api/node/text-unit:MAT.1.22").await;
    assert_eq!(st, 200, "{body}");
    let summary: Vec<String> = body["edge_summary"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap().to_string()).collect();
    assert!(summary.contains(&"fulfills".to_string()), "MAT.1.22 must carry a real fulfills (inverse) frontier: {summary:?}");

    let (st2, edges, _) = get(&app, "/api/node/text-unit:MAT.1.22/edges?kind=fulfills").await;
    assert_eq!(st2, 200, "{edges}");
    let entries = edges["entries"].as_array().unwrap();
    assert!(entries.iter().any(|e| e["neighbour"]["node"]["id"] == "text-unit:ISA.7.14"), "MAT.1.22 must fulfill ISA.7.14: {entries:?}");

    let (st3, prophecy_body, _) = get(&app, "/api/node/text-unit:ISA.7.14").await;
    assert_eq!(st3, 200, "{prophecy_body}");
    let prophecy_summary: Vec<String> = prophecy_body["edge_summary"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap().to_string()).collect();
    assert!(prophecy_summary.contains(&"fulfilled-in".to_string()), "ISA.7.14 must carry a real fulfilled-in (forward) frontier: {prophecy_summary:?}");

    let (st4, prophecy_edges, _) = get(&app, "/api/node/text-unit:ISA.7.14/edges?kind=fulfilled-in").await;
    assert_eq!(st4, 200, "{prophecy_edges}");
    let prophecy_entries = prophecy_edges["entries"].as_array().unwrap();
    assert!(prophecy_entries.iter().any(|e| e["neighbour"]["node"]["id"] == "text-unit:MAT.1.22"), "ISA.7.14 must be fulfilled-in MAT.1.22: {prophecy_entries:?}");
}

#[tokio::test]
async fn a_typology_edge_is_reachable_via_the_generic_frontier_for_the_melchizedek_case() {
    let app = compiled_app();

    let (st, body, _) = get(&app, "/api/node/text-unit:HEB.7.1").await;
    assert_eq!(st, 200, "{body}");
    let summary: Vec<String> = body["edge_summary"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap().to_string()).collect();
    assert!(summary.contains(&"prefigured-by".to_string()), "HEB.7.1 must carry a real prefigured-by (inverse) frontier: {summary:?}");

    let (st2, edges, _) = get(&app, "/api/node/text-unit:HEB.7.1/edges?kind=prefigured-by").await;
    assert_eq!(st2, 200, "{edges}");
    let entries = edges["entries"].as_array().unwrap();
    assert!(entries.iter().any(|e| e["neighbour"]["node"]["id"] == "text-unit:GEN.14.18"), "HEB.7.1 must be prefigured-by GEN.14.18: {entries:?}");

    let (st3, type_body, _) = get(&app, "/api/node/text-unit:GEN.14.18").await;
    assert_eq!(st3, 200, "{type_body}");
    let type_summary: Vec<String> = type_body["edge_summary"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap().to_string()).collect();
    assert!(type_summary.contains(&"prefigures".to_string()), "GEN.14.18 must carry a real prefigures (forward) frontier: {type_summary:?}");

    let (st4, type_edges, _) = get(&app, "/api/node/text-unit:GEN.14.18/edges?kind=prefigures").await;
    assert_eq!(st4, 200, "{type_edges}");
    let type_entries = type_edges["entries"].as_array().unwrap();
    assert!(type_entries.iter().any(|e| e["neighbour"]["node"]["id"] == "text-unit:HEB.7.1"), "GEN.14.18 must prefigure HEB.7.1: {type_entries:?}");
}

#[tokio::test]
async fn a_fulfillment_and_a_typology_rows_own_ground_carries_a_real_justifies_frontier() {
    let app = compiled_app();

    let (st, justifies, _) = get(&app, "/api/node/text-unit:MAT.1.22/edges?kind=justifies").await;
    assert_eq!(st, 200, "{justifies}");
    assert!(!justifies["entries"].as_array().unwrap().is_empty(), "MAT.1.22 must justify its own fulfills row (JB-1 rider)");

    let (st2, typology_justifies, _) = get(&app, "/api/node/text-unit:HEB.7.1/edges?kind=justifies").await;
    assert_eq!(st2, 200, "{typology_justifies}");
    assert!(!typology_justifies["entries"].as_array().unwrap().is_empty(), "HEB.7.1 must justify its own typology row (JB-1 rider)");
}

#[tokio::test]
async fn nodes_uninvolved_in_fulfillment_or_typology_carry_no_such_edge_summary_entries() {
    let app = compiled_app();
    let (st, body, _) = get(&app, "/api/node/text-unit:GEN.1.1").await;
    assert_eq!(st, 200, "{body}");
    let summary: Vec<String> = body["edge_summary"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap().to_string()).collect();
    for kind in ["fulfilled-in", "fulfills", "prefigures", "prefigured-by"] {
        assert!(!summary.contains(&kind.to_string()), "GEN.1.1 must carry no '{kind}' entry (uninvolved in either new relation): {summary:?}");
    }
}

#[tokio::test]
async fn chapter_container_card_and_frontiers_are_served_by_the_generic_endpoints() {
    let app = artifact_app();

    let (st, body, _) = get(&app, "/api/node/Container:bible-chapter-JHN-3").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["id"], "Container:bible-chapter-JHN-3");
    assert_eq!(body["kind"], "Container");
    assert_eq!(body["label"], "John 3", "the reader's own display name (canon::BOOKS name + chapter)");
    assert_eq!(body["provenance"], "kjv");
    let summary = body["edge_summary"].as_array().unwrap();
    let contains = summary.iter().find(|e| e["kind"] == "contains").expect("a chapter's Members frontier");
    assert_eq!(contains["count"], 36, "John 3 has 36 verses");
    assert!(summary.iter().any(|e| e["kind"] == "member-of"), "a chapter is a member of its book: {summary:?}");
    assert!(summary.iter().any(|e| e["kind"] == "follows-in"), "prev/next navigation forward: {summary:?}");
    assert!(summary.iter().any(|e| e["kind"] == "precedes-in"), "prev/next navigation backward: {summary:?}");

    let (st2, page, _) = get(&app, "/api/node/Container:bible-chapter-JHN-3/edges?kind=contains&limit=2").await;
    assert_eq!(st2, 200, "{page}");
    let entries = page["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["neighbour"]["node"]["id"], "text-unit:JHN.3.1");

    let (st3, page3, _) = get(&app, "/api/node/Container:bible-chapter-GEN-50/edges?kind=follows-in").await;
    assert_eq!(st3, 200, "{page3}");
    let entries3 = page3["entries"].as_array().unwrap();
    assert_eq!(entries3.len(), 1);
    assert_eq!(entries3[0]["neighbour"]["node"]["id"], "Container:bible-chapter-EXO-1");
    assert_eq!(entries3[0]["neighbour"]["node"]["label"], "Exodus 1");
}

#[tokio::test]
async fn book_container_card_is_served_and_a_verse_reaches_its_chapter_back() {
    let app = artifact_app();

    let (st, body, _) = get(&app, "/api/node/Container:bible-book-GEN").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["kind"], "Container");
    assert_eq!(body["label"], "Genesis");
    let summary = body["edge_summary"].as_array().unwrap();
    let contains = summary.iter().find(|e| e["kind"] == "contains").expect("a book's Members frontier");
    assert_eq!(contains["count"], 50, "Genesis has 50 chapters");

    let (st2, page, _) = get(&app, "/api/node/text-unit:JHN.3.16/edges?kind=member-of").await;
    assert_eq!(st2, 200, "{page}");
    let entries = page["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["neighbour"]["node"]["id"], "Container:bible-chapter-JHN-3");
}

#[tokio::test]
async fn verse_detail_carries_its_own_text_provenance_and_its_sections_sources() {
    let app = compiled_app();
    let (st, body, _h) = get(&app, "/api/verse/GEN.1.1").await;
    assert_eq!(st, StatusCode::OK);

    assert_eq!(body["provenance"], "kjv", "a verse's text is the King James Version's, and must say so");

    assert_eq!(
        body["cross_refs_provenance"].as_array().expect("cross_refs_provenance must be an array"),
        &vec![serde_json::json!("openbible.info-cross-references")],
        "'sourced from openbible.com' -- the owner's own example, at the section he named it about"
    );

    let events = body["events"].as_array().expect("events array");
    assert!(!events.is_empty(), "GEN.1.1 must belong to at least one event for this assertion to mean anything");
    for e in events {
        let p = e["provenance"].as_str().unwrap_or("");
        assert!(
            p == "theographic" || p == "curated",
            "every event-membership row must name its event's own source ('theographic' | 'curated'), got '{p}' for {}",
            e["id"]
        );
    }
}

#[tokio::test]
async fn event_detail_carries_its_own_provenance_and_its_sections_own_sources() {
    let app = compiled_app();

    let (st, body, _h) = get(&app, "/api/event/theo-249").await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["provenance"], "theographic");
    assert!(body.get("witnesses_provenance").is_none(), "an event with no accounts must not carry an accounts attribution");
    assert_eq!(
        body["mentions_provenance"].as_array().expect("a mention-only event must attribute its mentions"),
        &vec![serde_json::json!("attestation-corrections")],
        "the retyped mentions are ATTEST-1's own hand-authored corrections, and must report as such"
    );

    let (st, body, _h) = get(&app, "/api/event/mat_leper_healed").await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["provenance"], "curated", "a hand-authored event must never report as Theographic");

    let analogues = body["analogues"].as_array().expect("analogues array");
    assert_eq!(analogues.len(), 1);
    assert_eq!(analogues[0]["provenance"], "attestation-corrections");
}

#[tokio::test]
async fn no_provenance_field_the_wire_serves_is_ever_blank() {
    let app = compiled_app();

    let mut checked = 0usize;
    let mut populated = 0usize;
    let mut check = |label: &str, v: &serde_json::Value, require_non_empty: bool| {
        if let Some(s) = v.as_str() {
            assert!(!s.trim().is_empty(), "{label} rode the wire as a BLANK provenance -- the silent blank requirement 3 forbids");
            checked += 1;
        }
        if let Some(a) = v.as_array() {
            if require_non_empty {
                assert!(
                    !a.is_empty(),
                    "{label} is EMPTY on a section that HAS rows -- the client renders an empty list as no affordance at all, \
                     so this is the silent blank again, one type-level up: rows on screen with nothing naming their source"
                );
                populated += 1;
            }
            for x in a {
                let s = x.as_str().unwrap_or_else(|| panic!("{label} must be a list of strings"));
                assert!(!s.trim().is_empty(), "{label} carries a BLANK provenance id in its list");
                checked += 1;
            }
        }
    };
    let has_rows = |v: &serde_json::Value| v.as_array().is_some_and(|a| !a.is_empty());

    let (st, verse, _h) = get(&app, "/api/verse/GEN.1.1").await;
    assert_eq!(st, StatusCode::OK);
    check("verse.provenance", &verse["provenance"], false);
    check("verse.cross_refs_provenance", &verse["cross_refs_provenance"], has_rows(&verse["cross_refs"]));
    check("verse.catechism_provenance", &verse["catechism_provenance"], has_rows(&verse["catechism"]));
    for e in verse["events"].as_array().into_iter().flatten() {
        check("verse.events[].provenance", &e["provenance"], false);
    }
    for c in verse["cross_refs"].as_array().into_iter().flatten() {
        check("verse.cross_refs[].provenance", &c["provenance"], true);
    }
    for c in verse["catechism"].as_array().into_iter().flatten() {
        check("verse.catechism[].provenance", &c["provenance"], true);
    }

    let (st, event, _h) = get(&app, "/api/event/mat_leper_healed").await;
    assert_eq!(st, StatusCode::OK);
    check("event.provenance", &event["provenance"], false);
    check("event.witnesses_provenance", &event["witnesses_provenance"], event.get("witnesses_provenance").is_some());
    check("event.mentions_provenance", &event["mentions_provenance"], event.get("mentions_provenance").is_some());
    for a in event["analogues"].as_array().into_iter().flatten() {
        check("event.analogues[].provenance", &a["provenance"], false);
    }

    let (st, xrefs, _h) = get(&app, "/api/xrefs/EXO.20.3").await;
    assert_eq!(st, StatusCode::OK);
    assert!(has_rows(&xrefs), "EXO.20.3 must carry cross references for this sweep to reach the bare-array endpoint at all");
    for x in xrefs.as_array().into_iter().flatten() {
        check("xrefs[].provenance", &x["provenance"], true);
    }
    let (st, cat, _h) = get(&app, "/api/catechism/MAT.28.19").await;
    assert_eq!(st, StatusCode::OK);
    for c in cat.as_array().into_iter().flatten() {
        check("catechism[].provenance", &c["provenance"], true);
    }

    assert!(checked > 12, "the sweep must actually have found provenance fields to check (found {checked})");
    assert!(
        populated > 3,
        "the POPULATED-SECTION half of this sweep must actually have fired (fired {populated} times) -- \
         if it ever reaches zero, this test is back to proving only that no string is blank"
    );
}

#[tokio::test]
async fn the_bare_array_endpoints_attribute_their_rows_so_a_passage_gets_a_question_mark_too() {
    let app = compiled_app();

    let (st, xrefs, _h) = get(&app, "/api/xrefs/EXO.20.3-4").await;
    assert_eq!(st, StatusCode::OK);
    let xrefs = xrefs.as_array().expect("xrefs must be an array -- the wire SHAPE is unchanged, only the element grew");
    assert!(!xrefs.is_empty(), "EXO.20.3-4 must carry cross references for this assertion to mean anything");
    for x in xrefs {
        assert_eq!(
            x["provenance"].as_array().expect("every cross-reference row carries its own attribution"),
            &vec![serde_json::json!("openbible.info-cross-references")],
            "the owner's own headline source, now on a PASSAGE's rows too"
        );
    }

    let (st, cat, _h) = get(&app, "/api/catechism/MAT.28.19-20").await;
    assert_eq!(st, StatusCode::OK, "the catechism route must still resolve with its second extractor");
    assert!(cat.is_array(), "catechism must still be an ARRAY -- the wire shape is unchanged, only the element grew");
}

#[tokio::test]
async fn every_provenance_id_the_wire_serves_resolves_to_a_registry_source() {
    let registry: atlas_core::sources::SourcesDocument = {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled/sources.json");
        serde_json::from_str(&std::fs::read_to_string(&path).expect("sources.json must exist")).expect("sources.json must parse")
    };

    let app = compiled_app();
    let (_st, verse, _h) = get(&app, "/api/verse/GEN.1.1").await;
    let (_st, event, _h) = get(&app, "/api/event/mat_leper_healed").await;
    let (_st, xrefs, _h) = get(&app, "/api/xrefs/EXO.20.3-4").await;
    let (_st, catechism, _h) = get(&app, "/api/catechism/MAT.28.19-20").await;

    let mut served: Vec<String> = Vec::new();
    let mut push = |v: &serde_json::Value| {
        if let Some(s) = v.as_str() {
            served.push(s.to_string());
        }
        if let Some(a) = v.as_array() {
            served.extend(a.iter().filter_map(|x| x.as_str().map(str::to_string)));
        }
    };
    push(&verse["provenance"]);
    push(&verse["cross_refs_provenance"]);
    push(&verse["catechism_provenance"]);
    for e in verse["events"].as_array().into_iter().flatten() {
        push(&e["provenance"]);
    }
    push(&event["provenance"]);
    push(&event["witnesses_provenance"]);
    push(&event["mentions_provenance"]);
    for a in event["analogues"].as_array().into_iter().flatten() {
        push(&a["provenance"]);
    }
    for x in xrefs.as_array().into_iter().flatten() {
        push(&x["provenance"]);
    }
    for c in catechism.as_array().into_iter().flatten() {
        push(&c["provenance"]);
    }

    assert!(served.len() > 4, "the wire must actually be carrying provenance for this test to mean anything (got {served:?})");
    for id in &served {
        let kind = atlas_core::sources::split_provenance_id(id).0;
        let row = registry.provenances.iter().find(|p| p.id == kind);
        let row = row.unwrap_or_else(|| panic!("wire provenance id '{id}' resolves to no registry row"));
        assert!(
            registry.sources.iter().any(|s| s.id == row.source),
            "wire provenance id '{id}' names source '{}', which does not exist",
            row.source
        );
    }
}

#[tokio::test]
async fn chapter_verse_places_name_real_places_from_the_graph_backed_scene_source() {
    let app = artifact_app();

    let (st, chapter, _) = get(&app, "/api/chapter/GEN.13").await;
    assert_eq!(st, 200);
    let verses = chapter["verses"].as_array().expect("GEN.13 must serve verses");

    let v18 = verses.iter().find(|v| v["verse"] == 18).expect("GEN.13.18 must be in the chapter");
    let places = v18["places"].as_array().expect("places must always be present");
    println!("GEN.13.18 places = {}", serde_json::to_string(places).unwrap());
    assert!(
        !places.is_empty(),
        "GEN.13.18 ('Abram... dwelt in the plain of Mamre, which is in Hebron') must carry at least one mentioned place -- an empty array here is exactly the silent darkening OVERLAY-1's re-sourcing could have caused: {chapter}"
    );
    let hebron = places.iter().find(|p| p["id"] == "hebron").unwrap_or_else(|| panic!("GEN.13.18 must name Place:hebron: {places:?}"));
    assert!(
        hebron["name"].as_str().is_some_and(|n| !n.is_empty()),
        "the place must carry a resolved display name, not an empty string: {hebron}"
    );

    let empty: Vec<u64> = verses
        .iter()
        .filter(|v| v["places"].as_array().is_some_and(|p| p.is_empty()))
        .map(|v| v["verse"].as_u64().unwrap())
        .collect();
    println!("GEN.13 verses with NO place mention = {empty:?}");
    assert!(
        !empty.is_empty(),
        "at least one verse of GEN.13 must carry an empty places array -- otherwise this test would pass on a 'place on every verse' bug too"
    );
    assert!(
        empty.len() < verses.len(),
        "and not ALL of them, which the hebron assertion above already proves"
    );
}

fn artifact_app() -> axum::Router {
    static CACHED: std::sync::OnceLock<(
        Arc<AtlasData>,
        Arc<atlas_graph::GraphService>,
        Arc<atlas_core::sources::SourcesDocument>,
    )> = std::sync::OnceLock::new();
    let (data, graph, sources) = CACHED
        .get_or_init(|| {
            let compiled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled");
            let (graph, data) = atlas_contract::load::load_graph_and_data(&compiled)
                .expect("data/compiled/manifest.toml + sections/ must exist -- run atlas-graph-compile first");
            let sources = atlas_contract::load::load_sources(&compiled)
                .expect("data/compiled/sources.json must exist -- run `cargo run -p atlas-etl --bin gen_sources` first");
            assert!(
                data.events.is_empty() && data.places.is_empty() && data.narratives.is_empty(),
                "this builder's whole point is the REAL serving shape: AtlasData's events/places/narratives must be EMPTY here"
            );
            (Arc::new(data), Arc::new(graph), Arc::new(sources))
        })
        .clone();
    atlas_contract::load::LoadedAtlas { data, graph, sources }.into_router(None)
}

#[tokio::test]
async fn narrative_event_positions_has_adjacency_on_the_real_artifact_path() {
    let app = artifact_app();

    let (st, narratives, _) = get(&app, "/api/narratives").await;
    assert_eq!(st, 200);
    let exodus = narratives
        .as_array()
        .expect("/api/narratives serves an array")
        .iter()
        .find(|n| n["id"] == "exodus")
        .expect("the compiled atlas carries the 'exodus' narrative");
    let legs: Vec<&str> = exodus["legs"].as_array().expect("legs").iter().map(|l| l.as_str().unwrap()).collect();
    println!("exodus legs = {legs:?}");
    assert!(legs.len() >= 3, "this test needs a mid-chain leg, a head and a tail: {legs:?}");
    assert_eq!(legs[0], "ex_rameses");
    assert_eq!(legs[1], "ex_succoth");
    assert_eq!(legs[2], "ex_red_sea");

    let (st, body, _) = get(&app, "/api/narrative/event/ex_succoth").await;
    assert_eq!(st, 200);
    println!("ex_succoth = {}", serde_json::to_string(&body).unwrap());

    let row = body["narrative"]
        .as_array()
        .expect("narrative array")
        .iter()
        .find(|r| r["narrative_id"] == "exodus")
        .expect("ex_succoth is an exodus leg");
    assert_eq!(row["event_label"], "First camp at Succoth");

    for (half, expected_id, expected_label) in
        [("prior", "ex_rameses", "Israel departs Rameses"), ("following", "ex_red_sea", "Crossing the Red Sea")]
    {
        let adj = &row[half];
        assert!(
            !adj.is_null(),
            "ex_succoth is MID-CHAIN -- its `{half}` must be present; absent here is the OVERLAY-1 regression (AtlasData.events is empty on this path): {body}"
        );
        assert_eq!(adj["id"], expected_id, "{body}");
        assert_eq!(adj["label"], expected_label, "{body}");
        assert!(
            adj["places"].as_array().is_some_and(|p| !p.is_empty()),
            "`{half}.places` must be non-empty -- the client's traversal buttons are built from places[0]: {adj}"
        );
        assert!(
            adj["verse_groups"].as_array().is_some_and(|g| !g.is_empty()),
            "`{half}.verse_groups` must be populated -- the popover renders its passage list from it: {adj}"
        );
    }

    let timeline = &body["timeline"];
    assert!(!timeline.is_null(), "a dated event must carry a timeline position: {body}");
    for half in ["prior", "following"] {
        let adj = &timeline[half];
        assert!(!adj.is_null(), "`timeline.{half}` must be populated -- an empty timeline object IS the regression: {body}");
        assert!(adj["id"].as_str().is_some_and(|s| !s.is_empty()), "{adj}");
        assert!(adj["label"].as_str().is_some_and(|s| !s.is_empty()), "{adj}");
        assert!(adj["verse_groups"].as_array().is_some(), "{adj}");
    }
    assert_eq!(timeline["prior"]["id"], "ex_rameses", "{body}");
    assert_eq!(timeline["following"]["id"], "ex_red_sea", "{body}");

    let (st, head, _) = get(&app, &format!("/api/narrative/event/{}", legs[0])).await;
    assert_eq!(st, 200);
    let head_row = head["narrative"].as_array().unwrap().iter().find(|r| r["narrative_id"] == "exodus").unwrap();
    assert!(head_row["prior"].is_null(), "{} is exodus's first leg -- no narrative `prior`: {head}", legs[0]);
    assert_eq!(head_row["following"]["id"], legs[1], "{head}");
    assert!(!head["timeline"]["prior"].is_null(), "the global timeline is not the narrative chain: {head}");

    let tail = legs.last().unwrap();
    let (st, tail_body, _) = get(&app, &format!("/api/narrative/event/{tail}")).await;
    assert_eq!(st, 200);
    let tail_row = tail_body["narrative"].as_array().unwrap().iter().find(|r| r["narrative_id"] == "exodus").unwrap();
    assert!(tail_row["following"].is_null(), "{tail} is exodus's last leg -- no narrative `following`: {tail_body}");
    assert_eq!(tail_row["prior"]["id"], legs[legs.len() - 2], "{tail_body}");
}

#[tokio::test]
async fn person_card_carries_life_years_kin_and_events() {
    let app = artifact_app();

    let (st, body, _) = get(&app, "/api/node/Person:aaron_1").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["kind"], "Person");
    let person = &body["person"];
    assert_eq!(person["birth"]["value"], -1575, "{person}");
    assert_eq!(person["death"]["value"], -1452, "{person}");
    assert_eq!(person["eternal"], false);
    assert!(person["first"]["value"].is_i64() && person["last"]["value"].is_i64(), "the corpus-mention span is always computed: {person}");
    assert!(person["first"]["value"].as_i64().unwrap() <= person["last"]["value"].as_i64().unwrap());

    let count = |kind: &str| body["edge_summary"].as_array().unwrap().iter().find(|e| e["kind"] == kind).map(|e| e["count"].as_u64().unwrap()).unwrap_or(0);
    assert_eq!(count("child-of"), 2, "Amram and Jochebed: {}", body["edge_summary"]);
    assert_eq!(count("parent-of"), 4, "Nadab, Abihu, Eleazar, Ithamar: {}", body["edge_summary"]);
    assert_eq!(count("spouse-of"), 1, "Elisheba: {}", body["edge_summary"]);
    assert!(count("participates-in") >= 1, "Aaron's timeline must reach at least one real Event node: {}", body["edge_summary"]);

    let (st2, parents, _) = get(&app, "/api/node/Person:aaron_1/edges?kind=child-of").await;
    assert_eq!(st2, 200, "{parents}");
    let mut ids: Vec<String> = parents["entries"].as_array().unwrap().iter().map(|e| e["neighbour"]["node"]["id"].as_str().unwrap().to_string()).collect();
    ids.sort();
    assert_eq!(ids, vec!["Person:amram_242", "Person:jochebed_1645"]);

    let (st3, children, _) = get(&app, "/api/node/Person:amram_242/edges?kind=parent-of").await;
    assert_eq!(st3, 200, "{children}");
    assert!(children["entries"].as_array().unwrap().iter().any(|e| e["neighbour"]["node"]["id"] == "Person:aaron_1"), "{children}");

    let (st4, spouses, _) = get(&app, "/api/node/Person:elisheba_1162/edges?kind=spouse-of").await;
    assert_eq!(st4, 200, "{spouses}");
    assert_eq!(spouses["entries"].as_array().unwrap().len(), 1);
    assert_eq!(spouses["entries"][0]["neighbour"]["node"]["id"], "Person:aaron_1");

    let (_, event, _) = get(&app, "/api/node/Event:ab_ur").await;
    assert!(event.get("person").is_none(), "{event}");
}

#[tokio::test]
async fn god_is_eternal_and_the_card_says_why() {
    let app = artifact_app();
    let (st, body, _) = get(&app, "/api/node/Person:god_1324").await;
    assert_eq!(st, 200, "{body}");
    let person = &body["person"];
    assert_eq!(person["eternal"], true, "{person}");
    assert_eq!(person["birth"], serde_json::Value::Null);
    assert_eq!(person["death"], serde_json::Value::Null);
    let grounds: Vec<&str> = person["eternal_grounds"].as_array().unwrap().iter().map(|g| g.as_str().unwrap()).collect();
    assert_eq!(grounds, vec!["PSA.90.2", "REV.1.8"]);
}

const GENESIS_1: &str = "Container:bible-chapter-GEN-1";
const VERSES_IN_GENESIS_1: usize = 31;

#[tokio::test]
async fn the_card_for_genesis_1_names_its_kind_and_its_three_frontier_groups() {
    // Arrange
    let app = compiled_app();

    // Act
    let (status, body, _) = get(&app, &format!("/api/node/{GENESIS_1}")).await;

    // Assert
    assert_eq!(status, StatusCode::OK, "{body}");
    let version = body["version"].clone();
    assert_eq!(
        body,
        serde_json::json!({
            "id": GENESIS_1,
            "kind": "Container",
            "label": "Genesis 1",
            "provenance": "kjv",
            "edge_summary": [
                { "kind": "contains", "count": VERSES_IN_GENESIS_1 },
                { "kind": "member-of", "count": 1 },
                { "kind": "follows-in", "count": 1 },
            ],
            "version": version,
        })
    );
}

const AN_ANCHOR_WITH_ONE_DATING: &str = "Anchor:solomon-crowned";

#[test]
fn a_neighbour_is_published_as_a_node_position_or_an_edge_position_told_apart_by_its_position_tag() {
    // Arrange
    let document = serde_json::to_value(atlas_contract::document::openapi()).unwrap();
    let shapes = &document["components"]["schemas"];

    // Act
    let published = serde_json::json!({
        "PositionRef": shapes["PositionRef"],
        "NodePosition": shapes["NodePosition"],
        "EdgePosition": shapes["EdgePosition"],
        "NodeRef": shapes["NodeRef"],
        "EdgeRef": shapes["EdgeRef"],
        "PositionKind": shapes.get("PositionKind"),
        "neighbour": shapes["EdgeEntry"]["properties"]["neighbour"],
    });

    // Assert
    assert_eq!(
        published,
        serde_json::json!({
            "PositionRef": {
                "type": "object",
                "description": "What an edge leads to: a node, or, where one relation grounds another, the edge it grounds. `position` says which shape follows.",
                "required": ["position"],
                "properties": { "position": { "type": "string", "enum": ["node", "edge"] } },
                "additionalProperties": true,
                "discriminator": { "propertyName": "position", "mapping": { "node": "#/components/schemas/NodePosition", "edge": "#/components/schemas/EdgePosition" } },
            },
            "NodePosition": {
                "allOf": [
                    { "$ref": "#/components/schemas/PositionRef" },
                    { "type": "object", "required": ["node"], "properties": { "node": { "$ref": "#/components/schemas/NodeRef" } } },
                ],
                "description": "An edge's far end when it is a node.",
                "unevaluatedProperties": false,
            },
            "EdgePosition": {
                "allOf": [
                    { "$ref": "#/components/schemas/PositionRef" },
                    { "type": "object", "required": ["edge"], "properties": { "edge": { "$ref": "#/components/schemas/EdgeRef" } } },
                ],
                "description": "An edge's far end when it is another edge: a `justifies` page names the edge the node grounds.",
                "unevaluatedProperties": false,
            },
            "NodeRef": {
                "type": "object",
                "description": "A reference to a node: enough to show it, and the id to fetch it with.",
                "required": ["id", "kind", "label"],
                "properties": { "id": { "type": "string" }, "kind": { "$ref": "#/components/schemas/NodeKind" }, "label": { "type": "string" } },
                "additionalProperties": false,
            },
            "EdgeRef": {
                "type": "object",
                "description": "A reference to an edge: the id its own page carries as `edge`.",
                "required": ["id"],
                "properties": { "id": { "type": "string" } },
                "additionalProperties": false,
            },
            "PositionKind": null,
            "neighbour": { "$ref": "#/components/schemas/PositionRef" },
        })
    );
}

#[tokio::test]
async fn an_anchor_justifies_the_dating_it_grounds_and_that_neighbour_is_the_edge_itself() {
    // Arrange
    let app = compiled_app();
    let (_, dates, _) = get(&app, &format!("/api/node/{AN_ANCHOR_WITH_ONE_DATING}/edges?kind=dates")).await;

    // Act
    let (status, page, _) = get(&app, &format!("/api/node/{AN_ANCHOR_WITH_ONE_DATING}/edges?kind=justifies")).await;

    // Assert
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(
        page,
        serde_json::json!({
            "kind": "justifies",
            "entries": [{ "edge": page["entries"][0]["edge"], "neighbour": { "position": "edge", "edge": { "id": dates["entries"][0]["edge"] } } }],
            "next": null,
            "version": dates["version"],
        })
    );
}

const FIRST_ERA: &str = "primeval";
const FIRST_ERA_MAP: &str = "Map:era-primeval";
const FIRST_ERA_NAME: &str = "Primeval";
const THE_NEXT_MAP: usize = 1;
const FIRST_ERA_SHOWN_BY_KIND: [(&str, usize); 3] = [("Event", 5), ("Place", 235), ("Polity", 3)];

#[tokio::test]
async fn the_map_for_the_first_era_shows_its_events_places_and_polities_and_is_followed_by_the_next() {
    // Arrange
    let app = compiled_app();
    let (_, eras, _) = get(&app, "/api/eras").await;

    // Act
    let (status, card, _) = get(&app, &format!("/api/node/{FIRST_ERA_MAP}")).await;
    let shown = every_edge_of(&app, FIRST_ERA_MAP, "shows").await;

    // Assert
    assert_eq!(eras[0]["id"], FIRST_ERA);
    assert_eq!(status, StatusCode::OK, "{card}");
    let version = card["version"].clone();
    let shows: usize = FIRST_ERA_SHOWN_BY_KIND.iter().map(|(_, n)| n).sum();
    assert_eq!(
        card,
        serde_json::json!({
            "id": FIRST_ERA_MAP,
            "kind": "Map",
            "label": FIRST_ERA_NAME,
            "provenance": "curated-eras",
            "edge_summary": [
                { "kind": "follows-in", "count": THE_NEXT_MAP },
                { "kind": "shows", "count": shows },
            ],
            "version": version,
        })
    );
    let mut shown_by_kind: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for entry in &shown {
        *shown_by_kind.entry(entry["neighbour"]["node"]["kind"].as_str().unwrap()).or_default() += 1;
    }
    assert_eq!(shown_by_kind, std::collections::BTreeMap::from(FIRST_ERA_SHOWN_BY_KIND));
}

async fn every_edge_of(app: &axum::Router, id: &str, kind: &str) -> Vec<serde_json::Value> {
    let mut entries: Vec<serde_json::Value> = Vec::new();
    let mut cursor: Option<u64> = None;
    loop {
        let uri = match cursor {
            Some(c) => format!("/api/node/{id}/edges?kind={kind}&limit=200&cursor={c}"),
            None => format!("/api/node/{id}/edges?kind={kind}&limit=200"),
        };
        let (status, page, _) = get(app, &uri).await;
        assert_eq!(status, StatusCode::OK, "{page}");
        entries.extend(page["entries"].as_array().unwrap().iter().cloned());
        match page["next"].as_u64() {
            Some(next) => cursor = Some(next),
            None => break entries,
        }
    }
}

const BIBLE_ROOT: &str = "Container:bible";
const CONCORD_ROOT: &str = "Container:concord";
const BOOKS_IN_THE_BIBLE: usize = 66;
const DOCUMENTS_IN_THE_CONCORD: usize = 10;

#[tokio::test]
async fn each_corpus_has_one_root_that_contains_its_top_level_containers() {
    // Arrange
    let app = compiled_app();

    // Act
    let (bible_status, bible, _) = get(&app, &format!("/api/node/{BIBLE_ROOT}")).await;
    let (concord_status, concord, _) = get(&app, &format!("/api/node/{CONCORD_ROOT}")).await;

    // Assert
    assert_eq!((bible_status, concord_status), (StatusCode::OK, StatusCode::OK), "{bible} {concord}");
    let (bible_version, concord_version) = (bible["version"].clone(), concord["version"].clone());
    assert_eq!(
        bible,
        serde_json::json!({
            "id": BIBLE_ROOT,
            "kind": "Container",
            "label": "The Holy Bible",
            "provenance": "kjv",
            "edge_summary": [ { "kind": "contains", "count": BOOKS_IN_THE_BIBLE } ],
            "version": bible_version,
        })
    );
    assert_eq!(
        concord,
        serde_json::json!({
            "id": CONCORD_ROOT,
            "kind": "Container",
            "label": "The Book of Concord",
            "provenance": "concord",
            "edge_summary": [ { "kind": "contains", "count": DOCUMENTS_IN_THE_CONCORD } ],
            "version": concord_version,
        })
    );
}

#[tokio::test]
async fn genesis_is_a_member_of_the_bible_root() {
    // Arrange
    let app = compiled_app();

    // Act
    let (status, page, _) = get(&app, "/api/node/Container:bible-book-GEN/edges?kind=member-of").await;

    // Assert
    assert_eq!(status, StatusCode::OK, "{page}");
    let version = page["version"].clone();
    assert_eq!(
        page,
        serde_json::json!({
            "kind": "member-of",
            "entries": [ { "edge": page["entries"][0]["edge"].clone(), "neighbour": { "position": "node", "node": { "id": BIBLE_ROOT, "kind": "Container", "label": "The Holy Bible" } } } ],
            "next": null,
            "version": version,
        })
    );
}


const SMALL_CATECHISM: &str = "Container:concord-doc-small-catechism";
const LARGE_CATECHISM: &str = "Container:concord-doc-large-catechism";
const TEN_COMMANDMENTS: &str = "Container:concord-art-small-catechism-2";
const THE_CREED: &str = "Container:concord-art-small-catechism-3";

#[tokio::test]
async fn the_small_catechism_is_followed_by_the_large_and_the_commandments_by_the_creed() {
    // Arrange
    let app = compiled_app();

    // Act
    let (documents_status, documents, _) = get(&app, &format!("/api/node/{SMALL_CATECHISM}/edges?kind=follows-in")).await;
    let (articles_status, articles, _) = get(&app, &format!("/api/node/{TEN_COMMANDMENTS}/edges?kind=follows-in")).await;

    // Assert
    assert_eq!((documents_status, articles_status), (StatusCode::OK, StatusCode::OK), "{documents} {articles}");
    assert_eq!(
        documents,
        serde_json::json!({
            "kind": "follows-in",
            "entries": [ { "edge": documents["entries"][0]["edge"].clone(), "neighbour": { "position": "node", "node": { "id": LARGE_CATECHISM, "kind": "Container", "label": "The Large Catechism" } } } ],
            "next": null,
            "version": documents["version"].clone(),
        })
    );
    assert_eq!(
        articles,
        serde_json::json!({
            "kind": "follows-in",
            "entries": [ { "edge": articles["entries"][0]["edge"].clone(), "neighbour": { "position": "node", "node": { "id": THE_CREED, "kind": "Container", "label": "II. The Creed" } } } ],
            "next": null,
            "version": articles["version"].clone(),
        })
    );
}

const MOSES: &str = "Person:moses_2108";

#[tokio::test]
async fn a_person_card_carries_labelled_years() {
    // Arrange
    let app = compiled_app();

    // Act
    let (status, card, _) = get(&app, &format!("/api/node/{MOSES}")).await;

    // Assert
    assert_eq!(status, StatusCode::OK, "{card}");
    assert_eq!(
        card["person"],
        serde_json::json!({
            "gender": "Male",
            "birth": { "value": -1572, "label": "1572 BC" },
            "death": { "value": -1453, "label": "1453 BC" },
            "first": { "value": -1700, "label": "1700 BC" },
            "last": { "value": 96, "label": "AD 96" },
            "eternal": false,
            "eternal_grounds": [],
            "also_called": [],
        })
    );
}

#[tokio::test]
async fn a_text_unit_carries_its_structured_locus_beside_its_ref() {
    // Arrange
    let app = compiled_app();

    // Act
    let (status, window, _) = get(&app, "/api/text?ref=GEN.1.1&n=1").await;

    // Assert
    assert_eq!(status, StatusCode::OK, "{window}");
    assert_eq!(
        window["units"],
        serde_json::json!([{
            "ref": "GEN.1.1",
            "locus": { "corpus": "bible", "book": "GEN", "chapter": 1, "verse": 1 },
            "text": "In the beginning God created the heaven and the earth.",
            "words_of_christ": [],
            "heading": the_creation_heading(),
            "anchors": [god_named_at(17)],
            "edge_summary": [
                { "kind": "member-of", "count": 1 },
                { "kind": "attests", "count": 1 },
                { "kind": "mentions", "count": 1 },
                { "kind": "cites", "count": 61 },
                { "kind": "cited-by", "count": 35 },
                { "kind": "commented-on-by", "count": 1 },
                { "kind": "catechism-link", "count": 3 },
            ],
        }])
    );
}

#[tokio::test]
async fn a_concord_paragraph_carries_its_structured_locus_beside_its_ref() {
    // Arrange
    let app = compiled_app();

    // Act
    let (status, window, _) = get(&app, "/api/text?ref=BoC%207.2.1&corpus=concord").await;

    // Assert
    assert_eq!(status, StatusCode::OK, "{window}");
    assert_eq!(
        window["units"],
        serde_json::json!([{
            "ref": "BoC 7.2.1",
            "locus": { "corpus": "concord", "part": 7, "article": 2, "paragraph": 1 },
            "text": "Thou shalt have no other gods. What does this mean? \u{2013}Answer: We should fear, love, and trust in God above all things.",
            "words_of_christ": [],
            "anchors": [],
            "edge_summary": [{ "kind": "member-of", "count": 1 }, { "kind": "catechism-link", "count": 1 }],
        }])
    );
}

#[tokio::test]
async fn a_contents_child_carries_the_locus_it_opens_at() {
    // Arrange
    let app = compiled_app();

    // Act
    let (status, contents, _) = get(&app, "/api/contents/bible").await;

    // Assert
    assert_eq!(status, StatusCode::OK, "{contents}");
    assert_eq!(
        contents["roots"][0]["children"][0],
        serde_json::json!({
            "id": GENESIS_1,
            "title": "1",
            "kind": "chapter",
            "ref": "GEN.1",
            "locus": { "corpus": "bible", "book": "GEN", "chapter": 1, "verse": 1 },
            "count": VERSES_IN_GENESIS_1,
        })
    );
}

const PARAGRAPHS_IN_THE_PREFACE: usize = 25;

#[tokio::test]
async fn a_contents_root_opens_at_the_locus_its_first_child_opens_at() {
    // Arrange
    let app = compiled_app();
    let preface_opens_at = serde_json::json!({ "corpus": "concord", "part": 1, "article": 1, "paragraph": 1 });

    // Act
    let (status, contents, _) = get(&app, "/api/contents/concord").await;

    // Assert
    assert_eq!(status, StatusCode::OK, "{contents}");
    assert_eq!(
        contents["roots"][0],
        serde_json::json!({
            "id": "Container:concord-doc-preface",
            "title": "Preface to the Book of Concord",
            "kind": "document",
            "ref": "BoC 1.1.1",
            "locus": preface_opens_at,
            "children": [{
                "id": "Container:concord-art-preface-1",
                "title": "Preface to the Book of Concord",
                "kind": "article",
                "ref": "BoC 1.1.1",
                "locus": preface_opens_at,
                "count": PARAGRAPHS_IN_THE_PREFACE,
            }],
        })
    );
}

#[tokio::test]
async fn a_cites_entry_carries_the_votes_the_cross_reference_route_counts() {
    // Arrange
    let app = compiled_app();
    let (_, first, _) = get(&app, "/api/node/text-unit:JHN.3.16/edges?kind=cites&limit=1").await;
    let (_, legacy, _) = get(&app, "/api/xrefs/JHN.3.16").await;
    let cited = &first["entries"][0]["neighbour"]["node"];
    let votes = legacy.as_array().unwrap().iter().find(|x| format!("text-unit:{}", x["target"].as_str().unwrap()) == cited["id"]).expect("the route lists the same citation")["votes"].clone();
    // Act
    let (status, page, _) = get(&app, "/api/node/text-unit:JHN.3.16/edges?kind=cites&limit=1").await;
    // Assert
    assert_eq!(
        (status, page["entries"].clone()),
        (StatusCode::OK, serde_json::json!([{ "edge": page["entries"][0]["edge"], "neighbour": { "position": "node", "node": cited }, "votes": votes }]))
    );
}

#[tokio::test]
async fn a_follows_in_entry_carries_the_narrative_its_leg_belongs_to() {
    // Arrange
    let app = compiled_app();
    let (_, legacy, _) = get(&app, "/api/narrative/event/ab_ur").await;
    // Act
    let (status, page, _) = get(&app, "/api/node/Event:ab_ur/edges?kind=follows-in").await;
    // Assert
    assert_eq!(
        (status, page["entries"].clone()),
        (
            StatusCode::OK,
            serde_json::json!([{
                "edge": page["entries"][0]["edge"],
                "neighbour": { "position": "node", "node": { "id": "Event:ab_haran", "kind": "Event", "label": legacy["narrative"][0]["following"]["label"] } },
                "narrative": legacy["narrative"][0]["narrative_id"],
            }])
        )
    );
}

#[tokio::test]
async fn an_attested_in_entry_carries_its_accounts_runs_across_a_gap() {
    // Arrange
    let app = compiled_app();
    let (_, legacy, _) = get(&app, "/api/event/ab_ur").await;
    // Act
    let (status, page, _) = get(&app, "/api/node/Event:ab_ur/edges?kind=attested-in&limit=1").await;
    // Assert
    assert_eq!(
        (status, legacy["witnesses"][0]["verse_groups"].clone(), page["entries"].clone()),
        (
            StatusCode::OK,
            serde_json::json!([{ "book": "GEN", "chapter": 11, "verses": ["GEN.11.28", "GEN.11.31"], "count": 2 }]),
            serde_json::json!([{
                "edge": page["entries"][0]["edge"],
                "neighbour": { "position": "node", "node": { "id": "text-unit:GEN.11.28", "kind": "TextUnit", "label": "GEN.11.28" } },
                "loci": [whole_verse("GEN", 11, 28), whole_verse("GEN", 11, 31)],
            }])
        )
    );
}

#[tokio::test]
async fn an_attested_in_entry_carries_its_accounts_run_and_the_note_on_how_the_account_is_cited() {
    // Arrange
    let app = compiled_app();
    let (_, legacy, _) = get(&app, "/api/event/pw_bethany").await;
    let matthew = legacy["witnesses"].as_array().unwrap().iter().find(|w| w["book"] == "MAT").expect("Matthew recounts the anointing").clone();
    // Act
    let (status, page, _) = get(&app, "/api/node/Event:pw_bethany/edges?kind=attested-in&limit=1").await;
    // Assert
    assert_eq!(
        (status, page["entries"].clone()),
        (
            StatusCode::OK,
            serde_json::json!([{
                "edge": page["entries"][0]["edge"],
                "neighbour": { "position": "node", "node": { "id": "text-unit:MAT.26.6", "kind": "TextUnit", "label": "MAT.26.6" } },
                "loci": [{ "from": { "unit": bible_unit("MAT", 26, 6) }, "to": { "unit": bible_unit("MAT", 26, 13) } }],
                "note": matthew["ref_note"],
            }])
        )
    );
}

#[tokio::test]
async fn an_attests_entry_carries_the_verse_it_stands_on_and_the_note_on_its_account() {
    // Arrange
    let app = compiled_app();
    let (_, legacy, _) = get(&app, "/api/event/pw_bethany").await;
    let matthew = legacy["witnesses"].as_array().unwrap().iter().find(|w| w["book"] == "MAT").expect("Matthew recounts the anointing").clone();
    // Act
    let (status, page, _) = get(&app, "/api/node/text-unit:MAT.26.6/edges?kind=attests").await;
    // Assert
    assert_eq!(
        (status, page["entries"].clone()),
        (
            StatusCode::OK,
            serde_json::json!([{
                "edge": page["entries"][0]["edge"],
                "neighbour": { "position": "node", "node": { "id": "Event:pw_bethany", "kind": "Event", "label": legacy["title"] } },
                "loci": [whole_verse("MAT", 26, 6)],
                "note": matthew["ref_note"],
            }])
        )
    );
}

fn bible_unit(book: &str, chapter: u16, verse: u16) -> serde_json::Value {
    serde_json::json!({ "corpus": "bible", "book": book, "chapter": chapter, "verse": verse })
}

fn whole_verse(book: &str, chapter: u16, verse: u16) -> serde_json::Value {
    serde_json::json!({ "from": { "unit": bible_unit(book, chapter, verse) }, "to": { "unit": bible_unit(book, chapter, verse) } })
}

fn god_named_at(start: usize) -> serde_json::Value {
    serde_json::json!({ "start": start, "end": start + "God".chars().count(), "kind": "mentions", "node": { "id": "Person:god_1324", "kind": "Person", "label": "God" } })
}

fn the_creation_heading() -> serde_json::Value {
    serde_json::json!({ "event": { "id": "Event:theo-1", "kind": "Event", "label": "Creation of all things" }, "kind": "event", "is_continuation": false })
}

#[tokio::test]
async fn an_attested_in_entry_runs_its_account_on_across_every_chapter_read_to_its_end() {
    // Arrange
    let app = compiled_app();
    let (_, legacy, _) = get(&app, "/api/event/rob_sermon_on_the_mount").await;
    let matthew = legacy["witnesses"].as_array().unwrap().iter().find(|w| w["book"] == "MAT").expect("Matthew records the sermon").clone();
    // Act
    let (status, page, _) = get(&app, "/api/node/Event:rob_sermon_on_the_mount/edges?kind=attested-in&limit=1").await;
    // Assert
    assert_eq!(
        (status, page["entries"].clone()),
        (
            StatusCode::OK,
            serde_json::json!([{
                "edge": page["entries"][0]["edge"],
                "neighbour": { "position": "node", "node": { "id": "text-unit:MAT.5.1", "kind": "TextUnit", "label": "MAT.5.1" } },
                "loci": [{ "from": { "unit": bible_unit("MAT", 5, 1) }, "to": { "unit": bible_unit("MAT", 7, 29) } }],
                "note": matthew["ref_note"],
            }])
        )
    );
}

#[tokio::test]
async fn an_event_card_carries_the_details_the_legacy_route_served() {
    // Arrange
    let app = compiled_app();
    let (_, legacy, _) = get(&app, "/api/event/rob_sermon_on_the_mount").await;
    // Act
    let (status, card, _) = get(&app, "/api/node/Event:rob_sermon_on_the_mount").await;
    // Assert
    assert_eq!(
        (status, card["event"].clone()),
        (
            StatusCode::OK,
            serde_json::json!({
                "kind": "event",
                "when": { "from": { "value": 31, "label": "AD 31" }, "to": { "value": 31, "label": "AD 31" }, "label": "AD 31" },
                "robertson_section": legacy["robertson_section"],
                "ref_note": legacy["ref_note"],
            })
        )
    );
}

#[tokio::test]
async fn a_titled_passages_card_carries_its_kind_and_no_date() {
    // Arrange
    let app = compiled_app();
    let (_, legacy, _) = get(&app, "/api/event/gen_line_of_cain").await;
    // Act
    let (status, card, _) = get(&app, "/api/node/Event:gen_line_of_cain").await;
    // Assert
    assert_eq!(
        (status, card["event"].clone()),
        (StatusCode::OK, serde_json::json!({ "kind": "general", "atlas_section": legacy["atlas_section"], "ref_note": legacy["ref_note"] }))
    );
}

#[tokio::test]
async fn a_place_card_carries_its_coordinates_its_name_and_its_dated_founding_and_fall() {
    // Arrange
    let app = compiled_app();
    let (_, legacy, _) = get(&app, "/api/place/jerusalem").await;
    // Act
    let (status, card, _) = get(&app, "/api/node/Place:jerusalem").await;
    // Assert
    assert_eq!(
        (status, card["place"].clone()),
        (
            StatusCode::OK,
            serde_json::json!({
                "lat": legacy["lat"],
                "lon": legacy["lon"],
                "display_name": "Jerusalem",
                "established": {
                    "when": { "from": { "value": -1003, "label": "1003 BC" }, "to": { "value": -1003, "label": "1003 BC" }, "label": "1003 BC" },
                    "label": "c. 1003 BC",
                    "verses": [whole_verse("2SA", 5, 6), whole_verse("2SA", 5, 7), whole_verse("2SA", 5, 9)],
                    "note": "traditional",
                    "event": { "id": "Event:sam2_jerusalem_captured", "kind": "Event", "label": "David captures Jerusalem and makes it his capital" },
                },
                "destroyed": {
                    "when": { "from": { "value": -586, "label": "586 BC" }, "to": { "value": -586, "label": "586 BC" }, "label": "586 BC" },
                    "label": "586 BC",
                    "verses": [whole_verse("2KI", 25, 9), whole_verse("2KI", 25, 10)],
                    "event": { "id": "Event:exl_jerusalem", "kind": "Event", "label": "Jerusalem falls to Babylon" },
                },
            })
        )
    );
}

#[tokio::test]
async fn a_catechism_card_carries_its_prose() {
    // Arrange
    let app = compiled_app();
    let (_, legacy, _) = get(&app, "/api/catechism/item/commandment-1").await;
    // Act
    let (status, card, _) = get(&app, "/api/node/CatechismItem:commandment-1").await;
    // Assert
    assert_eq!(
        (status, card["catechism"].clone()),
        (
            StatusCode::OK,
            serde_json::json!({
                "part_title": "The Ten Commandments",
                "text": "Thou shalt have no other gods.",
                "explanation_heading": legacy["explanation_heading"],
                "explanation": legacy["explanation"],
            })
        )
    );
}

#[tokio::test]
async fn a_place_the_kjv_names_otherwise_carries_that_name_beside_its_canonical_one() {
    // Arrange
    let app = compiled_app();
    let (_, legacy, _) = get(&app, "/api/place/tigris").await;
    // Act
    let (status, card, _) = get(&app, "/api/node/Place:tigris").await;
    // Assert
    assert_eq!(
        (status, card["place"].clone()),
        (StatusCode::OK, serde_json::json!({ "lat": legacy["lat"], "lon": legacy["lon"], "display_name": "Hiddekel", "canonical_name": "Tigris" }))
    );
}

#[tokio::test]
async fn a_catechism_item_that_quotes_scripture_says_where_it_is_written() {
    // Arrange
    let app = compiled_app();
    let (_, legacy, _) = get(&app, "/api/catechism/item/baptism-1").await;
    // Act
    let (status, card, _) = get(&app, "/api/node/CatechismItem:baptism-1").await;
    // Assert
    assert_eq!(
        (status, card["catechism"].clone()),
        (
            StatusCode::OK,
            serde_json::json!({
                "part_title": "The Sacrament of Holy Baptism",
                "explanation_heading": "What is Baptism?",
                "explanation": legacy["explanation"],
                "where_written": legacy["where_written"],
            })
        )
    );
}

#[tokio::test]
async fn a_book_card_carries_its_authorship_and_its_writing() {
    // Arrange
    let app = compiled_app();
    let (_, legacy, _) = get(&app, "/api/verse/NEH.1.1").await;
    // Act
    let (status, card, _) = get(&app, "/api/node/Container:bible-book-NEH").await;
    // Assert
    assert_eq!(
        (status, card["book"].clone()),
        (
            StatusCode::OK,
            serde_json::json!({
                "author": legacy["book_meta"]["author"],
                "write_place": { "id": "Place:jerusalem", "kind": "Place", "label": "Jerusalem" },
                "written": { "from": { "value": -430, "label": "430 BC" }, "to": { "value": -400, "label": "400 BC" }, "label": "430 – 400 BC" },
            })
        )
    );
}

#[tokio::test]
async fn a_verse_that_opens_a_pericope_carries_its_heading_and_the_verse_after_it_none() {
    // Arrange
    let app = compiled_app();
    // Act
    let (status, window, _) = get(&app, "/api/text?ref=GEN.1.1&n=2").await;
    // Assert
    assert_eq!(
        (status, window["units"].clone()),
        (
            StatusCode::OK,
            serde_json::json!([
                {
                    "ref": "GEN.1.1",
                    "locus": bible_unit("GEN", 1, 1),
                    "text": "In the beginning God created the heaven and the earth.",
                    "words_of_christ": [],
                    "heading": the_creation_heading(),
                    "anchors": [god_named_at(17)],
                    "edge_summary": [
                        { "kind": "member-of", "count": 1 },
                        { "kind": "attests", "count": 1 },
                        { "kind": "mentions", "count": 1 },
                        { "kind": "cites", "count": 61 },
                        { "kind": "cited-by", "count": 35 },
                        { "kind": "commented-on-by", "count": 1 },
                        { "kind": "catechism-link", "count": 3 },
                    ],
                },
                {
                    "ref": "GEN.1.2",
                    "locus": bible_unit("GEN", 1, 2),
                    "text": "And the earth was without form and void; and darkness was upon the face of the deep. And the Spirit of God moved upon the face of the waters.",
                    "words_of_christ": [],
                    "anchors": [god_named_at(103)],
                    "edge_summary": [
                        { "kind": "member-of", "count": 1 },
                        { "kind": "attests", "count": 1 },
                        { "kind": "mentions", "count": 2 },
                        { "kind": "cites", "count": 7 },
                        { "kind": "cited-by", "count": 9 },
                        { "kind": "commented-on-by", "count": 3 },
                        { "kind": "catechism-link", "count": 3 },
                    ],
                },
            ])
        )
    );
}

#[tokio::test]
async fn a_verse_that_names_hazor_serves_an_anchor_over_the_name() {
    // Arrange
    let app = compiled_app();
    // Act
    let (status, window, _) = get(&app, "/api/text?ref=JOS.11.1").await;
    // Assert
    assert_eq!(
        (status, window["units"][0]["text"].clone(), window["units"][0]["anchors"].clone()),
        (
            StatusCode::OK,
            serde_json::json!("And it came to pass, when Jabin king of Hazor had heard those things, that he sent to Jobab king of Madon, and to the king of Shimron, and to the king of Achshaph,"),
            serde_json::json!([
                { "start": 26, "end": 31, "kind": "mentions", "node": { "id": "Person:jabin_676", "kind": "Person", "label": "Jabin" } },
                { "start": 40, "end": 45, "kind": "mentions", "node": { "id": "Place:hazor-1", "kind": "Place", "label": "Hazor 1" } },
                { "start": 86, "end": 91, "kind": "mentions", "node": { "id": "Person:jobab_1642", "kind": "Person", "label": "Jobab" } },
                { "start": 100, "end": 105, "kind": "mentions", "node": { "id": "Place:madon", "kind": "Place", "label": "Madon" } },
                { "start": 126, "end": 133, "kind": "mentions", "node": { "id": "Place:shimron", "kind": "Place", "label": "Shimron" } },
                { "start": 154, "end": 162, "kind": "mentions", "node": { "id": "Place:achshaph", "kind": "Place", "label": "Achshaph" } },
            ])
        )
    );
}

#[tokio::test]
async fn a_small_catechism_paragraph_that_cites_scripture_serves_cites_anchors() {
    // Arrange
    let app = compiled_app();
    // Act
    let (status, window, _) = get(&app, "/api/text?ref=BoC%207.9.6&corpus=concord").await;
    // Assert
    assert_eq!(
        (status, window["units"][0]["text"].clone(), window["units"][0]["anchors"].clone()),
        (
            StatusCode::OK,
            serde_json::json!("For Wives. Wives, submit yourselves unto your own husbands, as unto the Lord, even as Sarah obeyed Abraham, calling him lord; whose daughters ye are, as long as ye do well, and are not afraid with any amazement. 1 Pet. 3:6; Eph. 5:22."),
            serde_json::json!([
                { "start": 212, "end": 222, "kind": "cites", "node": { "id": "text-unit:1PE.3.6", "kind": "TextUnit", "label": "1PE.3.6" } },
                { "start": 224, "end": 233, "kind": "cites", "node": { "id": "text-unit:EPH.5.22", "kind": "TextUnit", "label": "EPH.5.22" } },
            ])
        )
    );
}

#[tokio::test]
async fn jesus_is_the_eternal_son_of_god_born_of_the_virgin_mary_and_the_supposed_son_of_joseph() {
    // Arrange
    let app = compiled_app();
    // Act
    let (status, page, _) = get(&app, "/api/node/Person:jesus_905/edges?kind=child-of").await;
    // Assert
    assert_eq!(
        (status, page["entries"].clone()),
        (
            StatusCode::OK,
            serde_json::json!([
                { "edge": page["entries"][0]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:god_1324", "kind": "Person", "label": "God" } }, "parentage": "eternal" },
                { "edge": page["entries"][1]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:joseph_1715", "kind": "Person", "label": "Joseph (Mary's Husband)" } }, "parentage": "legal" },
                { "edge": page["entries"][2]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:mary_1938", "kind": "Person", "label": "Mary (Mother of Jesus)" } }, "parentage": "virgin" },
            ])
        )
    );
}

#[tokio::test]
async fn abrahams_spouses_are_hagar_keturah_and_sarah() {
    // Arrange
    let app = compiled_app();
    // Act
    let (status, page, _) = get(&app, "/api/node/Person:abraham_58/edges?kind=spouse-of").await;
    // Assert
    assert_eq!(
        (status, page["kind"].clone(), page["entries"].clone()),
        (
            StatusCode::OK,
            serde_json::json!("spouse-of"),
            serde_json::json!([
                { "edge": page["entries"][0]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:hagar_1348", "kind": "Person", "label": "Hagar" } } },
                { "edge": page["entries"][1]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:keturah_1782", "kind": "Person", "label": "Keturah" } } },
                { "edge": page["entries"][2]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:sarah_2473", "kind": "Person", "label": "Sarah" } } },
            ])
        )
    );
}

#[tokio::test]
async fn adam_and_eve_were_created_by_god_and_seth_was_born_to_them() {
    // Arrange
    let app = compiled_app();
    // Act
    let (adam_status, adam, _) = get(&app, "/api/node/Person:adam_78/edges?kind=child-of").await;
    let (eve_status, eve, _) = get(&app, "/api/node/Person:eve_1231/edges?kind=child-of").await;
    let (seth_status, seth, _) = get(&app, "/api/node/Person:seth_2504/edges?kind=child-of").await;
    // Assert
    assert_eq!(
        (adam_status, adam["entries"].clone(), eve_status, eve["entries"].clone(), seth_status, seth["entries"].clone()),
        (
            StatusCode::OK,
            serde_json::json!([{ "edge": adam["entries"][0]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:god_1324", "kind": "Person", "label": "God" } }, "parentage": "created" }]),
            StatusCode::OK,
            serde_json::json!([{ "edge": eve["entries"][0]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:god_1324", "kind": "Person", "label": "God" } }, "parentage": "created" }]),
            StatusCode::OK,
            serde_json::json!([
                { "edge": seth["entries"][0]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:adam_78", "kind": "Person", "label": "Adam" } }, "parentage": "natural" },
                { "edge": seth["entries"][1]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:eve_1231", "kind": "Person", "label": "Eve" } }, "parentage": "natural" },
            ]),
        )
    );
}

#[tokio::test]
async fn the_brethren_of_jesus_are_james_joses_simon_and_jude() {
    // Arrange
    let app = compiled_app();
    // Act
    let (status, page, _) = get(&app, "/api/node/Person:jesus_905/edges?kind=brethren-of").await;
    // Assert
    assert_eq!(
        (status, page["entries"].clone()),
        (
            StatusCode::OK,
            serde_json::json!([
                { "edge": page["entries"][0]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:james_719", "kind": "Person", "label": "James (Brother of Jesus)" } } },
                { "edge": page["entries"][1]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:joses_1721", "kind": "Person", "label": "Joses" } } },
                { "edge": page["entries"][2]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:jude_1756", "kind": "Person", "label": "Jude" } } },
                { "edge": page["entries"][3]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:simon_2747", "kind": "Person", "label": "Simon" } } },
            ])
        )
    );
}

#[tokio::test]
async fn the_virgin_mary_is_the_mother_of_jesus_only_and_joseph_his_father_only_as_was_supposed() {
    // Arrange
    let app = compiled_app();
    // Act
    let (mary_status, mary, _) = get(&app, "/api/node/Person:mary_1938/edges?kind=parent-of").await;
    let (joseph_status, joseph, _) = get(&app, "/api/node/Person:joseph_1715/edges?kind=parent-of").await;
    let (james_status, james, _) = get(&app, "/api/node/Person:james_719/edges?kind=child-of").await;
    // Assert
    assert_eq!(
        (mary_status, mary["entries"].clone(), joseph_status, joseph["entries"].clone(), james_status, james["entries"].clone()),
        (
            StatusCode::OK,
            serde_json::json!([{ "edge": mary["entries"][0]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:jesus_905", "kind": "Person", "label": "Jesus Christ" } }, "parentage": "virgin" }]),
            StatusCode::OK,
            serde_json::json!([{ "edge": joseph["entries"][0]["edge"], "neighbour": { "position": "node", "node": { "id": "Person:jesus_905", "kind": "Person", "label": "Jesus Christ" } }, "parentage": "legal" }]),
            StatusCode::OK,
            serde_json::json!([]),
        )
    );
}
