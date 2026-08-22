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
-- Every relation R ⊆ S×O is one stored set viewed both ways: R and its
-- transpose Rᵀ are the SAME rows. dual is a total involution on kinds:
--   dual :: EdgeKind -> EdgeKind ;  dual . dual = id
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

/// LAW (owner directive, 2026-08-22): the base text abstraction is "a
/// more basic wrapper around strings" — NOT Bible-shaped. Text lives in
/// CORPORA; the KJV is one corpus whose addressing scheme is VerseRef.
/// Extrabiblical corpora (the Book of Concord first — the Bente/Dau
/// catechism data is already its seed) join as new corpora with their own
/// structural schemes, and cross-corpus connections become ordinary edges.
pub struct CorpusId(Interned);            // "kjv", "concord-bente-dau", ...

/// Per-corpus structural addressing: each corpus defines its typed
/// position scheme, its own total reading order, and its citation format.
/// A corpus is its own reading spine — there is no cross-corpus
/// reading-order, only cross-corpus EDGES.
pub trait CorpusScheme {
    type Ref: Ord;                        // VerseRef for KJV; Part/Article/Paragraph for Concord
    fn cite(&self, r: &Self::Ref) -> Citation;
}

/// Closed enum today; a new corpus is a deliberate variant (same
/// philosophy as NodeKind — every match site acknowledges it).
pub enum TextRef {
    Kjv(VerseRef),
    Concord(ConcordRef),                  // RESERVED: materializes when ingested
}

pub struct VerseRef { pub book: BookId, pub chapter: u16, pub verse: u16 }

/// Token indices into the unit's canonical tokenization (rules deferred;
/// the address space is reserved now). None = the whole unit. A verse —
/// or a Concord paragraph — is not an atom; sub-unit nodes materialize
/// later WITHOUT changing this type or anything built on it.
pub struct TokenSpan { pub start: u16, pub end: u16 } // start <= end, validated

/// The general text address: any corpus, any unit, optional sub-span.
pub struct TextLocus { pub at: TextRef, pub span: Option<TokenSpan> }

/// REFINEMENT where LAW demands Scripture specifically. The witness canon
/// ("the compiled KJV text is the canon of witnesses") is a type, not a
/// convention: `witnesses` accepts KjvLocus, so an extrabiblical witness
/// is a compile error, while `mentions`/future citation edges accept the
/// general TextLocus and may cross corpora freely.
pub struct KjvLocus { pub verse: VerseRef, pub span: Option<TokenSpan> }
// impl From<KjvLocus> for TextLocus — widening is free; narrowing is a
// checked parse.

/// LAW (owner, 2026-08-22): refs are RECURSIVE-BY-RELATION, and the Book
/// of Concord must never be confusable with Scripture. Both are enforced
/// at the corpus registry, not per assertion:
/// - RECURSION: addresses stay FLAT (corpus, unit, span); recursive
///   structure is relational — a Concord span QUOTES a KJV range via an
///   edge, and spans inside quoted regions bear onward edges of their
///   own. No nested addressing scheme exists; the graph carries the
///   recursion, explore's bind walks it.
/// - QUOTATION IS AN EDGE, NOT TRANSCLUSION: each corpus's text is its
///   own as published (Bente/Dau's quotation wording is Bente/Dau's
///   text, subject to Concord's own byte-fidelity boundary law). The KJV
///   corpus remains the only store OF the KJV; a quotation never
///   promotes text into Scripture.
/// - THE NORMA DISTINCTION AS A TYPE: corpus roles are asymmetric —
///   Scripture is the norming norm, the confessions the normed norm —
///   and edge legality + confidence derive from the ROLE, so
///   "CanonicalText Concord" or "Scripture witnessed by Concord" are
///   unrepresentable, not merely forbidden.
pub enum CorpusRole {
    NormaNormans,          // Scripture (kjv) — the norming norm; witness canon lives here
    NormaNormata,          // the confessions (Book of Concord) — normed BY Scripture
    Reference,             // Ussher, Robertson, Crockett, Theographic, ...
}
pub struct CorpusMeta {
    pub id: CorpusId,
    pub role: CorpusRole,
    // Confidence for a corpus's TEXT derives from its role — it is NOT
    // per-assertion: CanonicalText iff NormaNormans. Nothing an adapter
    // emits can claim canonical standing for extrabiblical text.
}

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
    TextUnit { corpus: CorpusId, text: String }, // the ONLY store of ANY corpus text (P1);
                                                 // KJV verses are TextUnits of the kjv corpus
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

## 3. Edges — per-kind typed tables, bidirectional by construction

The single most load-bearing decision: the graph is NOT one `Vec<Edge>`
with stringly kinds. It is a struct of per-kind tables whose endpoint
types make illegal edges unrepresentable, unified only at the query
boundary. The tables ARE the indexes ("the data structure does the work").

LAW (owner, 2026-08-22): "quotations are bijective. if something is
cited, the thing which is cited is CITED BY the thing which cites...
that node to node bijective correspondence must be baked into the
types." Baked in as follows: every relation is stored ONCE; its forward
and inverse readings are PROJECTIONS of the same rows, sharing one
EdgeId — the bijection's literal witness. An inverse view that disagrees
with its forward view is unrepresentable because neither is data.

