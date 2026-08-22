# The Explorable Graph — types and interfaces (review artifact)

Companion to 2026-08-22-explorable-graph-design.md. The rule for every
type here: it either makes an illegal state unrepresentable, or it names
the law a fail-loud validator enforces. Conceptual algebra is stated in
Haskell (the BibleGraph.hs idiom); realized types in Rust (server/ETL);
the client contract in C# at the end. Nothing here is implementation —
field names and exact widths are illustrative; the SHAPES are the design.

## 0. Conceptual spine (Haskell)

```haskell
-- Exploration is bind over heterogeneous nodes (BibleGraph.hs, kept):
class Explorable e where
  explore :: e a -> (a -> [e b]) -> [e b]

-- The graph is a family of typed edge relations over typed nodes:
--   edges  :: EdgeKind k => Node -> [Target k]
-- Presentation is a selection:  Surface = Set EdgeKind × DisplayRule
-- Honesty is totality: an affordance exists iff the relation is inhabited.
```

## 1. Identity and text addressing

```rust
/// Kind-tagged identity: a NodeId knows what sort of thing it names, so a
/// cross-kind reference is a type error, not a runtime surprise.
pub struct NodeId<K: NodeKindTag>(Interned, PhantomData<K>);
pub type EventId     = NodeId<EventTag>;
pub type ContainerId = NodeId<ContainerTag>;
pub type PlaceId     = NodeId<PlaceTag>;
pub type PersonId    = NodeId<PersonTag>;
pub type AnchorId    = NodeId<AnchorTag>;
pub type NarrativeId = NodeId<NarrativeTag>;
pub type SourceId    = NodeId<SourceTag>;
/// One erased form exists for the wire/UI boundary only:
pub struct AnyNodeId { kind: NodeKind, raw: Interned }

/// LAW (owner directive): a verse is not an atom. Every reference to text
/// is a locus that can already address below the verse; sub-verse nodes
/// materialize later WITHOUT changing this type or anything built on it.
pub struct VerseRef { pub book: BookId, pub chapter: u16, pub verse: u16 }

/// Token indices into the verse's canonical tokenization (rules deferred;
/// the address space is reserved now). None = the whole verse.
pub struct TokenSpan { pub start: u16, pub end: u16 } // start <= end, validated

pub struct TextLocus { pub verse: VerseRef, pub span: Option<TokenSpan> }

/// Container content is a SET of loci (container algebra: ∅ is lawful
/// identity; overlaps between containers are lawful; verses immutable).
/// Contiguity is NOT assumed — multi-chapter and gapped containers are
/// first-class, which retires the same-chapter parse limitation class.
pub struct LocusSet(BTreeSet<TextLocus>);
```

## 2. Nodes

```rust
/// Closed enumeration TODAY; extending it is a deliberate act that the
/// compiler forces every match site to acknowledge (P5: a new node kind
/// is a record + compiler rule + display rule — the exhaustive match IS
/// the checklist).
pub enum NodeKind { Book, Chapter, Verse, Container, Event, Narrative,
                    Place, Person, Anchor, Era, Polity, CatechismItem, Source }

pub struct Node { pub id: AnyNodeId, pub payload: NodePayload }

pub enum NodePayload {
    Verse    { text: KjvText },                  // the ONLY store of scripture (P1)
    Container{ title: DecisiveTitle, content: LocusSet, provenance: SourceRef },
    Event    { label: Label },                   // dates/places live on EDGES
    Narrative{ label: Label },                   // membership lives on succession edges
    Place    { canonical: Label },               // names live on `named` edges
    Anchor   { year: Year, citation: Citation }, // explorable "why this date?"
    // ... Person, Era, Polity, CatechismItem, Source, Book, Chapter
}

/// Signed year, no year zero — the invariant is in the constructor, not
/// in every consumer.
pub struct Year(NonZeroI32);
```

## 3. Edges — per-kind typed tables

The single most load-bearing decision: the graph is NOT one `Vec<Edge>`
with stringly kinds. It is a struct of per-kind tables whose endpoint
types make illegal edges unrepresentable, unified only at the query
boundary. The tables ARE the indexes ("the data structure does the work").

