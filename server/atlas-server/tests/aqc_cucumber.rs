//! Batch AQC-1's own Rust contract harness -- THIN, contract-ignorant
//! step-definition glue (spec §3: "Step definitions are thin glue, not
//! contract knowledge") binding every phrase in
//! `contracts/atlas-query-contract/features/*.feature` (glossary.md's own
//! phrase table) against the LIVE axum handlers, in-process, via the SAME
//! `tower::ServiceExt::oneshot` idiom `tests/graph_api.rs`'s own `real_app()`
//! already uses -- no socket bound, the real committed graph.
//!
//! `cucumber` (dev-dependency, MIT/Apache-2.0 dual-licensed -- disclosed in
//! the batch report) is the ONLY new test-time dependency this file adds.
//! `harness = false` (Cargo.toml) -- this binary IS its own test runner
//! (cucumber's own `World::run`), the crate's documented entry shape.
//!
//! Joins `cargo test --workspace` (the standing canonical count) as one
//! more `atlas-server` test section, same as every `tests/*.rs` file here.

use std::path::Path;
use std::sync::OnceLock;

use atlas_core::data::AtlasData;
use atlas_graph::GraphService;
use atlas_graph_types::edge::dual;
use atlas_server::graph_wire::parse_edge_kind;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use cucumber::{given, then, when, World};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// The real committed graph's own Router -- built ONCE (`OnceLock`, same
/// "one real compile, shared across every call site" discipline
/// `tests/graph_api.rs::real_atlas_data` already established), then
/// `.clone()`d per scenario (axum's `Router` is cheap to clone -- an `Arc`
/// internally). Cucumber scenarios run against the SAME live graph every
/// other real-data integration test in this crate exercises -- never a
/// synthetic fixture (glossary.md's own "the real committed graph").
fn app() -> axum::Router {
    static ROUTER: OnceLock<axum::Router> = OnceLock::new();
    ROUTER
        .get_or_init(|| {
            let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            let raw_dir = data_dir.join("raw");
            let compiled = atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first");
            let data: AtlasData = compiled.data;
            let graph = GraphService::build(&raw_dir, &data).expect("data/raw/{kjv.json,xrefs/cross_references.txt} must exist and satisfy the fidelity law");
            atlas_server::app::build(std::sync::Arc::new(data), std::sync::Arc::new(graph), None)
        })
        .clone()
}

/// Percent-encodes the one wire-id character that collides with axum's own
/// path-segment routing: `/` (e.g. `CommentaryItem:kretzmann/0.1.0`) --
/// SAME convention `tests/graph_api.rs`'s own literal
/// `"CommentaryItem:kretzmann%2F0.1.0"` URIs already establish. No other
/// character this contract's ids ever carry (letters/digits/`:`/`-`/`.`/`_`)
/// needs encoding.
fn path_encode(id: &str) -> String {
    id.replace('/', "%2F")
}

async fn get(uri: &str) -> (u16, serde_json::Value) {
    let response = app().oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status().as_u16();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() { serde_json::Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, json)
}

/// Q-2/Q-3 fix (Batch AQC-1 fix round 1, controller ruling): `aqc.schema.json`
/// itself, parsed ONCE and read at test time -- replaces the prior
/// hand-copied `required_fields()` match (three places carried the same
/// list by hand: the schema, this match, and AqcSteps.cs's own switch;
/// nothing enforced them staying in sync). "Step definitions are thin
/// glue, not contract knowledge" (spec §3) -- the schema IS the contract
/// knowledge; this function only reads it.
fn schema() -> &'static serde_json::Value {
    static SCHEMA: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/atlas-query-contract/aqc.schema.json");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("aqc_cucumber: could not read {}: {e}", path.display()));
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("aqc_cucumber: {} is not valid JSON: {e}", path.display()))
    })
}

fn shape_def(shape: &str) -> &'static serde_json::Value {
    schema()["$defs"].get(shape).unwrap_or_else(|| panic!("aqc_cucumber: unknown shape '{shape}' -- no aqc.schema.json $defs.{shape}"))
}

