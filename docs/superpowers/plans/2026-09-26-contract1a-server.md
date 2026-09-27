# CONTRACT-1a (server) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Rust the contract: `graph-types` kinds describe their own wire form and schema; the server's HTTP surface becomes the `atlas-contract` crate; utoipa emits `contracts/openapi.yaml` (plus the two derived JSON documents the existing harnesses read); every route is documented, pinned, and gated — with zero wire-byte change.

**Architecture:** `graph-types` gains pure `name`/`label` functions and one feature-gated `wire_form.rs`. Today's `atlas-server` library moves wholesale into a new `atlas-contract` crate (handlers split by route family, response types in `wire/` with the `Out` suffix dropped), registered through `utoipa-axum`'s `OpenApiRouter`; `atlas-server` keeps only `main.rs`. A `document` module derives the three committed contract documents; a regenerate-and-diff test gates them; a coverage test forbids any route without a contract scenario.

**Tech Stack:** Rust 1.97.1; axum 0.8.9; `utoipa = "6.0.0"` (features `yaml`), `utoipa-axum = "0.3.0"`, `utoipa-swagger-ui = "10.0.1"` (features `axum`, `vendored`); `serde 1`; cargo-mutants.

**Spec:** `docs/superpowers/specs/2026-09-26-contract-from-graph-types-design.md` (read it first; §2 decisions D1–D10 bind every task). Companion plan: `2026-09-26-contract1b-client.md` runs after this one.

## Global Constraints

- `docs/PRINCIPLES.md` binds: TDD (failing test first), no code a test does not need, no comments except `why`, tests carry only `// Arrange` `// Act` `// Assert`.
- **Tests as documentation (PRINCIPLES 15–18):** every assertion is whole-body — `assert_eq!(actual, expected)` where `expected` is the entire value written out in the test; one behaviour per test, named as a sentence; no bare numbers — `const DECLARED_NODE_KINDS: usize = 15;`, `const DECLARED_DIRECTED_RELATIONS: usize = 20;`, `const DECLARED_SYMMETRIC_RELATIONS: usize = 6;`, `const DECLARED_EDGE_KINDS: usize = 2 * DECLARED_DIRECTED_RELATIONS + DECLARED_SYMMETRIC_RELATIONS;` are the constants this plan uses; every file in newspaper order — the entry point or the test first, helpers below their first caller.
- **D5: zero wire-byte change.** Every AGC fixture, every AQC scenario, every Playwright spec that passes before a task passes after it. `contract_pact`, `aqc_cucumber`, `graph_api` are the proof and run after every task that touches a served type.
- **D9: `graph-types` default build has no dependencies.** `serde` and `utoipa` are optional, behind features `serde` and `openapi`, default off. `id.rs` and `edge.rs` never mention either crate.
- **D4: no `Out` suffix.** Struct names are not on the wire; renaming is byte-neutral.
- **D10: contract code lives only in `atlas-contract` (and `graph-types/src/wire_form.rs`).**
- Versions are pinned exactly as in Tech Stack; do not float them.
- All commands run from `server/` unless stated. The standing counting procedure in `server/Cargo.toml` (three commands) is the definition of "the suite is green".
- Commit per task with the attribution trailer the session provides; push at the end of the plan (owner authorization on record: "go means commit + push").
- Bins live in `src/bins/`, never `src/bin/` (root `.gitignore` has `**/bin/`).

---

### Task 1: `NodeKind::name`/`from_name` and `EdgeKind::all`/`labels`/`from_label` — pure, in `graph-types`

**Files:**
- Modify: `graph-types/src/id.rs` (the `NodeKind` enum at :22-42 and `ALL` at :123)
- Modify: `graph-types/src/edge.rs` (add an `impl EdgeKind` block after `dual` at :135-140)
- Create: `graph-types/tests/wire_names.rs`

**Interfaces:**
- Produces: `NodeKind::name(self) -> &'static str`, `NodeKind::from_name(&str) -> Option<NodeKind>`, `EdgeKind::all() -> impl Iterator<Item = EdgeKind>`, `EdgeKind::labels() -> impl Iterator<Item = &'static str>`, `EdgeKind::from_label(&str) -> Option<EdgeKind>`.

- [ ] **Step 1: Write the failing tests**

`graph-types/tests/wire_names.rs`:
```rust
use atlas_graph_types::{Direction, EdgeKind, NodeKind, RelationId, SymRelationId};

const DECLARED_DIRECTED_RELATIONS: usize = 20;
const DECLARED_SYMMETRIC_RELATIONS: usize = 6;
const DECLARED_EDGE_KINDS: usize = 2 * DECLARED_DIRECTED_RELATIONS + DECLARED_SYMMETRIC_RELATIONS;

#[test]
fn every_name_round_trips_through_from_name() {
    // Arrange
    let kinds = NodeKind::ALL;
    // Act
    let back: Vec<Option<NodeKind>> = kinds.iter().map(|k| NodeKind::from_name(k.name())).collect();
    // Assert
    assert_eq!(back, kinds.iter().map(|k| Some(*k)).collect::<Vec<_>>());
}

#[test]
fn names_are_the_debug_names_the_wire_already_carries() {
    // Arrange
    let kinds = NodeKind::ALL;
    // Act
    let names: Vec<&str> = kinds.iter().map(|k| k.name()).collect();
    let debug_names: Vec<String> = kinds.iter().map(|k| format!("{k:?}")).collect();
    // Assert
    assert_eq!(names, debug_names);
}

#[test]
fn from_name_rejects_an_undeclared_kind() {
    // Arrange
    let name = "Verse";
    // Act
    let kind = NodeKind::from_name(name);
    // Assert
    assert_eq!(kind, None);
}

#[test]
fn all_lists_forward_then_inverse_for_every_relation_then_every_symmetric() {
    // Arrange
    let expected: Vec<EdgeKind> = RelationId::ALL
        .iter()
        .flat_map(|r| [EdgeKind::Directed(*r, Direction::Forward), EdgeKind::Directed(*r, Direction::Inverse)])
        .chain(SymRelationId::ALL.iter().map(|s| EdgeKind::Symmetric(*s)))
        .collect();
    // Act
    let all: Vec<EdgeKind> = EdgeKind::all().collect();
    // Assert
    assert_eq!(all, expected);
    assert_eq!(all.len(), DECLARED_EDGE_KINDS);
}

#[test]
fn labels_are_distinct() {
    // Arrange
    let labels: Vec<&str> = EdgeKind::labels().collect();
    // Act
    let mut deduped = labels.clone();
    deduped.sort_unstable();
    deduped.dedup();
    // Assert
    assert_eq!(deduped.len(), labels.len());
}

#[test]
fn every_label_round_trips_through_from_label() {
    // Arrange
    let kinds: Vec<EdgeKind> = EdgeKind::all().collect();
    // Act
    let back: Vec<Option<EdgeKind>> = kinds.iter().map(|k| EdgeKind::from_label(k.label())).collect();
    // Assert
    assert_eq!(back, kinds.iter().map(|k| Some(*k)).collect::<Vec<_>>());
}

#[test]
fn from_label_rejects_an_undeclared_label() {
    // Arrange
    let label = "cited";
    // Act
    let kind = EdgeKind::from_label(label);
    // Assert
    assert_eq!(kind, None);
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p atlas-graph-types --test wire_names`
Expected: compile error — `no function or associated item named `name`` / `from_name` / `all` / `labels` / `from_label`.

- [ ] **Step 3: Implement — one list for `NodeKind`, generic iteration for `EdgeKind`**

In `graph-types/src/id.rs`, replace the hand-written `pub enum NodeKind { … }` (:22-42) **and** the `impl NodeKind { pub const ALL … }` (:123) with one macro invocation that emits both plus the two functions, so the variant list exists exactly once:

