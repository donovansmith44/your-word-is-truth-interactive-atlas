//! Batch AQC-1's own provider-side exporter (spec §3) -- the PURE
//! (no I/O) generation logic, factored out of `src/bins/
//! export_aqc_examples.rs` in fix round 1 (Q-6) so a standing test
//! (`tests/aqc_corpus_generation.rs`) can call the SAME functions the
//! exporter binary calls and assert the committed `.feature` files are
//! byte-identical to what they'd regenerate to -- "generated-output-is-
//! committed + a test that regenerates and diffs," the pattern this
//! repo already uses elsewhere (the review's own phrase). The bin itself
//! (`src/bins/export_aqc_examples.rs`) does everything ELSE this module
//! doesn't: opening the real graph, capturing real HTTP fixtures, and
//! writing files to disk -- all I/O, none of it here.
//!
//! Run the exporter from `server/`:
//! `cargo run -p atlas-server --bin export_aqc_examples`.

/// One (NodeKind Debug string, wire id) seed per real node kind this
/// contract samples -- SAME id list `graph_wire.rs`'s own round-trip unit
/// test and `tests/graph_api.rs`'s own real-data HTTP tests already prove
/// live against the committed graph (`ab_ur`, `aaron_1`, `kretzmann/0.1.0`,
/// ... -- not invented for this exporter). `commandment-1` (CatechismItem)
/// is `data/curated/catechism.toml`'s own first commandment item id;
/// `latin_vulgate` (Translation) is one of the six CORP-1a-ingested
/// parallel editions -- deliberately NOT "kjv", which
/// `brainfuel_layers.rs::no_kjv_translation_node_is_authored_this_batch`
/// proves does NOT exist as a Translation node (the KJV is the canonical
/// TextUnit layer itself, never a Translation-kind node of its own).
pub const SEEDS: &[(&str, &str)] = &[
    ("TextUnit", "text-unit:JHN.3.16"),
    ("Event", "Event:ab_ur"),
    ("Narrative", "Narrative:abraham-migration"),
    ("Anchor", "Anchor:solomon-crowned"),
    ("Place", "Place:ur-1"),
    ("Era", "Era:primeval"),
    ("Polity", "Polity:egypt"),
    ("Person", "Person:aaron_1"),
    ("Translation", "Translation:latin_vulgate"),
    ("CommentaryItem", "CommentaryItem:kretzmann/0.1.0"),
    ("CatechismItem", "CatechismItem:commandment-1"),
];

/// S-1 fix (Batch AQC-1 fix round 1, controller ruling): a second,
/// non-SEEDS FocusQuery identity capture -- exploration-roundtrip.feature's
/// own "a traversal target's own id round-trips too" scenario captures a
/// TraversalQuery entry's id (`text-unit:ROM.5.8`, JHN.3.16's own real
/// first "cites" target) and re-runs FocusQuery on it. Its fixture name
/// joins the SEEDS-derived ones in the identity INDEX (`index.json`) the
/// bin writes.
pub const FOCUS_IDENTITY_EXTRA: &[(&str, &str)] = &[("text-unit:ROM.5.8", "focus-traversal-target")];

/// Provider-exported response FIXTURES (brief deliverable 3, "Fixtures:
/// provider-exported response fixtures committed pact-style"): one JSON
/// file per DISTINCT real HTTP request this contract's Gherkin corpus
/// makes across ALL SIX feature files (deduplicated -- a request repeated
/// across two scenarios, e.g. TextUnit's FocusQuery in both
/// focus-query.feature and exploration-roundtrip.feature, gets ONE
/// fixture, reused). Captured via the SAME in-process
/// `tower::ServiceExt::oneshot` idiom the Rust cucumber harness uses --
/// real committed-graph HTTP responses, never hand-typed JSON. Each file
/// is `{"status": <u16>, "body": <json>}` so error-path scenarios (400/404)
/// are provable from a fixture too, not just success shapes. The C#
/// harness (thin, contract-ignorant) deserializes these through the
/// client's own DTOs -- it never talks to a live server.
pub const FIXTURES: &[(&str, &str)] = &[
    // FocusQuery error cases (the 11 success cases are derived from SEEDS
    // below -- one "focus-<kind-lowercase>" fixture per real node kind).
    ("focus-not-found", "/api/node/Person:nonexistent-xyz"),
    ("focus-bad-ref", "/api/node/not-even-a-colon-pair"),
    // exploration-roundtrip.feature's own "a traversal target's own id
    // round-trips too" scenario captures a TraversalQuery entry's own id
    // (text-unit:ROM.5.8, JHN.3.16's own first "cites" target) and
    // re-runs FocusQuery on it -- this is that second FocusQuery's fixture.
    ("focus-traversal-target", "/api/node/text-unit:ROM.5.8"),
    // TraversalQuery
    ("traversal-cites", "/api/node/text-unit:JHN.3.16/edges?kind=cites"),
    ("traversal-cites-limit1", "/api/node/text-unit:JHN.3.16/edges?kind=cites&limit=1"),
    ("traversal-located-at", "/api/node/Event:ab_ur/edges?kind=located-at"),
    ("traversal-bad-kind", "/api/node/text-unit:JHN.3.16/edges?kind=not-a-real-kind"),
    // TextWindowQuery
    ("text-window-single", "/api/text?ref=JHN.3.16&n=1"),
    ("text-window-multi", "/api/text?ref=JHN.3.16&n=3"),
    ("text-window-mat-4-19", "/api/text?ref=MAT.4.19&n=1"),
    ("text-window-mat-5-4", "/api/text?ref=MAT.5.4&n=1"),
    ("text-window-chapter-backward-bad-dir", "/api/text?ref=JHN.3&scope=chapter&dir=backward"),
    ("text-window-bad-corpus", "/api/text?ref=JHN.3.16&n=1&corpus=not-a-real-corpus"),
    // SceneQuery
    ("scene-time", "/api/scene?from=-2100&to=-2000"),
    ("scene-scripture", "/api/scene/scripture?ref=JHN.3.16"),
    ("scene-bad-window", "/api/scene?from=100&to=-100"),
    ("scene-bad-ref", "/api/scene/scripture?ref=not-a-ref-at-all"),
    // Versioning -- the one new behavioral endpoint (contract.rs).
    ("contract", "/api/contract"),
];