#[derive(Debug, Default, World)]
pub struct AqcWorld {
    status: u16,
    body: serde_json::Value,
    captured_ref: Option<String>,
    /// S-1 fix (Batch AQC-1 fix round 1, controller ruling -- applied here
    /// too for symmetry with the C# side's own fix, though this review
    /// flagged only C# as vacuous): the id THIS scenario's own
    /// `when_focus_query` requested, `None` when the capture instead
    /// originated from a TraversalQuery target (there, the captured id is
    /// legitimately DIFFERENT from anything requested so far). When
    /// `Some`, `then_round_trips` asserts the captured reference equals
    /// THIS -- the actual descriptor round-trip identity law, not merely
    /// self-consistency between the capture and the second live fetch.
    focus_requested_id: Option<String>,
    last_traversal_id: String,
    last_traversal_kind: String,
    advertised_min: String,
    advertised_max: String,
}

/// The Rust test harness's own stand-in for "the consumer's compiled AQC
/// version" -- there is no Rust CONSUMER in this app (the real client is
/// C#, `client/AqcContract.cs::ClientVersion`); this mirrors that constant
/// so `versioning.feature`'s client-acceptance phrases bind on BOTH sides
/// (spec §3's phrase-parity law), each side proving the SAME semver-range
/// check independently.
///
/// Q-6 fix (Batch AQC-1 fix round 1, controller ruling -- "single-source
/// them or extend the cross-check to all five" hand-kept "0.1.0" copies):
/// this one is SINGLE-SOURCED, not hand-kept -- read from `contracts/
/// atlas-query-contract/VERSION` at test-run time (this is a TEST binary,
/// unlike `contract.rs`'s own compiled server constants, which genuinely
/// cannot read a repo-relative file at runtime once deployed) rather than
/// duplicated as a literal a future VERSION bump could silently leave
/// stale.
fn harness_client_version() -> &'static str {
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/atlas-query-contract/VERSION");
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("aqc_cucumber: could not read {}: {e}", path.display())).trim().to_string()
    })
}

/// `Err` on a malformed semver string -- the Rust mirror of
/// `AqcContract.ParseSemver`'s own deliberate C# `FormatException` throw
/// (Q-4 fix, fix round 1): a malformed advertisement must fail LOUD, not
/// silently pass, on both sides.
fn parse_semver(s: &str) -> Result<(u32, u32, u32), String> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("'{s}' is not a MAJOR.MINOR.PATCH semver string"));
    }
    let mut nums = [0u32; 3];
    for (i, p) in parts.iter().enumerate() {
        nums[i] = p.parse::<u32>().map_err(|_| format!("'{s}' is not a MAJOR.MINOR.PATCH semver string"))?;
    }
    Ok((nums[0], nums[1], nums[2]))
}

fn satisfies(client: &str, min: &str, max: &str) -> Result<bool, String> {
    let c = parse_semver(client)?;
    Ok(c >= parse_semver(min)? && c <= parse_semver(max)?)
}

// ---------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------

#[given(expr = "a node of kind {string} with id {string}")]
fn given_a_node(_world: &mut AqcWorld, _kind: String, _id: String) {
    // Documentation-only Given (glossary.md): the exporter already
    // verified this id resolves against the real committed graph before
    // it was ever written into the Examples: table (fail-loud at export
    // time, not here) -- no HTTP call needed to re-prove it per scenario.
}

#[given(expr = "the server advertises AQC version {string} through {string}")]
fn given_advertised_range(world: &mut AqcWorld, min: String, max: String) {
    world.advertised_min = min;
    world.advertised_max = max;
}

// ---------------------------------------------------------------------
// When
// ---------------------------------------------------------------------

#[when(expr = "I run FocusQuery for {string}")]
async fn when_focus_query(world: &mut AqcWorld, id: String) {
    let (status, body) = get(&format!("/api/node/{}", path_encode(&id))).await;
    world.status = status;
    world.body = body;
    world.focus_requested_id = Some(id);
}

