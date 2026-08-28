# AQC Gherkin phrase glossary

Spec §3: "both bindings implement the same glossary." Every phrase used
across `features/*.feature` means EXACTLY the thing defined here, in BOTH
the Rust (`cucumber`) and C# (Reqnroll) step definitions. A phrase not
listed here has no business appearing in a `.feature` file — add it here
first, in the same commit as its first use.

## Setup / fixture phrases

- **"the real committed graph"** — the graph built from `data/raw` +
  `data/curated` via the SAME compile path `atlas_etl::compile::compile` +
  `GraphService::build` every real-data integration test in
  `server/atlas-server/tests/` already uses (`real_app()`'s own pattern) —
  never a synthetic fixture. On the C# side: the committed provider-exported
  fixture files under `contracts/atlas-query-contract/fixtures/` (§ below),
  since the C# harness proves DESERIALIZATION, not a second live server.
- **"a node of kind `<Kind>`"** — one seed `AnyNodeId{kind,raw}` this
  contract's exporter has verified resolves against the real committed
  graph (`export_aqc_examples` bin) — see each feature's own `Examples:`
  table for the concrete `id`.
- **"the live server"** (Rust only) — the real `atlas_server::app::build`
  `Router`, exercised in-process via `tower::ServiceExt::oneshot` (no socket
  bound) — the SAME idiom `tests/graph_api.rs`'s own `real_app()` uses.
- **"the committed fixture for `<query>`"** (C# only) — the JSON file
  `contracts/atlas-query-contract/fixtures/<query>.json`, byte-identical
  provider output captured by the same exporter.

## Query phrases

- **"I run FocusQuery for `<id>`"** — `GET /api/node/{id}`.
- **"I run TraversalQuery for `<id>` frontier `<kind>`"** — `GET
  /api/node/{id}/edges?kind=<kind>`.
- **"I run TextWindowQuery for `<ref>` radius `<n>`"** — `GET
  /api/text?ref=<ref>&n=<n>`.
- **"I run SceneQuery for the time window `<from>`-`<to>`"** — `GET
  /api/scene?from=<from>&to=<to>`.
- **"I run SceneQuery for scripture ref `<ref>`"** — `GET
  /api/scene/scripture?ref=<ref>`.
- **"I capture the returned focus reference"** — read the response's own
  `id` field (FocusQuery) or one `entries[].node.id` (TraversalQuery) —
  the wire node id, `graph_wire::encode_node_id`'s own grammar.
- **"I run FocusQuery again for the captured reference"** — re-issues
  FocusQuery with the id captured immediately above.
- **"I query `/api/contract`"** — `GET /api/contract`.

## Assertion phrases ("Then")

- **"the response is a valid `<Shape>`"** — every REQUIRED field in
  `aqc.schema.json`'s own `$defs.<Shape>` is present with the documented
  type; no field outside that schema's own `properties` appears
  (`additionalProperties: false`).
- **"every frontier group is a relations! family"** — every
  `edge_summary[].kind` (FocusQuery) or `kind` (TraversalQuery response)
  string is a label `graph_wire::parse_edge_kind` accepts — i.e. drawn from
  `RelationId::ALL`/`SymRelationId::ALL`, the crate's own relation
  manifest, never a hand-invented string.
- **"every traversal target resolves to a live node"** — for every
  `entries[].node.id` a TraversalQuery page returns, FocusQuery on that
  SAME id succeeds (200, a valid NodeCardOut) — never a dangling reference.
- **"the focus reference round-trips identically"** — the id captured
  ("I capture the returned focus reference") is byte-identical to the `id`
  field FocusQuery-on-that-id returns the second time — the G2 seam's
  wire-level bijection law (`encode_node_id(decode_node_id(s)) == s`).
- **"every `words_of_christ` span lies within its own verse's text
  length"** — for every unit in a TextWindowQuery response, every
  `words_of_christ[].start`/`.end` satisfies `0 <= start <= end <=
  text.length` (char count, RED-1's alignment law) for that SAME unit's
  own `text`.
- **"`quiet_places` is empty"** — the scripture-mode Scene law (E2/
  QUIET-1): a scripture-mode `Scene.quiet_places` is always `[]`.
- **"the server advertises AQC version `<min>` through `<max>`"** —
  `/api/contract`'s own `min_version`/`max_version` fields match exactly.
- **"the client accepts the advertised range"** / **"the client rejects
  the advertised range"** — `AqcContract.Satisfies(ContractDto)` (client)
  returns true/false for the given `min_version`/`max_version` pair against
  the client's own compiled `AqcContract.ClientVersion`.
- **"the app shows the contract-mismatch page"** (Playwright only) — the
  fail-loud house law's visible surface: `Pages/ContractMismatch.razor`'s
  own testid is present in the DOM instead of the ordinary app shell.
