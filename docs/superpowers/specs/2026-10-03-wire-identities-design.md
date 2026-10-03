# A-WIRE-IDENTITIES: name the contract's identities, close the corpus path

- **Status:** design for owner sign-off (PRINCIPLES 12). Planning only; no code in this change.
- **Queue item:** A-WIRE-IDENTITIES (`ops` QUEUE.md), from Claude's CX-FSHARP review C1/I6 (`lane/claude/CX-FSHARP-review` 9dd57f0).
- **Base read:** `worktree-bible-atlas-m1` 25203b8 (AQC 0.27.0, AGC 0.26.0, graph-types 0.15.0). Codex's F# generator read at `lane/codex/CX-FSHARP` 7da4d70.
- **Owner's standing priority:** "Readability and clarity and capturing everything under the type system are still our priorities."

## 1. What this changes, in one paragraph

Today the published contract says `type: string` or `type: integer` for every artifact root, page cursor, node id, edge id and text reference. A generated client therefore accepts a root where a node id belongs, a cursor from one read where another read's cursor belongs, and a Concord reference where a chapter belongs. This item gives each identity a named schema, declared once in Rust where the server already holds the value, and makes every site in the document refer to it. The `/api/contents/{corpus}` path parameter refers to the existing closed `Corpus`. Not one response byte changes: the named types serialize exactly as the strings and integers they replace. Both generators then produce distinct types from the names, and a site that mixes two identities stops compiling.

**The key design choice:** a reference or id that may take several shapes is a named **union** of named **leaf** shapes (`oneOf`). The leaves have patterns that do not overlap. A generated client gets one opaque type per site. It can **widen** a leaf into a union that contains it (a `NodeId` is an `ElementId`), but it can never narrow a union back into a leaf, because narrowing would mean parsing the text, and the client never parses a reference (rule 25). Only the server parses.

## 2. What the contract publishes today

### 2.1 Every identity site, with its operation and JSON path

Found by walking every `200` response schema of `contracts/openapi.yaml` from its operation, plus every parameter. "Schema site" means one property or parameter declaration. "Reach" means one JSON path in one operation's response.

**Artifact root** (the `version` stamp, 32 lowercase hex): 6 schema sites, 7 reaches, 6 operations.

| Operation | JSON path | Declared at |
|---|---|---|
| `node_record` | `$.version` | `NodeRecord.version` |
| `elements` | `$.version` | `ElementPage.version` |
| `elements` | `$.elements[].node.version` | `NodeRecord.version` |
| `node_edges` | `$.version` | `EdgePage.version` |
| `text_window` | `$.version` | `TextWindow.version` |
| `contents` | `$.version` | `Contents.version` |
| `kretzmann_chapter` | `$.version` | `KretzmannChapter.version` |

`text_window` also sends the same root as a quoted `ETag` header. The document does not publish headers, so the header is out of scope. It does derive from the same `ArtifactRoot` (§5).

**Page cursor:** 6 schema sites, 2 operations.

| Operation | Where | Declared at |
|---|---|---|
| `elements` | query `cursor` | `ElementsQuery.cursor` (`integer`, default 0, minimum 0) |
| `elements` | `$.previous`, `$.next` | `ElementPage.previous`, `.next` (`integer \| null`) |
| `node_edges` | query `cursor` | `EdgePageQuery.cursor` (`integer`, default 0, minimum 0) |
| `node_edges` | `$.previous`, `$.next` | `EdgePage.previous`, `.next` (`integer \| null`) |

The two cursors are **different spaces**. An edge page's cursor is an ordinal in the position's edge index (`adjacency.rs`: `Cursor(self.edges[from - back] as usize)`, keyset). An element page's cursor is an offset into the caller's own `ids` list (`graph.rs::elements`). An edge page's `next` passed to `/api/elements` is well-formed and meaningless. `TextWindow.next` is not a cursor. It is the reference that continues the window (see references below).

