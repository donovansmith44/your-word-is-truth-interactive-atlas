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

// M-C2 DELETION EVENT: `AtlasData::load`'s own five retiring-file reads
// return empty now -- `atlas_etl::compile::compile` is this crate's own
// real-data source from here on. Cached (`OnceLock`) so this file's own
// 15 `real_app()` call sites share one real compile -- `GraphService::
// build` itself still runs fresh per call (unchanged from before this
// batch), since a real app needs its own graph instance.
fn real_atlas_data() -> AtlasData {
    static CACHED: std::sync::OnceLock<AtlasData> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify")
                .data
        })
        .clone()
}

fn real_app() -> axum::Router {
    let raw = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let data = real_atlas_data();
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

// ---------------------------------------------------------------------
// CORP-2a (decision 8): corpus=concord through the SAME `/api/text`
// route -- `real_app()` above already carries the real Book of Concord
// data (`GraphService::build` loads `data/raw/concord/` + `data/curated/
// concord-sc-overlap.toml` automatically, the identical "real vendored
// tree present -> real content" path brainfuel/eras already use), so
// these ARE the real, vendored First Commandment/Article IV text, not a
// fixture double.
// ---------------------------------------------------------------------

#[tokio::test]
async fn text_window_concord_single_paragraph_is_the_real_sc_first_commandment() {
    let app = real_app();
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
    let app = real_app();
    // Augsburg Confession (part 3), Article IV (Of Justification), which
    // this batch's own report quotes as a 3-paragraph article.
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
    let app = real_app();
    let (st, body, _) = get(&app, "/api/text?ref=BoC%207.2.1&scope=chapter&corpus=concord").await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(body["error"]["code"], "bad_dir");

    let (st, body, _) = get(&app, "/api/text?ref=JHN.3.16&corpus=concord").await;
    assert_eq!(st, 400, "a Bible-shaped ref under corpus=concord is bad_ref, never silently reinterpreted: {body}");
    assert_eq!(body["error"]["code"], "bad_ref");

    let (st, body, _) = get(&app, "/api/text?ref=BoC%207.2.1&corpus=lxx").await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(body["error"]["code"], "bad_corpus");
}

#[tokio::test]
async fn text_window_bible_default_corpus_is_unchanged_by_the_new_param() {
    // Zero client changes (decision 8): omitting `corpus` entirely must
    // still serve the Bible corpus, byte-identical to `text_window_
    // single_verse_matches_the_compiled_verse_map` above.
    let app = real_app();
    let (st, body, _) = get(&app, "/api/text?ref=JHN.3.16&corpus=bible").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["units"][0]["text"].as_str().unwrap(), "For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life.");
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

/// Batch P (the extensibility proof): a Person's own card + "mentioned-in"
/// frontier, served by the SAME generic endpoints every other kind above
/// already uses -- zero new bespoke endpoints, the batch's own thesis.
/// `aaron_1` is a real Theographic person (`data/raw/theographic/.../
/// people.json`); `331` is that record's own real, committed
/// `verseCount`/resolved `verse_links` length -- an end-to-end fidelity
/// check (raw source count -> ETL -> graph adapter -> this wire), not a
/// number invented for the test.
#[tokio::test]
async fn person_card_and_mentioned_in_frontier_are_served_by_the_generic_endpoints() {
    let app = real_app();

    let (st, body, _) = get(&app, "/api/node/Person:aaron_1").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["id"], "Person:aaron_1");
    assert_eq!(body["kind"], "Person");
    assert_eq!(body["label"], "Aaron");
    assert_eq!(body["provenance"], "theographic-people");
    let summary: Vec<serde_json::Value> = body["edge_summary"].as_array().unwrap().clone();
    let mentioned_in = summary.iter().find(|e| e["kind"] == "mentioned-in").expect("aaron_1 must carry a real mentioned-in frontier");
    assert_eq!(mentioned_in["count"], 331, "must equal the real Theographic record's own resolved verse_links count");

    let (st2, page, _) = get(&app, "/api/node/Person:aaron_1/edges?kind=mentioned-in&limit=3").await;
    assert_eq!(st2, 200, "{page}");
    let entries = page["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    // CANON ORDER (batch-p-brief.md: "mentioned-in: every mention in
    // Scripture, canon order") -- the graph's own bijection-witnessed row
    // order, over real data, not a fixture.
    assert_eq!(entries[0]["node"]["id"], "text-unit:EXO.4.14");
    assert_eq!(entries[1]["node"]["id"], "text-unit:EXO.4.27");
    assert_eq!(entries[2]["node"]["id"], "text-unit:EXO.4.28");
    assert_eq!(page["next"], 3, "a 331-entry frontier at limit=3 must page, not silently truncate");
}

/// ENT-1a: the generic node card's own additive `description` field, over
/// HTTP, over REAL data -- `aaron_1` is this file's own established
/// exemplar (the test above) and carries a real, tier-(a) Theographic-
/// pre-joined Easton's description (`description_adapter.rs`'s own unit
/// tests already prove the MATCHING logic in isolation; this proves the
/// wire actually carries it end to end, additively, through the SAME
/// generic `GET /api/node/{id}` this file's other Person tests already
/// exercise -- no new endpoint).
#[tokio::test]
async fn person_card_carries_a_real_easton_description_when_a_match_exists() {
    let app = real_app();
    let (st, body, _) = get(&app, "/api/node/Person:aaron_1").await;
    assert_eq!(st, 200, "{body}");
    let description = body["description"].as_str().expect("Aaron must carry a real description over the real compiled data");
    assert!(description.starts_with("The eldest son of Amram"), "must be Easton's own verbatim prose, got: {description}");
}

/// ENT-1a: the SAME additive field, on the OTHER wire surface it was added
/// to (`handlers::PlaceDetailOut`, the legacy `/api/place/{id}` endpoint,
/// distinct code from the generic node card above) -- `hebron` is a real
/// compiled geo place with a real tier-(b) Easton's match.
#[tokio::test]
async fn place_detail_carries_a_real_easton_description_when_a_match_exists() {
    let app = real_app();
    let (st, body, _) = get(&app, "/api/place/hebron").await;
    assert_eq!(st, 200, "{body}");
    let description = body["description"].as_str().expect("Hebron must carry a real description over the real compiled data");
    assert!(!description.trim().is_empty());
    assert!(description.contains("Eshcol") || description.contains("Jerusalem"), "must be Easton's own real Hebron prose, got: {description}");
}

/// Batch CORP-1b (owner authorization, resolving CORP-1's own disclosed
/// NEEDS_CONTEXT gap: "no existing server query exposes a CommentaryItem's
/// own prose"): the SAME additive `description` seam, widened to a FOURTH
/// kind (`atlas_graph::legacy::node_description`'s new CommentaryItem arm)
/// -- over HTTP, over REAL compiled data, proving the wire actually carries
/// a real Kretzmann unit's own prose end to end through the SAME generic
/// `GET /api/node/{id}` this file's own Person/Place tests above already
/// exercise (no new endpoint, no new wire type). `kretzmann/0.1.0` is
/// GEN.1.1's own real first commentary unit (kretzmann_adapter.rs's own
/// "NODE IDENTITY" doc comment: `kretzmann/{book}.{chapter}.{ordinal}`).
#[tokio::test]
async fn commentary_item_card_carries_its_own_real_kretzmann_prose_via_description() {
    let app = real_app();
    let (st, body, _) = get(&app, "/api/node/CommentaryItem:kretzmann%2F0.1.0").await;
    assert_eq!(st, 200, "{body}");
    assert_eq!(body["label"], "The Creation of the World.: The Creation of Chaos and Light");
    let description = body["description"].as_str().expect("a real CommentaryItem must carry its own prose over the real compiled data");
    assert!(description.starts_with("In the beginning, cp. John 1, 1"), "must be Kretzmann's own verbatim prose (the lemma already excised, KRETZ-1), got: {description}");
    assert!(description.contains("the heaven"), "must be the FULL unit text, not truncated, got: {description}");
}

/// ENT-1a (additive-only, batch-ent1a-brief.md controller decision 3): when
/// no description match exists, the JSON key is OMITTED entirely (never a
/// present `null`) -- the SAME `skip_serializing_if` discipline every other
/// optional field on this wire already uses (`PlaceDetailOut::history`,
/// `::canonical_name`), proven here rather than merely claimed. An Era node
/// carries no `description` field on its `NodePayload` variant at all, so
/// this also proves the OTHER kinds sharing the generic card never gain a
/// stray key.
#[tokio::test]
async fn node_card_omits_description_for_a_kind_that_never_carries_one() {
    let app = real_app();
    let (st, body, _) = get(&app, "/api/node/Era:primeval").await;
    assert_eq!(st, 200, "{body}");
    assert!(body.get("description").is_none(), "an Era card must never carry a description key at all, got: {body}");
}

/// The inverse direction of the same relation: a VERSE's own `mentions`
/// frontier carries BOTH Place and Person entities under one edge kind
/// (design doc §4: `mentions` locus -> `Place | Person`) -- EXO.4.14 (the
/// FIRST entry in the test above) is a real, committed verse citing all
/// three of Aaron, God, and Moses by Theographic's own tagging.
#[tokio::test]
async fn a_verses_mentions_frontier_carries_person_entities_alongside_place() {
    let app = real_app();

    let (st, page, _) = get(&app, "/api/node/text-unit:EXO.4.14/edges?kind=mentions").await;
    assert_eq!(st, 200, "{page}");
    let entries = page["entries"].as_array().unwrap();
    let person_labels: Vec<String> = entries.iter().filter(|e| e["node"]["kind"] == "Person").map(|e| e["node"]["label"].as_str().unwrap().to_string()).collect();
    assert!(person_labels.contains(&"Aaron".to_string()), "{person_labels:?}");
    assert!(person_labels.contains(&"Moses".to_string()), "{person_labels:?}");
}

/// PG-1a wire seam (batch-pg1a-brief.md decision 6): a PeopleGroup mentions
/// row is REAL, present content in the built graph (the node card's own
/// `edge_summary` proves it, unfiltered) -- but the generic entity-LIST
/// page for that same relation must show none of it, the current client
/// having no rendering surface for the new kind (`graph_handlers::
/// node_edges`'s own filter, `graph_wire::decode_node_id`'s own missing
/// "PeopleGroup" arm). GEN.10.16 ("And the Jebusite, and the Amorite, and
/// the Girgasite,") is a real reclassified-gentilic locus (decision 1c).
#[tokio::test]
async fn peoplegroup_mentions_are_real_in_the_graph_but_filtered_from_the_generic_edges_page() {
    let app = real_app();

    let (st, card, _) = get(&app, "/api/node/text-unit:GEN.10.16").await;
    assert_eq!(st, 200, "{card}");
    let summary = card["edge_summary"].as_array().unwrap();
    let mentions_count = summary.iter().find(|e| e["kind"] == "mentions").and_then(|e| e["count"].as_u64()).unwrap_or(0);
    assert!(mentions_count >= 3, "GEN.10.16 must carry >=3 real mentions (Jebusite/Amorite/Girgasite) in the built graph, unfiltered: {summary:?}");

    let (st2, page, _) = get(&app, "/api/node/text-unit:GEN.10.16/edges?kind=mentions").await;
    assert_eq!(st2, 200, "{page}");
    let entries = page["entries"].as_array().unwrap();
    assert!(entries.iter().all(|e| e["node"]["kind"] != "PeopleGroup"), "no edge-page entry may carry kind=PeopleGroup -- the current client cannot render or re-fetch it: {entries:?}");
}

/// A PeopleGroup id cannot even be constructed on the wire today --
/// `graph_wire::decode_node_id` has no "PeopleGroup" arm (deliberately;
/// that round-trip completion is U5 rebinding's own job) -- so a direct
/// fetch 400s `bad_ref`, the same convention any syntactically-unsupported
/// kind prefix already gets. Reinforces the edges-page filter test above:
/// even if a PeopleGroup entry DID leak onto a page, the current client
/// could not follow it anywhere.
#[tokio::test]
async fn peoplegroup_node_id_cannot_be_fetched_directly_yet() {
    let app = real_app();
    let (st, body, _) = get(&app, "/api/node/PeopleGroup:jebusite_748").await;
    assert_eq!(st, 400, "{body}");
    assert_eq!(body["error"]["code"], "bad_ref");
}

/// PG-1a decision 7 ("wire filter test: a chapter response containing a
/// gentilic locus has NO peoplegroup-kind span"): `VerseOut` carries only
/// `places`/`persons` (no third, PeopleGroup-shaped field exists at all),
/// so the only OBSERVABLE consequence is that the three gentilics named at
/// this real locus no longer appear in `persons` -- verified directly
/// rather than assumed, per decision 6's own "verify and disclose"
/// instruction (they WOULD have appeared here before PG-1a's own
/// reclassification, back when Jebusite/Amorite/Girgasite were still
/// Theographic PERSON records).
#[tokio::test]
async fn chapter_response_for_a_gentilic_locus_carries_no_peoplegroup_kind_span() {
    let app = real_app();
    let (st, chapter, _) = get(&app, "/api/chapter/GEN.10").await;
    assert_eq!(st, 200);
    let v16 = chapter["verses"].as_array().unwrap().iter().find(|v| v["verse"] == 16).expect("GEN.10.16 must be in the chapter");
    let persons = v16["persons"].as_array().expect("persons must always be present, even at 0, never an omitted key");
    let names: Vec<&str> = persons.iter().map(|p| p["name"].as_str().unwrap()).collect();
    for gone in ["Jebusite", "Amorite", "Girgasite"] {
        assert!(!names.contains(&gone), "'{gone}' must NOT appear in GEN.10.16's own persons list any more -- it is a PeopleGroup now: {names:?}");
    }
}

/// U5: the chapter view's own precomputed `persons` field (`VerseOut.persons`,
/// backed by `GraphService::persons_by_verse`) must agree with the generic
/// mentions-frontier query for the SAME verse -- the same
/// precomputed-index-agrees-with-live-query law `chapter_verse_xref_count_
/// is_always_present_and_matches_the_generic_edges_page` above already
/// proves for `xref_count`/`cites`. EXO.4.14 is this file's own established
/// Aaron+Moses exemplar (see the mentions-frontier test above); a Person id
/// only ever carries a bare id + curated label on the wire (no `kind`, unlike
/// `PlaceRefOut`, since there is exactly one node kind this field can name).
#[tokio::test]
async fn chapter_verse_persons_is_always_present_and_matches_the_generic_mentions_frontier() {
    let app = real_app();

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
        .filter(|e| e["node"]["kind"] == "Person")
        .map(|e| e["node"]["label"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(chapter_names, frontier_names, "the chapter view's own persons list must equal the generic mentions frontier's own Person entries for the SAME verse");

    // And a verse with no person mentions at all still carries the key,
    // empty -- the same "always present, never omitted" shape `places`
    // already establishes in `tests/api.rs`'s own `verse_chapter_place_and_404`.
    // EXO.4.7 ("And he said, Put thine hand into thy bosom again...") names
    // no one -- confirmed against the real compiled data (every OTHER verse
    // in this chapter carries at least one attested person).
    let v7 = chapter["verses"].as_array().unwrap().iter().find(|v| v["verse"] == 7).expect("EXO.4.7 must be in the chapter");
    assert_eq!(v7["persons"], serde_json::json!([]), "{v7:?}");
}

/// A bogus Person id resolves 404 (not_found), not a 500 or a bespoke
/// error shape -- the SAME honest-not-found convention every other kind's
/// own unknown-id case already gets (node_card_unknown_id_is_404...
/// above), proven for Person specifically since `graph_wire::decode_node_id`
/// gained its own new match arm this batch.
#[tokio::test]
async fn person_card_unknown_id_is_404() {
    let app = real_app();
    let (st, body, _) = get(&app, "/api/node/Person:nonexistent-xyz").await;
    assert_eq!(st, 404, "{body}");
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

// ---------------------------------------------------------------------
// Batch M-D2 (the owner's cross-reference superscript directive, closed on
// the graph platform): `VerseOut.xref_count` -- a NEW, additive field on the
// bespoke `/api/chapter/{cref}` response, sourced from THE PORT's own
// `edge_summary` for the `cites` relation at each verse's TextUnit locus (the
// exact same generic query `GET /api/node/{id}` itself answers, reused
// server-side so a whole chapter's superscript counts ship in ONE round
// trip). These tests pin the two claims the client-side superscript
// rendering depends on without re-deriving them itself: (1) the count is the
// SAME true count the generic node/edges endpoints already serve, and (2)
// the generic `cites` edge page is ALREADY votes-descending, matching the
// bespoke, provably-votes-sorted `/api/verse/{vref}` endpoint exactly --
// "votes-ranked order comes from EdgeMeta::Votes on the entries (already on
// the wire)" is a tested fact here, not an assumed one.
// ---------------------------------------------------------------------

/// Extracts a cross-reference target's own FIRST verse ref
/// (`"COL.1.16-19"` -> `"COL.1.16"`, `"MAT.5.3-MAT.6.2"` -> `"MAT.5.3"`,
/// `"JOB.26.13"` -> itself unchanged) -- mirrors the three-shape parse this
/// whole codebase already duplicates a few times over (`handlers::
/// first_verse_of_target`, `atlas_core::xrefs::target_span`,
/// `xref_adapter::target_span`), reimplemented locally here (test-only, no
/// production `pub` surface to reuse) rather than exposing a fourth `pub fn`
/// just for this assertion.
fn first_verse_of(target: &str) -> String {
    let head = target.split('-').next().unwrap_or(target);
    let parts: Vec<&str> = head.split('.').collect();
    assert!(parts.len() >= 3, "a cross-reference target's own head must be a canonical BOOK.CHAPTER.VERSE ref: {target}");
    format!("{}.{}.{}", parts[0], parts[1], parts[2])
}

#[tokio::test]
async fn chapter_verse_xref_count_is_always_present_and_matches_the_generic_edges_page() {
    let app = real_app();

    // JHN.3.16 -- already this file's own "many real cross-references"
    // exemplar (the pagination/bijection tests above).
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

    // And the generic node card's own edge_summary (a DIFFERENT read of the
    // exact same underlying generic query) must agree too.
    let (st3, card, _) = get(&app, "/api/node/text-unit:JHN.3.16").await;
    assert_eq!(st3, 200);
    let cites_summary = card["edge_summary"].as_array().unwrap().iter().find(|e| e["kind"] == "cites").expect("JHN.3.16 must summarize a real cites frontier");
    assert_eq!(chapter_count, cites_summary["count"].as_u64().unwrap() as usize, "the chapter view's own xref_count must equal the node card's own edge_summary count");
}

#[tokio::test]
async fn chapter_verse_xref_count_is_always_present_never_omitted_across_a_whole_chapter() {
    let app = real_app();

    // Shape/presence, proven over EVERY verse of a real chapter (not one
    // hand-picked verse): `xref_count` is a plain non-negative integer key
    // on every single entry, never a conditionally-omitted field the way
    // `heading` is. (GEN.1.1 itself turns out to carry 61 REAL
    // cross-references in the full compiled openbible.info dataset --
    // densely cited, not the zero-xref exemplar a stale assumption might
    // suggest -- so this test proves presence structurally rather than
    // asserting a specific hardcoded verse's own count.)
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
    let app = real_app();

    // Discovers a real zero-xref verse by scanning real chapters (sample-
    // driven, never a hardcoded assumption about which specific verse
    // qualifies -- the prior draft of this test hardcoded GEN.1.1 and was
    // wrong: real data cites it 61 times). Genealogical/list-heavy chapters
    // are the likeliest real candidates; falls through gracefully (no hard
    // failure) if this bounded scan happens not to find one -- a data fact,
    // not a logic bug, the SAME "skip rather than flake" discipline this
    // project's own Playwright suites already apply to sample-driven finds.
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
                // The generic port must independently agree: no `cites`
                // entry at all in this verse's own edge_summary (edge_summary
                // only ever lists INHABITED kinds -- absence IS zero).
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
    let app = real_app();

    // The bespoke endpoint's own `cross_refs` is built by iterating
    // `graph.cross_refs_by_from` with no re-sort -- which is itself built,
    // unsorted-again, directly from `xref_adapter::read_xrefs_ordered`'s own
    // per-`from`-key votes-descending order (that module's own doc comment).
    // So `/api/verse/{vref}`'s own `cross_refs` order IS the votes-ranked
    // order, independently of the generic endpoint entirely -- the oracle
    // for this test, not a second copy of the claim under test.
    let (st, verse, _) = get(&app, "/api/verse/JHN.3.16").await;
    assert_eq!(st, 200);
    let bespoke: Vec<String> = verse["cross_refs"].as_array().unwrap().iter().map(|cr| first_verse_of(cr["target"].as_str().unwrap())).collect();
    assert!(bespoke.len() > 1, "need >1 real cross-references to prove an ORDER, not just a singleton");

    let (st2, edges, _) = get(&app, "/api/node/text-unit:JHN.3.16/edges?kind=cites&limit=200").await;
    assert_eq!(st2, 200);
    let generic: Vec<String> = edges["entries"].as_array().unwrap().iter().map(|e| e["node"]["id"].as_str().unwrap().trim_start_matches("text-unit:").to_string()).collect();

    assert_eq!(
        generic, bespoke,
        "the generic `cites` edge page must already be votes-descending, position for position matching the bespoke, provably-votes-sorted /api/verse endpoint -- no client-side re-sort should ever be needed"
    );
}

// ---------------------------------------------------------------------
// EDGE-1a ("Prophecy & typology: the seed data" -- batch-edge1a-brief.md
// controller decision 4: "the generic edge/explore endpoints will
// naturally serve the new relations through the existing typed-edge
// machinery -- verify with an HTTP-level test that a fulfillment edge is
// reachable via the generic frontier for MAT 1:22's own verse node (and
// one typology case)"). NO new bespoke endpoints, NO client changes --
// these two relations ride the SAME generic `/api/node/{id}` and
// `/api/node/{id}/edges?kind=...` surface every other relation already
// uses. `Fulfillment => "fulfilled-in" / "fulfills"` and
// `Typology => "prefigures" / "prefigured-by"` (graph-types' own relation
// manifest): querying from the PROPHECY/TYPE side yields the forward
// label; from the FULFILLMENT/ANTITYPE side, the inverse -- the SAME
// forward-from-subject/inverse-from-object convention this file's own
// `located-at`/`site-of` tests (`event_card_and_frontiers_...`/
// `narrative_card_and_place_stub_card_...`) already establish.
// ---------------------------------------------------------------------

/// ISA 7:14 -> MAT 1:22-23 ("that it might be fulfilled which was spoken
/// of the Lord by the prophet, saying, Behold, a virgin shall be with
/// child") -- the brief's own worked example. The fulfillment edge's own
/// endpoint is the range's FIRST verse (graph-types/src/graph.rs's own
/// "edge endpoint = a range's first verse" lowering), so MAT.1.22 (not
/// .23) is the real wire node.
#[tokio::test]
async fn a_fulfillment_edge_is_reachable_via_the_generic_frontier_for_mat_1_22() {
    let app = real_app();

    let (st, body, _) = get(&app, "/api/node/text-unit:MAT.1.22").await;
    assert_eq!(st, 200, "{body}");
    let summary: Vec<String> = body["edge_summary"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap().to_string()).collect();
    assert!(summary.contains(&"fulfills".to_string()), "MAT.1.22 must carry a real fulfills (inverse) frontier: {summary:?}");

    let (st2, edges, _) = get(&app, "/api/node/text-unit:MAT.1.22/edges?kind=fulfills").await;
    assert_eq!(st2, 200, "{edges}");
    let entries = edges["entries"].as_array().unwrap();
    assert!(entries.iter().any(|e| e["node"]["id"] == "text-unit:ISA.7.14"), "MAT.1.22 must fulfill ISA.7.14: {entries:?}");

    // And the inverse direction, from the prophecy's own node: forward
    // label "fulfilled-in".
    let (st3, prophecy_body, _) = get(&app, "/api/node/text-unit:ISA.7.14").await;
    assert_eq!(st3, 200, "{prophecy_body}");
    let prophecy_summary: Vec<String> = prophecy_body["edge_summary"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap().to_string()).collect();
    assert!(prophecy_summary.contains(&"fulfilled-in".to_string()), "ISA.7.14 must carry a real fulfilled-in (forward) frontier: {prophecy_summary:?}");

    let (st4, prophecy_edges, _) = get(&app, "/api/node/text-unit:ISA.7.14/edges?kind=fulfilled-in").await;
    assert_eq!(st4, 200, "{prophecy_edges}");
    let prophecy_entries = prophecy_edges["entries"].as_array().unwrap();
    assert!(prophecy_entries.iter().any(|e| e["node"]["id"] == "text-unit:MAT.1.22"), "ISA.7.14 must be fulfilled-in MAT.1.22: {prophecy_entries:?}");
}

/// One typology case (decision 4's own "and one typology case"):
/// Melchizedek, GEN 14:18-20 -> HEB 7:1-17.
#[tokio::test]
async fn a_typology_edge_is_reachable_via_the_generic_frontier_for_the_melchizedek_case() {
    let app = real_app();

    let (st, body, _) = get(&app, "/api/node/text-unit:HEB.7.1").await;
    assert_eq!(st, 200, "{body}");
    let summary: Vec<String> = body["edge_summary"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap().to_string()).collect();
    assert!(summary.contains(&"prefigured-by".to_string()), "HEB.7.1 must carry a real prefigured-by (inverse) frontier: {summary:?}");

    let (st2, edges, _) = get(&app, "/api/node/text-unit:HEB.7.1/edges?kind=prefigured-by").await;
    assert_eq!(st2, 200, "{edges}");
    let entries = edges["entries"].as_array().unwrap();
    assert!(entries.iter().any(|e| e["node"]["id"] == "text-unit:GEN.14.18"), "HEB.7.1 must be prefigured-by GEN.14.18: {entries:?}");

    let (st3, type_body, _) = get(&app, "/api/node/text-unit:GEN.14.18").await;
    assert_eq!(st3, 200, "{type_body}");
    let type_summary: Vec<String> = type_body["edge_summary"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap().to_string()).collect();
    assert!(type_summary.contains(&"prefigures".to_string()), "GEN.14.18 must carry a real prefigures (forward) frontier: {type_summary:?}");

    let (st4, type_edges, _) = get(&app, "/api/node/text-unit:GEN.14.18/edges?kind=prefigures").await;
    assert_eq!(st4, 200, "{type_edges}");
    let type_entries = type_edges["entries"].as_array().unwrap();
    assert!(type_entries.iter().any(|e| e["node"]["id"] == "text-unit:HEB.7.1"), "GEN.14.18 must prefigure HEB.7.1: {type_entries:?}");
}

/// JB-1 rider (batch-edge1a-brief.md controller decision 3): the SAME
/// `justified-by`/`justifies` wiring `anchor_card_carries_its_citation_and_
/// dates_frontier` already proves for DatedBy rows, now generalized onto
/// Fulfills/Typology rows too -- each row's own self-attesting
/// `Ground::Scripture` (the fulfillment/antitype passage itself) must
/// carry a real `justifies` frontier back to the claim it grounds, over
/// HTTP, through the SAME generic endpoint, no new bespoke wiring.
#[tokio::test]
async fn a_fulfillment_and_a_typology_rows_own_ground_carries_a_real_justifies_frontier() {
    let app = real_app();

    // MAT.1.22 is the fulfillment passage AND (fulfillment_adapter's own
    // "self-attesting" convention) its own row's Ground::Scripture.
    let (st, justifies, _) = get(&app, "/api/node/text-unit:MAT.1.22/edges?kind=justifies").await;
    assert_eq!(st, 200, "{justifies}");
    assert!(!justifies["entries"].as_array().unwrap().is_empty(), "MAT.1.22 must justify its own fulfills row (JB-1 rider)");

    // HEB.7.1 is the antitype passage AND its own row's Ground::Scripture
    // (the Melchizedek case).
    let (st2, typology_justifies, _) = get(&app, "/api/node/text-unit:HEB.7.1/edges?kind=justifies").await;
    assert_eq!(st2, 200, "{typology_justifies}");
    assert!(!typology_justifies["entries"].as_array().unwrap().is_empty(), "HEB.7.1 must justify its own typology row (JB-1 rider)");
}

/// Decision 4's own "and that the CURRENT client surface is otherwise
/// unchanged": a node with NEITHER relation (a plain, previously-untouched
/// verse) must carry no fulfills/fulfilled-in/prefigures/prefigured-by
/// entry at all -- these two relations are additive, never a default
/// entry every node now carries.
#[tokio::test]
async fn nodes_uninvolved_in_fulfillment_or_typology_carry_no_such_edge_summary_entries() {
    let app = real_app();
    let (st, body, _) = get(&app, "/api/node/text-unit:GEN.1.1").await;
    assert_eq!(st, 200, "{body}");
    let summary: Vec<String> = body["edge_summary"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap().to_string()).collect();
    for kind in ["fulfilled-in", "fulfills", "prefigures", "prefigured-by"] {
        assert!(!summary.contains(&kind.to_string()), "GEN.1.1 must carry no '{kind}' entry (uninvolved in either new relation): {summary:?}");
    }
}