#[when(expr = "I run FocusQuery again for the captured reference")]
async fn when_focus_query_captured(world: &mut AqcWorld) {
    let id = world.captured_ref.clone().expect("no focus reference was captured yet");
    let (status, body) = get(&format!("/api/node/{}", path_encode(&id))).await;
    world.status = status;
    world.body = body;
}

#[when(expr = "I run TraversalQuery for {string} frontier {string}")]
async fn when_traversal_query(world: &mut AqcWorld, id: String, kind: String) {
    // Not a FocusQuery -- see AqcWorld::focus_requested_id's own doc
    // comment for why this scenario shape skips the original-id check.
    world.focus_requested_id = None;
    world.last_traversal_id = id.clone();
    world.last_traversal_kind = kind.clone();
    let (status, body) = get(&format!("/api/node/{}/edges?kind={kind}", path_encode(&id))).await;
    world.status = status;
    world.body = body;
}

#[when(expr = "I run TraversalQuery for {string} frontier {string} with limit {int}")]
async fn when_traversal_query_limit(world: &mut AqcWorld, id: String, kind: String, limit: usize) {
    world.last_traversal_id = id.clone();
    world.last_traversal_kind = kind.clone();
    let (status, body) = get(&format!("/api/node/{}/edges?kind={kind}&limit={limit}", path_encode(&id))).await;
    world.status = status;
    world.body = body;
}

#[when(expr = "I run TextWindowQuery for {string} radius {int}")]
async fn when_text_window(world: &mut AqcWorld, sref: String, n: usize) {
    let (status, body) = get(&format!("/api/text?ref={sref}&n={n}")).await;
    world.status = status;
    world.body = body;
}

#[when(expr = "I run TextWindowQuery for {string} radius {int} with corpus {string}")]
async fn when_text_window_corpus(world: &mut AqcWorld, sref: String, n: usize, corpus: String) {
    let (status, body) = get(&format!("/api/text?ref={sref}&n={n}&corpus={corpus}")).await;
    world.status = status;
    world.body = body;
}

#[when(expr = "I run a chapter-scoped TextWindowQuery for {string} with dir {string}")]
async fn when_text_window_chapter_dir(world: &mut AqcWorld, cref: String, dir: String) {
    let (status, body) = get(&format!("/api/text?ref={cref}&scope=chapter&dir={dir}")).await;
    world.status = status;
    world.body = body;
}

#[when(expr = "I run SceneQuery for the time window {string}-{string}")]
async fn when_scene_time(world: &mut AqcWorld, from: String, to: String) {
    let (status, body) = get(&format!("/api/scene?from={from}&to={to}")).await;
    world.status = status;
    world.body = body;
}

#[when(expr = "I run SceneQuery for scripture ref {string}")]
async fn when_scene_scripture(world: &mut AqcWorld, sref: String) {
    let (status, body) = get(&format!("/api/scene/scripture?ref={sref}")).await;
    world.status = status;
    world.body = body;
}

#[when(expr = "I query {string}")]
async fn when_query_path(world: &mut AqcWorld, path: String) {
    let (status, body) = get(&path).await;
    world.status = status;
    world.body = body;
}

#[when(expr = "I capture the returned focus reference")]
fn when_capture_focus_ref(world: &mut AqcWorld) {
    // FocusQuery response: a top-level "id". TraversalQuery response: no
    // top-level id -- the FIRST entry's own node.id (glossary.md).
    let captured = if let Some(id) = world.body.get("id").and_then(|v| v.as_str()) {
        id.to_string()
    } else {
        world
            .body
            .get("entries")
            .and_then(|e| e.as_array())
            .and_then(|arr| arr.first())
            .and_then(|entry| entry.get("node"))
            .and_then(|node| node.get("id"))
            .and_then(|v| v.as_str())
            .expect("no capturable focus reference on the last response (expected a top-level 'id' or a non-empty 'entries' array)")
            .to_string()
    };
    world.captured_ref = Some(captured);
}

