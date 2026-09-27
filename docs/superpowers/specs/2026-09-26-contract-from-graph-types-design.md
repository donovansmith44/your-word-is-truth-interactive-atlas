# CONTRACT-1: the graph is the contract — design

**Status:** AWAITS OWNER SIGN-OFF (types and signatures in §5–§6 are the
sign-off surface; §10 lists what is still open).
**Governed by:** `docs/PRINCIPLES.md`.
**Successors:** CONTRACT-2 (logic pushdown), FOCUS-1…n (Explorable /
Exploration / Focus redesign). Each gets its own spec; this one stops at the
seams named in §3.

## 1. Problem

Every wire shape exists three times by hand — the Rust `*Out` struct, its
`$defs` entry in `contracts/atlas-query-contract/aqc.schema.json`, and a C#
record in `client/Dtos.cs` — and the vocabulary declared once in
`graph-types` (`relations!`, `kind_tags!`) is re-typed on the client as
`EdgeKindId("mentions")` literals and kind strings. The contract version is
a hand-bumped constant on both sides. The existing AGC/AQC suites detect
drift; nothing prevents it. Seven of the 24 routes have no contract at all,
and only 21 of the ~60 served types have any published shape.

## 2. Decisions (owner, 2026-09-26)

| # | Decision | Ruling |
|---|---|---|
| D1 | What is the contract? | **The Rust is the contract**: `graph-types` for vocabulary, the server's response types for shapes. Everything else is derived. (Neutral-IDL rejected.) |
| D2 | Where do derived artifacts live? | **Generated *documents* are committed and gated by regenerate-and-diff; generated *code* is built, never committed** (owner, 2026-09-26: "can't we just generate it as a CI/CD step"). `contracts/openapi.yaml` is committed — it is the contract, the thing reviewers diff, and the input to every consumer. `Wire.g.cs` is produced into `obj/` by the client build from that committed document; no cargo, no network beyond `dotnet tool restore`. |
| D3 | How is the C# produced? | **Off-the-shelf, no hand-rolled emitter.** `utoipa` + `utoipa-axum` derive an OpenAPI document from the handlers and their response types; **NSwag** (a dotnet tool) generates the C# records from it. Nothing in this repo writes C# source by string templating. |
| D4 | Type names? | **One name per type, the domain's, in every language.** The Rust `*Out` suffix is dropped at the source (it meant "response struct, not the domain struct" — a module boundary says that better): response types live in `wire` modules (`wire::Polity` beside `data::Polity`, as `atlas-core/src/wire.rs` already does for `Scene`). The OpenAPI component, the C# record and the Rust struct are all `NodeCard`, `EdgePage`, `TextWindow`, `Chapter`, `PlaceDetail`. Generated names are never remapped. |
| D5 | Behaviour change? | **None.** A snapshot batch, like AQC-1: every AGC/AQC fixture and all Playwright specs stay byte/behaviour-identical. |
| D6 | Is the frontier a wire concept? | **Yes** — `edge_summary` names its groups, `/api/node/{id}/edges?kind=` pages them (verified live: `Container:bible-chapter-GEN-1` → `follows-in` → `Container:bible-chapter-GEN-2`). Presentation, NavigationRules and EscapeHatches are client UX, never wire. Closes AQC §9 Q5. |
| D7 | Runtime handshake? | **Deleted, not rewritten.** Agreement is proven in CI; client and server ship together. `App.razor` becomes the router; `AqcContract.cs`, `ContractMismatch.razor`, `AtlasClient.Contract()` go. Supersedes AQC-1's startup law. |
| D8 | The published contract | **One source document, `contracts/openapi.yaml`**, generated, committed, YAML — what NSwag reads and what an outside consumer reads. The hand-written `aqc.schema.json` (with its `x-queries`) and the hand-blessed `graph-vocabulary.json` are **replaced by derivations of that document at the same paths**, emitted by the same exporter and covered by the same regenerate-and-diff gate: `aqc.schema.json` = the document's `components.schemas` as `$defs` (so the Rust and C# AQC schema steps are untouched), `graph-vocabulary.json` = its kind enums + `x-atlas-relations` in the shape AGC's vendored runner already pins. Nothing is hand-maintained. No custom content hashes: the document is the declaration, and CI asserts served ≡ committed. |
| D10 | Where does the contract live? | **In one crate, `server/atlas-contract`, which *is* the server's HTTP surface** (owner: "the contract defined in one place … minimal code"). Handlers, response types, `AppState`, `ApiError`, the route registry, the document builder and the exporter all live there; `atlas-server` becomes the binary alone. If a route is not in `atlas-contract`, it is not promised. Moved, not written: no macro of ours; utoipa's `routes!`, `IntoParams` and `IntoResponses` keep each route's declaration to one line. |
| D9 | Where do the kinds' wire form and schema live? | **On the originals, in `graph-types`, once** (owner: "D.R.Y."). No mirror enums, no schema builders, no `serialize_with` in `atlas-server`. The macros emit the pure data (`NodeKind::name()`, `EdgeKind::labels()`, both inverses); the four `Serialize`/`ToSchema` impls live in one feature-gated `graph-types/src/wire_form.rs` — "how a kind appears on the wire" — so `id.rs`/`edge.rs` never mention a dependency. This **amends the zero-dependency law**: `graph-types` may carry *optional* dependencies behind default-off features (`serde`, `openapi`); its default build stays dependency-free, so map-generator's path dependency is unaffected. |

## 3. Scope

**In:** `utoipa` annotations on the 24 handlers and `ToSchema` on every
served type; typed kind fields on the Rust response structs
(wire-identical); the generated `openapi.yaml`; NSwag-generated wire
records; a hand-written `EdgeKinds` helper (`Dual`, `IsSymmetric`) over the
generated `EdgeKind` enum;
deletion of `Dtos.cs`, `AqcContract.cs`, `ContractMismatch.razor`,
`AtlasClient.Contract()`, `EdgeKindId`, `aqc.schema.json`,
`graph-vocabulary.json` — the full deletion inventory is §11 and every
line of it is a task in this batch; the regenerate-and-diff gates; the
no-route-without-a-contract law with pins for the seven uncovered routes;
Stryker.NET and cargo-mutants tooling; optional Swagger UI behind a dev
feature.