```rust
pub struct Graph {
    pub nodes:       NodeTable,
    // -------- authored relations (each row carries provenance) --------
    pub contains:    Table<Contains>,     // Container → LocusSet entries
    pub witnesses:   Table<Witnesses>,    // Event → witness locus-range per translation
    pub succession:  Table<Succession>,   // doubly-linked, per narrative
    pub dated_by:    Table<DatedBy>,      // Event → chronology placement
    pub located_at:  Table<LocatedAt>,    // Event → Place
    pub catechism:   Table<CatechismLink>,// TextLocus ↔ CatechismItem
    pub named:       Table<Named>,        // Place → alias (KJV naming)
    // -------- imported relations --------
    pub mentions:    Table<Mentions>,     // TextLocus → Place | Person
    pub cross_refs:  Table<CrossRef>,     // TextLocus → TextLocus, votes-ranked
    // -------- derived relations (compiler output; no authored rows) ----
    pub reading:     ReadingOrder,        // total order on materialized TextSpans
    pub temporal:    TemporalAdjacency,   // ONLY where resolved dates differ
    pub member_of:   MemberIndex,         // inverse of contains/witnesses
    pub parallels:   ParallelIndex,       // via co-witnessing one Event
}

/// A typed authored edge; the pattern for all of them:
pub struct Contains {
    pub container: ContainerId,
    pub content:   LocusSet,
    pub provenance: ProvenanceId,
}

/// LAW (witness canon): parallel accounts are witnesses on ONE event —
/// so `parallel` is DERIVED from co-witnessing and cannot disagree with it.
pub struct Witnesses {
    pub event: EventId,
    pub witness: LocusRange,              // clamped, translation-keyed text
    pub provenance: ProvenanceId,
}

/// LAW (owner): "a narrative is a doubly linked list of events."
/// The list is stored as the list — prev/next are derived views of
/// position, so a malformed chain (orphan, fork, cycle) cannot exist as
/// data; it can only fail to construct.
pub struct Succession {
    pub narrative: NarrativeId,
    pub chain: Vec<EventId>,              // ≥1, validated distinct
    pub provenance: ProvenanceId,
}
```

## 4. Chronology — placement, resolution, and honest derivation

```rust
/// LAW (single-feed): no event carries a year literal. Its date is a
/// PLACEMENT that resolves against the anchor table at compile time.
pub enum DatePlacement {
    AnchorBinding { anchor: AnchorId, offset_years: i32 },
    ReignYear     { reign: AnchorId, year_of_reign: u8 },
    SequenceAfter { prior: EventId, spacing_years: u8 },   // chain interiors
    EraOnly       { era: EraId },                          // era-precision truth
}
pub struct DatedBy { pub event: EventId, pub placement: DatePlacement,
                     pub provenance: ProvenanceId }

pub struct ResolvedDate { pub from: Year, pub to: Year }   // from <= to

/// THE HONESTY SIGNATURE. Temporal order between two events exists only
/// when their resolved dates actually distinguish them. Equal or
/// overlapping-at-year-precision ⇒ None ⇒ NO temporal edge exists.
/// The David's-death lie is unrepresentable — not caught by audit,
/// impossible to store.
pub fn temporal_order(a: ResolvedDate, b: ResolvedDate) -> Option<Ordering>;

/// Same-year events with no succession edge form an honest antichain;
/// the traversal surface receives them AS a cluster:
pub enum TimeNeighbor {
    Succession { narrative: NarrativeId, event: EventId }, // authored truth
    Temporal   { event: EventId },                         // derived, distinct styling
    SameTime   { events: Vec<EventId> },                   // no order asserted
}
```

## 5. Ingestion — sources, assertions, adapters, laws

```rust
/// Uniform provenance envelope on every fact from every source (P5).
pub struct Provenance {
    pub source: SourceId,                  // KJV text, Ussher, Theographic, curated file, ...
    pub locator: Locator,                  // file+key / paragraph / URL
    pub confidence: Confidence,
}
pub enum Confidence { CanonicalText, Curated, Imported, Derived }

pub struct Asserted<T> { pub fact: T, pub provenance: Provenance }

/// The source-agnostic boundary: an adapter turns ONE source's bytes into
/// typed assertions. TOML is one impl; JSON another; a future database a
/// third. NOTHING downstream knows the difference.
pub trait SourceAdapter {
    fn source(&self) -> SourceId;
    fn emit(&self, input: &SourceBytes) -> Result<Vec<Asserted<Assertion>>, AdapterError>;
}

pub enum Assertion {
    Verse(VerseAssertion),                 // canonical text — KJV adapter only
    Container(ContainerAssertion),
    Event(EventAssertion),
    Witness(WitnessAssertion),
    SuccessionChain(SuccessionAssertion),
    Anchor(AnchorAssertion),
    Placement(DatedByAssertion),
    Mention(MentionAssertion),
    CrossRef(CrossRefAssertion),
    // ... Place, Named, CatechismLink, Era, Polity, Person, Merge, Exemption
}

/// LAW (P3, owner: "the data structure itself should prove this"): each
/// adapter ships boundary laws checked at compile time, fail-loud. For
/// the KJV adapter: bijection (every source verse ↔ exactly one Verse
/// assertion) and reconstruction (the reading-order walk over the built
/// graph reproduces the source text byte-for-byte). After the boundary,
/// scripture exists in ONE place; drift has nowhere to live.
pub trait BoundaryLaw {
    fn check(&self, source: &SourceBytes, built: &Graph) -> Result<(), LawViolation>;
}

/// Exemptions are typed, reasoned, and enumerable — never silent:
pub enum ExemptionKind { Recounting, DeferredMigration }
pub struct Exemption { pub kind: ExemptionKind, pub subject: AnyNodeId,
                       pub reason: String, pub provenance: ProvenanceId }
```