/// Percent-encodes the one wire-id character that collides with axum's own
/// path-segment routing (`/`, e.g. `CommentaryItem:kretzmann/0.1.0`) --
/// same convention `tests/graph_api.rs`'s own literal
/// `"CommentaryItem:kretzmann%2F0.1.0"` URIs already establish.
pub fn path_encode(id: &str) -> String {
    id.replace('/', "%2F")
}

pub fn examples_table(indent: &str) -> String {
    let kind_w = SEEDS.iter().map(|(k, _)| k.len()).chain(["kind".len()]).max().unwrap();
    let id_w = SEEDS.iter().map(|(_, i)| i.len()).chain(["id".len()]).max().unwrap();
    let mut out = String::new();
    out.push_str(&format!("{indent}| {:<kind_w$} | {:<id_w$} |\n", "kind", "id"));
    for (kind, id) in SEEDS {
        out.push_str(&format!("{indent}| {kind:<kind_w$} | {id:<id_w$} |\n"));
    }
    out
}

pub fn focus_query_feature() -> String {
    format!(
        r#"# AQC v0.1.0 -- FocusQuery(descriptor) -> Focus (spec §2, §3).
# GET /api/node/{{id}} -- server/atlas-server/src/graph_handlers.rs::node_card.
#
# The Examples: table below is GENERATED, not hand-authored -- see
# server/atlas-server/src/bins/export_aqc_examples.rs. It draws one seed id per
# NODE KIND the real committed graph materializes (spec §3: "every node kind
# sampled from the graph"), verified live against that graph at export time
# (a stale seed id fails the exporter loud, not silently). Re-running the
# exporter against an unchanged graph reproduces this table byte-identical
# (deterministic; never wall-clock random).
Feature: FocusQuery -- one node's card, by descriptor

  Scenario Outline: every sampled node kind resolves to a valid Focus card
    Given a node of kind "<kind>" with id "<id>"
    When I run FocusQuery for "<id>"
    Then the response is a valid "NodeCardOut"
    And the response "id" field equals "<id>"
    And the response "kind" field equals "<kind>"
    And every frontier group is a relations! family

    Examples:
{examples}
  Scenario: an id that parses but names no real node is not_found
    Given a node of kind "Person" with id "Person:nonexistent-xyz"
    When I run FocusQuery for "Person:nonexistent-xyz"
    Then the request fails with status 404 and code "not_found"

  Scenario: a malformed id is bad_ref
    When I run FocusQuery for "not-even-a-colon-pair"
    Then the request fails with status 400 and code "bad_ref"
"#,
        examples = examples_table("      ")
    )
}

pub fn exploration_roundtrip_feature() -> String {
    format!(
        r#"# AQC v0.1.0 -- ExplorationRoundTrip(descriptor): descriptor -> node ->
# descriptor identity (the G2 seam, now a contract law -- spec §2). At the
# WIRE level this is graph_wire::encode_node_id/decode_node_id's own
# bijection: the id a FocusQuery/TraversalQuery response hands back always
# re-resolves the SAME node when fed straight back into FocusQuery.
#
# The Examples: table is GENERATED by the same exporter as focus-query.feature
# (server/atlas-server/src/bins/export_aqc_examples.rs) -- one seed id per real
# node kind, verified live against the committed graph.
Feature: ExplorationRoundTrip -- descriptor -> node -> descriptor identity

  Scenario Outline: a captured focus reference round-trips identically
    Given a node of kind "<kind>" with id "<id>"
    When I run FocusQuery for "<id>"
    And I capture the returned focus reference
    And I run FocusQuery again for the captured reference
    Then the focus reference round-trips identically

    Examples:
{examples}
  Scenario: a traversal target's own id round-trips too
    Given a node of kind "TextUnit" with id "text-unit:JHN.3.16"
    When I run TraversalQuery for "text-unit:JHN.3.16" frontier "cites"
    And I capture the returned focus reference
    And I run FocusQuery again for the captured reference
    Then the focus reference round-trips identically
"#,
        examples = examples_table("      ")
    )
}