**Out (the seams):**
- `ExplorationDescriptor`'s client kind strings and `PopoverSectionRegistry`
  stay. Retyping them onto `NodeKind` is the Explorable factory of FOCUS-1.
- Logic pushdown (display years, versification, ref-span resolution, mention
  spans) is CONTRACT-2.
- `ContentsRoot.kind` / `ContentsChild.kind` stay `String`: the live
  value is a container sub-kind (`"book"`, `"chapter"`), not a `NodeKind`.
  If that set is closed, a Rust enum in FOCUS-1 makes it generated for free.
- A generated HTTP client (NSwag can emit one) is **not** in scope:
  `AtlasClient`'s caching (`AsyncMemo`, `LruCache`) is behaviour we keep.
  Models only.

## 4. The wire surface

All 24 routes; every handler returns a typed struct (zero `Json<Value>`).
Verified against the handler signatures 2026-09-26. `/health` returns a bare
string and is documented but has no schema.

| Route | Handler | Root type |
|---|---|---|
| `/health` | `meta::health` | (string) |
| `/api/contract` | `meta::contract` | `Contract` |
| `/api/scene` | `map::scene_time` | `Scene` |
| `/api/scene/scripture` | `map::scene_scripture` | `Scene` |
| `/api/books` | `reading::books` | `Vec<CanonBook>` |
| `/api/chapter/{cref}` | `reading::chapter` | `Chapter` |
| `/api/kretzmann/chapter/{cref}` | `reading::kretzmann_chapter` | `KretzmannChapter` |
| `/api/verse/{vref}` | `reading::verse` | `VerseDetail` |
| `/api/xrefs/{sref}` | `reading::xrefs` | `Vec<CrossRef>` |
| `/api/catechism/item/{id}` | `catechism::catechism_item` | `CatechismItem` |
| `/api/catechism/{sref}` | `catechism::catechism_for_span` | `Vec<CatechismRef>` |
| `/api/place/{id}` | `places::place` | `PlaceDetail` |
| `/api/narratives` | `events::narratives` | `Vec<Narrative>` |
| `/api/narrative/event/{id}` | `events::narrative_event_positions` | `NarrativeEventPositions` |
| `/api/event/{id}` | `events::event` | `EventDetail` |
| `/api/eras` | `map::eras` | `Vec<Era>` |
| `/api/polities` | `map::polities` | `Polities` |
| `/api/landmarks` | `map::landmarks` | `Vec<Landmark>` |
| `/api/land-mask` | `map::land_mask` | `LandMask` |
| `/api/sources` | `meta::sources` | `SourcesDocument` |
| `/api/node/{id}` | `graph::node_card` | `NodeCard` |
| `/api/node/{id}/edges` | `graph::node_edges` | `EdgePage` |
| `/api/text` | `graph::text_window` | `TextWindow` |
| `/api/contents/{corpus}` | `contents::contents` | `Contents` |

(Handler modules and type names as they will be after D4/D10 — today the
handlers sit in `atlas-server`'s `handlers.rs` / `graph_handlers.rs` /
`contents.rs` / `contract.rs` and each type carries an `Out` suffix. Struct
names are not on the wire, so the rename is byte-neutral.)

The schema set is the transitive closure of these root types. Error
responses keep AQC's taxonomy (`bad_ref`, `bad_kind`, `bad_window`,
`not_found`) and are documented on each path.

## 5. Server design

### 5.1 The contract crate — `server/atlas-contract` (D10)

The contract is the server's HTTP surface, so the HTTP surface becomes one
crate. Today's `atlas-server` *library* — handlers, response types,
`AppState`, `ApiError` — moves there by `git mv`, split by route family;
`atlas-server` keeps only `main.rs`. No cycle: `atlas-server` (bin) →
`atlas-contract` → `atlas-graph` / `atlas-core` → `graph-types`.

```
server/atlas-contract/
  src/lib.rs                  the families merged; router(state); openapi()
  src/graph.rs                node_card, node_edges, text_window        ─┐ handlers, their attributes,
  src/reading.rs              books, chapter, verse, xrefs, kretzmann_chapter │ and each family's routes()
  src/catechism.rs            catechism_item, catechism_for_span             │
  src/places.rs               place                                          │
  src/events.rs               event, narrative_event_positions, narratives   │
  src/map.rs                  scene_time, scene_scripture, eras, polities, landmarks, land_mask
  src/contents.rs             contents                                       │
  src/meta.rs                 health, contract, sources, openapi_yaml       ─┘
  src/wire/                   every response type, Out dropped (D4), one file per family
  src/state.rs, src/error.rs  AppState, ApiError (moved)
  src/document.rs             openapi() + x-atlas-relations + to_yaml()
  src/bins/export_contract.rs, src/bins/export_aqc_examples.rs
  tests/                      graph_api, aqc_cucumber, contract_api (moved); contract_generation, contract_coverage (new)
server/atlas-server/src/main.rs   config → state → atlas_contract::router(state) → serve
```

Each family file holds its handlers and enrols them; `lib.rs` merges the
families. Nothing else, anywhere, lists a route:

```rust
// atlas-contract/src/graph.rs
#[utoipa::path(get, path = "/api/node/{id}/edges",
    params(("kind" = EdgeKind, Query), ("cursor" = Option<usize>, Query), ("limit" = Option<usize>, Query)),
    responses((status = 200, body = wire::EdgePage), ApiError), tag = "graph")]
pub async fn node_edges(State(g): State<Arc<GraphService>>, Path(id): Path<String>, Query(params): Query<HashMap<String, String>>)
    -> Result<Json<wire::EdgePage>, ApiError> { … }   // body unchanged; kind parsed with EdgeKind::from_label

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(node_card)).routes(routes!(node_edges)).routes(routes!(text_window))
}

// atlas-contract/src/lib.rs
pub fn openapi_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .merge(meta::routes()).merge(map::routes()).merge(reading::routes()).merge(catechism::routes())
        .merge(places::routes()).merge(events::routes()).merge(graph::routes()).merge(contents::routes())
}
pub fn router(state: AppState) -> axum::Router { openapi_router().split_for_parts().0.with_state(state) }
pub fn openapi() -> OpenApi                   { openapi_router().split_for_parts().1 }
```

