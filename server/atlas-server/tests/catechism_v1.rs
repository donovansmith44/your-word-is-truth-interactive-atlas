//! CATECH-V1 integration tests: the outline endpoint, the reflection/media
//! fields on an item, and the PDF print endpoint.
//!
//! Same harness discipline as `api.rs` -- `tower::ServiceExt::oneshot` over a
//! router built from `demo_fixture()`, no socket bound. The fixture's own
//! `demo-item-1` carries two reflection prompts at DIFFERENT tiers and one
//! media link precisely so these tests can assert order and shape without
//! authoring a second, drift-prone fixture (see `demo_fixture`'s own comment
//! in atlas-core).

use std::sync::Arc;

use atlas_core::data::{demo_fixture, AtlasData};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn graph_fixture_for(data: &AtlasData) -> Arc<atlas_graph::GraphService> {
    // Same derivation `api.rs::xrefs_tsv_from` uses -- one authored
    // cross-ref fixture, re-expressed as the raw TSV the graph builds from,
    // rather than a second hand-written table that could drift from it.
    let mut tsv = String::from("From Verse\tTo Verse\tVotes\t#comment\n");
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
            tsv.push_str(&format!("{from}\t{to}\t{}\n", cr.votes));
        }
    }
    Arc::new(
        atlas_graph::GraphService::from_canon_and_verses_with_eras(&data.canon, &data.verses, &tsv, data, &data.eras)
            .expect("fixture graph must build"),
    )
}

fn app() -> axum::Router {
    let data = demo_fixture();
    let graph = graph_fixture_for(&data);
    atlas_server::app::build(Arc::new(data), graph, None)
}

async fn call(app: &axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let response =
        app.clone().oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    (status, json)
}

async fn call_bytes(app: &axum::Router, uri: &str) -> (StatusCode, Vec<String>, Vec<u8>) {
    let response =
        app.clone().oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let headers = response
        .headers()
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v.to_str().unwrap_or("")))
        .collect::<Vec<_>>();
    let bytes = response.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, headers, bytes)
}

#[tokio::test]
async fn outline_lists_parts_items_and_readiness_counts() {
    let app = app();
    let (st, body) = call(&app, "/api/catechism").await;
    assert_eq!(st, 200);

    let parts = body["parts"].as_array().expect("parts array");
    assert_eq!(parts.len(), 1, "{body}");
    assert_eq!(parts[0]["id"], "demo-part");
    assert_eq!(parts[0]["title"], "Demo Part");

    let items = parts[0]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], "demo-item-1");
    assert_eq!(items[0]["explanation_heading"], "What does this mean?");

    // The verse count spans BOTH citation granularities -- the fixture item
    // has one item-level verse (JOS.6.20) and one question-level verse
    // (JOS.6.21). A count of 1 here would mean one granularity was dropped.
    assert_eq!(items[0]["verse_count"], 2, "{body}");
    assert_eq!(items[0]["reflection_count"], 2);
    assert_eq!(items[0]["media_count"], 1);
}

#[tokio::test]
async fn item_carries_reflection_in_curated_order_and_media() {
    let app = app();
    let (st, body) = call(&app, "/api/catechism/item/demo-item-1").await;
    assert_eq!(st, 200);

    assert_eq!(body["part_id"], "demo-part", "part_id must be addressable, not only its title");
    assert_eq!(body["part_title"], "Demo Part");

    let reflection = body["reflection"].as_array().expect("reflection array");
    assert_eq!(reflection.len(), 2);
    // Curated order is easy-to-difficult and the wire must not re-sort it:
    // the fixture authored child THEN adult, so a response that came back
    // adult-first would mean something sorted (or reversed) on the way out.
    assert_eq!(reflection[0]["tier"], "child");
    assert_eq!(reflection[0]["prompt"], "Demo easy prompt.");
    assert_eq!(reflection[1]["tier"], "adult");
    assert_eq!(reflection[1]["prompt"], "Demo harder prompt.");

    let media = body["media"].as_array().expect("media array");
    assert_eq!(media.len(), 1);
    assert_eq!(media[0]["kind"], "youtube");
    assert_eq!(media[0]["title"], "Demo Hymn");
}

#[tokio::test]
async fn print_returns_a_real_pdf_with_an_inline_filename() {
    let app = app();
    let (st, headers, bytes) = call_bytes(&app, "/api/catechism/print?scope=item&item=demo-item-1").await;
    assert_eq!(st, 200);

    let joined = headers.join("\n");
    assert!(joined.contains("content-type: application/pdf"), "{joined}");
    // The filename is derived from the item name, so a teacher's downloads
    // folder does not fill with `print.pdf`.
    assert!(joined.contains("demo-catechism-item.pdf"), "{joined}");

    // A PDF, not an error page rendered with the wrong content type.
    assert!(bytes.starts_with(b"%PDF-"), "body must be a PDF, got {:?}", &bytes[..bytes.len().min(16)]);
    assert!(bytes.len() > 500, "a PDF carrying real content should not be near-empty");
}