// ---------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------

#[then(expr = "the response is a valid {string}")]
fn then_valid_shape(world: &mut AqcWorld, shape: String) {
    assert_eq!(world.status, 200, "expected 200 for a valid {shape}, got {} -- body: {}", world.status, world.body);
    let obj = world.body.as_object().unwrap_or_else(|| panic!("{shape} response must be a JSON object: {}", world.body));
    let def = shape_def(&shape);

    let required = def["required"].as_array().unwrap_or_else(|| panic!("aqc.schema.json $defs.{shape} has no 'required' array"));
    for field in required {
        let field = field.as_str().unwrap();
        assert!(obj.contains_key(field), "{shape} response missing required field '{field}': {}", world.body);
    }

    // Q-3 fix: additionalProperties: false, enforced -- glossary.md's own
    // "the response is a valid <Shape>" definition names this as HALF of
    // what the phrase means; only the required-fields half was checked
    // before this fix.
    assert_eq!(def["additionalProperties"], serde_json::json!(false), "aqc.schema.json $defs.{shape} must declare additionalProperties: false");
    let allowed = def["properties"].as_object().unwrap_or_else(|| panic!("aqc.schema.json $defs.{shape} has no 'properties' object"));
    for key in obj.keys() {
        assert!(allowed.contains_key(key), "{shape} response has field '{key}' outside aqc.schema.json's own $defs.{shape}.properties: {}", world.body);
    }
}

#[then(expr = "the response {string} field equals {string}")]
fn then_field_equals(world: &mut AqcWorld, field: String, expected: String) {
    let actual = world.body.get(&field).and_then(|v| v.as_str()).unwrap_or_else(|| panic!("response has no string field '{field}': {}", world.body));
    assert_eq!(actual, expected, "response field '{field}'");
}

#[then(expr = "every frontier group is a relations! family")]
fn then_every_frontier_is_a_relations_family(world: &mut AqcWorld) {
    if let Some(summary) = world.body.get("edge_summary").and_then(|v| v.as_array()) {
        for entry in summary {
            let kind = entry["kind"].as_str().unwrap();
            assert!(parse_edge_kind(kind).is_some(), "edge_summary kind '{kind}' is not a relations! family label");
        }
    } else if let Some(kind) = world.body.get("kind").and_then(|v| v.as_str()) {
        assert!(parse_edge_kind(kind).is_some(), "EdgePageOut kind '{kind}' is not a relations! family label");
    } else {
        panic!("no frontier-bearing field (edge_summary or kind) on the last response: {}", world.body);
    }
}

#[then(expr = "the request fails with status {int} and code {string}")]
fn then_request_fails(world: &mut AqcWorld, status: u16, code: String) {
    assert_eq!(world.status, status, "status code -- body: {}", world.body);
    assert_eq!(world.body["error"]["code"].as_str().unwrap(), code, "error code");
}

#[then(expr = "the focus reference round-trips identically")]
fn then_round_trips(world: &mut AqcWorld) {
    let captured = world.captured_ref.clone().expect("no focus reference was captured");
    let second = world.body["id"].as_str().expect("second FocusQuery response has no 'id'");
    assert_eq!(second, captured, "the id must round-trip byte-identically (encode_node_id(decode_node_id(s)) == s)");
    // S-1 fix (fix round 1): the ACTUAL round-trip identity law -- captured
    // must ALSO equal what this scenario originally requested, not merely
    // equal the second (live, independently re-fetched) response. Skipped
    // when the capture originated from a TraversalQuery target
    // (AqcWorld::focus_requested_id's own doc comment).
    if let Some(requested) = &world.focus_requested_id {
        assert_eq!(captured, *requested, "the captured reference must equal the id this scenario originally requested");
    }
}

