# AQC Gherkin phrase glossary

Spec §3: "both bindings implement the same glossary." Every phrase used
across `features/*.feature` means EXACTLY the thing defined here, in BOTH
the Rust (`cucumber`) and C# (Reqnroll) step definitions. A phrase not
listed here has no business appearing in a `.feature` file — add it here
first, in the same commit as its first use.

**Fix round 1 (S-2, controller ruling):** the corpus uses 30 distinct
step phrases (verified by extracting every `Given`/`When`/`Then`/`And`
line across all six `.feature` files and normalizing quoted values/
integers). All 30 are defined below. Where the two bindings genuinely
differ in PROOF DEPTH (not merely implementation), the difference is
stated explicitly in that phrase's own entry — never left to a code
comment alone (that was S-2's own finding: the divergence between the
two bindings is exactly what an incomplete glossary hides).

## Setup / fixture phrases

- **"the real committed graph"** — the graph built from `data/raw` +
  `data/curated` via the SAME compile path `atlas_etl::compile::compile` +
  `GraphService::build` every real-data integration test in
  `server/atlas-server/tests/` already uses (`real_app()`'s own pattern) —
  never a synthetic fixture. On the C# side: the committed provider-exported
  fixture files under `contracts/atlas-query-contract/fixtures/` (§ below),
  since the C# harness proves DESERIALIZATION, not a second live server.
- **"a node of kind `<Kind>` with id `<id>`"** (Given) — one seed
  `AnyNodeId{kind,raw}` this contract's exporter has verified resolves
  against the real committed graph (`export_aqc_examples` bin) — see each
  feature's own `Examples:` table for the concrete `id`. Documentation-only
  on BOTH sides (neither binding makes an HTTP call or fixture load for
  this step) — the exporter's own fail-loud verification (parses,
  round-trips through `graph_wire::{encode,decode}_node_id`, resolves
  against the live graph) is what actually proves the claim, BEFORE the
  id is ever written into an `Examples:` table.
- **"the live server"** (Rust only) — the real `atlas_server::app::build`
  `Router`, exercised in-process via `tower::ServiceExt::oneshot` (no socket
  bound) — the SAME idiom `tests/graph_api.rs`'s own `real_app()` uses.