```rust
macro_rules! node_kinds {
    ( $($kind:ident),+ $(,)? ) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum NodeKind { $($kind),+ }

        impl NodeKind {
            pub const ALL: [NodeKind; [$(stringify!($kind)),+].len()] = [$(NodeKind::$kind),+];

            pub fn name(self) -> &'static str {
                match self { $(NodeKind::$kind => stringify!($kind)),+ }
            }

            pub fn from_name(name: &str) -> Option<NodeKind> {
                Self::ALL.iter().copied().find(|k| k.name() == name)
            }
        }
    };
}

node_kinds! {
    TextUnit, Container, Event, Narrative, Place, Person, Anchor, Era, Polity,
    CatechismItem, Source, Translation, PeopleGroup, CommentaryItem, LexiconEntry,
}
```
Keep the variant order exactly as the existing enum (it is `edge_index.kind`'s ordinal). Leave `kind_tags!` and everything else in `id.rs` untouched. If a doc comment on the old enum stated a `why`, move it onto the invocation.

In `graph-types/src/edge.rs`, after `pub fn dual(k: EdgeKind) -> EdgeKind { … }` (:135-140), add:

```rust
impl EdgeKind {
    pub fn all() -> impl Iterator<Item = EdgeKind> {
        RelationId::ALL
            .iter()
            .flat_map(|r| [EdgeKind::Directed(*r, Direction::Forward), EdgeKind::Directed(*r, Direction::Inverse)])
            .chain(SymRelationId::ALL.iter().map(|s| EdgeKind::Symmetric(*s)))
    }

    pub fn labels() -> impl Iterator<Item = &'static str> {
        Self::all().map(EdgeKind::label)
    }

    pub fn from_label(label: &str) -> Option<EdgeKind> {
        Self::all().find(|k| k.label() == label)
    }
}
```
(`EdgeKind::label` already exists at :143.)

- [ ] **Step 4: Run the new tests and the crate's own suite**

Run: `cargo test -p atlas-graph-types --test wire_names && cargo test -p atlas-graph-types`
Expected: 7 passed; the existing 13 law tests still pass (the `node_kind_manifest!` exhaustiveness macro in `atlas-server/tests/contract_pact.rs` still compiles because the variant set is unchanged).

- [ ] **Step 5: Commit**

```bash
git add graph-types/src/id.rs graph-types/src/edge.rs graph-types/tests/wire_names.rs
git commit -m "graph-types: kinds name themselves (NodeKind::name/from_name, EdgeKind::all/labels/from_label) from the one list each"
```

---

### Task 2: `graph-types/src/wire_form.rs` — `Serialize` and `ToSchema` for the kinds, behind features

**Files:**
- Modify: `graph-types/Cargo.toml`
- Modify: `graph-types/src/lib.rs` (add the cfg-gated module)
- Create: `graph-types/src/wire_form.rs`
- Create: `graph-types/tests/wire_form.rs`

**Interfaces:**
- Consumes: Task 1's functions.
- Produces: `impl serde::Serialize for NodeKind` (emits `name()`), `impl serde::Serialize for EdgeKind` (emits `label()`); `impl utoipa::ToSchema for NodeKind` (component `NodeKind`, string enum of the 15 names); `impl utoipa::ToSchema for EdgeKind` (component `EdgeKind`, string enum of the 46 labels).

- [ ] **Step 1: Declare the features**

`graph-types/Cargo.toml` — replace the empty `[dependencies]` and the `[features]` table (keep every existing comment):

```toml
[dependencies]
serde = { version = "1", optional = true, default-features = false }
utoipa = { version = "6.0.0", optional = true }

[dev-dependencies]
serde_json = "1"

[features]
canon-ids = []
serde = ["dep:serde"]
openapi = ["dep:utoipa"]
```
`default` stays absent (no default features), so `cargo build -p atlas-graph-types` still resolves zero dependencies. `serde_json` is a dev-dependency only, for the tests below.

- [ ] **Step 2: Write the failing tests**

`graph-types/tests/wire_form.rs`:
```rust
#![cfg(all(feature = "serde", feature = "openapi"))]

use atlas_graph_types::{EdgeKind, NodeKind};
use utoipa::PartialSchema;

const DECLARED_NODE_KINDS: usize = 15;
const DECLARED_EDGE_KINDS: usize = 46;

#[test]
fn serialize_emits_the_name_and_the_label() {
    // Arrange
    let kind = NodeKind::Container;
    let edge = EdgeKind::from_label("member-of").unwrap();
    // Act
    let kind_json = serde_json::to_string(&kind).unwrap();
    let edge_json = serde_json::to_string(&edge).unwrap();
    // Assert
    assert_eq!(kind_json, "\"Container\"");
    assert_eq!(edge_json, "\"member-of\"");
}

#[test]
fn node_kind_schema_is_the_closed_enum_of_names() {
    // Arrange
    let expected: Vec<&str> = NodeKind::ALL.iter().map(|k| k.name()).collect();
    // Act
    let schema = serde_json::to_value(NodeKind::schema()).unwrap();
    // Assert
    assert_eq!(expected.len(), DECLARED_NODE_KINDS);
    assert_eq!(schema, serde_json::json!({ "type": "string", "enum": expected }));
}

#[test]
fn edge_kind_schema_is_the_closed_enum_of_labels() {
    // Arrange
    let expected: Vec<&str> = EdgeKind::labels().collect();
    // Act
    let schema = serde_json::to_value(EdgeKind::schema()).unwrap();
    // Assert
    assert_eq!(expected.len(), DECLARED_EDGE_KINDS);
    assert_eq!(schema, serde_json::json!({ "type": "string", "enum": expected }));
}

#[test]
fn schema_component_names_are_the_type_names() {
    // Arrange
    use utoipa::ToSchema;
    // Act
    let node = <NodeKind as ToSchema>::name();
    let edge = <EdgeKind as ToSchema>::name();
    // Assert
    assert_eq!(node, "NodeKind");
    assert_eq!(edge, "EdgeKind");
}
```

- [ ] **Step 3: Run to verify they fail**

Run: `cargo test -p atlas-graph-types --features serde,openapi --test wire_form`
Expected: compile errors — `the trait bound `NodeKind: Serialize` is not satisfied`, `PartialSchema` not implemented.

- [ ] **Step 4: Implement `wire_form.rs`**

`graph-types/src/lib.rs` — add beside the other `pub mod` lines:
```rust
#[cfg(any(feature = "serde", feature = "openapi"))]
pub mod wire_form;
```

`graph-types/src/wire_form.rs`:
```rust
#[cfg(feature = "serde")]
mod serialize {
    use crate::{EdgeKind, NodeKind};
    use serde::{Serialize, Serializer};

    impl Serialize for NodeKind {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_str(self.name())
        }
    }

    impl Serialize for EdgeKind {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_str(self.label())
        }
    }
}

#[cfg(feature = "openapi")]
mod schema {
    use crate::{EdgeKind, NodeKind};
    use utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
    use utoipa::openapi::{RefOr, Schema};
    use utoipa::{PartialSchema, ToSchema};

    fn string_enum<'a>(values: impl Iterator<Item = &'a str>) -> RefOr<Schema> {
        ObjectBuilder::new()
            .schema_type(SchemaType::Type(Type::String))
            .enum_values(Some(values.collect::<Vec<_>>()))
            .into()
    }

    impl PartialSchema for NodeKind {
        fn schema() -> RefOr<Schema> {
            string_enum(NodeKind::ALL.iter().map(|k| k.name()))
        }
    }
    impl ToSchema for NodeKind {}

    impl PartialSchema for EdgeKind {
        fn schema() -> RefOr<Schema> {
            string_enum(EdgeKind::labels())
        }
    }
    impl ToSchema for EdgeKind {}
}
```
(`ToSchema::name` defaults to the type's name, which is what the test pins. If utoipa 6's builder names differ from `ObjectBuilder`/`SchemaType::Type(Type::String)`, follow `utoipa::openapi::schema`'s docs for the string-enum builder — the test is the specification, not the builder call.)

- [ ] **Step 5: Run the tests in all three feature states**

Run:
```
cargo test -p atlas-graph-types --features serde,openapi
cargo test -p atlas-graph-types
cargo build -p atlas-graph-types --no-default-features
```
Expected: all pass; the second and third compile with no `serde`/`utoipa` in the dependency graph (`cargo tree -p atlas-graph-types` shows no dependencies).

- [ ] **Step 6: Commit**

```bash
git add graph-types/Cargo.toml graph-types/src/lib.rs graph-types/src/wire_form.rs graph-types/tests/wire_form.rs
git commit -m "graph-types: wire_form -- Serialize and ToSchema for NodeKind/EdgeKind behind optional serde/openapi features (D9)"
```

---

### Task 3: Create `atlas-contract`; move the `atlas-server` library into it

**Files:**
- Create: `server/atlas-contract/Cargo.toml`
- Move (git mv): `server/atlas-server/src/{app,aqc_export,contents,contract,error,graph_handlers,graph_wire,handlers,load,lib}.rs` → `server/atlas-contract/src/`
- Move: `server/atlas-server/src/bins/` → `server/atlas-contract/src/bins/`; `server/atlas-server/tests/` → `server/atlas-contract/tests/`; `server/atlas-server/benches/` → `server/atlas-contract/benches/` (if present)
- Modify: `server/Cargo.toml` (members), `server/atlas-server/Cargo.toml`, `server/atlas-server/src/main.rs`
- Modify: `scripts/timing-gates.sh`, `scripts/contract-gate.sh`, `scripts/gate-selftest.sh`, `scripts/timing-gates-selftest.sh` — every `-p atlas-server` that names a **test** target becomes `-p atlas-contract`

**Interfaces:**
- Produces: crate `atlas_contract` exposing exactly the modules `atlas_server` exposed (`app, aqc_export, contents, contract, error, graph_handlers, graph_wire, handlers, load`).

- [ ] **Step 1: Write the failing check**

There is no new behaviour; the "test" is that the workspace builds with `atlas-server` as a binary-only package. Run now to record the baseline:
`cargo test --workspace 2>&1 | tail -3` → note the pass count (the standing block's first number).

- [ ] **Step 2: Move the files**

```bash
mkdir -p atlas-contract/src
git mv atlas-server/src/app.rs atlas-server/src/aqc_export.rs atlas-server/src/contents.rs atlas-server/src/contract.rs \
       atlas-server/src/error.rs atlas-server/src/graph_handlers.rs atlas-server/src/graph_wire.rs \
       atlas-server/src/handlers.rs atlas-server/src/load.rs atlas-server/src/lib.rs atlas-contract/src/
git mv atlas-server/src/bins atlas-contract/src/bins
git mv atlas-server/tests atlas-contract/tests
[ -d atlas-server/benches ] && git mv atlas-server/benches atlas-contract/benches
```

- [ ] **Step 3: Write `atlas-contract/Cargo.toml`**

Copy `atlas-server/Cargo.toml` and edit: `name = "atlas-contract"`, remove `default-run`, keep every `[dependencies]`, `[dev-dependencies]`, `[[test]]` (`aqc_cucumber`, `harness = false`), `[[bin]]` (`export_aqc_examples`, path `src/bins/export_aqc_examples.rs`) and `[[bench]]` entry exactly as they were. Add nothing yet (utoipa arrives in Task 6).

- [ ] **Step 4: Reduce `atlas-server/Cargo.toml` to the binary**

```toml
[package]
name = "atlas-server"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "atlas-server"
path = "src/main.rs"

[dependencies]
atlas-contract = { path = "../atlas-contract" }
atlas-graph = { path = "../atlas-graph" }
atlas-etl = { path = "../atlas-etl" }
anyhow = "1"
axum = "0.8"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net"] }
```
(Keep whatever `version`/`edition`/`license` fields the old file had.) In `main.rs`, replace every `atlas_server::load::` with `atlas_contract::load::` — nothing else in `main.rs` changes.

- [ ] **Step 5: Workspace, tests, scripts**

- `server/Cargo.toml`: `members = ["atlas-core", "atlas-etl", "atlas-server", "atlas-contract", "atlas-graph", "atlas-cli"]`; in the STANDING COUNTING PROCEDURE comment, no command changes (`--workspace` covers the new member).
- In `atlas-contract/tests/*.rs`, `atlas-contract/benches/*.rs` and `atlas-contract/src/bins/*.rs`: replace `atlas_server::` with `atlas_contract::` (`grep -rl "atlas_server::" atlas-contract | xargs sed -i 's/atlas_server::/atlas_contract::/g'`).
- Scripts: `grep -n "atlas-server" ../scripts/*.sh`; each `cargo test -p atlas-server --test <name>` becomes `-p atlas-contract` (known: `scripts/contract-gate.sh:573` leg 3 `contract_pact`; `timing-gates.sh` wherever it names an `atlas-server` test binary). `cargo run -p atlas-server` (the binary) stays.

- [ ] **Step 6: Verify the suite is unchanged**

Run: `cargo build --workspace && cargo test --workspace 2>&1 | tail -3 && cargo test -p atlas-graph-types 2>&1 | tail -2`
Expected: builds; the pass count equals Step 1's baseline; `cargo run -p atlas-server -- --data-dir ../data` still starts and answers `curl -s localhost:8000/health`.

- [ ] **Step 7: Commit**

```bash
git add -A atlas-contract atlas-server Cargo.toml Cargo.lock ../scripts
git commit -m "atlas-contract: the HTTP surface becomes its own crate; atlas-server keeps only main.rs (D10)"
```

---

### Task 4: Split by route family; response types into `wire/`; drop the `Out` suffix

**Files:**
- Modify/Create in `server/atlas-contract/src/`: `handlers.rs` → `reading.rs`, `catechism.rs`, `places.rs`, `events.rs`, `map.rs`, `meta.rs`; `graph_handlers.rs` → `graph.rs`; `contents.rs` stays; `contract.rs` → folded into `meta.rs`; `wire/mod.rs`, `wire/{graph,reading,catechism,places,events,map,contents,meta}.rs`; `lib.rs`
- Modify: every test under `atlas-contract/tests/` that names an `*Out` type

**Interfaces:**
- Produces: `atlas_contract::wire::*` (the 43 + 7 Scene-family names, see table); handler modules `graph, reading, catechism, places, events, map, contents, meta`.

- [ ] **Step 1: Move handlers into family files (pure move)**

| new file | handlers moved from `handlers.rs` (line) | helpers |
|---|---|---|
| `meta.rs` | `health` (:30), `sources` (:163), and `contract` + `ContractOut` + `MIN/MAX_SUPPORTED_VERSION` from `contract.rs` | |
| `map.rs` | `scene_time` (:56), `scene_scripture` (:84), `eras` (:109), `narratives` (:142), `landmarks` (:153), `land_mask` (:181), `polities` (:267) | `parse_year` (:34) |
| `reading.rs` | `books` (:94), `chapter` (:502), `kretzmann_chapter` (:660), `verse` (:1044), `xrefs` (:1966) | `first_verse_of_target`, `drain_edges` (used by `verse`/`event` — put in `events.rs` if only `event` uses it, else keep in `reading.rs` and `use` it) |
| `catechism.rs` | `catechism_for_span` (:1804), `catechism_item` (:1872) | |
| `places.rs` | `place` (:2071) | |
| `events.rs` | `narrative_event_positions` (:1297), `event` (:1645) | |
| `graph.rs` | all of `graph_handlers.rs` (`node_card` :144, `node_edges` :233, `text_window` :395) | |
| `contents.rs` | unchanged | |

Delete `handlers.rs`, `graph_handlers.rs`, `contract.rs` once empty. `lib.rs`: `pub mod app; pub mod aqc_export; pub mod catechism; pub mod contents; pub mod error; pub mod events; pub mod graph; pub mod graph_wire; pub mod load; pub mod map; pub mod meta; pub mod places; pub mod reading; pub mod wire;`. Update `app.rs`'s 24 `.route(...)` lines to the new module paths (e.g. `get(reading::chapter)`, `get(meta::contract)`, `get(graph::node_card)`). Update tests that import `atlas_contract::handlers::…` or `graph_handlers::…`.

- [ ] **Step 2: Move the response structs into `wire/`**

Create `wire/mod.rs` with `pub mod graph; pub mod reading; pub mod catechism; pub mod places; pub mod events; pub mod map; pub mod contents; pub mod meta; pub use graph::*; pub use reading::*; pub use catechism::*; pub use places::*; pub use events::*; pub use map::*; pub use contents::*; pub use meta::*;`. Move each `pub struct *Out` (with its `impl`s and serde attributes) into the file of the family that returns it:

| wire file | structs |
|---|---|
| `wire/graph.rs` | `EdgeSummaryEntryOut, PersonLifeOut, NodeCardOut, NodeRefOut, EdgeEntryOut, EdgePageOut, TextUnitOut, TextWindowOut` |
| `wire/reading.rs` | `ChapterOut, HeadingOut, VerseOut, PlaceRefOut, PersonRefOut, WordsOfChristSpanOut, KretzmannChapterOut, KretzmannChapterItemOut, KretzmannChapterVerseOut, BookMetaOut, VerseDetailOut, VerseEventOut, CrossRefOut` |
| `wire/catechism.rs` | `CatechismRefOut, CatechismItemOut, CatechismProofVerseOut` |
| `wire/places.rs` | `PlaceDetailOut, HistoryOut, DateClaimOut` |
| `wire/events.rs` | `EventDetailOut, EventPlaceOut, EventWitnessOut, EventAnalogueOut, NarrativeEventPositionsOut, NarrativePositionOut, NarrativeAdjacentEventOut, TimelinePositionOut` |
| `wire/map.rs` | `LandMaskOut, PolitiesOut, PolityOut, PolityDeltaOut` |
| `wire/contents.rs` | `ContentsOut, ContentsRootOut, ContentsChildOut` |
| `wire/meta.rs` | `ContractOut` |

Handlers `use crate::wire;` and refer to `wire::VerseOut` etc.

- [ ] **Step 3: Drop the `Out` suffix everywhere it names a type**

From `server/`:
```bash
names="EdgeSummaryEntry PersonLife NodeCard NodeRef EdgeEntry EdgePage TextUnit TextWindow Chapter Heading Verse PlaceRef PersonRef WordsOfChristSpan KretzmannChapter KretzmannChapterItem KretzmannChapterVerse BookMeta VerseDetail VerseEvent CrossRef CatechismRef CatechismItem CatechismProofVerse PlaceDetail History DateClaim EventDetail EventPlace EventWitness EventAnalogue NarrativeEventPositions NarrativePosition NarrativeAdjacentEvent TimelinePosition LandMask Polities Polity PolityDelta Contents ContentsRoot ContentsChild Contract"
files=$(grep -rlE "\b($(echo $names | sed 's/ /|/g'))Out\b" atlas-contract atlas-cli ../graph-types 2>/dev/null)
for n in $names; do sed -i -E "s/\b${n}Out\b/${n}/g" $files; done
```
Then fix the collisions the compiler reports by qualifying: in files that import both, `use atlas_core::data::Polity as DataPolity;` style aliases are **not** allowed (two names for one thing); use the module path at the use site instead (`data::Polity`, `wire::Polity`). Expect these in `map.rs` (`Polity`, `PolityDelta` vs `atlas_core::data`), `catechism.rs` (`CatechismItem`), and tests.

- [ ] **Step 4: Verify byte-identity**

Run: `cargo test -p atlas-contract --test contract_pact --test aqc_cucumber --test graph_api --test scene_byte_identity --test api 2>&1 | grep -E "test result|FAILED|panicked"`
Expected: every `test result: ok`; no fixture diff (struct names are not serialised).

- [ ] **Step 5: Commit**

```bash
git add -A atlas-contract
git commit -m "atlas-contract: handlers by route family, response types in wire/, Out suffix dropped (D4)"
```

---

### Task 5: Typed kinds on the wire structs; closed shapes; `ToSchema` everywhere; `graph_wire::parse_edge_kind` dies

**Files:**
- Modify: `atlas-contract/Cargo.toml`, `atlas-core/Cargo.toml`
- Modify: `atlas-contract/src/wire/graph.rs` (`EdgeSummaryEntry.kind`, `NodeCard.kind`, `NodeRef.kind`, `EdgePage.kind`), `src/graph.rs`, `src/graph_wire.rs`, every `wire/*.rs`, `atlas-core/src/wire.rs`, `atlas-core/src/{data,sources,time}.rs`
- Test: `atlas-contract/tests/graph_api.rs` (extend), `atlas-contract/tests/contract_pact.rs` (unchanged, run)

**Interfaces:**
- Consumes: Task 2's impls (`atlas-contract` enables `graph-types` features `serde`, `openapi`).
- Produces: `wire::NodeCard { kind: NodeKind, … }`, `wire::NodeRef { kind: NodeKind, … }`, `wire::EdgeSummaryEntry { kind: EdgeKind, … }`, `wire::EdgePage { kind: EdgeKind, … }`; every served type derives `utoipa::ToSchema`; every wire struct carries `#[serde(deny_unknown_fields)]`.

- [ ] **Step 1: Dependencies**

`atlas-contract/Cargo.toml`:
```toml
utoipa = { version = "6.0.0", features = ["yaml", "axum_extras"] }
utoipa-axum = "0.3.0"
utoipa-swagger-ui = { version = "10.0.1", features = ["axum", "vendored"], optional = true }
atlas-graph-types = { path = "../../graph-types", features = ["canon-ids", "serde", "openapi"] }

[features]
dev-docs = ["dep:utoipa-swagger-ui"]
```
`atlas-core/Cargo.toml`: `utoipa = "6.0.0"`.

- [ ] **Step 2: Write the failing test (typed kinds, unchanged bytes)**

Append to `atlas-contract/tests/graph_api.rs` (it already has `compiled_app()` and a `get_json(app, uri)` style helper — reuse whatever helper the file uses to GET a JSON body; the file's existing tests show the exact call):
```rust
const GENESIS_1: &str = "Container:bible-chapter-GEN-1";
const VERSES_IN_GENESIS_1: usize = 31;

#[test]
fn the_card_for_genesis_1_names_its_kind_and_its_three_frontier_groups() {
    // Arrange
    let app = compiled_app();
    // Act
    let body = get_json(&app, &format!("/api/node/{GENESIS_1}"));
    // Assert
    let version = body["version"].clone();
    assert_eq!(body, serde_json::json!({
        "id": GENESIS_1,
        "kind": "Container",
        "label": "Genesis 1",
        "provenance": "kjv",
        "edge_summary": [
            { "kind": "contains",   "count": VERSES_IN_GENESIS_1 },
            { "kind": "member-of",  "count": 1 },
            { "kind": "follows-in", "count": 1 },
        ],
        "version": version,
    }));
}
```
This passes today (strings) and must still pass after the retype — it is the byte-identity witness for this exact card. `version` is the content root; it is taken from the response because it is pinned separately by AGC's `version-root.feature`.

- [ ] **Step 3: Retype the four fields; delete the string conversions**

In `wire/graph.rs`:
```rust
pub struct EdgeSummaryEntry { pub kind: EdgeKind, pub count: usize }
pub struct NodeCard { pub id: String, pub kind: NodeKind, pub label: String, pub provenance: String, pub edge_summary: Vec<EdgeSummaryEntry>, pub version: String, #[serde(skip_serializing_if = "Option::is_none")] pub person: Option<PersonLife>, #[serde(skip_serializing_if = "Option::is_none")] pub description: Option<String> }
pub struct NodeRef { pub id: String, pub kind: NodeKind, pub label: String }
pub struct EdgePage { pub kind: EdgeKind, pub entries: Vec<EdgeEntry>, pub next: Option<usize>, pub version: String }
```
(`use atlas_graph_types::{EdgeKind, NodeKind};`.) In `src/graph.rs`: `kind: format!("{:?}", node_id.kind)` → `kind: node_id.kind`; where `describe_node` supplied a kind string, change `graph_wire::describe_node` to return `(String, NodeKind)` and pass it through; `node_edges` parses `kind` with `EdgeKind::from_label(raw).ok_or_else(|| ApiError::bad_kind(raw))`. Delete `graph_wire::parse_edge_kind` and its unit tests (Task 1's tests cover the inverse now). In `src/graph.rs`'s `EdgeSummary` → `Vec<EdgeSummaryEntry>` conversion, the kind is already an `EdgeKind` — drop the `.label()` call.

- [ ] **Step 4: Closed shapes and schemas**

On **every** struct under `atlas-contract/src/wire/` and the seven in `atlas-core/src/wire.rs`, add to the derive line `utoipa::ToSchema` and add the container attribute `#[serde(deny_unknown_fields)]`:
```rust
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NodeCard { … }
```
On the served `atlas-core` data structs — `CanonBook` (data.rs:11), `Narrative` (:594), `Era` (:602), `Landmark` (:646), `SourcesDocument` (sources.rs:56), `SourceCategory` (:21), `SourceEntry` (:37), `ProvenanceEntry` (:95), `TimeRange` (time.rs:13) — add `utoipa::ToSchema` to the derive **only**; they are deserialised from data files, so `deny_unknown_fields` does not go on them. `ErrorBody`/`ErrorInner` in `error.rs` become owned and public with `ToSchema` (used by Task 6):
```rust
#[derive(Serialize, utoipa::ToSchema)] #[serde(deny_unknown_fields)] pub struct ErrorBody { pub error: ErrorInner }
#[derive(Serialize, utoipa::ToSchema)] #[serde(deny_unknown_fields)] pub struct ErrorInner { pub code: String, pub message: String }
```
and `into_response` builds them with `.to_string()`.

- [ ] **Step 5: Verify**

Run: `cargo test -p atlas-contract --test graph_api --test contract_pact --test aqc_cucumber --test scene_byte_identity 2>&1 | grep -E "test result|FAILED"`
Expected: all ok — including Step 2's test and every AGC fixture comparison (`node-place-hazor-1`, `edges-hazor-1-site-of`, `node-event-ab-ur`).

- [ ] **Step 6: Commit**

```bash
git add -A atlas-contract atlas-core Cargo.lock
git commit -m "wire: kinds are NodeKind/EdgeKind on the structs (bytes unchanged); every served type ToSchema; shapes closed with deny_unknown_fields; parse_edge_kind replaced by EdgeKind::from_label"
```

---

### Task 6: `#[utoipa::path]` on every handler, `routes()` per family, `ApiError: IntoResponses`, the `OpenApiRouter`

**Files:**
- Modify: `atlas-contract/src/{graph,reading,catechism,places,events,map,contents,meta}.rs`, `src/error.rs`, `src/app.rs`, `src/lib.rs`
- Test: `atlas-contract/tests/contract_api.rs`

**Interfaces:**
- Produces: `atlas_contract::openapi_router() -> OpenApiRouter<AppState>`, `atlas_contract::openapi() -> utoipa::openapi::OpenApi`; `app::build_with_sources` unchanged in signature, now built from `openapi_router()`.

- [ ] **Step 1: Write the failing test**

Append to `atlas-contract/tests/contract_api.rs`:
```rust
#[test]
fn every_served_route_is_documented() {
    // Arrange
    let expected = [
        "/health", "/api/contract", "/api/scene", "/api/scene/scripture", "/api/books",
        "/api/chapter/{cref}", "/api/kretzmann/chapter/{cref}", "/api/verse/{vref}", "/api/xrefs/{sref}",
        "/api/catechism/item/{id}", "/api/catechism/{sref}", "/api/place/{id}", "/api/narratives",
        "/api/narrative/event/{id}", "/api/event/{id}", "/api/eras", "/api/polities", "/api/landmarks",
        "/api/land-mask", "/api/sources", "/api/node/{id}", "/api/node/{id}/edges", "/api/text",
        "/api/contents/{corpus}",
    ];
    // Act
    let doc = atlas_contract::openapi();
    let mut paths: Vec<&str> = doc.paths.paths.keys().map(String::as_str).collect();
    paths.sort_unstable();
    // Assert
    let mut want = expected.to_vec();
    want.sort_unstable();
    assert_eq!(paths, want);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p atlas-contract --test contract_api every_served_route_is_documented`
Expected: compile error — `atlas_contract::openapi` not found.

- [ ] **Step 3: `ApiError: IntoResponses`**

In `error.rs`:
```rust
use std::collections::BTreeMap;
use utoipa::openapi::{ContentBuilder, RefOr, Response, ResponseBuilder, ResponsesBuilder};
use utoipa::{IntoResponses, PartialSchema};

impl IntoResponses for ApiError {
    fn responses() -> BTreeMap<String, RefOr<Response>> {
        let json = || ContentBuilder::new().schema(Some(ErrorBody::schema())).build();
        ResponsesBuilder::new()
            .response("400", ResponseBuilder::new().description("bad_window | bad_ref | bad_kind | bad_dir | bad_corpus").content("application/json", json()))
            .response("404", ResponseBuilder::new().description("not_found").content("application/json", json()))
            .response("500", ResponseBuilder::new().description("internal").content("application/json", json()))
            .build()
            .into()
    }
}
```

- [ ] **Step 4: Annotate the 24 handlers**

One attribute per handler, directly above the `pub async fn`. Exact lines (body types are Task 4/5 names; `Q` = query, `P` = path):

```rust
// meta.rs
#[utoipa::path(get, path = "/health", responses((status = 200, body = String, content_type = "text/plain")), tag = "meta")]
#[utoipa::path(get, path = "/api/contract", responses((status = 200, body = wire::Contract)), tag = "meta")]
#[utoipa::path(get, path = "/api/sources", responses((status = 200, body = atlas_core::sources::SourcesDocument)), tag = "meta")]
// map.rs
#[utoipa::path(get, path = "/api/scene", params(("from" = i32, Query), ("to" = i32, Query)), responses((status = 200, body = atlas_core::wire::Scene), ApiError), tag = "map")]
#[utoipa::path(get, path = "/api/scene/scripture", params(("ref" = String, Query)), responses((status = 200, body = atlas_core::wire::Scene), ApiError), tag = "map")]
#[utoipa::path(get, path = "/api/eras", responses((status = 200, body = Vec<atlas_core::data::Era>)), tag = "map")]
#[utoipa::path(get, path = "/api/narratives", responses((status = 200, body = Vec<atlas_core::data::Narrative>)), tag = "map")]
#[utoipa::path(get, path = "/api/landmarks", responses((status = 200, body = Vec<atlas_core::data::Landmark>)), tag = "map")]
#[utoipa::path(get, path = "/api/land-mask", responses((status = 200, body = wire::LandMask)), tag = "map")]
#[utoipa::path(get, path = "/api/polities", params(("from" = i32, Query), ("to" = i32, Query)), responses((status = 200, body = wire::Polities), ApiError), tag = "map")]
// reading.rs
#[utoipa::path(get, path = "/api/books", responses((status = 200, body = Vec<atlas_core::data::CanonBook>)), tag = "reading")]
#[utoipa::path(get, path = "/api/chapter/{cref}", params(("cref" = String, Path)), responses((status = 200, body = wire::Chapter), ApiError), tag = "reading")]
#[utoipa::path(get, path = "/api/kretzmann/chapter/{cref}", params(("cref" = String, Path)), responses((status = 200, body = wire::KretzmannChapter), ApiError), tag = "reading")]
#[utoipa::path(get, path = "/api/verse/{vref}", params(("vref" = String, Path)), responses((status = 200, body = wire::VerseDetail), ApiError), tag = "reading")]
#[utoipa::path(get, path = "/api/xrefs/{sref}", params(("sref" = String, Path)), responses((status = 200, body = Vec<wire::CrossRef>), ApiError), tag = "reading")]
// catechism.rs
#[utoipa::path(get, path = "/api/catechism/item/{id}", params(("id" = String, Path)), responses((status = 200, body = wire::CatechismItem), ApiError), tag = "catechism")]
#[utoipa::path(get, path = "/api/catechism/{sref}", params(("sref" = String, Path)), responses((status = 200, body = Vec<wire::CatechismRef>), ApiError), tag = "catechism")]
// places.rs
#[utoipa::path(get, path = "/api/place/{id}", params(("id" = String, Path), ("from" = Option<i32>, Query), ("to" = Option<i32>, Query)), responses((status = 200, body = wire::PlaceDetail), ApiError), tag = "places")]
// events.rs
#[utoipa::path(get, path = "/api/narrative/event/{id}", params(("id" = String, Path)), responses((status = 200, body = wire::NarrativeEventPositions), ApiError), tag = "events")]
#[utoipa::path(get, path = "/api/event/{id}", params(("id" = String, Path)), responses((status = 200, body = wire::EventDetail), ApiError), tag = "events")]
// graph.rs
#[utoipa::path(get, path = "/api/node/{id}", params(("id" = String, Path)), responses((status = 200, body = wire::NodeCard), ApiError), tag = "graph")]
#[utoipa::path(get, path = "/api/node/{id}/edges", params(("id" = String, Path), ("kind" = EdgeKind, Query), ("cursor" = Option<usize>, Query), ("limit" = Option<usize>, Query)), responses((status = 200, body = wire::EdgePage), ApiError), tag = "graph")]
#[utoipa::path(get, path = "/api/text", params(("ref" = String, Query), ("n" = Option<usize>, Query), ("dir" = Option<String>, Query), ("scope" = Option<String>, Query), ("corpus" = Option<String>, Query)), responses((status = 200, body = wire::TextWindow), ApiError), tag = "graph")]
// contents.rs
#[utoipa::path(get, path = "/api/contents/{corpus}", params(("corpus" = String, Path)), responses((status = 200, body = wire::Contents), ApiError), tag = "contents")]
```
`text_window` returns `Response`; its body is stated explicitly above, which utoipa accepts.

- [ ] **Step 5: `routes()` per family and the registry**

At the bottom of each family file:
```rust
pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(health))
        .routes(routes!(contract))
        .routes(routes!(sources))
}
```
(each file lists its own handlers: `map` → `scene_time, scene_scripture, eras, narratives, landmarks, land_mask, polities`; `reading` → `books, chapter, kretzmann_chapter, verse, xrefs`; `catechism` → `catechism_item, catechism_for_span`; `places` → `place`; `events` → `narrative_event_positions, event`; `graph` → `node_card, node_edges, text_window`; `contents` → `contents`).

In `lib.rs`:
```rust
pub fn openapi_router() -> utoipa_axum::router::OpenApiRouter<app::AppState> {
    utoipa_axum::router::OpenApiRouter::new()
        .merge(meta::routes()).merge(map::routes()).merge(reading::routes()).merge(catechism::routes())
        .merge(places::routes()).merge(events::routes()).merge(graph::routes()).merge(contents::routes())
}

pub fn openapi() -> utoipa::openapi::OpenApi {
    openapi_router().split_for_parts().1
}
```
In `app.rs::build_with_sources`, replace the 24 `.route(...)` lines with:
```rust
let (api, _) = crate::openapi_router().split_for_parts();
let api = api.with_state(state);
```
(the static-dir fallback and `CorsLayer` stay as they are).

- [ ] **Step 6: Verify**

Run: `cargo test -p atlas-contract 2>&1 | grep -E "test result|FAILED"`
Expected: all ok, including `every_served_route_is_documented` and the AGC/AQC pacts (routing is unchanged).

- [ ] **Step 7: Commit**

```bash
git add -A atlas-contract
git commit -m "atlas-contract: every route declared once -- utoipa::path on the handler, routes() per family, OpenApiRouter as the one registry; ApiError describes its own responses"
```

---

### Task 7: `document.rs`, the exporter, the committed documents, the regenerate-and-diff gate, `/api/openapi.yaml`, Swagger UI

**Files:**
- Create: `atlas-contract/src/document.rs`, `atlas-contract/src/bins/export_contract.rs`, `atlas-contract/tests/contract_generation.rs`
- Create (generated): `contracts/openapi.yaml`; regenerate: `contracts/atlas-query-contract/aqc.schema.json`, `contracts/atlas-graph-contract/fixtures/graph-vocabulary.json`
- Modify: `atlas-contract/src/meta.rs` (new handler), `src/lib.rs`, `src/app.rs` (swagger, feature-gated), `Cargo.toml` (`[[bin]]`), `atlas-contract/tests/contract_pact.rs` (`graph_vocabulary()` reads the document), `atlas-contract/tests/contract_api.rs`

**Interfaces:**
- Produces: `document::{openapi, openapi_yaml, aqc_schema_json, graph_vocabulary_json, relations_json, generated_files}` (signatures in spec §5.3); route `GET /api/openapi.yaml`.

- [ ] **Step 1: Write the failing tests**

`atlas-contract/tests/contract_generation.rs`:
```rust
#[test]
fn every_generated_document_is_byte_identical_to_the_committed_one() {
    // Arrange
    let files = atlas_contract::document::generated_files();
    // Act
    let stale: Vec<String> = files
        .iter()
        .filter(|(path, expected)| std::fs::read_to_string(path).ok().as_deref() != Some(expected.as_str()))
        .map(|(path, _)| path.display().to_string())
        .collect();
    // Assert
    assert!(stale.is_empty(), "stale: {stale:?} -- run `cargo run -p atlas-contract --bin export_contract`");
}

#[test]
fn x_atlas_relations_is_the_relations_manifest_in_declaration_order() {
    // Arrange
    let expected = serde_json::json!({
        "directed": [
            { "name": "Contains",     "forward": "contains",        "inverse": "member-of" },
            { "name": "Attests",      "forward": "attested-in",     "inverse": "attests" },
            { "name": "Succession",   "forward": "follows-in",      "inverse": "precedes-in" },
            { "name": "DatedBy",      "forward": "dated-by",        "inverse": "dates" },
            { "name": "LocatedAt",    "forward": "located-at",      "inverse": "site-of" },
            { "name": "Mentions",     "forward": "mentions",        "inverse": "mentioned-in" },
            { "name": "Cites",        "forward": "cites",           "inverse": "cited-by" },
            { "name": "Quotes",       "forward": "quotes",          "inverse": "quoted-by" },
            { "name": "Confesses",    "forward": "confesses",       "inverse": "confessed-in" },
            { "name": "Fulfillment",  "forward": "fulfilled-in",    "inverse": "fulfills" },
            { "name": "Typology",     "forward": "prefigures",      "inverse": "prefigured-by" },
            { "name": "NamedAfter",   "forward": "named-after",     "inverse": "namesake-of" },
            { "name": "JustifiedBy",  "forward": "justified-by",    "inverse": "justifies" },
            { "name": "CommentsOn",   "forward": "comments-on",     "inverse": "commented-on-by" },
            { "name": "SpokenBy",     "forward": "spoken-by",       "inverse": "speech-of" },
            { "name": "SpokenAt",     "forward": "spoken-at",       "inverse": "site-of-speech" },
            { "name": "DerivedFrom",  "forward": "derived-from",    "inverse": "derives" },
            { "name": "Occurs",       "forward": "occurs-in",       "inverse": "words" },
            { "name": "ParentOf",     "forward": "parent-of",       "inverse": "child-of" },
            { "name": "Participates", "forward": "participates-in", "inverse": "participants" },
        ],
        "symmetric": [
            { "name": "Analogue",          "label": "analogous-to" },
            { "name": "CatechismLink",     "label": "catechism-link" },
            { "name": "Corresponds",       "label": "corresponds-to" },
            { "name": "Parallel",          "label": "parallel" },
            { "name": "TemporalAdjacency", "label": "temporal-adjacency" },
            { "name": "Partners",          "label": "partner-of" },
        ],
    });
    // Act
    let actual = atlas_contract::document::relations_json();
    // Assert
    assert_eq!(actual, expected);
}
```
Append to `contract_api.rs`:
```rust
#[test]
fn the_served_openapi_document_equals_the_committed_one() {
    // Arrange
    let app = app();
    let committed = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../contracts/openapi.yaml")).unwrap();
    // Act
    let served = get_text(&app, "/api/openapi.yaml");
    // Assert
    assert_eq!(served, committed);
}
```
(`get_text` = the file's existing body-as-string helper; if it only has a JSON helper, add a sibling that returns the body as `String`.)

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p atlas-contract --test contract_generation --test contract_api`
Expected: compile errors — `document` module missing; `/api/openapi.yaml` 404.

- [ ] **Step 3: `document.rs`**

Newspaper order: what the exporter writes first, then each document, then
the pieces they share.
```rust
use std::path::PathBuf;

use atlas_graph_types::{NodeKind, RelationId, SymRelationId};
use serde_json::{json, Value};
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa::openapi::OpenApi;

const RELATIONS_EXTENSION: &str = "x-atlas-relations";

pub fn generated_files() -> Vec<(PathBuf, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../contracts");
    vec![
        (root.join("openapi.yaml"), openapi_yaml()),
        (root.join("atlas-query-contract/aqc.schema.json"), aqc_schema_json()),
        (root.join("atlas-graph-contract/fixtures/graph-vocabulary.json"), graph_vocabulary_json()),
    ]
}

pub fn openapi_yaml() -> String {
    openapi().to_yaml().expect("the OpenAPI document serialises")
}

pub fn openapi() -> OpenApi {
    let mut doc = crate::openapi();
    doc.extensions = Some(ExtensionsBuilder::new().add(RELATIONS_EXTENSION, relations_json()).build());
    doc
}

pub fn relations_json() -> Value {
    json!({
        "directed": RelationId::ALL.iter().map(|r| json!({"name": format!("{r:?}"), "forward": r.forward_label(), "inverse": r.inverse_label()})).collect::<Vec<_>>(),
        "symmetric": SymRelationId::ALL.iter().map(|s| json!({"name": format!("{s:?}"), "label": s.label()})).collect::<Vec<_>>(),
    })
}

pub fn aqc_schema_json() -> String {
    let doc = serde_json::to_value(openapi()).expect("document to json");
    let defs = doc["components"]["schemas"].clone();
    let out = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://bible-atlas/contracts/atlas-query-contract/aqc.schema.json",
        "title": "Atlas Query Contract -- shapes",
        "description": "Derived from contracts/openapi.yaml by export_contract; do not edit.",
        "$defs": defs,
    });
    serde_json::to_string_pretty(&out).unwrap() + "\n"
}

pub fn graph_vocabulary_json() -> String {
    let rel = relations_json();
    let out = json!({
        "manifest_schema": atlas_graph::sqlite::manifest::MANIFEST_SCHEMA,
        "section_schema_version": atlas_graph::sections::SECTION_SCHEMA_VERSION,
        "node_kinds": NodeKind::ALL.iter().map(|k| k.name()).collect::<Vec<_>>(),
        "relations": rel["directed"],
        "symmetric": rel["symmetric"],
    });
    serde_json::to_string_pretty(&out).unwrap() + "\n"
}
```
Before writing, compare `graph_vocabulary_json()`'s key set with `contract_pact.rs:119`'s existing `graph_vocabulary()` output (the shape the Haskell runner pins): match it exactly, including key order and `EdgeKind` label usage; `serde_json::to_string_pretty` on a `json!` map orders keys alphabetically — if the committed fixture is not alphabetical, build with `serde_json::Map` in the fixture's order. `openapi.yaml` uses `NodeKind`/`EdgeKind` component names and, because Task 5 put `deny_unknown_fields` on every wire struct, each schema carries `additionalProperties: false` (utoipa emits it from that serde attribute); `aqc_cucumber.rs:297`'s assertion depends on it.

`lib.rs`: `pub mod document;`. `meta.rs`:
```rust
#[utoipa::path(get, path = "/api/openapi.yaml", responses((status = 200, body = String, content_type = "application/yaml")), tag = "meta")]
pub async fn openapi_yaml() -> ([(axum::http::HeaderName, &'static str); 1], String) {
    ([(axum::http::header::CONTENT_TYPE, "application/yaml")], crate::document::openapi_yaml())
}
```
and add `.routes(routes!(openapi_yaml))` to `meta::routes()`; add `"/api/openapi.yaml"` to Task 6's `expected` list (25 documented paths).

- [ ] **Step 4: The exporter binary**

`atlas-contract/src/bins/export_contract.rs`:
```rust
fn main() {
    let check = std::env::args().any(|a| a == "--check");
    let mut stale = Vec::new();
    for (path, contents) in atlas_contract::document::generated_files() {
        let current = std::fs::read_to_string(&path).ok();
        if current.as_deref() == Some(contents.as_str()) {
            continue;
        }
        if check {
            stale.push(path.display().to_string());
        } else {
            std::fs::write(&path, contents).unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
            println!("wrote {}", path.display());
        }
    }
    if check && !stale.is_empty() {
        eprintln!("stale generated documents: {stale:?}");
        std::process::exit(1);
    }
}
```
`Cargo.toml`: `[[bin]] name = "export_contract" path = "src/bins/export_contract.rs"`.

- [ ] **Step 5: `contract_pact.rs` reads the document; Swagger UI**

In `contract_pact.rs`, replace the body of `graph_vocabulary()` (:119) with `serde_json::from_str(&atlas_contract::document::graph_vocabulary_json()).unwrap()`; keep `the_published_vocabulary_is_drawn_from_the_macros` (:698) — it now proves document ≡ macros, which is the law's meaning under D8.

In `app.rs::build_with_sources`, after `let api = api.with_state(state);`:
```rust
#[cfg(feature = "dev-docs")]
let api = api.merge(utoipa_swagger_ui::SwaggerUi::new("/swagger-ui").url("/api/openapi.json", crate::document::openapi()));
```

- [ ] **Step 6: Generate, then verify**

Run:
```
cargo run -p atlas-contract --bin export_contract
git diff --stat contracts/
cargo run -p atlas-contract --bin export_contract -- --check ; echo "check exit $?"
cargo test -p atlas-contract --test contract_generation --test contract_api --test contract_pact --test aqc_cucumber 2>&1 | grep -E "test result|FAILED"
cargo build -p atlas-contract --features dev-docs
```
Expected: three files written; `--check` exits 0; all four test binaries ok (AQC's schema step still finds every shape under `$defs` with `additionalProperties: false`; the vocabulary pact still matches); the `dev-docs` build succeeds and `cargo run -p atlas-server --features dev-docs` is NOT expected (the feature is on `atlas-contract`; run the server with `cargo run -p atlas-contract`? No — the binary is `atlas-server`: add `dev-docs = ["atlas-contract/dev-docs"]` to `atlas-server/Cargo.toml [features]` so `cargo run -p atlas-server --features dev-docs -- --data-dir ../data` serves `/swagger-ui`).

- [ ] **Step 7: Commit**

```bash
git add -A atlas-contract atlas-server ../contracts/openapi.yaml ../contracts/atlas-query-contract/aqc.schema.json ../contracts/atlas-graph-contract/fixtures/graph-vocabulary.json
git commit -m "contract: openapi.yaml is the published contract; aqc.schema.json and graph-vocabulary.json derived from it; regenerate-and-diff gate; /api/openapi.yaml; swagger-ui behind dev-docs (D8)"
```

---

### Task 8: No route without a contract — the coverage law and the seven pins

**Files:**
- Create: `atlas-contract/tests/contract_coverage.rs`
- Create: `contracts/atlas-graph-contract/graph/detail-routes.feature`, seven fixtures under `contracts/atlas-graph-contract/fixtures/`
- Modify: `atlas-contract/tests/contract_pact.rs` (record the seven projections), `contracts/atlas-graph-contract/CHANGELOG.md`, `VERSION` (minor bump — additive)

- [ ] **Step 1: Write the failing law**

`atlas-contract/tests/contract_coverage.rs` (newspaper order: the law first, then what it reads):
```rust
use std::path::{Path, PathBuf};

const CONTRACT_SUITES: [&str; 3] = ["atlas-graph-contract", "atlas-query-contract", "atlas-edge"];
const NOT_A_PROMISE: [&str; 2] = ["/health", "/api/openapi.yaml"];

#[test]
fn every_served_route_is_referenced_by_a_contract_scenario() {
    // Arrange
    let features = every_feature_file_concatenated();
    let doc = atlas_contract::openapi();
    // Act
    let uncovered: Vec<String> = doc.paths.paths.keys()
        .filter(|p| !NOT_A_PROMISE.contains(&p.as_str()))
        .map(|p| route_prefix(p))
        .filter(|prefix| !features.contains(prefix.as_str()))
        .collect();
    // Assert
    assert_eq!(uncovered, Vec::<String>::new());
}

fn every_feature_file_concatenated() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts");
    CONTRACT_SUITES
        .iter()
        .flat_map(|suite| files_under(&root.join(suite)))
        .filter(|file| file.extension().is_some_and(|e| e == "feature"))
        .map(|file| std::fs::read_to_string(&file).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir).unwrap().flatten().map(|e| e.path())
        .flat_map(|p| if p.is_dir() { files_under(&p) } else { vec![p] })
        .collect()
}

fn route_prefix(path: &str) -> String {
    path.split('{').next().unwrap().trim_end_matches('/').to_string()
}
```
(`/api/openapi.yaml` is the contract itself and `/health` is a liveness string: `NOT_A_PROMISE` names them.)

- [ ] **Step 2: Run to verify it fails, naming the seven**

Run: `cargo test -p atlas-contract --test contract_coverage`
Expected: FAIL listing `/api/books`, `/api/chapter`, `/api/kretzmann/chapter`, `/api/verse`, `/api/catechism/item`, `/api/place`, `/api/narrative/event` (and nothing else — `/api/catechism` matches the existing `catechism-list` scenario's `/api/catechism/{sref}` reference by prefix; if the prefix logic over-matches, tighten `route_prefix` to keep the segment count).

- [ ] **Step 3: Record seven extensional pins**

`contracts/atlas-graph-contract/graph/detail-routes.feature`:
```gherkin
Feature: the detail routes — pinned whole until the graph API subsumes them
  These seven routes are consumed only by the atlas's own client today. Each
  is pinned as its entire response so that FOCUS can retire them one at a
  time with a visible diff, and so that no route is served without a promise.

  Vocabulary:
    | projection | any of: books, chapter, kretzmann-chapter, verse, catechism-item, place, narrative-event |

  Scenario: the canon's books
    When I GET /api/books
    Then the consumed projection books equals fixture "books"

  Scenario: a chapter
    When I GET /api/chapter/GEN.1
    Then the consumed projection chapter equals fixture "chapter-gen-1"

  Scenario: a Kretzmann chapter
    When I GET /api/kretzmann/chapter/GEN.1
    Then the consumed projection kretzmann-chapter equals fixture "kretzmann-chapter-gen-1"

  Scenario: a verse
    When I GET /api/verse/JHN.3.16
    Then the consumed projection verse equals fixture "verse-jhn-3-16"

  Scenario: a catechism item
    When I GET /api/catechism/item/commandment-1
    Then the consumed projection catechism-item equals fixture "catechism-item-commandment-1"

  Scenario: a place
    When I GET /api/place/hazor_1
    Then the consumed projection place equals fixture "place-hazor-1"

  Scenario: an event's narrative positions
    When I GET /api/narrative/event/ab_ur
    Then the consumed projection narrative-event equals fixture "narrative-event-ab-ur"
```
Record the seven bodies into the pact and the fixtures the way `contract_pact.rs` records `edge-page` today (the recording call at `contract_pact.rs:417` and the fixture write beside it): add one recording per projection name above with the same URLs; the runner's "consumed projection" for these is the whole body, so the projection name maps to the full response (follow how `node-card` declares its consumed fields, and declare all fields). If `/api/place/hazor_1` answers 404 on the committed graph (it did on 2026-09-26 via the running server), use a place id that `/api/contents` or `edges-hazor-1-site-of.json` proves live, and name the fixture after it.

- [ ] **Step 4: Verify**

Run:
```
cargo test -p atlas-contract --test contract_pact --test contract_coverage 2>&1 | grep -E "test result|FAILED"
bash ../scripts/contract-gate.sh --fast
```
Expected: coverage law green; the AGC runner replays the seven new scenarios green against the regenerated pact; the semver gate classifies the change as additive (bump `contracts/atlas-graph-contract/VERSION` minor and add a CHANGELOG line: "detail routes pinned whole: books, chapter, kretzmann-chapter, verse, catechism-item, place, narrative-event").

- [ ] **Step 5: Commit**

```bash
git add -A atlas-contract/tests ../contracts/atlas-graph-contract
git commit -m "contract: no route without a contract -- coverage law; the seven detail routes pinned whole (AGC minor)"
```

---

### Task 9: Mutation tooling and the closing verification

**Files:**
- Create: `server/mutants.toml`
- Modify: `server/Cargo.toml` (counting-procedure comment: add the mutation leg), `scripts/contract-gate.sh` (add `export_contract --check`)

- [ ] **Step 1: cargo-mutants, scoped**

`server/mutants.toml`:
```toml
examine_globs = [
  "atlas-contract/src/document.rs",
  "atlas-contract/src/error.rs",
  "atlas-contract/src/meta.rs", "atlas-contract/src/map.rs", "atlas-contract/src/reading.rs",
  "atlas-contract/src/catechism.rs", "atlas-contract/src/places.rs", "atlas-contract/src/events.rs",
  "atlas-contract/src/graph.rs", "atlas-contract/src/contents.rs",
  "../graph-types/src/id.rs", "../graph-types/src/edge.rs", "../graph-types/src/wire_form.rs",
]
exclude_re = ["wall-clock gate"]
test_tool = "cargo"
additional_cargo_test_args = ["--workspace", "--exclude", "atlas-etl"]
```
Install once: `cargo install cargo-mutants --locked`. Run: `cargo mutants -f atlas-contract/src/document.rs -f ../graph-types/src/edge.rs -f ../graph-types/src/id.rs -f ../graph-types/src/wire_form.rs --features serde,openapi`.
Expected: every mutant caught or listed under `[[skip]]`-style exclusions in `mutants.toml` with a one-line reason each (equivalent mutants only). Add `#[mutants::skip]` to nothing without a reason recorded in `mutants.toml`.

- [ ] **Step 2: The gate learns about the exporter**

In `scripts/contract-gate.sh`, before leg 2, add a leg:
```bash
step "leg 1b: generated contract documents are current"
( cd server && cargo run -q -p atlas-contract --bin export_contract -- --check ); check $? "export_contract --check"
```

- [ ] **Step 3: The standing block**

Run, from `server/`, the three commands in `Cargo.toml`'s STANDING COUNTING PROCEDURE and record the per-command numbers and `TIMING GATES: N/8 passed`; then `bash ../scripts/contract-gate.sh` (full) and `bash ../scripts/contract-semver-gate.sh`.
Expected: workspace green with the count from Task 3 Step 1 plus the tests added here (Tasks 1–8 add 16); gates 8/8; contract gate all legs pass; semver gate: AGC minor (Task 8), AQC unchanged.

- [ ] **Step 4: Commit and push**

```bash
git add server/mutants.toml server/Cargo.toml scripts/contract-gate.sh
git commit -m "gates: export_contract --check in the contract gate; cargo-mutants scoped to the contract crate and graph-types' wire form"
git push origin worktree-bible-atlas-m1
```

---

## Self-review against the spec

- **Coverage:** D1/D3 (Tasks 6–7), D2 (Task 7 commits documents only), D4 (Task 4), D5 (every task's verification runs the pacts), D6 (no wire change needed), D7 (client plan), D8 (Task 7), D9 (Tasks 1–2), D10 (Tasks 3–7); §5.4 gates (Tasks 7–9); §5.5 is informational; the seven pins (Task 8); Swagger UI (Task 7). Client-side items (§6, §11 client rows, Stryker) are in `2026-09-26-contract1b-client.md`.
- **Placeholders:** none; where a builder API name may differ from utoipa 6's, the test is named as the specification.
- **Type consistency:** `wire::NodeCard{kind: NodeKind}` (Task 5) is what Task 6's attributes reference; `document::relations_json` (Task 7) is what `graph_vocabulary_json` and the test consume; `atlas_contract::openapi()` (Task 6) is what Tasks 7–8 call.