#[then(expr = "every traversal target resolves to a live node")]
async fn then_every_target_resolves(world: &mut AqcWorld) {
    let entries = world.body["entries"].as_array().expect("EdgePageOut.entries must be an array");
    assert!(!entries.is_empty(), "test needs at least one real entry to prove resolution over -- pick a richer seed if this fires");
    for entry in entries {
        let id = entry["node"]["id"].as_str().unwrap();
        let (status, _) = get(&format!("/api/node/{}", path_encode(id))).await;
        assert_eq!(status, StatusCode::OK.as_u16(), "traversal target '{id}' must resolve via FocusQuery");
    }
}

#[then(expr = "every entry's {string} id is present on the matching inverse-kind page of its own target node")]
async fn then_bijection_witness(world: &mut AqcWorld, field: String) {
    assert_eq!(field, "edge");
    let kind = parse_edge_kind(&world.last_traversal_kind).expect("last TraversalQuery kind must be a real relations! family label");
    let inverse_label = dual(kind).label();
    let entries = world.body["entries"].as_array().expect("EdgePageOut.entries must be an array").clone();
    assert!(!entries.is_empty(), "test needs at least one real entry to prove the bijection over");
    for entry in &entries {
        let target_id = entry["node"]["id"].as_str().unwrap();
        let this_edge = entry["edge"].as_str().unwrap();
        let (status, inverse_page) = get(&format!("/api/node/{}/edges?kind={inverse_label}&limit=200", path_encode(target_id))).await;
        assert_eq!(status, StatusCode::OK.as_u16(), "the target's own inverse-kind page must resolve");
        let inverse_entries = inverse_page["entries"].as_array().unwrap();
        assert!(
            inverse_entries.iter().any(|e| e["edge"].as_str() == Some(this_edge)),
            "edge id '{this_edge}' from '{}' must appear on '{target_id}''s own '{inverse_label}' page",
            world.last_traversal_id
        );
    }
}

#[then(regex = r#"^the response "entries" array has at most (\d+) entry$"#)]
fn then_entries_at_most(world: &mut AqcWorld, max: usize) {
    let entries = world.body["entries"].as_array().unwrap();
    assert!(entries.len() <= max, "expected at most {max} entries, got {}", entries.len());
}

#[then(expr = "a further page reached by following {string} never repeats an entry already seen")]
async fn then_pagination_no_repeats(world: &mut AqcWorld, next_field: String) {
    assert_eq!(next_field, "next");
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let id = world.last_traversal_id.clone();
    let kind = world.last_traversal_kind.clone();
    for entry in world.body["entries"].as_array().unwrap() {
        seen.insert(entry["edge"].as_str().unwrap().to_string());
    }
    let mut cursor = world.body["next"].as_u64().map(|n| n as usize);
    let mut pages_walked = 0;
    while let Some(c) = cursor {
        pages_walked += 1;
        assert!(pages_walked < 10_000, "pagination did not terminate within a sane number of pages");
        let (status, page) = get(&format!("/api/node/{}/edges?kind={kind}&limit=1&cursor={c}", path_encode(&id))).await;
        assert_eq!(status, StatusCode::OK.as_u16());
        let entries = page["entries"].as_array().unwrap();
        if entries.is_empty() {
            break;
        }
        for entry in entries {
            let edge_id = entry["edge"].as_str().unwrap().to_string();
            assert!(seen.insert(edge_id.clone()), "edge id '{edge_id}' was already seen on an earlier page -- pagination repeated an entry");
        }
        cursor = page["next"].as_u64().map(|n| n as usize);
    }
}

#[then(regex = r#"^the response has exactly (\d+) units?$"#)]
fn then_exactly_n_units(world: &mut AqcWorld, n: usize) {
    let units = world.body["units"].as_array().unwrap();
    assert_eq!(units.len(), n);
}

#[then(expr = "unit {int}'s {string} field equals {string}")]
fn then_unit_field_equals(world: &mut AqcWorld, one_based_index: usize, field: String, expected: String) {
    let units = world.body["units"].as_array().unwrap();
    let actual = units[one_based_index - 1][&field].as_str().unwrap();
    assert_eq!(actual, expected);
}