- **"the committed fixture for `<query>`"** (C# only) — the JSON file
  `contracts/atlas-query-contract/fixtures/<query>.json`, byte-identical
  provider output captured by the same exporter.
- **"the identity index"** (C# only, fix round 1 / S-1) — `contracts/
  atlas-query-contract/fixtures/index.json`, a wire-id → fixture-name map
  the exporter builds from the REQUEST id of every FocusQuery it captures
  (never from a hand-typed C# switch). See "the focus reference
  round-trips identically" below for what this makes provable.

## Query phrases ("When")

- **"I run FocusQuery for `<id>`"** — `GET /api/node/{id}`. Rust: a live
  HTTP call. C# (fix round 1): looks `<id>` up in the identity index
  (real node ids) or a small fixed table of the two deliberately-invalid
  request ids (`Person:nonexistent-xyz`, `not-even-a-colon-pair`) — this
  step ALSO records `<id>` as "what this scenario originally requested,"
  read by "the focus reference round-trips identically" below.
- **"I run FocusQuery again for the captured reference"** — re-issues
  FocusQuery with the id captured by "I capture the returned focus
  reference." Rust: a second live HTTP call. C# (fix round 1): looks the
  captured id up in the identity index ONLY (no error-case fallback — a
  captured reference is never one of the two deliberately-invalid
  inputs); a captured id with no index entry throws immediately.
- **"I run TraversalQuery for `<id>` frontier `<kind>`"** — `GET
  /api/node/{id}/edges?kind=<kind>`. Also records `<id>`/`<kind>` as "the
  last traversal this scenario ran" (read by the bijection-witness and
  pagination assertions below), and clears the FocusQuery-only
  "originally requested id" tracking (this step is not a FocusQuery).
- **"I run TraversalQuery for `<id>` frontier `<kind>` with limit
  `<n>`"** — the same request with `&limit=<n>` appended.
- **"I run TextWindowQuery for `<ref>` radius `<n>`"** — `GET
  /api/text?ref=<ref>&n=<n>`.
- **"I run TextWindowQuery for `<ref>` radius `<n>` with corpus
  `<corpus>`"** — the same request with `&corpus=<corpus>` appended.
- **"I run a chapter-scoped TextWindowQuery for `<ref>` with dir
  `<dir>`"** — `GET /api/text?ref=<ref>&scope=chapter&dir=<dir>`.
- **"I run SceneQuery for the time window `<from>`-`<to>`"** — `GET
  /api/scene?from=<from>&to=<to>`.
- **"I run SceneQuery for scripture ref `<ref>`"** — `GET
  /api/scene/scripture?ref=<ref>`.
- **"I capture the returned focus reference"** — read the response's own
  `id` field (FocusQuery) or the FIRST `entries[].node.id` (TraversalQuery)
  — the wire node id, `graph_wire::encode_node_id`'s own grammar. Both
  sides read this from the SAME field of the SAME already-loaded
  response; no separate request is made by this step on either side.
- **"I query `<path>`"** — `GET <path>` (only ever `/api/contract` in
  this corpus).

## Setup / assertion phrases ("Given"/"Then" on the versioning surface)

- **"the server advertises AQC version `<min>` through `<max>`"** — used
  as BOTH a `Given` (sets the scenario's own advertised-range state
  directly, no HTTP call, for testing `AqcContract.Satisfies` against an
  arbitrary range) and a `Then` (asserts the LAST response's own
  `min_version`/`max_version` fields equal `<min>`/`<max>` exactly — only
  meaningful after "I query `/api/contract`"). Two separate step bindings
  per side, same phrase text, disambiguated by Gherkin keyword.
- **"the client accepts the advertised range"** / **"the client rejects
  the advertised range"** — `AqcContract.Satisfies(ContractDto)` (C#, the
  real production implementation) returns true/false for the scenario's
  own advertised `min_version`/`max_version` against the client's own
  compiled `AqcContract.ClientVersion`. Rust: an independent local mirror
  of the SAME semver-range check (`satisfies()`, this file's own — there
  is no Rust CONSUMER in this app for `AqcContract` to live on) — the two
  sides prove the SAME pass/fail outcome via two separately-written
  implementations, which is itself part of what phrase parity is for.

## Assertion phrases ("Then")

- **"the response is a valid `<Shape>`"** — every REQUIRED field in
  `aqc.schema.json`'s own `$defs.<Shape>` is present with the documented
  type; no field outside that schema's own `properties` appears
  (`additionalProperties: false`).
- **"the response `<field>` field equals `<value>`"** — the last
  response's own `<field>` (a top-level string field) equals `<value>`
  exactly.
- **"the request fails with status `<n>` and code `<code>`"** — the last
  response's own HTTP status equals `<n>` and its `error.code` field
  equals `<code>` (`ApiError`'s own `{"error":{"code","message"}}` shape).
- **"every frontier group is a relations! family"** — **DEPTH DIFFERENCE,
  disclosed (spec §2's own "hand-written duplicates of contract shapes
  are forbidden" forces this):** Rust checks every `edge_summary[].kind`
  (FocusQuery) or `kind` (TraversalQuery response) string against
  `graph_wire::parse_edge_kind` — i.e. drawn from `RelationId::ALL`/
  `SymRelationId::ALL`, the crate's own relation manifest, never a
  hand-invented string. C# checks only that the SAME field deserializes
  as a non-empty string — duplicating the relation manifest in C# would
  itself be the forbidden "hand-written duplicate of contract shapes";
  the real fix is spec §2's not-yet-built "generated types on both
  sides." Same phrase, same pass/fail on every fixture this corpus
  carries today, different depth.
- **"every traversal target resolves to a live node"** — **DEPTH
  DIFFERENCE, disclosed:** Rust re-fetches EVERY entry's own id live
  (FocusQuery, expects 200) — a page can carry up to 20 entries, all
  proven live, every `cargo test`. C# structurally validates every entry
  (non-empty id/kind/label) and additionally cross-checks the FIRST
  entry's id against its own committed FocusQuery fixture (200, matching
  `id`) — fixturing all 20 entries' own FocusQuery responses to re-prove
  a property the live Rust side already proves would buy nothing.
- **"every entry's `edge` id is present on the matching inverse-kind page
  of its own target node"** (the bijection witness) — **DEPTH DIFFERENCE,
  disclosed:** Rust walks the REAL inverse-kind page
  (`atlas_graph_types::edge::dual`) for every entry and asserts the SAME
  edge id appears there — the real graph-structural bijection law, proven
  live. C# asserts every entry's own `edge` field deserializes as a
  non-empty string — the wire FIELD the bijection travels on, not the
  bijection itself (this corpus deliberately carries only one direction's
  fixture per pair, e.g. `traversal-located-at.json` but not its own
  inverse `site-of` page — capturing both would double the fixture count
  for a property already proven live, server-side).
- **"a further page reached by following `next` never repeats an entry
  already seen"** — **DEPTH DIFFERENCE, disclosed:** Rust walks EVERY
  page live (`cursor` from each response's own `next`), accumulating a
  seen-set of `edge` ids, asserting no repeat, until `next` is `null` —
  the real pagination law, proven end-to-end against the live graph. C#
  checks the single committed page (`traversal-cites-limit1.json`, one
  entry, `limit=1`) has at most one entry and that its own `next` field
  deserializes as the expected non-null int — the wire FIELD the live
  pagination walk depends on, not a fixture-only re-walk of every page
  (which would mean committing and hand-verifying an unbounded fixture
  chain for a property the live Rust side already proves exhaustively).
- **"the focus reference round-trips identically"** — the id captured
  ("I capture the returned focus reference") is byte-identical to the
  `id` field FocusQuery-on-that-id returns the second time — the G2
  seam's wire-level bijection law
  (`encode_node_id(decode_node_id(s)) == s`). **Fix round 1 (S-1,
  formerly Critical — the prior C# binding loaded the SAME fixture file
  via the SAME lookup function on both sides of the comparison, making
  this assertion true by construction for all 11 outline rows, provable
  false verified empirically during the fix by corrupting a committed
  fixture's own `id` field and confirming the test then failed.**
  Now, on BOTH sides, this step ALSO asserts the captured reference
  equals what the scenario's own initiating FocusQuery originally
  requested (skipped for the one scenario whose capture instead
  originates from a TraversalQuery target — there the captured id is
  legitimately different from anything requested so far, by design; the
  captured id must still resolve back to itself on a second, genuinely
  independent lookup). Rust: the second fetch is a real live HTTP call.
  C#: the second lookup goes through the identity index ("the identity
  index" above), keyed by the response's OWN echoed id, not re-derived
  via the same function call the first lookup used — a captured id with
  no index entry fails loud (`InvalidOperationException`) rather than
  silently resolving.
- **"every `words_of_christ` span lies within its own verse's text
  length"** — for every unit in a TextWindowQuery response, every
  `words_of_christ[].start`/`.end` satisfies `0 <= start <= end <=
  text.length` for that SAME unit's own `text` (RED-1's alignment law).
  **T-5 note (trivia, disclosed rather than silently divergent):** Rust
  measures `text.length` as `chars().count()` (Unicode scalar values);
  C# measures it as `string.Length` (UTF-16 code units). The two are
  equal for every verse this contract ever serves (`handlers.rs`'s own
  doc comment: this app's KJV text is entirely within the Basic
  Multilingual Plane, so char count == UTF-16 code unit count always) —
  and the C# unit is arguably the more CONSUMER-TRUE one, since it is the
  unit `WordsOfChristSpanDto`'s own consumers (`string.Substring`) index
  by. Not a defect; recorded here so the equivalence is stated once
  rather than left as two silently different definitions of "length."
- **"`<field>` is empty"** — the last response's own `<field>` (an array)
  has zero elements. Used in this corpus only for `"quiet_places" is
  empty` (the scripture-mode Scene law, E2/QUIET-1: a scripture-mode
  `Scene.quiet_places` is always `[]`).
- **"the response `<field>` array has at most `<n>` entry"** — the last
  response's own `<field>` array (always `entries` in this corpus) has at
  most `<n>` elements.
- **"the response has exactly `<n>` unit(s)"** — the last TextWindowQuery
  response's own `units` array has exactly `<n>` elements (both sides
  match the singular and plural wording via one regex/expression).
- **"unit `<n>`'s `<field>` field equals `<value>`"** — the last
  TextWindowQuery response's own `units[<n>-1].<field>` (1-based index)
  equals `<value>`.
- **"the units' `ref` fields are `<a>`, `<b>`, `<c>` in order"** — the
  last TextWindowQuery response's own `units[].ref` values equal
  `[<a>,<b>,<c>]` in that exact order.

## Playwright-only phrases (T-4, marked explicitly — not bound by either Gherkin harness)

- **"the app shows the contract-mismatch page"** — never appears in a
  `.feature` file; describes `tests/ux/contract-versioning.spec.ts`'s own
  browser-level assertion (`Pages/ContractMismatch.razor`'s own testid
  present in the DOM instead of the ordinary app shell — see that spec's
  own two cases, happy path and mocked mismatch). Listed here only
  because this file's own stated contract is phrase parity between the
  two Gherkin bindings, and a reader should not mistake this entry for a
  third binding of a corpus phrase.
- **"the app loads normally despite an unreachable `/api/contract`"** —
  Playwright-only (fix round 1, Q-5/§0): `route.abort()` on `/api/
  contract`, asserting the ordinary app shell (`nav-world`) renders
  regardless — the surviving policy `App.razor`'s narrowed catch still
  covers (network failures are not a mismatch).