The attribute is short because the error taxonomy is declared once, on the
existing `ApiError`. It is a struct (`status`, `code`, `message`) with
seven constructors, not an enum, so `IntoResponses` is a manual impl in
`error.rs` — about a dozen lines — that lists exactly the responses those
constructors build: `bad_window`, `bad_ref`, `bad_kind`, `bad_dir`,
`bad_corpus` → 400; `not_found` → 404; `internal` → 500; body
`{ "error": { code, message } }` as an `ErrorBody` schema. Every route then
says `ApiError` rather than restating statuses. Shapes are **closed**: AQC's
schema step asserts `additionalProperties: false` on every `$defs` entry
today, and that law survives by putting `#[serde(deny_unknown_fields)]` on
every served struct — utoipa turns that one serde attribute into
`additionalProperties: false`, so the closedness is declared once, where
the struct is. Path parameters are inferred from `{id}` in the
path string; query parameters are listed inline. The handlers today read
queries as `HashMap<String, String>` and produce AQC's error bodies
themselves; that stays as it is (D5). Typed extractor structs with
`IntoParams` would shorten the attributes further, but a typed `Query<T>`
rejection is axum's plain 400, not `bad_window`, so that move needs a
rejection-to-`ApiError` mapping and belongs to a later batch.

`routes!` refuses a handler that lacks its attribute, and a handler that is
not enrolled is unreachable, so "no route without a contract" is structural
before the coverage test in §5.4 runs. Where a wire name meets a domain name
(`wire::Polity`, `data::Polity`) the module path says which is which.

Every served type (Appendix A) gains `#[derive(utoipa::ToSchema)]`. `utoipa`
and `utoipa-axum` are dependencies of `atlas-contract`; `utoipa` of
`atlas-core`; `atlas-contract` enables `graph-types`' `serde` and `openapi`
features.

### 5.2 The kinds describe themselves (D9)

`graph-types/Cargo.toml`:

```toml
[dependencies]
serde  = { version = "1", optional = true, default-features = false }
utoipa = { version = "5", optional = true }

[features]
default = []          # the default build has no dependencies, as before
serde   = ["dep:serde"]
openapi = ["dep:utoipa"]
# canon-ids stays exactly as it is; the two new features sit beside it
```

The type files stay pure data and gain only what the wire needs to *read*,
emitted by the macros that already own the names (no dependencies, no
attributes):

```rust
// graph-types/src/id.rs  — NodeKind is a hand-written enum with `ALL: [NodeKind; 15]` (id.rs:123);
// these two read that list, they do not repeat it:
impl NodeKind {
    pub fn name(self) -> &'static str;                  // "TextUnit", "Container", … — the Debug text the server already serves
    pub fn from_name(name: &str) -> Option<NodeKind>;   // ALL.iter().find(|k| k.name() == name)
}

// graph-types/src/edge.rs — relations! also emits:
impl EdgeKind {
    pub fn label(self) -> &'static str;                 // existing
    pub fn all() -> impl Iterator<Item = EdgeKind>;     // every Directed(r, d) then every Symmetric(s), declaration order
    pub fn labels() -> impl Iterator<Item = &'static str> { Self::all().map(EdgeKind::label) }
    pub fn from_label(label: &str) -> Option<EdgeKind>; // inverse of label(), total over labels()
}
```

Everything that is *about the wire* lives in one feature-gated file and
nowhere else — `id.rs` and `edge.rs` never mention serde or utoipa:

```rust
// graph-types/src/wire_form.rs   (lib.rs: #[cfg(any(feature = "serde", feature = "openapi"))] pub mod wire_form;)

#[cfg(feature = "serde")]
impl serde::Serialize for NodeKind { /* serialize_str(self.name()) */ }
#[cfg(feature = "serde")]
impl serde::Serialize for EdgeKind { /* serialize_str(self.label()) */ }

#[cfg(feature = "openapi")]
impl utoipa::ToSchema for NodeKind { /* component "NodeKind": string, enum = NodeKind::ALL.map(name) */ }
#[cfg(feature = "openapi")]
impl utoipa::ToSchema for EdgeKind { /* component "EdgeKind": string, enum = EdgeKind::labels() */ }
```

Four impls, each one line of substance, each reading the list from the
macro that declares it.

`graph-types` tests (its own suite, run with `--all-features`):
`every_name_round_trips_through_from_name`,
`every_label_round_trips_through_from_label`, `labels_are_distinct`,
`serialize_emits_name_and_label`, `schema_enums_equal_names_and_labels`.

The response structs then carry the originals with **no attributes**:

```rust
pub struct EdgeSummaryEntry { pub kind: EdgeKind, pub count: usize }
pub struct NodeCard         { pub id: String, pub kind: NodeKind, pub label: String, … }
pub struct NodeRef          { pub id: String, pub kind: NodeKind, pub label: String }
pub struct EdgePage         { pub kind: EdgeKind, pub entries: Vec<EdgeEntry>, pub next: Option<usize>, pub version: String }
```

Wire bytes are unchanged (`"Container"`, `"follows-in"`), proven by the AGC
fixtures `node-place-hazor-1.json`, `edges-hazor-1-site-of.json`,
`node-event-ab-ur.json` staying byte-identical. `node_edges`' query parsing
uses `EdgeKind::from_label` in place of the server's own lookup, which is
deleted.

### 5.3 The relation structure, published