#[then(regex = r#"^the units' "ref" fields are "([^"]+)", "([^"]+)", "([^"]+)" in order$"#)]
fn then_units_refs_in_order(world: &mut AqcWorld, a: String, b: String, c: String) {
    let units = world.body["units"].as_array().unwrap();
    let refs: Vec<&str> = units.iter().map(|u| u["ref"].as_str().unwrap()).collect();
    assert_eq!(refs, vec![a.as_str(), b.as_str(), c.as_str()]);
}

#[then(expr = "every {string} span lies within its own verse's text length")]
fn then_spans_within_length(world: &mut AqcWorld, field: String) {
    assert_eq!(field, "words_of_christ");
    let units = world.body["units"].as_array().unwrap();
    for unit in units {
        let len = unit["text"].as_str().unwrap().chars().count();
        for span in unit["words_of_christ"].as_array().unwrap() {
            let start = span["start"].as_u64().unwrap() as usize;
            let end = span["end"].as_u64().unwrap() as usize;
            assert!(start <= end && end <= len, "span [{start},{end}) is outside its own verse's text length {len} (ref {})", unit["ref"]);
        }
    }
}

#[then(expr = "{string} is empty")]
fn then_field_is_empty_array(world: &mut AqcWorld, field: String) {
    let arr = world.body[&field].as_array().unwrap_or_else(|| panic!("field '{field}' is not an array: {}", world.body));
    assert!(arr.is_empty(), "field '{field}' expected empty, got {} entries", arr.len());
}

#[then(expr = "the server advertises AQC version {string} through {string}")]
fn then_server_advertises(world: &mut AqcWorld, min: String, max: String) {
    assert_eq!(world.body["min_version"].as_str().unwrap(), min);
    assert_eq!(world.body["max_version"].as_str().unwrap(), max);
}

#[then(expr = "the client accepts the advertised range")]
fn then_client_accepts(world: &mut AqcWorld) {
    let result = satisfies(harness_client_version(), &world.advertised_min, &world.advertised_max)
        .unwrap_or_else(|e| panic!("expected a well-formed advertised range [{}, {}], got: {e}", world.advertised_min, world.advertised_max));
    assert!(result, "expected client version {} to satisfy [{}, {}]", harness_client_version(), world.advertised_min, world.advertised_max);
}

#[then(expr = "the client rejects the advertised range")]
fn then_client_rejects(world: &mut AqcWorld) {
    let result = satisfies(harness_client_version(), &world.advertised_min, &world.advertised_max)
        .unwrap_or_else(|e| panic!("expected a well-formed advertised range [{}, {}], got: {e}", world.advertised_min, world.advertised_max));
    assert!(!result, "expected client version {} to be REJECTED by [{}, {}]", harness_client_version(), world.advertised_min, world.advertised_max);
}

/// Q-4 fix (fix round 1, controller ruling): a MALFORMED advertised
/// version must fail LOUD -- `satisfies` returning `Err`, the Rust mirror
/// of `AqcContract.ParseSemver`'s own C# `FormatException` throw.
#[then(expr = "the malformed advertisement fails loud")]
fn then_malformed_advertisement_fails_loud(world: &mut AqcWorld) {
    let result = satisfies(harness_client_version(), &world.advertised_min, &world.advertised_max);
    assert!(
        result.is_err(),
        "expected the semver-range check to fail loud (Err) on advertised range [{}, {}], got {:?}",
        world.advertised_min,
        world.advertised_max,
        result
    );
}

#[tokio::main]
async fn main() {
    // `env!("CARGO_MANIFEST_DIR")`-anchored (compile-time), NOT a bare
    // runtime-relative string -- robust regardless of the CWD `cargo
    // test` happens to launch this binary from (same discipline `app()`'s
    // own `data_dir` above already uses).
    let features_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/atlas-query-contract/features");
    AqcWorld::cucumber().fail_on_skipped().run_and_exit(features_dir).await;
}