Compiler pipeline (each pass a total function; order fixed):
`normalize → merge/alias → resolve(DatePlacement) → derive(reading, temporal,
member_of, parallels) → index → law-check (boundary laws, E1–E4 properties,
per-kind invariants)`.

## 6. The Explorable runtime contract

```rust
/// What every node kind implements; the ONLY thing surfaces consume.
pub trait Explorable {
    fn card(&self, g: &Graph) -> Card;                       // identity + decisive label + provenance
    fn edge_summary(&self, g: &Graph) -> EdgeSummary;        // kind → count (cheap; popover opener)
    fn edges(&self, g: &Graph, q: EdgeQuery) -> EdgePage;    // one kind, one page
}

pub struct EdgeQuery { pub kind: EdgeKind, pub cursor: Option<Cursor>, pub limit: u16 }
pub struct EdgePage  { pub kind: EdgeKind, pub entries: Vec<EdgeEntry>, pub next: Option<Cursor> }
pub struct EdgeEntry { pub node: CardSummary, pub meta: EdgeMeta }   // meta: e.g. narrative tag, vote rank
pub struct Cursor(Opaque);            // stable within one graph version
pub struct GraphVersion(Hash);        // one stamp invalidates all caches

/// The owner's bind, recovered: neighbors then flat_map onward.
///   explore n f  ≡  edges(n).entries.flat_map(f)
/// — paging composes under bind because Cursor is positional in a fixed
/// per-kind order (deterministic graph ⇒ deterministic pages).

/// The reader is not special — it is this query:
pub fn reading_window(g: &Graph, from: TextLocus, n: u16, dir: Direction) -> Window;
/// LAW: for any partition into windows, concatenation is invariant
/// (lazy loading can never change what the Bible says).
```

Wire (two endpoints total; all existing routes become views then retire):
```
GET /api/node/{id}                      → Card + EdgeSummary   (+ GraphVersion)
GET /api/node/{id}/edges?kind&cursor&n  → EdgePage
GET /api/text?from=<locus>&n=&dir=      → Window               (reader spine)
```

## 7. Surfaces = selections (presentation discipline as data)

```rust
pub struct SurfaceSpec { pub sections: Vec<SectionSpec> }
pub struct SectionSpec {
    pub kind: EdgeKind,
    pub style: SectionStyle,        // standard | quiet | superscript-marker ...
    pub initial: u8,                // clamp; honest "+N more" beyond it
    pub order: SectionOrder,        // votes-ranked | chain | canonical | resolved-date
}
/// Affordance honesty is now a totality property: the renderer can only
/// draw entries returned by `edges`; a section renders iff count > 0.
```

## 8. Client contract (C#, Blazor)

```csharp
// One popover machinery (owner decree), typed by edge kind:
interface IExplorableClient {
    Task<NodeCard>  Card(AnyNodeId id);
    Task<EdgePage>  Edges(AnyNodeId id, EdgeKind kind, Cursor? cursor, int limit);
    Task<TextWindow> Reading(TextLocus from, int n, Direction dir);
}
// Section registry: EdgeKind → renderer + SectionSpec. Registering a kind
// is the ONLY act needed to surface a new relation anywhere popovers open.
IReadOnlyDictionary<EdgeKind, ISectionRenderer> Registry { get; }
```

## 9. What these types make impossible (the review checklist)

- A traversal affordance with nothing behind it (edges are the only render source).
- A temporal edge between same-year events (`temporal_order` returns None).
- A malformed narrative chain (chains are stored as lists, not stitched pointers).
- A second copy of scripture (Verse payload is the only text; views query it).
- An event with a year literal (DatePlacement is the only way to be dated).
- A cross-kind id confusion (NodeId<K> phantom types).
- A fact without provenance (`Asserted<T>` is the only ingestion currency).
- A silent exemption (typed kind + reason + subject, enumerable).
- A verse-atom assumption (every text endpoint is a TextLocus).

## 10. Open type questions (flagged, not hidden)

- `Interned` and store format (implementation planning).
- TokenSpan semantics await the tokenization rules (deferred by directive).
- Whether `SameTime` clusters paginate (large same-year antichains in dense eras).
- EdgeMeta variants per kind (finalized per surface during M-batches).