The OpenAPI enum gives outsiders the 46 labels. The *structure* — which
labels pair as forward/inverse, which are symmetric — is published as one
vendor extension on the document, generated from `RelationId::ALL` /
`SymRelationId::ALL`:

```yaml
x-atlas-relations:
  directed:
    - { name: Contains, forward: contains, inverse: member-of }
    - …                                            # 20 entries, declaration order
  symmetric:
    - { name: Analogue, label: analogous-to }
    - …                                            # 6 entries
```

```rust
// atlas-contract/src/document.rs
pub fn openapi() -> utoipa::openapi::OpenApi;         // crate::openapi_router() + x-atlas-relations
pub fn openapi_yaml() -> String;                      // contracts/openapi.yaml
pub fn aqc_schema_json() -> String;                   // contracts/atlas-query-contract/aqc.schema.json = {"$defs": components.schemas}
pub fn graph_vocabulary_json() -> String;             // contracts/atlas-graph-contract/fixtures/graph-vocabulary.json, today's shape, from the document
pub fn relations_json() -> serde_json::Value;         // the x-atlas-relations value, from RelationId::ALL / SymRelationId::ALL
pub fn generated_files() -> Vec<(std::path::PathBuf, String)>;   // the three above, repo-relative, LF
```

### 5.4 Exporter and gates

```
cargo run -p atlas-contract --bin export_contract              # writes contracts/openapi.yaml
cargo run -p atlas-contract --bin export_contract -- --check   # exit 1 with a diff if it differs from the committed file
```

- `atlas-contract/tests/contract_generation.rs` —
  `openapi_yaml_is_byte_identical_to_the_committed_document`.
- `atlas-contract/tests/contract_api.rs` (moved, extended) —
  `the_served_openapi_document_equals_the_committed_one` (served at
  `/api/openapi.yaml`), and `every_route_in_the_router_is_documented`
  (structural: `routes!` cannot register an undocumented handler, so this
  asserts the count equals §4).
- `/api/contract` is **unchanged** (min/max version, manifest schema,
  section schema): AGC's `version-root`/`versioning` features pin it and the
  semver classifier still consumes it. No hash fields are added (D8).
- **No route without a contract** — `atlas-contract/tests/contract_coverage.rs`:
  every documented path is referenced by at least one scenario under
  `contracts/`. Red today for seven routes; each gets one extensional AGC pin
  (one scenario, one fixture) in this batch.
- `utoipa-swagger-ui` at `/swagger-ui` behind a `dev-docs` cargo feature —
  the browsable, try-it view of the same document; not built in release.

### 5.5 The move, weighed

`handlers.rs` is 2,122 lines today. CONTRACT-1 moves all of it and deletes
none; the crate is deliberately heavier on day one than a contract crate
should be, because shrinking a handler is only *visibly* a contract change
once it lives in the contract crate.

| Group | Handlers (≈ lines) | Fate |
|---|---|---|
| Legacy detail endpoints | `verse` 253, `chapter` 158, `kretzmann_chapter` 357, `event` 159, `narrative_event_positions` 160, `xrefs` 105, `catechism_item` 94, `catechism_for_span` 68, `place` 52 (≈ 1,400) | FOCUS reads node card + edges + text window instead; each retires, or shrinks to an adapter over an `atlas-graph` service (`drain_edges`, 188 lines, is service code in a handler file today). |
| Map surface | `polities` 235, `land_mask` 86, `eras` 33, `scene_*` 38, `landmarks` 10, `narratives` 11 (≈ 400) | Leave with the map-view migration. |
| Thin adapters | `books` 15, `sources` 18, `health` 4 | Stay. |
| Response structs | 31 declarations (≈ 300) | `wire/`, now. |

End state after those batches: one adapter per route, ten to twenty lines
each, plus the wire types and the document.

## 6. Client design

### 6.1 NSwag configuration — `client/Contract/nswag.json`

`client/Contract/` is the client's one place (D10): the generator config
and the one hand-written file. The generated records are not in the repo
(D2). `nswag` is a local dotnet tool in `.config/dotnet-tools.json`
(beside `dotnet-stryker`), invoked by the client build:

```xml
<!-- BibleAtlas.Client.csproj -->
<Target Name="GenerateContract" BeforeTargets="CoreCompile"
        Inputs="../contracts/openapi.yaml;Contract/nswag.json"
        Outputs="$(IntermediateOutputPath)Contract/Wire.g.cs">
  <Exec Command="dotnet nswag run Contract/nswag.json /variables:Output=$(IntermediateOutputPath)Contract/Wire.g.cs" />
  <ItemGroup><Compile Include="$(IntermediateOutputPath)Contract/Wire.g.cs" /></ItemGroup>
</Target>
```

`Inputs`/`Outputs` make it incremental: the generator runs only when the
document or the config changed.

```json
{
  "documentGenerator": { "fromDocument": { "url": "../../contracts/openapi.yaml" } },
  "codeGenerators": { "openApiToCSharpClient": {
    "generateClientClasses": false,
    "generateDtoTypes": true,
    "namespace": "BibleAtlas.Client.Contract",
    "classStyle": "Record",
    "generateNativeRecords": true,
    "jsonLibrary": "SystemTextJson",
    "arrayType": "System.Collections.Generic.IReadOnlyList",
    "arrayInstanceType": "System.Collections.Generic.List",
    "generateNullableReferenceTypes": true,
    "generateOptionalPropertiesAsNullable": true,
    "enumNameGeneratorType": null,
    "output": "$(Output)"
  } }
}
```

Lean-output settings go in the same file (`generateDataAnnotations: false`,
no `AdditionalProperties` dictionaries, no per-class `GeneratedCode`
attributes); the plan pins the exact keys.

### 6.2 Generated at build — `obj/…/Contract/Wire.g.cs`