```rust
/// Row identity: the SAME id reached from either end. Traversing
/// forward and then asking the target for the inverse entry returns an
/// entry carrying this same EdgeId.
pub struct EdgeId(Interned);

/// EdgeKind enumerates BOTH readings of every relation; dual is a total
/// involution, exhaustively matched (dual(dual(k)) == k is a property
/// test over all variants):
pub enum EdgeKind {
    Contains,   MemberOf,       // dual pair (derived member_of retired as a
                                // separate index — it IS the inverse view)
    Witnesses,  WitnessedBy,    // dual pair
    Quotes,     QuotedBy,       // dual pair
    Cites,      CitedBy,        // dual pair (cross-refs)
    Mentions,   MentionedIn,    // dual pair
    LocatedAt,  SiteOf,         // dual pair
    DatedBy,    Dates,          // dual pair (an Anchor explores its events)
    Named,      NameOf,         // dual pair
    CatechismLink,              // self-dual (symmetric)
    Parallel,                   // self-dual (symmetric, derived)
    TemporalAdjacency,          // self-dual (symmetric, derived)
    FollowsIn,  PrecedesIn,     // succession's two readings, per narrative
}
pub fn dual(k: EdgeKind) -> EdgeKind;   // total; involution

/// A relation declares its stored row, its endpoints, and its dual pair:
pub trait Relation {
    type Row;
    const FORWARD: EdgeKind;
    const INVERSE: EdgeKind;            // == dual(FORWARD), asserted
    fn endpoints(r: &Self::Row) -> (AnyNodeId, AnyNodeId);
}

/// Built, never authored: both adjacency maps from one row table.
pub struct BiIndex { /* fwd: subject → [(EdgeId, object)], inv: object → [(EdgeId, subject)] */ }
impl BiIndex { pub fn build<R: Relation>(rows: &Table<R::Row>) -> BiIndex { /* one pass */ } }
```

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
    pub quotes:      Table<Quotes>,       // span-level quotation toward the norming norm
    // -------- derived relations (compiler output; no authored rows) ----
    pub reading:     PerCorpusReadingOrder, // one total order PER corpus; no cross-corpus spine
    pub temporal:    TemporalAdjacency,   // ONLY where resolved dates differ
    // member_of needs no separate index — MemberOf IS the inverse
    // projection of contains/witnesses via their BiIndexes
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
    pub witness: KjvLocusRange,           // Scripture-typed: the witness canon as a type
    pub provenance: ProvenanceId,
}

/// LAW (norma distinction): quotation points TOWARD the norming norm —
/// the subject is any corpus's span, the object is Scripture. The
/// reverse direction has no constructor; wording divergence between the
/// quoting corpus and the KJV is expected and carried as data, never
/// "corrected" in either corpus (both fidelity laws stand).
pub struct Quotes {
    pub quoting: TextLocus,               // e.g. a span within an SD paragraph
    pub quoted:  KjvLocusRange,           // Scripture, by refinement
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
    TextUnit(TextUnitAssertion),           // corpus text — KJV adapter today; Concord later
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
pub struct EdgeEntry { pub edge: EdgeId, pub node: CardSummary, pub meta: EdgeMeta }
// EdgeId is the bijection witness: the same id appears in the entry
// whichever end you query from. edge_summary() and edges() accept
// inverse kinds identically — a Verse pages its QuotedBy/CitedBy/
// MentionedIn exactly as a Concord span pages its Quotes.
pub struct Cursor(Opaque);            // stable within one graph version
pub struct GraphVersion(Hash);        // one stamp invalidates all caches

/// The owner's bind, recovered: neighbors then flat_map onward.
///   explore n f  ≡  edges(n).entries.flat_map(f)
/// — paging composes under bind because Cursor is positional in a fixed
/// per-kind order (deterministic graph ⇒ deterministic pages).

/// The reader is not special — it is this query:
pub fn reading_window(g: &Graph, from: TextLocus, n: u16, dir: Direction) -> Window; // corpus from the locus
/// LAW: for any partition into windows, concatenation is invariant
/// (lazy loading can never change what the Bible says).
```

Wire (two endpoints total; all existing routes become views then retire):
```
GET /api/node/{id}                      → Card + EdgeSummary   (+ GraphVersion)
GET /api/node/{id}/edges?kind&cursor&n  → EdgePage
GET /api/text?from=<locus>&n=&dir=      → Window               (per-corpus reading spine)
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
- A Bible-shaped assumption about text (corpora are first-class; the KJV
  is one corpus; extrabiblical witnesses are a type error, extrabiblical
  mentions/citations are not).
- The Book of Concord mistaken for Scripture (roles are asymmetric at the
  registry: canonical confidence derives from NormaNormans; quotes has no
  Scripture-quoting-Concord constructor; a quotation never promotes its
  text).
- A nested address (recursion is relational — flat loci, edges carry the
  structure).
- An inverse view that disagrees with its forward view (both are
  projections of one row; the shared EdgeId is the bijection witness;
  dual is a tested total involution).

## 10. Open type questions (flagged, not hidden)

- `Interned` and store format (implementation planning).
- TokenSpan semantics await the tokenization rules (deferred by directive).
- Whether `SameTime` clusters paginate (large same-year antichains in dense eras).
- EdgeMeta variants per kind (finalized per surface during M-batches).
- ConcordRef structural scheme (Part/Article/Paragraph vs edition-keyed;
  decided when the Book of Concord corpus is ingested).