**Node id** (`Kind:name`, or `text-unit:` plus the unit's reference): 6 schema sites, 27 reaches, 12 operations.

| Operation | JSON path | Declared at |
|---|---|---|
| `node_record` | path `{id}` | `("id" = String, Path)` |
| `node_record` | `$.id` | `NodeRecord.id` |
| `node_record` | `$.text.anchors[].node.id`, `$.place.established.event.id`, `$.place.destroyed.event.id`, `$.book.write_place.id` | `NodeRef.id` |
| `elements` | `$.elements[].node.id` | `NodeRecord.id` |
| `elements` | `$.elements[].node.text.anchors[].node.id`, `$.elements[].node.place.established.event.id`, `$.elements[].node.place.destroyed.event.id`, `$.elements[].node.book.write_place.id`, `$.elements[].edge.subject.node.id`, `$.elements[].edge.object.node.id` | `NodeRef.id` |
| `node_edges` | `$.entries[].neighbour.node.id` | `NodeRef.id` |
| `text_window` | `$.units[].node.id`, `$.units[].body.anchors[].node.id`, `$.units[].heading.event.id` | `NodeRef.id` |
| `contents` | `$.roots[].id`, `$.roots[].children[].id` | `ContentsRoot.id`, `ContentsChild.id` |
| `kretzmann_chapter` | `$.verses[].items[].id` | `KretzmannChapterItem.id` |
| `chapter` | `$.verses[].places[].node.id` | `NodeRef.id` |
| `event` | `$.places[].node.id` | `NodeRef.id` |
| `eras` | `$[].node.id` | `NodeRef.id` |
| `polities` | `$.polities[].node.id` | `NodeRef.id` |
| `scene_time`, `scene_scripture` | `$.places[].node.id`, `$.quiet_places[].node.id` | `NodeRef.id` |

**Edge id** (`Relation:` plus 32 hex): 2 schema sites, 5 reaches, 2 operations.

| Operation | JSON path | Declared at |
|---|---|---|
| `node_edges` | `$.entries[].edge.id`, `$.entries[].neighbour.edge.id` | `EdgeRef.id` |
| `elements` | `$.elements[].edge.id` | `EdgeRecord.id` |
| `elements` | `$.elements[].edge.subject.edge.id`, `$.elements[].edge.object.edge.id` | `EdgeRef.id` |

**Node id or edge id:** 3 schema sites, 2 operations.

| Operation | Where | Declared at |
|---|---|---|
| `node_edges` | path `{id}` | `("id" = String, Path)`, read as `PositionReference` |
| `elements` | query `ids` (array items, comma form) | `#[param(value_type = Vec<String>)]` on `ElementIds` |
| `elements` | `$.elements[].id` (the `missing` case) | `MissingElement.id`, built by `case_of` with `String::schema()` |

**Bible and Concord references:** 17 schema sites (11 properties and 6 parameters), 19 response reaches, 10 operations. The shape column comes from the Rust that parses or produces each value, not from the field's name.

| Operation | Where | Declared at | Shapes actually used |
|---|---|---|---|
| `chapter` | path `{cref}` | `("cref" = String, Path)` → `ChapterReference` | chapter `EXO.14` |
| `kretzmann_chapter` | path `{cref}` | same | chapter |
| `catechism_for_span` | path `{sref}` | `("sref" = String, Path)` → `VerseSpan` | verse `MAT.28.19`, same-chapter passage `GEN.1.1-5` |
| `xrefs` | path `{sref}` | same | verse, passage |
| `scene_scripture` | query `ref` | `ScripturePassage.r#ref: String` → `ScriptureRef::parse` | book `GEN`, chapter, verse, passage (the published prose names three. `parse` also accepts a book.) |
| `text_window` | query `ref` | `TextWindowQuery.r#ref: String` | Bible verse or chapter (`ReadingReference`) when `corpus=bible`; Concord paragraph `BoC 7.2.1` when `corpus=concord` |
| `chapter` | `$.ref` | `Chapter.ref` | chapter |
| `catechism_item` | `$.verses[].vref` | `CatechismProofVerse.vref` | verse |
| `xrefs` | `$[].target` | `CrossRef.target` | verse, passage, **and a cross-chapter range** `MAT.5.3-MAT.6.2` (`atlas-core/src/xrefs.rs` names all three) |
| `scene_time`, `scene_scripture` | `$.ref` | `Scene.ref` (`string \| null`) | any `ScriptureRef`, written by its `Display` |
| `event` | `$.mentioned_in[]` | `EventPage.mentioned_in` | verse |
| `event`, `narrative_event_positions` (×4 paths), `scene_time`, `scene_scripture` | `…verse_groups[].verses[]` | `VerseGroup.verses` | verse |
| `polities` | `$.polities[].transition.verses[]`, `$.polities[].fall.verses[]` | `PolityDelta.verses` | verse |
| `text_window` | `$.units[].ref`, `$.next` | `TextUnit.ref`, `TextWindow.next` | Bible verse or Concord paragraph |
| `contents` | `$.roots[].ref`, `$.roots[].children[].ref` | `ContentsRoot.ref`, `ContentsChild.ref` | Bible chapter (`GEN.1`) or Concord paragraph (`BoC 1.1.1`) |

**Corpus:** 4 sites, 1 of them open.

| Operation | Where | Today |
|---|---|---|
| `contents` | path `{corpus}` | **`type: string`**, read as `Path<String>` then `Corpus::named`, `not_found` otherwise |
| `text_window` | query `corpus` | `$ref: Corpus` |
| `contents` | `$.corpus` | `$ref: Corpus` |
| every `TextRef` | `corpus` (the union's tag) | an inline `enum: [bible, concord]` built from `TextRef`'s own `Case` literals (`locus.rs`). It restates `Corpus`'s values rather than deriving them. |

### 2.2 What graph-types and core already model, and where the wire flattens it

| Identity | Typed value the server already holds | Flattened at the wire to |
|---|---|---|
| Artifact root | `graph_types::store::GraphVersion(ContentHash)` | `String` by `atlas_graph::version_hex` (6 wire sites, plus the ETag, `compile_graph`'s manifest and `atlas-server` `main`) |
| Edge page cursor | `graph_types::adjacency::Cursor(usize)` | `usize` (`page.next.map(\|after\| after.0)`), request `AsGiven<usize>` |
| Element page cursor | none (computed through `Cursor` arithmetic, then `.0`) | `usize` |
| Node id | `graph_types::id::AnyNodeId { kind, raw }`; per-kind `NodeId<K>` | `String` by `atlas_graph::node_ref::encode_node_id` (a text unit's wire id needs the compiled reference, so the wire form is not a pure function of `AnyNodeId`) |
| Edge id | `graph_types::edge::EdgeId(String)` | `String` by `id.0.clone()` (`graph_wire::edge_ref`, `graph.rs::read_edge_record`) |
| Node id or edge id | `reference::ElementId { Node(AnyNodeId), Edge(EdgeId) }` (request side only) | `String` (`AskedElement.asked`) |
| Verse | `atlas_core::refs::VerseId` (already serializes as `BOOK.C.V`) | `String` everywhere it is published |
| Chapter, verse-or-chapter, verse-or-passage, Concord paragraph | `reference::{ChapterReference, ReadingReference, VerseSpan, ConcordParagraphReference}` (request side only) | `String` on responses, built by `format!` |
| Any Bible reference | `atlas_core::refs::ScriptureRef` | `String` by `to_string()` (`Scene.ref`) |
| Cross-reference target | none (`AggregatedXref.target: String`, re-parsed by `target_span`) | `String` |
| Text unit reference (compiled) | `GraphQuery::references` answers `Option<String>` | `String` |
| Corpus | `wire::Corpus` (closed) | `String` at `/api/contents/{corpus}` only |

Three kinds of node already have named schemas through graph-types' `NodeId<K>` (`EraId`, `PolityId`, `NarrativeId`). Note: those carry a kind's **local name** (`patriarchs`), not the wire node id (`Era:patriarchs`). §7 turns that difference into a description and a law.

### 2.3 Do node ids and edge ids share one string space?

**Yes.** Both are `Prefix:rest`. The prefix sets do not overlap today. Node prefixes are `text-unit` plus 13 `NodeKind` names that `decode_node_id` lists by hand. Edge prefixes are the 22 directed and 7 symmetric relation names that `EdgeId::recorded_kind` reads. A reader can never mistake one for the other, but nothing guarantees it: no law says the sets stay apart, and `decode_node_id`'s list is a hand-written literal that already omits two kinds (`Source`, `PeopleGroup`).

**How the wire tells them apart:**

- **Responses never ask the id.** `Element` carries the discriminator `element: node | edge | missing` and `PositionRef` carries `position: node | edge`. A generated client switches on the tag.
- **Requests are told apart by prefix.** `decode_element_id` (`reference.rs`) tries `decode_node_id` first, then `decode_edge_id`, which requires a relation name and a 32-hex hash. This happens at the two sites that take either id: `ids` and `/api/node/{id}/edges`.
- **`MissingElement.id` is the one response value that may be either.** It echoes what the caller asked for, so it is typed `ElementId` below.

## 3. The design

### 3.1 Principles the catalogue follows

1. **Each type is declared once, where the server holds the value.** An existing Rust type gains its schema in place: `VerseId`, `ScriptureRef`, `EdgeId`. A value the server holds only as text, or only as raw arithmetic, gets a newtype in one new module, `server/atlas-core/src/identity.rs`. It sits in `atlas-core` because `NodeRef`, `Scene`, `VerseGroup` and `PolityDelta` live there. The `atlas-contract` wire modules re-export it.
2. **One mechanism.** Two declaration macros in `identity.rs`, `wire_identity!` (a leaf) and `wire_union!` (a union), generate the newtype, its `Serialize`, its `PartialSchema`/`ToSchema`, its widening `From` impls and its entry in `identity::ALL`. This mirrors `vocabulary!`. Without them, 14b would find 18 copies of the same boilerplate.
3. **Patterns come from vocabularies, never from literals.** They are built from `canon::BOOKS`, the addressable `NodeKind`s, `RelationId::ALL`/`SymRelationId::ALL` and `ContentHash`'s width. A leaf's pattern is what makes its union's `oneOf` sound, because exactly one member matches any value. It also describes a format the server already serves; it changes no byte.
4. **Private constructors.** Each newtype's field is private. The server makes a `NodeId` only through `NodeId::encoded` (moved here from `atlas_graph::node_ref`) or `NodeId::asked` (a request id the decoder already accepted). The same holds for `ElementId`, `ArtifactRoot::of` and the cursors. `asked` must be `pub` across crates, so a source law (§6, L2d) fences its one caller.
5. **Requests keep reading their raw text where the order of refusals depends on it.** For example, `text_window` checks `dir`/`scope` before it reads `ref`. The published schema comes from `#[param(value_type = …)]` naming the identity type, so every refusal stays byte-identical.

### 3.2 The catalogue

Ten leaves and eight unions, plus the existing `BookId` and `Corpus`.

| Schema | Rust type | Kind | Members (unions) | Sites |
|---|---|---|---|---|
| `ArtifactRoot` | `identity::ArtifactRoot(GraphVersion)` | leaf string | | 6 |
| `EdgePageCursor` | `identity::EdgePageCursor(Cursor)` | leaf integer | | 3 |
| `ElementPageCursor` | `identity::ElementPageCursor(Cursor)` | leaf integer | | 3 |
| `NodeId` | `identity::NodeId(String)` | leaf string | | 6 |
| `EdgeId` | `graph_types::edge::EdgeId` (existing) | leaf string | | 2 |
| `ElementId` | `identity::ElementId` | union | `NodeId`, `EdgeId` | 3 |
| `VerseReference` | `atlas_core::refs::VerseId` (existing) | leaf string | | 4 (`vref`, `mentioned_in`, `VerseGroup.verses`, `PolityDelta.verses`) |
| `ChapterReference` | `identity::ChapterReference` (moved from `reference.rs`) | leaf string | | 3 (`{cref}` ×2, `Chapter.ref`) |
| `PassageReference` | `identity::PassageReference` (new) | leaf string | | member only |
| `VerseRangeReference` | `identity::VerseRangeReference` (new) | leaf string | | member only |
| `ConcordReference` | `identity::ConcordReference` (renamed from `ConcordParagraphReference`) | leaf string | | member only |
| `BibleReference` | `atlas_core::refs::ScriptureRef` (existing) | union | `BookId`, `ChapterReference`, `VerseReference`, `PassageReference` | 2 (`scene_scripture` `ref`, `Scene.ref`) |
| `VerseSpanReference` | `identity::VerseSpanReference` (renamed from `VerseSpan`) | union | `VerseReference`, `PassageReference` | 2 (`{sref}` ×2) |
| `CrossReferenceTarget` | `identity::CrossReferenceTarget` | union | `VerseReference`, `PassageReference`, `VerseRangeReference` | 1 |
| `ReadingReference` | `identity::ReadingReference` (reshaped from the struct) | union | `VerseReference`, `ChapterReference` | member only |
| `UnitReference` | `identity::UnitReference` | union | `VerseReference`, `ConcordReference` | 2 (`TextUnit.ref`, `TextWindow.next`) |
| `ContentsReference` | `identity::ContentsReference` | union | `ChapterReference`, `ConcordReference` | 2 |
| `TextWindowReference` | `identity::TextWindowReference` | union | `VerseReference`, `ChapterReference`, `ConcordReference` | 1 (`text_window` `ref`) |
| `Corpus` (existing) | `wire::Corpus` | enum | | +1 (`contents` `{corpus}`) |

**Widening is total and derived.** Every union lists its members. Widening is also transitive over set inclusion: a union whose members are all in another union widens into it. So `ContentsReference → TextWindowReference`, `UnitReference → TextWindowReference`, `ReadingReference → TextWindowReference` and `VerseSpanReference → CrossReferenceTarget`. That is how a client takes a contents entry, or a unit's `ref`, straight into `/api/text` without parsing anything. The full widening graph is one whole value pinned by law L1b.

**Two Rust names differ from their schema names:** `VerseId` publishes as `VerseReference` and `ScriptureRef` as `BibleReference`. Renaming either touches 23 or more files outside this item's subject. Law L1c pins the name map so the correspondence is written down once. The renames belong to A-BACKLOG's FOCUS-1 R36 typed-ids batch, which already names `BookId`.

### 3.3 Each type in full

`{BOOK}` below stands for the alternation the generator derives from `canon::BOOKS`, in canon order. It expands to:
`GEN|EXO|LEV|NUM|DEU|JOS|JDG|RUT|1SA|2SA|1KI|2KI|1CH|2CH|EZR|NEH|EST|JOB|PSA|PRO|ECC|SNG|ISA|JER|LAM|EZK|DAN|HOS|JOL|AMO|OBA|JON|MIC|NAM|HAB|ZEP|HAG|ZEC|MAL|MAT|MRK|LUK|JHN|ACT|ROM|1CO|2CO|GAL|EPH|PHP|COL|1TH|2TH|1TI|2TI|TIT|PHM|HEB|JAS|1PE|2PE|1JN|2JN|3JN|JUD|REV`.

`{N}` is `[1-9][0-9]*` (`parse_positive` refuses 0). `{P}` is `[0-9]+` (a Concord paragraph may be 0). `{HASH}` is `[0-9a-f]{32}` (the `canon-ids` `ContentHash`, which the workspace builds with).

#### The two macros (Rust)

```rust
macro_rules! wire_identity {
    ($name:ident ( $inner:ty ) as string, $description:expr, $pattern:expr) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($inner);

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.collect_str(&self.0)
            }
        }

        impl utoipa::PartialSchema for $name {
            fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
                utoipa::openapi::schema::ObjectBuilder::new()
                    .schema_type(utoipa::openapi::schema::SchemaType::Type(utoipa::openapi::schema::Type::String))
                    .description(Some($description))
                    .pattern(Some($pattern))
                    .into()
            }
        }

        impl utoipa::ToSchema for $name {}
    };
    ($name:ident ( $inner:ty ) as cursor, $description:expr) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($inner);

        impl $name {
            pub const FIRST: $name = $name(Cursor::FIRST);
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_u64(self.0 .0 as u64)
            }
        }

        impl utoipa::PartialSchema for $name {
            fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
                utoipa::openapi::schema::ObjectBuilder::new()
                    .schema_type(utoipa::openapi::schema::SchemaType::Type(utoipa::openapi::schema::Type::Integer))
                    .description(Some($description))
                    .minimum(Some(0))
                    .default(Some(serde_json::json!($name::FIRST.0 .0)))
                    .into()
            }
        }

        impl utoipa::ToSchema for $name {}
    };
}

macro_rules! wire_union {
    ($name:ident, $description:expr, { $($case:ident($member:ty)),+ $(,)? }) => {
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum $name { $($case($member)),+ }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                match self { $($name::$case(member) => member.serialize(s)),+ }
            }
        }

        $(impl From<$member> for $name {
            fn from(member: $member) -> $name { $name::$case(member) }
        })+

        impl utoipa::PartialSchema for $name {
            fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
                utoipa::openapi::schema::OneOfBuilder::new()
                    $(.item(utoipa::openapi::Ref::from_schema_name(<$member as utoipa::ToSchema>::name())))+
                    .description(Some($description))
                    .into()
            }
        }

        impl utoipa::ToSchema for $name {
            fn schemas(schemas: &mut Vec<(String, utoipa::openapi::RefOr<utoipa::openapi::Schema>)>) {
                $(schemas.push((<$member as utoipa::ToSchema>::name().to_string(), <$member as utoipa::PartialSchema>::schema()));)+
            }
        }
    };
}
```

Union-to-union widening (`ContentsReference → TextWindowReference` and the rest) is generated from `identity::WIDENINGS`, the one table L1b pins. It is not hand-written.

#### `ArtifactRoot`

```rust
wire_identity!(ArtifactRoot(RootHex) as string,
    "The root of the compiled artifact a response was read from: two responses that carry the same root were read from the same data.",
    ArtifactRoot::pattern());

impl ArtifactRoot {
    pub fn of(version: GraphVersion) -> ArtifactRoot { ArtifactRoot(RootHex(version)) }
    fn pattern() -> String { format!("^[0-9a-f]{{{}}}$", ContentHash::HEX_WIDTH) }
}
```

`RootHex` is a private `Display` adapter that writes `version.0.hex()`, the same bytes `version_hex` writes today. `ContentHash::HEX_WIDTH` is new in graph-types (16 without `canon-ids`, 32 with). It replaces the two literal widths in `from_hex`.

```yaml
    ArtifactRoot:
      type: string
      description: 'The root of the compiled artifact a response was read from: two responses that carry the same root were read from the same data.'
      pattern: ^[0-9a-f]{32}$
```

```csharp
[System.Text.Json.Serialization.JsonConverter(typeof(ArtifactRoot.Json))]
public sealed record ArtifactRoot
{
    private ArtifactRoot(string value) => Value = value;

    public string Value { get; }

    public sealed class Json : System.Text.Json.Serialization.JsonConverter<ArtifactRoot>
    {
        public override ArtifactRoot Read(ref System.Text.Json.Utf8JsonReader reader, System.Type typeToConvert, System.Text.Json.JsonSerializerOptions options) =>
            reader.TokenType == System.Text.Json.JsonTokenType.String
                ? new ArtifactRoot(reader.GetString()!)
                : throw new System.Text.Json.JsonException("an ArtifactRoot is a JSON string");

        public override void Write(System.Text.Json.Utf8JsonWriter writer, ArtifactRoot value, System.Text.Json.JsonSerializerOptions options) =>
            writer.WriteStringValue(value.Value);
    }
}
```

F#, as Codex's generator already shapes a named scalar: `and [<Struct; JsonConverter(typeof<ArtifactRootJsonConverter>)>] ArtifactRoot = private | ArtifactRoot of string`, with its private converter refusing `null`.

**Every other string leaf has the same C# and F# shape with its own name.** The C# shape is a sealed record, a private constructor, `Value` and a nested `Json` converter. The F# shape is a private single-case struct union with a converter. Only the Rust declaration and the OpenAPI fragment are repeated below.

#### `EdgePageCursor` and `ElementPageCursor`

```rust
wire_identity!(EdgePageCursor(Cursor) as cursor,
    "Where a page of one position's neighbours resumes, as an edge page's `next` or `previous` hands it back; the first page is 0.");
wire_identity!(ElementPageCursor(Cursor) as cursor,
    "Where a read of many elements resumes in the list of ids it was asked, as an element page's `next` or `previous` hands it back; the first page is 0.");
```

```yaml
    EdgePageCursor:
      type: integer
      description: Where a page of one position's neighbours resumes, as an edge page's `next` or `previous` hands it back; the first page is 0.
      default: 0
      minimum: 0
    ElementPageCursor:
      type: integer
      description: Where a read of many elements resumes in the list of ids it was asked, as an element page's `next` or `previous` hands it back; the first page is 0.
      default: 0
      minimum: 0
```

The default moves from the parameter onto the type, because "0 is the first page" is a fact about the cursor, not about one parameter. Each parameter becomes `schema: { $ref: '#/components/schemas/EdgePageCursor' }`. `EdgePage.next` becomes `oneOf: [$ref EdgePageCursor, type: 'null']`, which is how utoipa writes every `Option<Named>` already (`BookDetail.write_place`).

C#: `public sealed record EdgePageCursor { private EdgePageCursor(int value) …; public int Value { get; } public static EdgePageCursor First { get; } = new(0); … }`. `First` comes from the schema's `default`. It replaces the generator's `PagedReads.FirstPage`, whose "every paged read publishes one default" check becomes per-type. F#: `private | EdgePageCursor of int`. Codex's generator also needs to read `default` to emit `EdgePageCursor.first`.

#### `NodeId`

```rust
wire_identity!(NodeId(String) as string,
    "The id of one node of the graph, as every response names it and every read takes it: its kind, a colon, and its name within that kind -- or, for a unit of text, `text-unit:` and the unit's reference.",
    NodeId::pattern());

impl NodeId {
    pub fn encoded(ids: &[AnyNodeId], query: &(impl GraphQuery + ?Sized)) -> Result<Vec<NodeId>, UnreferencedUnit> { … }
    pub fn asked(accepted: &str) -> NodeId { NodeId(accepted.to_string()) }
    pub const TEXT_UNIT: &'static str = "text-unit";
    pub const ADDRESSABLE: [NodeKind; 13] = [ … NodeKind::ALL minus TextUnit, Source, PeopleGroup … ];
    fn pattern() -> String {
        format!("^(?:{TEXT}:(?:{VERSE}|{CONCORD})|(?:{KINDS}):.+)$", …)
    }
}
```

`encoded` is `atlas_graph::node_ref::encode_node_ids` moved here verbatim. `UnreferencedUnit` moves with it. `ADDRESSABLE` becomes the one table both `encoded` and `graph_wire::decode_node_id` read. Today `decode_node_id`'s `match` lists 13 literal names. `Source` and `PeopleGroup` stay out because they are out today: changing what decodes would change a response (§8 F-WI-2).

```yaml
    NodeId:
      type: string
      description: 'The id of one node of the graph, as every response names it and every read takes it: its kind, a colon, and its name within that kind -- or, for a unit of text, `text-unit:` and the unit''s reference.'
      pattern: ^(?:text-unit:(?:(?:{BOOK})\.{N}\.{N}|BoC {P}\.{P}\.{P})|(?:Container|Event|Narrative|Place|Person|Anchor|Era|Polity|CatechismItem|Translation|CommentaryItem|LexiconEntry|Map):.+)$
```

#### `EdgeId` (graph-types, existing type)

```rust
impl Serialize for EdgeId { fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.serialize_str(&self.0) } }

impl PartialSchema for EdgeId {
    fn schema() -> RefOr<Schema> {
        ObjectBuilder::new().schema_type(SchemaType::Type(Type::String))
            .description(Some(EDGE_ID))
            .pattern(Some(format!("^(?:{}):[0-9a-f]{{{}}}$", relation_names().join("|"), ContentHash::HEX_WIDTH)))
            .into()
    }
}

impl ToSchema for EdgeId {}
```

This goes in `graph-types/src/wire_form.rs`, beside `NodeId<K>` and `EdgeKind`, behind the existing `serde`/`openapi` features. `relation_names()` is `RelationId::ALL` then `SymRelationId::ALL`, by `name()`.

```yaml
    EdgeId:
      type: string
      description: 'The id of one edge of the graph: the relation it is recorded in, a colon, and its content address. Either end''s neighbour page and the element read carry the same id for the same connection.'
      pattern: ^(?:Contains|Attests|Succession|DatedBy|LocatedAt|Mentions|Cites|Quotes|Confesses|Fulfillment|Typology|NamedAfter|JustifiedBy|CommentsOn|SpokenBy|SpokenAt|DerivedFrom|Occurs|ParentOf|Participates|AuthoredBy|Shows|Analogue|CatechismLink|Corresponds|Parallel|TemporalAdjacency|Spouses|Brethren):[0-9a-f]{32}$
```

#### `ElementId`

```rust
wire_union!(ElementId,
    "A node's id or an edge's id: the form the element read and a neighbour page take a position in. The two never share a prefix.",
    { Node(NodeId), Edge(EdgeId) });
```

```yaml
    ElementId:
      description: 'A node''s id or an edge''s id: the form the element read and a neighbour page take a position in. The two never share a prefix.'
      oneOf:
      - $ref: '#/components/schemas/NodeId'
      - $ref: '#/components/schemas/EdgeId'
```

The request-side enum `reference::ElementId { Node(AnyNodeId), Edge(EdgeId) }` is renamed `ElementPosition`, so the two never share a name. `AskedElement.asked: String` becomes `ElementId`. `Element::Missing { id: String }` becomes `{ id: ElementId }`, and its `case_of` part refers to `ElementId` instead of `String::schema()`.

C#: a sealed record like the leaves, plus `public static implicit operator ElementId(NodeId id) => new(id.Value);` and the same for `EdgeId`. F#: `private | ElementId of string` plus `ElementId.ofNodeId : NodeId -> ElementId` and `ElementId.ofEdgeId`. Codex's generator must learn this shape: §4.3 says what it needs.

#### `VerseReference` (core, existing `VerseId`)

`VerseId` already serializes as `BOOK.C.V` and deserializes through `parse_canonical`. It gains:

```rust
impl utoipa::PartialSchema for VerseId { fn schema() -> RefOr<Schema> { reference_schema(VERSE_REFERENCE, &format!("^(?:{}){}$", book_alternation(), r"\.[1-9][0-9]*\.[1-9][0-9]*")) } }
impl utoipa::ToSchema for VerseId { fn name() -> Cow<'static, str> { Cow::Borrowed("VerseReference") } }
```

```yaml
    VerseReference:
      type: string
      description: One verse of the Bible, as `BOOK.CHAPTER.VERSE`.
      pattern: ^(?:{BOOK})\.{N}\.{N}$
```

#### `ChapterReference`, `PassageReference`, `VerseRangeReference`, `ConcordReference`

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChapterReference { pub book: BookId, pub chapter: u16 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassageReference { pub book: BookId, pub chapter: u16, pub from_verse: u16, pub to_verse: u16 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerseRangeReference { pub from: VerseId, pub to: VerseId }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConcordReference { pub part: u8, pub article: u16, pub paragraph: u16 }
```

Each has `FromStr` (the parsers that are in `reference.rs` today, moved), `Display` (the inverse) and a schema through `reference_schema(description, pattern)`. `Serialize` writes `Display`. These are structured values, not strings, so the type system holds the parts.

```yaml
    ChapterReference:
      type: string
      description: One whole chapter of the Bible, as `BOOK.CHAPTER`.
      pattern: ^(?:{BOOK})\.{N}$
    PassageReference:
      type: string
      description: A run of verses within one chapter of the Bible, as `BOOK.CHAPTER.FIRST-LAST`, the first verse before the last.
      pattern: ^(?:{BOOK})\.{N}\.{N}-{N}$
    VerseRangeReference:
      type: string
      description: A run of verses that crosses a chapter boundary, as two verse references joined by `-`.
      pattern: ^(?:{BOOK})\.{N}\.{N}-(?:{BOOK})\.{N}\.{N}$
    ConcordReference:
      type: string
      description: One paragraph of the Book of Concord, as `BoC PART.ARTICLE.PARAGRAPH`.
      pattern: ^BoC {P}\.{P}\.{P}$
```

`ScriptureRef`'s `Chapter { book, chapter }` and `Passage { … }` variants become `Chapter(ChapterReference)` and `Passage(PassageReference)`. Without that, the same four fields are declared twice (14b). The change touches 36 match sites in 14 files (§5).

#### `BibleReference` (core, existing `ScriptureRef`)

```rust
impl Serialize for ScriptureRef { fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.collect_str(self) } }
impl PartialSchema for ScriptureRef { fn schema() -> RefOr<Schema> { one_of(BIBLE_REFERENCE, [BookId::name(), ChapterReference::name(), VerseId::name(), PassageReference::name()]) } }
impl ToSchema for ScriptureRef { fn name() -> Cow<'static, str> { Cow::Borrowed("BibleReference") } }
```

```yaml
    BibleReference:
      description: 'Any reference into the Bible: a whole book, a chapter, a verse, or a run of verses within one chapter.'
      oneOf:
      - $ref: '#/components/schemas/BookId'
      - $ref: '#/components/schemas/ChapterReference'
      - $ref: '#/components/schemas/VerseReference'
      - $ref: '#/components/schemas/PassageReference'
```

`BookId` is already a published string enum, and its 66 codes are exactly the book-only shape, so it is reused. Its enum and the other three patterns are pairwise disjoint.

#### The remaining unions

```rust
wire_union!(VerseSpanReference, "One verse, or a run of verses within one chapter.", { Verse(VerseId), Passage(PassageReference) });
wire_union!(CrossReferenceTarget, "Where a cross reference points: one verse, a run of verses within a chapter, or a run that crosses chapters.", { Verse(VerseId), Passage(PassageReference), Range(VerseRangeReference) });
wire_union!(ReadingReference, "Where a reading of the Bible opens: a verse, or a whole chapter.", { Verse(VerseId), Chapter(ChapterReference) });
wire_union!(UnitReference, "One unit of a corpus's text by its reference: a verse of the Bible, or a paragraph of the Book of Concord.", { Verse(VerseId), Concord(ConcordReference) });
wire_union!(ContentsReference, "The reference a contents entry opens at: a chapter of the Bible, or a paragraph of the Book of Concord.", { Chapter(ChapterReference), Concord(ConcordReference) });
wire_union!(TextWindowReference, "Where a window of a corpus's reading spine opens: a verse or a chapter of the Bible, or a paragraph of the Book of Concord.", { Verse(VerseId), Chapter(ChapterReference), Concord(ConcordReference) });
```

Each generates `oneOf` over its members' `$ref`s, exactly like `ElementId` above. `UnitReference` and `TextWindowReference` are needed because `/api/text` serves either corpus. **What the types do not capture:** which member `/api/text` accepts, and which member a `TextUnit.ref` or a contents entry carries, still depends on the `corpus` the caller sent. Capturing that means splitting `Contents` and `TextWindow` per corpus. The bytes would stay the same, but the document would change shape (MAJOR class), so it is proposed as a follow-on (§8 F-WI-5) and not done here.

### 3.4 Structured references: what is named and what is not

`TextRef` (`UnitText.locus`, `ContentsRoot.locus`, `ContentsChild.locus`, `TextPoint.unit` inside every `TextSpan`) is already a closed tagged union. It has two cases: `BibleRef { book: BookId, chapter, verse }` and `ConcordRef { part, article, paragraph }`.

- **Named already, and unchanged:** `TextRef`, `BibleRef`, `ConcordRef`, `BookId`, `TextPoint`, `TextSpan`.
- **Not named, and staying so:** the integer coordinates `chapter`, `verse`, `part`, `article`, `paragraph`, `word`. They are positions inside a reference, not identities a caller hands back. Giving each its own type is possible, but nothing in this item mixes them up.
- **Changed:** the `corpus` tag's values come from `Corpus`. `locus.rs`'s `BIBLE`/`CONCORD` cases take `tag: Corpus::Bible.name()` and `Corpus::Concord.name()`. Today `Corpus`'s values come from `BIBLE_CORPUS`/`CONCORD_CORPUS`, so the bytes stay the same. The tag stays an inline `enum` in the document, because both generators read the discriminator from the inline property. Law L2c pins that its values equal `Corpus`'s.
- **A naming hazard this item creates and documents:** `BibleRef` (a structured verse locus) now sits beside `BibleReference` (the dotted text), and `ConcordRef` beside `ConcordReference`. The item's names are kept, and each description says which form it is. Renaming the structured cases (say `BibleLocus`) is a MAJOR-class schema rename (precedent: 0.13.0 `NodeCard` to `NodeRecord`), so it is not done here.

`Corpus` itself gets the description it lacks. Today it publishes `description: ''`, a gap in the existing `vocabulary!` output: "Which corpus of text something belongs to: the Bible or the Book of Concord."

## 4. Generated client code

### 4.1 C#: `client.ContractGenerator`

NSwag/NJsonSchema turns a named scalar schema into a plain `string` today. The generated `Era.Id` is `string` although the document names `EraId`. So the generator gains two things:

1. **`IdentityTypes.Emit(OpenApiDocument) : string`.** It walks `document.Definitions` and selects every *identity*: a schema that is a `string`/`integer` with no `enum` and no properties, or a `oneOf` whose members are all identities. It prints one sealed record per identity (§3.3's shape). Each union also gets an `implicit operator` from every identity whose member set it contains, computed from the `oneOf` lists, and each cursor gets `First` from its `default`. The output is appended like `PagedReads` is today, and `PagedReads` retires.
2. **`IdentityTypeResolver : CSharpTypeResolver`.** It overrides `Resolve(JsonSchema schema, bool isNullable, string? typeNameHint)`, so a property or item whose actual schema is an identity resolves to the identity's name (with `?` when nullable). The identity names join `ExcludedTypeNames`, so NJsonSchema prints nothing of its own for them. `Program.cs` passes the resolver to `new CSharpClientGenerator(document, settings, resolver)`.

Whether NJsonSchema 11.6 lets `Resolve` be overridden this way is the first thing the generator's red test confirms. If it does not, the fallback is a pre-pass that rewrites each identity `$ref` to an `x-` type hint. Both are named here so the review knows which one shipped.

The generated records then read, for example:

```csharp
public partial record NodeRef
{
    [System.Text.Json.Serialization.JsonConstructor]
    public NodeRef(NodeId @id, NodeKind @kind, string @label) { … }

    [System.Text.Json.Serialization.JsonPropertyName("id")]
    public NodeId Id { get; init; }
    …
}

public partial record EdgePage
{
    public System.Collections.Generic.IReadOnlyList<EdgeEntry> Entries { get; init; }
    public EdgeKind Kind { get; init; }
    public EdgePageCursor? Next { get; init; }
    public EdgePageCursor? Previous { get; init; }
    public ArtifactRoot Version { get; init; }
}

public partial record TextWindow
{
    public UnitReference? Next { get; init; }
    public System.Collections.Generic.IReadOnlyList<TextUnit> Units { get; init; }
    public ArtifactRoot Version { get; init; }
}
```

### 4.2 C#: the client's own signatures

| Today | After |
|---|---|
| `IExplorableClient.Card(string id)` | `Card(NodeId id)` |
| `IExplorableClient.Elements(IReadOnlyList<string> ids)` | `Elements(IReadOnlyList<ElementId> ids)` |
| `IExplorableClient.Edges(string positionId, EdgeKind kind, int? cursor = null, int limit = …)` | `Edges(ElementId position, EdgeKind kind, EdgePageCursor? cursor = null, int limit = …)` |
| `IExplorableClient.Reading(string fromRef, int n, WindowDir dir, Corpus corpus)` | `Reading(TextWindowReference from, int n, WindowDir dir, Corpus corpus)` |
| `GraphExplorableClient`: the four above, and `ElementsAt(string asked, int? cursor)` | the same types; `ElementsAt(string asked, ElementPageCursor? cursor)` |
| `AtlasClient.SceneScripture(string sref)` | `SceneScripture(BibleReference passage)` |
| `AtlasClient.Chapter(string book, int chapter)`, `ChapterText(string book, int chapter)`, `KretzmannChapter(string book, int chapter)` | `…(ChapterReference chapter)`; the client composing `{book}.{chapter}` today is a rule-25 offender |
| `AtlasClient.Xrefs(string sref)`, `Catechism(string sref)` | `…(VerseSpanReference span)` |
| `AtlasClient.NodeRecord(string nodeId)` | `NodeRecord(NodeId id)` |
| `PagedReads.FirstPage` (4 uses) | `EdgePageCursor.First` / `ElementPageCursor.First` |

`.Value` on an identity is read only by the transport (`GraphExplorableClient`, `AtlasClient`) when it writes a URL. A client source law, added to the existing `OneDoorLawTests`, refuses `.Value` on an identity anywhere else. Without it, `.Value` would reopen the parsing rule 25 closes.

**The client builds and takes apart node ids by hand in 29 places.** These are `Exploring/Contract/NodeIds.Of` and `NodeIds.LocalPart`, in `Legacy/*.cs` (21), `Pages/Kretzmann.razor`, `Components/MentionScan.razor` (2), `Exploring/KretzmannCitationScan.cs` and `Exploring/EventAccounts.cs`. Once `NodeId` has a private constructor, none of them compiles. That compile failure enumerates the category. Question O3 decides where they go. A rough grep upper bound for C# sites the compiler will walk: `client/` ~300 identity-shaped reads, `client.Tests/` ~350. The exact list is the first build's error list, not a hand count.

This also closes F-31 ("node constructors disagree on local vs wire ids; closure: one typed id on the client"): `NodeId` is that one id.

### 4.3 F#: Codex's `client-fsharp.ContractGenerator` (described here, not touched)

At 7da4d70 the generator already prints every named scalar as a private single-case struct union with a null-refusing converter (`isScalarIdentity`, `Generator.fs:103`). Its `Reads` take parameters typed by their schemas. With this document, with no change on Codex's side, it generates:

- `ArtifactRoot`, `EdgePageCursor`, `ElementPageCursor`, `NodeId`, `EdgeId`, `VerseReference`, `ChapterReference`, `PassageReference`, `VerseRangeReference` and `ConcordReference` as `private | X of string` (or `of int`);
- `Reads.contents (corpus: Corpus)`, `Reads.nodeRecord (id: NodeId)`, `Reads.chapter (cref: ChapterReference)` and so on.

It needs two additions, which are Codex's to make, in Codex's style:

1. **A `oneOf` whose members are all identities becomes an identity.** It takes the members' primitive and gains widening functions, for example `ElementId.ofNodeId : NodeId -> ElementId`, and transitively `TextWindowReference.ofContentsReference`. It has no narrowing function. Today `isScalarIdentity` requires a `type`, so a union falls through to `shape`.
2. **A cursor schema's `default` becomes `EdgePageCursor.first`.**

Optionally, the converters may check the published `pattern` at the JSON door. That is generated validation of the contract, not hand-written domain parsing.

## 5. Every changed signature and site (PRINCIPLES 12)

**graph-types** (crate 0.15.x → PATCH: additive impls only)

- `wire_form.rs`: `impl Serialize, Deserialize, PartialSchema, ToSchema for edge::EdgeId`. Also `node_id_noun` reworded for `EraId`/`PolityId`/`NarrativeId`: "the local name of one Era node, without its kind" (description only, §7 L1d).
- `id.rs`: `ContentHash::HEX_WIDTH`; `from_hex` reads it.

**atlas-core**

- New `identity.rs` (+ `pub mod identity` in `lib.rs`): `wire_identity!`, `wire_union!`, and the leaves and unions of §3.2 except `VerseId`/`ScriptureRef`/`EdgeId`. Also `NodeId::{encoded, asked, ADDRESSABLE, TEXT_UNIT}`, `UnreferencedUnit`, `identity::ALL`, `identity::WIDENINGS`, `identity::schemas()`.
- `refs.rs`: `VerseId` gains its schema (named `VerseReference`). `ScriptureRef` gains `Serialize` and its schema (named `BibleReference`), and its variants become `Chapter(ChapterReference)` and `Passage(PassageReference)`. The 36 match sites in 14 files that destructure them follow.
- `wire.rs`: `NodeRef.id: String → NodeId`; `Scene.r#ref: Option<String> → Option<ScriptureRef>`; `VerseGroup.verses: Vec<String> → Vec<VerseId>`.
- `data.rs`: `PolityDelta.verses: Vec<String> → Vec<VerseId>`. `AtlasData::place_node` (the `SceneSource` impl, `data.rs:838`) builds its id with `format!("{kind:?}:{id}")`, a **second node-id encoder** (§8 F-WI-1). It goes through `NodeId::encoded`.
- `scene.rs`: `Scene { r#ref: Some(r.to_string()) }` becomes `Some(r.clone())`. The `VerseGroup` builders read the event's verse strings with `VerseId::parse_canonical(…).expect("checked when the atlas is compiled")`, the idiom `graph.rs::date_claim` already uses.
- `xrefs.rs`: `AggregatedXref.target: String → CrossReferenceTarget`; `target_span` becomes `CrossReferenceTarget::first_and_last`.

**atlas-graph**

- `node_ref.rs`: `encode_node_id`/`encode_node_ids`/`UnreferencedUnit` move to `atlas_core::identity`; `node_ref()` stays and builds `NodeRef { id: NodeId }`.
- `lib.rs`: `version_hex` retires. Its callers use `ArtifactRoot::of(v)` (wire) and `ArtifactRoot::of(v).to_string()` (`bins/compile_graph.rs` ×6, `atlas-server/src/main.rs:84`).
- `scene_source.rs`: unchanged signatures; `place_nodes_of` already goes through `node_ref`.

**atlas-contract**

- `wire/mod.rs`: `pub use atlas_core::identity::*`.
- `wire/graph.rs`:
  - `NodeRecord.id: NodeId`, `.version: ArtifactRoot`
  - `EdgeRef.id: EdgeId`; `EdgeRecord.id: EdgeId`
  - `Element::Missing { id: ElementId }`
  - `ElementPage.{previous, next}: Option<ElementPageCursor>`, `.version: ArtifactRoot`
  - `EdgePage.{previous, next}: Option<EdgePageCursor>`, `.version: ArtifactRoot`
  - `TextWindow.next: Option<UnitReference>`, `.version: ArtifactRoot`
  - `TextUnit.r#ref: UnitReference`
  - `Element`'s `ToSchema` `MISSING_ELEMENT` part refers to `ElementId`
- `wire/contents.rs`: `Contents.version: ArtifactRoot`; `ContentsRoot.{id: NodeId, r#ref: ContentsReference}`; `ContentsChild.{id: NodeId, r#ref: ContentsReference}`.
- `wire/reading.rs`: `Chapter.r#ref: ChapterReference`; `KretzmannChapter.version: ArtifactRoot`; `KretzmannChapterItem.id: NodeId`; `CrossRef.target: CrossReferenceTarget`.
- `wire/catechism.rs`: `CatechismProofVerse.vref: VerseId`.
- `wire/events.rs`: `EventPage.mentioned_in: Vec<VerseId>`.
- `wire/locus.rs`: `BIBLE.tag`/`CONCORD.tag` from `Corpus`.
- `reference.rs`:
  - `ChapterReference`, `ReadingReference`, `ConcordParagraphReference` (→ `ConcordReference`) and `VerseSpan` (→ `VerseSpanReference`) move to `identity.rs`
  - `ElementId` → `ElementPosition`
  - `AskedElement.asked: ElementId`
  - `Reference<T>`, `NodeReference`, `PositionReference`, `ElementIds` and `decode_element_id` keep their signatures over the renamed types
- `graph_wire.rs`: `decode_node_id` reads `NodeId::ADDRESSABLE` and `NodeId::TEXT_UNIT` instead of its literal `match`. `describe_nodes`/`describe_positions`/`node_ref` follow `NodeId::encoded`. `edge_ref` builds `EdgeRef { id: id.clone() }`.
- `graph.rs`:
  - `node_record`: `params(("id" = NodeId, Path))`
  - `read_node_record`: `id`, `version`
  - `read_edge_record`: `id`
  - `described_as`
  - `read_elements`: `Missing { id: element.asked.clone() }`
  - `elements`: cursors and version; `ElementsQuery`: `#[param(value_type = Vec<ElementId>, …)]` and `#[param(value_type = Option<ElementPageCursor>)]`
  - `node_edges`: `params(("id" = ElementId, Path), EdgePageQuery)`, the page cursors and version; `EdgePageQuery.cursor`: `#[param(value_type = Option<EdgePageCursor>)]`
  - `text_window`: the ETag from `ArtifactRoot`'s `Display`; `TextWindowQuery.r#ref`: `#[param(value_type = TextWindowReference)]`, still read as text at the same point
  - `next_reference`, `references_of`: `-> Option<UnitReference>` / `Vec<UnitReference>`. The compiled reference string parses once, and a failure is `internal`.
  - `text_units`: `TextUnit.r#ref`
- `contents.rs`: `params(("corpus" = Corpus, Path))`. The handler keeps `Path<String>` + `Corpus::named` so an unknown corpus is still `not_found` (`contents-bad-corpus` fixture). `r#ref: format!("{code}.{chapter}")` becomes `ChapterReference { book, chapter }.into()`. Concord refs become `ConcordReference`; `id`s become `NodeId`; version.
- `reading.rs`: `chapter`/`kretzmann_chapter`: `("cref" = ChapterReference, Path)`, `Chapter.r#ref` is the asked `ChapterReference`; `KretzmannChapterItem.id`; version; `xrefs`: `("sref" = VerseSpanReference, Path)`.
- `catechism.rs`: `catechism_for_span`: `("sref" = VerseSpanReference, Path)`; `vref` from the item's verse string via `VerseId::parse_canonical`.
- `events.rs`: `mentioned_in` built through `kjv_adapter::decode_text_unit` → `VerseId`. Today it does `id.raw.strip_prefix("bible/")` and splits by hand (§8 F-WI-3). The site has to be migrated to produce the type.
- `map.rs`: `ScripturePassage.r#ref`: `#[param(value_type = ScriptureRef)]`.
- `document.rs`: registers `identity::schemas()`, so an identity named only by a parameter (`ChapterReference`, `VerseSpanReference`, `TextWindowReference`, `ElementId`) is in `components`. The existing law `every_reference_in_the_published_document_resolves` catches a miss.
- `bins/export_aqc_examples.rs`, `aqc_export.rs`: follow the `decode_node_id` signature (unchanged) and the moved encoder.

**atlas-cli**: `commands/{node,edges,find,verse}.rs` follow the moved encoder/decoder (12 call sites). The CLI pact output (`bibex node`, `bibex edges`) serializes the same `NodeRecord`/`EdgePage`, so its bytes are unchanged too.

**C#**: `client.ContractGenerator/{ContractGeneration.cs, Program.cs}` plus the new `IdentityTypes.cs` and `IdentityTypeResolver.cs`; the §4.2 signatures and the sites the compiler lists; `client.Tests`, `client.ContractTests` (§6).

**Contracts** (regenerated under the `contract` lock, never hand-edited): `contracts/openapi.yaml`, `contracts/atlas-query-contract/aqc.schema.json`, `atlas-query-contract/VERSION`, `atlas-query-contract/CHANGELOG.md`. Nothing else under `contracts/` changes.

## 6. Proof that no wire byte changes, and the version class

**Why the bytes cannot move.** Every identity serializes through the value it replaces:

| Identity | Before | After |
|---|---|---|
| `ArtifactRoot` | `version_hex(v)` = `v.0.hex()` | `collect_str` of the same `hex()` |
| cursors | the `usize` | `serialize_u64` of the same `usize` |
| `NodeId` | `encode_node_ids` | the same function, moved |
| `EdgeId` | `id.0` | `serialize_str(&id.0)` |
| `VerseId` | `format!("{}.{}.{}")` | the same (`VerseId`'s serializer is unchanged) |
| `ScriptureRef` | `r.to_string()` | `collect_str` of the same `Display` |
| `ChapterReference` | `format!("{code}.{chapter}")` | its `Display` writes the same |
| unions | — | serialize as their member |

Two places parse a stored string where the server copied it through before: `VerseGroup`/`PolityDelta`/`vref` from data, and `CrossRef.target`. A stored value that is not already canonical (for example a lower-case book code) would come back canonical, and its bytes would move. The byte gates below fail on any such value. If one fails, the implementation stops and reports it. It does not re-bless.

**The gates that prove it, all existing:**

- `atlas-contract/tests/contract_pact.rs` replays `contracts/pacts/http.json` (18 routes plus the vocabulary) and `cli.json` (2 commands) unchanged.
- `regenerated_aqc_corpus.rs` re-exports every AQC fixture through `export_aqc_examples` and compares bytes.
- `scene_byte_identity.rs`.
- `client.ContractTests/WireFixtureTests` (extended in L3c).
- `git diff --name-only <base>..HEAD -- contracts/` lists exactly the four files at the end of §5.

**What `contract-gate` reports:**

- legs 4 and 6: every expectation replays, against unchanged pacts;
- leg 1b: the two regenerated documents are current;
- leg 5, AGC: "unchanged in this range -- nothing to classify";
- leg 5, AQC: "diff requires patch". No `MINOR:`/`MAJOR:` line appears, because no fixture, feature or expectation line changed and `aqc.schema.json` is the only file that moved. The script grades that as `patch` (`contract-semver-gate.sh`, `required=patch` when nothing else fires).

That is the document-only signature.

**Version class**, by the CHANGELOG's 0.x convention ("MAJOR class, MINOR bump under the 0.x rule"; "the additive field would have been PATCH alone", 0.6.0):

- **AQC: MINOR class (guarantees added), PATCH bump under the 0.x rule.** The document gains named shapes, patterns that describe what is already served, and an enum on `{corpus}`, whose other values the server already answered `not_found`. No conforming request or response is excluded, and no schema is renamed or removed. The bump is `0.N.0 → 0.N.1` on whatever A-PROVENANCE leaves (expected 0.29.0 → 0.29.1, §9). The gate reads a declared PATCH as MINOR, which covers the PATCH it requires. Over-declaring stays allowed.
- **Disclosed in the entry:** the generated C# and F# signatures change from `string`/`int` to the named types. That is a source-level change for generated code, not a wire change.
- **AGC: no bump.** `graph-vocabulary.json`, its fixtures and its features are untouched.
- **graph-types: PATCH** (additive impls, precedent 0.2.1).

## 7. Laws, each red first

Every law below is written before the type it names exists. Its red is the failing or non-compiling run, recorded in the task report. Whole-value assertions follow rules 15 and 17.

**L1. Distinct identities do not unify.**

- **L1a (Rust, wire disjointness).** `node_and_edge_ids_never_share_a_prefix`: `NodeId::ADDRESSABLE` names plus `text-unit`, intersected with `relation_names()`, is empty. And every value in `identity::ALL` matches **exactly one** leaf pattern among its union's members, over a generated sample (proptest, already a dev-dependency of atlas-core) of every leaf. Red: the names do not exist yet.
- **L1b (Rust, the widening graph).** `the_identities_widen_only_as_declared`: `identity::WIDENINGS` equals the whole expected table (member → union, including the transitive union-to-union rows), and each union's `oneOf` in `document::openapi()` lists exactly its declared members. Red: no table.
- **L1c (Rust, each site's type).** `every_identity_site_has_its_own_type`: a whole-value map from site (`"NodeRef.id"`, `"EdgeRef.id"`, `"EdgePage.next"`, …) to `std::any::type_name` of the field's type. It also pins the Rust-name → schema-name map (`VerseId → VerseReference`, `ScriptureRef → BibleReference`, …). Red today: every entry reads `alloc::string::String` or `usize`.
- **L1d (Rust, local names vs node ids).** Every `NodeId<K>` schema (`EraId`, `PolityId`, `NarrativeId`) has a description saying it is a local name without its kind. No served value of one of those matches `NodeId`'s pattern, and no served `NodeId` matches theirs. Red: today's description says "The id of one Era node".
- **L1e (C#, reflection over the generated assembly).** `Identities_convert_only_along_the_documents_unions`: over every type in `BibleAtlas.Client.Contract` that `IdentityTypes` emitted, the set of `op_Implicit` (from → to) equals the set derived from the document's `oneOf` lists. No identity has a public constructor, and `typeof(X).IsAssignableFrom(typeof(Y))` is false for every distinct pair (the whole matrix). Red: the types do not exist.
- **F#: Codex's own law** (C1 fix 1 already asks for one).

**L2. Every identity site in the document uses its named schema (24b, enumerating the document).**

- **L2a (inventory).** `every_identity_site_names_its_schema`: walking every operation's parameters and `200` response from `document::openapi()` yields a whole-value map from schema name to sorted `(operationId, location, JSON path)` lists. That map equals §2.1's tables: 7 + 6 + 28 + 5 + 3 + 25 entries, plus the `{corpus}` parameter. Red: today the map is empty.
- **L2b (closure).** `no_unnamed_identity_can_be_published`: every inline `type: string` or `type: integer` in the document with no `enum`, `$ref` or `pattern` is in one of two pinned lists. The **prose** list holds labels, titles, text, notes, descriptions, colours and counts. The **named-later** list holds the §8 F-WI-4 sites (28 local-name ids, 3 book codes, the provenance ids A-PROVENANCE reshapes). The named-later list is a ratchet: it may only shrink, the mechanism `client.Tests/SourceRatchet.cs` uses. A new identity published as a raw string fails until it is named or argued into the prose list in review. Red: today's 41 identity schema sites are in neither list.
- **L2c (corpus).** The `{corpus}` parameter's schema is `$ref: Corpus`, and every `TextRef` discriminator's `enum` equals `Corpus`'s values. Red: `type: string`, and literal tags.
- **L2d (one door, Rust source law).** `NodeId::asked` and `ElementId`'s request constructor are called only from `reference.rs`. `NodeId::encoded` is the only other constructor. The same holds for `ArtifactRoot::of` and the cursors in `graph.rs`. This is in the style of `no_served_label_composition.rs`. Red: the constructors do not exist.
- **L2e (served values).** Every value at every L2a path, in every committed fixture and pact, matches its schema's pattern, checked against `aqc.schema.json`. Red: no patterns.

**L3. Round trips preserve bytes.**

- **L3a (Rust property).** For each leaf, over generated inner values: `serde_json::to_string(&Identity::from(inner)) == serde_json::to_string(&before(inner))`, where `before` is the expression the site uses today (`version_hex`, `.0`, `format!`, `to_string`). Also `parse(display(x)) == x` for every leaf that parses. Red: the identities do not exist.
- **L3b (Rust, the whole corpus).** The existing byte gates (§6) stay green unchanged. That is the proof, not new red.
- **L3c (C#).** `WireFixtureTests.A_recorded_server_body_round_trips_through_the_generated_record_unchanged` reads its cases from the committed fixture indexes (`atlas-query-contract/fixtures/index.json` and the AGC fixtures), not from five hand-written `InlineData` rows. So every fixture with a typed body deserializes into the generated record and re-serializes to the same canonical JSON. Red: the identity converters do not exist, so the theory cannot compile, and today's five rows do not cover the 20 other recorded bodies.
- **F#: Codex's generated round-trip law** covers its side (the 7da4d70 handoff reports one over every published identity).

## 8. Findings this design records (category plus sites; the owner decides)

- **F-WI-1, a second node-id encoder.** `atlas-core/src/data.rs:838` (`AtlasData::place_node`) writes `format!("{kind:?}:{id}")` beside `encode_node_ids`. The NodeId change routes it through the one encoder, which is what makes the duplicate visible.
- **F-WI-2, the decodable node kinds are a hand list.** `graph_wire::decode_node_id` lists 13 kind names and omits `Source` and `PeopleGroup`, while `encode` writes every kind. A served id of either kind would not be accepted back. This design makes the list one table (`NodeId::ADDRESSABLE`) and keeps its contents. Whether those two kinds should be addressable is a behaviour change for later.
- **F-WI-3, server-side string parsing of a node's raw id.** `events.rs:132` reads `id.raw.strip_prefix("bible/")` and splits it by hand instead of `kjv_adapter::decode_text_unit`. The site is migrated here because it must produce a `VerseId`.
- **F-WI-4, unnamed identities outside this item's five kinds.** They are pinned by L2b's shrinking list:
  - 25 properties and 3 path parameters that carry a kind's local name (`ur-1`, `ab_ur`, `commandment-1`, `abraham-migration`):
    - `CatechismRef.id`, `CatechismItem.id`, path `/api/catechism/item/{id}`
    - `EventPage.id`, `EventAnalogue.id`, path `/api/event/{id}`, path `/api/narrative/event/{id}`
    - `Heading.event_id`
    - `NarrativePosition.{narrative_id, event_id}`, `NarrativeAdjacentEvent.{id, places[]}`
    - `Narrative.{id, legs[]}`
    - `PlaceRef.id`, `PersonRef.id`
    - `QuietPlace.{id, merged_ids[]}`, `ScenePlace.{id, merged_ids[]}`
    - `SceneEvent.id`, `SceneArrow.{from_event, to_event, from_place, to_place, narrative}`, `SceneNarrative.id`
    - `PolityDelta.event`
  - 3 book codes published as plain text, not `BookId`: `CanonBook.code`, `VerseGroup.book`, `EventWitness.book`.
  - The provenance ids, which A-PROVENANCE reshapes to `{ id, title }`.

  graph-types already has a typed local id for most of these (`PlaceId`, `PersonId`, `EventId`, `CatechismItemId`). Question O4.
- **F-WI-5, references whose shape depends on the `corpus` the caller sent** (`TextWindowReference`, `UnitReference`, `ContentsReference`). Closing it means splitting `Contents`/`TextWindow` per corpus: same bytes, MAJOR-class document change. Proposed as a follow-on.
- **F-WI-6, the client composes Bible references and node ids.** `AtlasClient.Chapter/ChapterText/KretzmannChapter` build `{book}.{chapter}`, and `NodeIds` has 29 sites. These are rule-25 offenders that the types turn into compile errors. Question O3.

## 9. Order against the queue

- **This item stacks after A-F39 and A-PROVENANCE.** All three regenerate `contracts/openapi.yaml` and `aqc.schema.json` and bump AQC. A-F39 (claimed) moves the version root and re-records every root-carrying fixture. A-PROVENANCE (O-PROVENANCE approved) changes every provenance field's shape: MAJOR class, and it re-records fixtures. Landing this first would make both of them regenerate the documents over it, and would leave its byte-identity proof stranded on a superseded root.
- **The base** is the trunk commit after the later of those two lands. It is written down at the claim (rule 22). The documents are regenerated holding the `contract` lock.
- **The pacts and fixtures it must leave byte-identical** are whatever those two items re-blessed. This item re-blesses none.
- **Hand-offs:** after A-PROVENANCE lands, its provenance id joins L2b's named-later ratchet, ready for O4's follow-on. After this item lands, Codex consumes the regenerated document in CX-FSHARP (§4.3's two generator additions). The queue's own C1 note says C1's identities "guard nothing the client uses until the owner rules on A-WIRE-IDENTITIES".

## 10. Owner questions

**O1. How finely should references be named?** The operations use five Bible shapes: book, chapter, verse, a run of verses within a chapter, and a run across chapters. They use one Concord shape. Different reads accept different mixes of them. This design gives each shape its own name and each accepted mix its own name: 6 reference shapes, including `BookId`, and 7 named mixes. A mistake like passing a verse where only a chapter is accepted then fails to compile. The alternative is just two names, `BibleReference` and `ConcordReference`. That is simpler, but a chapter-only read would accept any Bible reference again. **Recommendation: name every shape and every mix, as designed.**

**O2. One page-cursor type, or one per read?** A neighbour page's cursor and an element read's cursor are different counters. One counts edges, the other counts the ids you asked for. With one shared `PageCursor`, either can be passed to the other read and quietly fetch the wrong page. **Recommendation: two types, `EdgePageCursor` and `ElementPageCursor`.**

**O3. Where do the C# client's 29 hand-built node ids go?** About 21 of the 29 are in `Legacy/`. Once a node id is a closed type, none of them compiles. **Option (a):** keep one fenced door, `LegacyNodeIds`, the only code allowed to make a node id from text. A ratchet lets its count only fall, until FOCUS retires those views. **Option (b):** remove all 29 now by having the server serve the ids those views need, which widens this item into several routes. **Recommendation: (a)**, so this item stays a type change and the removal rides with the FOCUS work already planned.

**O4. Name the other 31 unnamed ids here, or next?** Older routes have 28 fields and path segments that carry a kind's short local name (`ur-1`, `ab_ur`, `commandment-1`). Three more carry a book code as plain text. graph-types already has a type for most of them. **Recommendation: a follow-on item, A-WIRE-LOCAL-IDS, queued right after this one.** This design pins all 31 in a list that can only shrink, so none can grow meanwhile. Most of those routes are ones FOCUS is retiring.