/// Does the rendered document SAY this?
///
/// Two earlier versions of these assertions were wrong in instructive ways.
/// The first compared PDF byte LENGTHS, and both tests failed for reasons
/// that turned out to be correct behavior: the tier-filtered document is
/// LARGER (it gains a "filtered to: child" note and a longer section label
/// while shedding one short prompt), and part scope of a one-item part is
/// SMALLER (the part title is shorter than the item name, and the redundant
/// per-item heading is deliberately suppressed). Size was measuring
/// formatting, not content.
///
/// The second byte-SEARCHED the raw file, which worked only because printpdf
/// 0.12.6 wrote content streams uncompressed. 0.12.7 Flate-compresses them
/// and both assertions silently went dark -- a false PASS on the negative
/// assertion (`!contains(child_only, "Demo harder prompt.")` would have held
/// no matter what the document said) and a false FAIL on the positive ones.
/// That is the worse failure of the two: an assertion that cannot fail is
/// not a test.
///
/// So: decode the content streams and search the text-showing operands, the
/// way a reader resolves the page. Independent of how any printpdf version
/// chooses to encode.
fn contains(pdf: &[u8], needle: &str) -> bool {
    let doc = lopdf::Document::load_mem(pdf).expect("the endpoint must emit a parseable PDF");
    // Collected first: `page_iter` borrows `doc`, and the closure below
    // borrows it again to read each page's content.
    let page_ids: Vec<_> = doc.page_iter().collect();
    page_ids.into_iter().any(|page_id| {
        // `get_page_content` decompresses the stream and returns the bytes
        // directly (an unparseable page yields an empty Vec, which decodes
        // to zero operations -- no match, no panic).
        lopdf::content::Content::decode(&doc.get_page_content(page_id))
            .ok()
            .is_some_and(|content| {
                content.operations.iter().any(|op| {
                    // Tj / TJ / ' / " -- every operator that shows text.
                    op.operands.iter().any(|operand| operand_says(operand, needle))
                })
            })
    })
}

/// A text operand is either a string or (for TJ) an array of strings and
/// kerning numbers. Recurse so a needle split across kerning still matches
/// when the pieces are adjacent -- and so a nested array never hides text.
fn operand_says(operand: &lopdf::Object, needle: &str) -> bool {
    match operand {
        lopdf::Object::String(bytes, _) => String::from_utf8_lossy(bytes).contains(needle),
        lopdf::Object::Array(items) => {
            let joined: String = items
                .iter()
                .filter_map(|i| match i {
                    lopdf::Object::String(b, _) => Some(String::from_utf8_lossy(b).into_owned()),
                    _ => None,
                })
                .collect();
            joined.contains(needle) || items.iter().any(|i| operand_says(i, needle))
        }
        _ => false,
    }
}

#[tokio::test]
async fn print_part_scope_is_titled_by_the_part_not_the_item() {
    let app = app();
    let (_, _, item_pdf) = call_bytes(&app, "/api/catechism/print?scope=item&item=demo-item-1").await;
    let (st, _, part_pdf) = call_bytes(&app, "/api/catechism/print?scope=part&item=demo-item-1").await;
    assert_eq!(st, 200);
    assert!(part_pdf.starts_with(b"%PDF-"));

    // Both carry the item's actual teaching content...
    assert!(contains(&item_pdf, "Demo item explanation."), "item scope must carry the explanation");
    assert!(contains(&part_pdf, "Demo item explanation."), "part scope must carry its items' explanations");

    // ...but the part-scoped document is titled by the PART, which is what
    // makes it the "print the whole part for the class" artifact.
    assert!(contains(&part_pdf, "Demo Part"), "part scope must be titled by the part");
    assert!(contains(&item_pdf, "Demo Catechism Item"), "item scope must be titled by the item");
}

#[tokio::test]
async fn print_tier_filter_drops_the_other_tiers_prompts() {
    let app = app();
    let (_, _, unfiltered) = call_bytes(&app, "/api/catechism/print?scope=item&item=demo-item-1").await;
    let (st, _, child_only) =
        call_bytes(&app, "/api/catechism/print?scope=item&item=demo-item-1&tier=child").await;
    assert_eq!(st, 200);

    // Unfiltered carries both tiers.
    assert!(contains(&unfiltered, "Demo easy prompt."), "unfiltered must carry the child prompt");
    assert!(contains(&unfiltered, "Demo harder prompt."), "unfiltered must carry the adult prompt");

    // Filtered to child: the child prompt survives, the adult one is gone.
    // This is the assertion that actually proves the filter works -- and the
    // one a byte-length comparison could not make.
    assert!(contains(&child_only, "Demo easy prompt."), "child filter must keep the child prompt");
    assert!(!contains(&child_only, "Demo harder prompt."), "child filter must drop the adult prompt");
    assert!(contains(&child_only, "filtered to: child"), "a filtered printout must say so on its face");
}

#[tokio::test]
async fn print_rejects_malformed_parameters_rather_than_defaulting() {
    let app = app();

    let (st, body) = call(&app, "/api/catechism/print").await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "bad_request");

    let (st, _) = call(&app, "/api/catechism/print?item=demo-item-1&scope=everything").await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    let (st, _) = call(&app, "/api/catechism/print?item=demo-item-1&tier=grandparent").await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // An unknown item id is a 404, not a 400 -- the request was well-formed,
    // the resource just does not exist (handlers::catechism_item's precedent).
    let (st, _) = call(&app, "/api/catechism/print?item=no-such-item").await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}