One record per schema in the closure, in NSwag's native-record style. The
AQC core, as NSwag emits it (abridged to the shape; attribute spelling is
NSwag's):

```csharp
namespace BibleAtlas.Client.Contract;

[JsonConverter(typeof(JsonStringEnumConverter))]
public enum NodeKind { TextUnit, Container, Event, Narrative, Place, Person, Anchor, Era, Polity, CatechismItem, Source, Translation, PeopleGroup, CommentaryItem, LexiconEntry }

[JsonConverter(typeof(JsonStringEnumConverter))]
public enum EdgeKind
{
    [EnumMember(Value = "contains")] Contains, [EnumMember(Value = "member-of")] MemberOf,
    /* … 46 members, one per label, in graph-types' declaration order … */
    [EnumMember(Value = "partner-of")] PartnerOf,
}

public sealed record EdgeSummaryEntry
{
    [JsonPropertyName("kind")]  public EdgeKind Kind { get; init; }
    [JsonPropertyName("count")] public int Count { get; init; }
}

public sealed record NodeCard
{
    [JsonPropertyName("id")]           public string Id { get; init; } = default!;
    [JsonPropertyName("kind")]         public NodeKind Kind { get; init; }
    [JsonPropertyName("label")]        public string Label { get; init; } = default!;
    [JsonPropertyName("provenance")]   public string Provenance { get; init; } = default!;
    [JsonPropertyName("edge_summary")] public IReadOnlyList<EdgeSummaryEntry> EdgeSummary { get; init; } = new List<EdgeSummaryEntry>();
    [JsonPropertyName("version")]      public string Version { get; init; } = default!;
    [JsonPropertyName("person")]       public PersonLife? Person { get; init; }
    [JsonPropertyName("description")]  public string? Description { get; init; }
}

public sealed record NodeRef       { Id, NodeKind Kind, Label }
public sealed record EdgeEntry     { Edge, NodeRef Node }
public sealed record EdgePage      { EdgeKind Kind, IReadOnlyList<EdgeEntry> Entries, int? Next, Version }
public sealed record TextUnit      { [JsonPropertyName("ref")] Ref, Text, IReadOnlyList<WordsOfChristSpan> WordsOfChrist, IReadOnlyList<EdgeSummaryEntry> EdgeSummary }
public sealed record TextWindow    { IReadOnlyList<TextUnit> Units, string? Next, Version }
public sealed record Contents      { Corpus, Version, IReadOnlyList<ContentsRoot> Roots }
public sealed record ContentsRoot  { Id, Title, string Kind, string? Group, [JsonPropertyName("ref")] Ref, IReadOnlyList<ContentsChild> Children }
public sealed record ContentsChild { Id, Title, string Kind, [JsonPropertyName("ref")] Ref, int Count }
public sealed record Contract      { MinVersion, MaxVersion, int ManifestSchema, int SectionSchemaVersion }
public sealed record PersonLife    { string? Gender, int? BirthYear, int? DeathYear, int? FirstYear, int? LastYear, bool Eternal, IReadOnlyList<string> EternalGrounds, IReadOnlyList<string> AlsoCalled }
```

`EdgeKind` and `NodeKind` here are the same names as in Rust and in the
document; nothing is remapped anywhere.

Mapping (NSwag's, configured above): `Option<T>` → `T?`; `Vec<T>` →
`IReadOnlyList<T>` defaulting to empty; `usize`/`u32` → `int`;
`#[serde(rename = "ref")]` → `[JsonPropertyName("ref")] Ref` — **member name
= PascalCase(wire name)**, never the Rust field name. Style is the
generator's; we do not fight it.

### 6.3 Hand-written — `client/Contract/EdgeKinds.cs` (the only hand-rolled contract code)

The generated `EdgeKind` is flat: 46 values. The one structural fact the
client needs that OpenAPI cannot express — which values are each other's
duals — comes from the document's `x-atlas-relations`, as two functions over
the generated enum:

```csharp
namespace BibleAtlas.Client.Contract;

public static class EdgeKinds
{
    public static EdgeKind Dual(this EdgeKind kind);      // member-of <-> contains; a symmetric kind is its own dual
    public static bool IsSymmetric(this EdgeKind kind);   // 6 of the 46
}
```

Backed by no table at all: the derived `graph-vocabulary.json` (D8) is
embedded into the client assembly at build, and `EdgeKinds` reads its
`relations` (forward/inverse pairs) and `symmetric` entries once. The pairs
therefore come from the same document the enum came from, and cannot
disagree with it without the server-side regenerate-and-diff gate failing
first. `Label`/`Parse` read the generated enum's `EnumMember` values by
reflection — also no second list. Tests: `Dual_is_an_involution` and
`Label_round_trips_through_Parse_for_every_kind` over
`Enum.GetValues<EdgeKind>()` are the client-side totality checks. No
`RelationId`/`Direction` types exist on the client: nothing needs them yet,
and adding them when something does is one function away.

### 6.4 Retyped call sites

```csharp
public interface IExplorableClient
{
    Task<NodeCard>   Card(string id);
    Task<EdgePage>   Edges(string id, EdgeKind kind, int? cursor = null, int limit = 20);
    Task<TextWindow> Reading(string fromRef, int n, string dir = "onward", string corpus = "bible");
}

public Task<EdgePage> NodeEdges(string nodeId, EdgeKind kind, int? cursor = null, int limit = 200);   // AtlasClient

public sealed record EdgeSectionSpec(EdgeKind EdgeKind, SectionStyle Style, int InitialClamp, SectionOrder Order);
public static readonly EdgeSectionSpec Cites         = new(EdgeKind.Cites,         SectionStyle.Quiet,    InitialClamp: 3,  SectionOrder.VotesRanked);
public static readonly EdgeSectionSpec Mentions      = new(EdgeKind.Mentions,      SectionStyle.Standard, InitialClamp: 50, SectionOrder.Canonical);
public static readonly EdgeSectionSpec MentionedIn   = new(EdgeKind.MentionedIn,   SectionStyle.Standard, InitialClamp: 12, SectionOrder.Canonical);
public static readonly EdgeSectionSpec CommentedOnBy = new(EdgeKind.CommentedOnBy, SectionStyle.Standard, InitialClamp: 20, SectionOrder.Canonical);
```

`GraphExplorableClient` sends the enum's `EnumMember` value where it sent
`kind.Value`.

### 6.5 Deleted

`client/Dtos.cs` (59 records), `client/AqcContract.cs`,
`client/Pages/ContractMismatch.razor`, `AtlasClient.Contract()`,
`EdgeKindId`, `contracts/atlas-query-contract/aqc.schema.json`,
`contracts/atlas-graph-contract/fixtures/graph-vocabulary.json`, and every
client-only type name (`*Dto`, `NarrativeEventPositionsResult`,
`CatechismItemDetail`, …). The compiler is the rename checklist.

## 7. Contract suites after this batch

| Suite | Before | After |
|---|---|---|
| AQC schema-validity law | validates against hand-written `aqc.schema.json` | validates against `aqc.schema.json` **derived** from `openapi.yaml`'s components — harness code unchanged, file no longer hand-written |
| AQC `x-queries` | hand-written for 6 routes | gone; the OpenAPI paths are the query table for all 24 |
| AGC `vocabulary.feature` | pins `graph-vocabulary.json` built in-test from the macros | pins `graph-vocabulary.json` **derived** from `openapi.yaml`; `contract_pact.rs::graph_vocabulary()` reads the document, and the "drawn from the macros" law compares document to macros |
| AGC pins, CLI≡HTTP | unchanged | unchanged, plus seven new pins |
| `client.ContractTests` | replays fixtures via `JsonDocument` | deserialises the same fixtures into the generated records with typed kinds |
| Version | hand-bumped on both sides | `/api/contract` unchanged for AGC's semver classifier; the client no longer reads it |

Mutation: `.config/dotnet-tools.json` (dotnet-stryker, nswag);
`client.Tests/stryker-config.json` covers `client/Contract/EdgeKinds.cs`
and the retyped call sites (build-generated code is not on disk as project
source, so Stryker cannot mutate it — see §10); `server/mutants.toml` examines `atlas-contract/src/document.rs`,
each family's `routes()`, and `graph-types/src/{id,edge,wire_form}.rs`'s new
functions, and excludes the eight wall-clock gates. 100% killed; equivalents listed in those
files with a reason, never as inline comments.

## 8. Tests (written first, in this order)

Rust:
1. `graph-types` (§5.2): `every_label_round_trips_through_from_label`,
   `labels_are_distinct`, `serialize_emits_the_label`, `schema_enum_equals_labels`;
   `document`: `x_atlas_relations_matches_RelationId_ALL`
2. `contract_generation.rs`: regenerate-and-diff
3. `contract_api.rs`: served document ≡ committed; documented-route count
4. `contract_coverage.rs`: no route without a contract (red until the seven
   pins land; each pin is its own commit)
5. AGC/AQC fixtures byte-identical after the kind retype (existing gates)

C#:
6. `Contract/EdgeKindsTests`: `Dual_is_an_involution`,
   `Symmetric_kinds_are_their_own_dual`, `Every_generated_kind_has_a_dual`,
   `Dual_table_matches_x_atlas_relations`
7. `client.ContractTests/WireFixtureTests`: the committed AGC fixtures
   deserialise into the generated records with typed kinds — real server
   output, no mocks
8. `client.Tests/Contract/GeneratedUsageTests`:
   `Every_generated_type_is_referenced` (§10)

Every test file's only comments are `// Arrange`, `// Act`, `// Assert`.

## 9. Ripple (mechanical)

- Every consumer of a `*Dto` name: Pages, Components, Explore, `AtlasClient`,
  `GraphExplorableClient`, `MapInterop`, tests. Rename only.
- `client.Tests/ConformanceTests` has no entries naming the deleted files
  (verified 2026-09-26); the deletion tasks re-check rather than assume.
- Versions (verified against crates.io 2026-09-26, toolchain 1.97.1, axum
  0.8.9): `utoipa 6.0.0` with its `yaml` feature, `utoipa-axum 0.3.0`,
  `utoipa-swagger-ui 10.0.1` with `axum` + `vendored` features (no
  build-time download).
- `graph-vocabulary.json` is produced in-test today by
  `contract_pact.rs::graph_vocabulary()` and pinned by `vocabulary.feature`;
  the feature is re-pointed at `openapi.yaml`'s enums + `x-atlas-relations`
  and that generator function is deleted with the fixture.
- `text_window` returns `Response`, not `Json<T>`; its attribute states
  `body = wire::TextWindow` explicitly.
- The owner's untracked `FrontierMatrix*Tests.cs` pin a hand transcription
  of the vocabulary; they either consume the generated enums or retire —
  flagged in the plan, the owner's call.
- `Wire.Options` keeps `SnakeCaseLower` for query parameters; generated
  records carry explicit names and no longer depend on it.
- `-p atlas-server` in `scripts/timing-gates.sh`, `contract-gate.sh` and the
  standing counting procedure in `server/Cargo.toml` becomes
  `-p atlas-contract` for the moved test binaries; `cargo test --workspace`
  totals are unchanged.
- Once map-generator's `map-types` lives in this repo it is a second set of
  `ToSchema` types under the same `OpenApiRouter` — one document, no new
  machinery.

## 10. Rulings and what remains open

**Generated code is not exempt from the laws** (owner: "if they're dead
then why are we bothering to generate"):
- A generated *type* with no reference in client code is dead and is not
  generated: `excludedTypeNames` in `nswag.json` is the client's declared
  consumption, and `client.Tests/Contract/GeneratedUsageTests.cs`
  (`Every_generated_type_is_referenced`: every schema in `openapi.yaml`'s
  components not in `excludedTypeNames` has a reader in client source — a
  scan in the existing `ConformanceTests` style) fails when one does not.
- Mutation: with D2 the generated C# exists only in `obj/`, which Stryker
  does not mutate. The records carry no logic (properties and
  `JsonPropertyName` attributes), so nothing is lost; noted here so the
  "not exempt" ruling is read as applying to the dead-code law, which it
  does, and not as a promise Stryker cannot keep.
- Fields are not pruned: an unread field is the shape of the promise, not
  behaviour, and a client `NodeCard` narrower than the server's would be a
  second type under one name (D4). If the owner wants field-level pruning
  it is a consumer-projection design of its own, not a config change.

**No `facet-generate` spike** (owner: "nah"). NSwag is the generator.

**Swagger UI: yes** (owner: "ah okay sure") — `utoipa-swagger-ui` mounted
at `/swagger-ui` behind a `dev-docs` cargo feature, absent from release
builds. Nothing open remains; the spec awaits sign-off as a whole.

Resolved earlier: `IReadOnlyList<T>` (NSwag `arrayType`, §6.1); `served!`
macro (replaced by `utoipa-axum`); contents kinds (container sub-kind, stay
`String`); the seven pins (in, under the coverage law).

## 11. What dies with the migration

Owner: the removal of everything the OpenAPI contract makes obsolete is
part of this batch, not a follow-up. Each line is a deletion task in the
plan; "proof" is what turns red if the deletion is skipped or if the thing
was still needed.

| Dies | Because | Proof it is dead | Batch |
|---|---|---|---|
| `client/Dtos.cs` — 59 hand-written records | replaced by the build-generated `Wire.g.cs` | compiler (duplicate names) + `GeneratedUsageTests` | CONTRACT-1 |
| `EdgeKindId` (`IExplorableClient.cs`) and every `new EdgeKindId("…")` literal | replaced by the generated `EdgeKind` enum | compiler; source scan for string literals matching a declared label | CONTRACT-1 |
| `client/AqcContract.cs` (`ClientVersion`, semver parsing, `Satisfies`) | D7: agreement proven in CI | compiler | CONTRACT-1 |
| `client/Pages/ContractMismatch.razor`, the check state machine in `App.razor`, `AtlasClient.Contract()` | D7 | compiler; Playwright spec for the mismatch page retires with it | CONTRACT-1 |
| `client/Wire.cs` (`Wire.Options`, `SnakeCaseLower`) | every generated record carries explicit `JsonPropertyName`s; the policy has no remaining reader once `Dtos.cs` is gone (verify `MapInterop`'s C#→JS serialisation, which used it, still produces snake_case through the attributes) | compiler after removal; `WireFixtureTests`; the world-map Playwright specs | CONTRACT-1 (verify first) |
| the hand-written `contracts/atlas-query-contract/aqc.schema.json` incl. its `x-queries` | D8: replaced at the same path by a derivation of `openapi.yaml` | `contract_generation.rs` (the derived file is gated); AQC harnesses unchanged | CONTRACT-1 |
| the in-test generator `contract_pact.rs::graph_vocabulary()` and the hand-blessed `graph-vocabulary.json` | D8: the fixture is derived from `openapi.yaml`; the pact reads the document | `contract_generation.rs`; `the_published_vocabulary_is_drawn_from_the_macros` now compares document to macros | CONTRACT-1 |
| `atlas-server/src/app.rs`'s 24 `.route(...)` lines | replaced by the families' `routes()` in `atlas-contract` | `contract_api.rs` route count; `contract_coverage.rs` | CONTRACT-1 |
| the server's own edge-kind label lookup used by `node_edges` (`kind` query → `EdgeKind`) and any node-kind name matching in handlers | `EdgeKind::from_label` / `NodeKind::from_name` in `graph-types` | compiler; `graph-types` round-trip tests | CONTRACT-1 |
| `atlas-server` as a library: `app.rs`, `handlers.rs`, `graph_handlers.rs`, `graph_wire.rs`, `contents.rs`, `contract.rs`, `error.rs`, `load.rs`, `aqc_export.rs`, `lib.rs` | moved into `atlas-contract` (D10); the crate keeps `main.rs` only | `cargo build --workspace`; `-p atlas-server` has no lib target | CONTRACT-1 |
| `tests/ux/contract-versioning.spec.ts` (94 lines, the mismatch-page spec) | D7 | the page it drives no longer exists | CONTRACT-1 |
| `graph_wire::parse_edge_kind` and the server's `format!("{:?}")` kind-to-string sites | `EdgeKind::from_label`, `NodeKind::name`, `EdgeKind::label` via the `Serialize` impls | compiler; graph-types round-trip tests | CONTRACT-1 |
| the 31 `*Out` names | D4 | compiler | CONTRACT-1 |
| `client.ContractTests/Steps/AqcSteps.cs` `JsonDocument` path-poking | replaced by deserialisation into generated records | the same scenarios pass on the typed path | CONTRACT-1 |
| `client.Tests/ConformanceTests` scan entries naming deleted files | their subjects no longer exist | the scans themselves | CONTRACT-1 |
| the owner's untracked `FrontierMatrixRustParityTests.cs` / `FrontierMatrixConformanceTests.cs` hand transcription of the vocabulary | superseded by generated `EdgeKind`/`NodeKind` | owner's call: retarget onto the generated enums or delete | CONTRACT-1 (owner decides) |
| `contract.rs`'s `MIN_SUPPORTED_VERSION` / `MAX_SUPPORTED_VERSION` and the AQC `VERSION` file | still consumed by `contract-semver-gate.sh` and AGC's `versioning` features; **not dead yet** | — | candidate for CONTRACT-2 once the classifier reads `openapi.yaml` diffs directly |
| the legacy detail handlers (≈1,400 lines, §5.5) and the map handlers (≈400) | subsumed by the graph API / the map migration; **not dead yet** | FOCUS / MAPS remove them route by route, each removal a visible `openapi.yaml` diff | FOCUS, MAPS |

Nothing on the CONTRACT-1 rows is left behind "for compatibility": there is
no second consumer of the old shapes in this repository, and the outside
consumer (map-generator) pins consumed projections, not type names.

## Appendix A — served types in the closure (name after D4 : fields)

`atlas-core/src/wire.rs`: Scene 7, ScenePlace 10, QuietPlace 8, SceneEvent 4, VerseGroup 4, SceneArrow 7, SceneNarrative 4.
`atlas-core` data served directly: CanonBook, Narrative, Era, Landmark, SourcesDocument, SourceCategory, SourceEntry, ProvenanceEntry, TimeRange.
`atlas-contract/src/wire/contents.rs`: Contents 3, ContentsRoot 6, ContentsChild 5.
`atlas-contract/src/wire/meta.rs`: Contract 4.
`atlas-contract/src/wire/graph.rs`: EdgeSummaryEntry 2, PersonLife 8, NodeCard 8, NodeRef 3, EdgeEntry 2, EdgePage 4, TextUnit 4, TextWindow 3.
`atlas-contract/src/wire/{reading,catechism,places,events,map}.rs` (from `handlers.rs`): LandMask 1, PolityDelta 3, Polity 8, Polities 1, PlaceRef 2, PersonRef 2, WordsOfChristSpan 2, Heading 4, Verse 7, Chapter 4, KretzmannChapterItem 2, KretzmannChapterVerse 2, KretzmannChapter 2, BookMeta 4, VerseEvent 7, CrossRef 4, CatechismRef 4, VerseDetail 10, NarrativeAdjacentEvent 4, NarrativePosition 6, TimelinePosition 2, NarrativeEventPositions 2, EventPlace 2, EventWitness 4, EventDetail 16, EventAnalogue 3, CatechismProofVerse 3, CatechismItem 8, DateClaim 3, History 4, PlaceDetail 8.

Where a wire name coincides with an `atlas-core::data` name (`Polity`,
`PolityDelta`, `CatechismItem`, `Narrative` is served directly and has no
wire twin), the module path disambiguates in Rust; in the OpenAPI document
only served types appear, so every component name is unique.

Serde attributes in use: `skip_serializing_if = "Option::is_none"` (35),
`rename = "ref"` (6), `skip_serializing_if = "Vec::is_empty"` (6, two with
`default`). utoipa reads all three; anything new fails `ToSchema` loudly.

## Appendix B — file structure

```
+ new   − deleted   ~ changed   ⇐ generated (committed, gated)

graph-types/
  Cargo.toml                                            ~ optional serde, utoipa behind default-off features (D9)
  src/lib.rs                                            ~ cfg-gated `pub mod wire_form;`
  src/id.rs                                             ~ kind_tags! also emits NodeKind::name / from_name (pure)
  src/edge.rs                                           ~ relations! also emits EdgeKind::all / labels / from_label (pure)
  src/wire_form.rs                                      + the four Serialize / ToSchema impls, and nothing else

server/
  Cargo.toml                                            ~ members += atlas-contract; counting procedure -p atlas-contract
  mutants.toml                                          + cargo-mutants scope + equivalents
  atlas-core/src/wire.rs, data.rs, sources.rs, time.rs  ~ #[derive(ToSchema)] on served types
  atlas-contract/                                       + THE crate (D10) — the HTTP surface, moved from atlas-server
    Cargo.toml                                          +   utoipa, utoipa-axum, axum, atlas-graph, atlas-core, graph-types (serde, openapi)
    src/lib.rs                                          +   families merged; router(state); openapi()
    src/{graph,reading,catechism,places,events,map,contents,meta}.rs
                                                        +   handlers (moved from handlers.rs / graph_handlers.rs / contents.rs / contract.rs), #[utoipa::path], routes()
    src/wire/*.rs                                       +   response types, Out suffix dropped, ToSchema
    src/state.rs, src/error.rs                          +   AppState; ApiError + IntoResponses
    src/document.rs                                     +   openapi_yaml(), x-atlas-relations
    src/bins/export_contract.rs                         +   writes contracts/openapi.yaml; --check
    src/bins/export_aqc_examples.rs                     ~   moved
    tests/graph_api.rs, aqc_cucumber.rs, contract_api.rs ~  moved; contract_api extended (served ≡ committed; route count)
    tests/contract_generation.rs                        +   regenerate-and-diff
    tests/contract_coverage.rs                          +   no route without a contract
  atlas-server/
    src/main.rs                                         ~ config → state → atlas_contract::router(state) → serve
    src/{app,handlers,graph_handlers,contents,contract,aqc_export}.rs
                                                        − moved into atlas-contract

contracts/
  openapi.yaml                                          + ⇐ THE contract
  atlas-query-contract/aqc.schema.json                  −
  atlas-graph-contract/fixtures/graph-vocabulary.json   −
  atlas-graph-contract/graph/*.feature                  ~ vocabulary law → openapi.yaml; + 7 pins and fixtures
  atlas-query-contract/features/*.feature               ~ schema law → openapi.yaml

scripts/contract-gate.sh                                ~ + export_contract --check (the client build generates its own C#; nothing to diff)

.config/dotnet-tools.json                               + nswag, dotnet-stryker

client/
  BibleAtlas.Client.csproj                              ~ GenerateContract target (§6.1); Wire.g.cs lands in obj/, untracked
  Contract/                                             + the client's one place (D10)
    nswag.json                                          +   §6.1
    EdgeKinds.cs                                        +   §6.3 (the only hand-written contract code)
  Dtos.cs, AqcContract.cs, Pages/ContractMismatch.razor −
  App.razor                                             ~ the Router alone
  IExplorableClient.cs                                  ~ EdgeKindId gone; EdgeKind parameter
  AtlasClient.cs, GraphExplorableClient.cs              ~ renamed returns; label value; Contract() gone
  Explore/EdgeSectionRegistry.cs                        ~ EdgeKind members
  (every *Dto consumer)                                 ~ rename

client.Tests/
  Contract/EdgeKindsTests.cs, GeneratedUsageTests.cs    + §8
  stryker-config.json                                   + scope + equivalents
  ConformanceTests.cs                                   ~ scans that named deleted files
client.ContractTests/
  WireFixtureTests.cs                                   + §8, on the generated records
  Steps/AqcSteps.cs                                     ~ typed records instead of JsonDocument

docs/PRINCIPLES.md                                      + owner's principles
```
