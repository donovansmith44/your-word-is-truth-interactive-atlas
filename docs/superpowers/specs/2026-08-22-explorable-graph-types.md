# The Explorable Graph — types and interfaces (review artifact)

Companion to 2026-08-22-explorable-graph-design.md. The rule for every
type here: it either makes an illegal state unrepresentable, or it names
the law a fail-loud validator enforces. Conceptual algebra is stated in
Haskell (the BibleGraph.hs idiom); realized types in Rust (server/ETL);
the client contract in C# at the end. Nothing here is implementation —
field names and exact widths are illustrative; the SHAPES are the design.

## 0. Conceptual spine (Haskell)

```haskell
-- Exploration is PROPER monadic bind. The monad ranges over exploration
-- STATE (holdings: a set of focused positions), not over nodes — the
-- multiplicity lives INSIDE the monad, which is what makes the bind
-- lawful. (BibleGraph.hs's `a -> [e b]` was an encoding workaround for
-- an Ord constraint — its own comment says so — and is SUPERSEDED: it
-- put multiplicity outside e and broke the monad shape. Debate settled
-- 2026-08-22.)
newtype Explore a = Explore (Set (Position, a))   -- holdings with payloads

instance Monad Explore where
  return a = Explore {(here, a)}                  -- hold ONE thing, frontier
                                                  -- not yet consulted
  m >>= f  = unions (f applied at every held position of m)
                                                  -- follow-and-pool

-- Exploration reads the graph:      type Exploration = ReaderT Graph Explore
-- The graph supplies the generating Kleisli arrows, one per edge kind:
--   step :: EdgeKind -> NodeId -> Exploration NodeId
-- Multi-hop questions are Kleisli composition:
--   step Cites >=> step CatechismLink
-- POSITIONS include edges (owner, 2026-08-22: edges hold explorable
-- information — a claim's justification is itself walkable):
--   data Position = AtNode NodeId | AtEdge EdgeId
-- Dedup is two-layered: HOLDINGS are a Set (you arrive at a node once);
-- EDGES never dedup (every distinct connection stays visible, each with
-- its own EdgeId).
-- Laws are property tests on the query layer: left identity = the
-- popover's semantics; right identity = no phantom hops; associativity =
-- incremental clicking agrees with the one-shot deep query, so lazy
-- pages are windows over a lawful whole.

-- The graph is a family of typed edge relations over typed nodes:
--   edges  :: EdgeKind k => Node -> [Target k]
-- Every relation R ⊆ S×O is one stored set viewed both ways: R and its
-- transpose Rᵀ are the SAME rows. dual is a total involution on kinds:
--   EdgeKind = (Relation, Direction) | Symmetric SymRelation
--   dual (r, d) = (r, flip d) ;  dual (Symmetric s) = Symmetric s
--   involution BY CONSTRUCTION — not a tested property, a shape
-- Presentation is a selection:  Surface = Set EdgeKind × DisplayRule
-- Honesty is totality: an affordance exists iff the relation is inhabited.
```

GLOSSARY (owner-calibrated, 2026-08-22):
- FOCUS: the node you stand on. One thing.
- FRONTIER: derived, never stored — the one-hop reach of what you hold,
  per edge kind. frontier(focus) exists IN FULL in the graph; what any
  screen shows is policy (below). The popover = card(focus) + a chosen
  presentation of frontier(focus).
- HOLDINGS: the set of positions a pooled multi-hop question currently
  stands on. Simple navigation never pools (holdings = one focus);
  bind advances holdings through kind-filtered frontiers. return x =
  "hold just x, frontier not yet consulted."
- The exploration monad is the powerset monad over positions read
  against the graph (ReaderT Graph); explore ≡ bind, laws are property
  tests (left/right identity = popover semantics / no phantom hops;
  associativity = incremental clicking agrees with one-shot deep
  queries, so lazy pages are windows over a lawful whole).
- The sketch's Discovery {focus, crossRefs, timeAndPlace} survives as
  the VIEW — the screen shape for one position: card(focus) plus paged
  frontier sections. Views render; the monad composes; the frontier
  function bridges them.

## 1. Identity and text addressing

```rust
/// LAW (owner, 2026-08-22): "we need IDs for everything... IDs must be
/// hashes of the thing to which they correspond from which we can derive
/// the thing." Identity is CONTENT-ADDRESSED, and the id is a key into a
/// content-addressed store — the git model, fitting a community project
/// that lives on git:
///
///   pid            = kind-tag ++ hash(canonical_bytes(thing))
///   derive         : Store -> Pid -> Thing          (lookup)
///   VERIFICATION   : hash(canonical_bytes(derive(store, pid))) == pid
///                    — every fetch is SELF-VERIFYING; a tampered or
///                    corrupted thing cannot satisfy its own id.
///
/// - CANONICAL BYTES: every thing kind defines one deterministic
///   serialization (fixed field order, normalized text) — the same form
///   derive returns. Identical content ⇒ identical pid ⇒ stored once
///   (dedup is free).
/// - MERKLE STRUCTURE: things reference other things BY PID (edges name
///   endpoint pids; containers name locus pids; explorations name step
///   pids), so a pid roots a verifiable subgraph and deriving a thing
///   can transitively derive everything it references. GraphVersion IS
///   the root hash: two compiles are equal iff their roots are equal.
/// - CHANGE = NEW PID: fixing a thing creates a new thing (the avalanche
///   through referencing pids is honest — a claim about a corrected node
///   is a different claim). Continuity is SUPERSESSION: each recompile
///   emits old-pid -> new-pid records (tombstones with superseded_by),
///   so a community citation never rots — it resolves to the exact
///   version discussed, with the chain forward. Comments attach to
///   pids, i.e., to exact content versions; the governance loop is
///   comment -> curation -> assertion change -> recompile ->
///   supersession keeps the discussion attached.
/// - NAMES ARE REFS, NOT IDENTITY: human handles (jm_jordan, JHN.3.16)
///   are a mutable name->pid layer (git refs, exactly) — readable,
///   curated, re-pointable; the hash beneath never lies.
/// - EXPLORATIONS: serialized walks, content-addressed like everything
///   else — shareable, commentable, never compiled truth.
pub struct Pid { pub kind: PositionKindTag, pub hash: Multihash }
pub struct Tombstone { pub retired: Pid, pub superseded_by: Option<Pid>,
                       pub reason: ProvenanceId }
pub trait ContentAddressed {
    fn canonical_bytes(&self) -> Vec<u8>;        // the derivation format
    fn pid(&self) -> Pid;                        // = hash of canonical_bytes
}

/// Kind-tagged identity: a NodeId knows what sort of thing it names, so a
/// cross-kind reference is a type error, not a runtime surprise.
/// (NodeId/EdgeId are TYPED in-memory handles resolving to Pids.)
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

/// LAW (owner, 2026-08-22): "translations are explorable. we definitely
/// need to leave room in our types for N translations." A corpus is a
/// SKELETON (addressing scheme + reading-order spine, layer-neutral)
/// plus N TEXT LAYERS. The Bible skeleton is the canon structure; each
/// translation is a layer over it; the KJV is the CANONICAL layer
/// (inerrancy directive: it IS the text — law-bearing positions
/// evaluate against it; other layers are explorable references, and
/// translation FACTS are an allowed register). The same shape serves
/// Concord later (German/Latin/English editions as layers) for free.
pub struct TranslationId(Interned);       // "kjv" = the Bible corpus's canonical layer

/// LAW (owner, 2026-08-22): "we are gonna need to be able to make
/// correspondences between any translation corpora of the same type
/// (particularly between bible translations)." Three levels, one free:
/// - UNIT-LEVEL correspondence between layers is FREE: layers share the
///   skeleton, so the same VerseRef IS the correspondence.
/// - SPAN-LEVEL correspondence (which KJV words render which words of
///   another translation) is the symmetric Corresponds<C> relation --
///   both ends layer-tagged spans of the SAME corpus family, enforced
///   by the type parameter (a Bible-Concord pairing cannot inhabit it;
///   cross-TYPE links go through quotes/cites/confesses). Rows may be
///   curated or emitted as derived by alignment analyses; provenance
///   records the deriving process; symmetric semantics, one stored row.
/// - VERSIFICATION: a translation whose native versification deviates
///   from the canonical skeleton (KJV versification) is NORMALIZED AT
///   THE ADAPTER BOUNDARY -- internally everything is canonical-skeleton
///   addressed -- and the native/canonical map is PRESERVED as
///   correspondence data (explorable, auditable, fail-loud where
///   unmappable). The layer model survives deviation; nothing
///   downstream ever sees two skeletons.

/// Closed enum today; a new corpus is a deliberate variant (same
/// philosophy as NodeKind — every match site acknowledges it).
/// Addresses are LAYER-NEUTRAL: a TextRef names skeleton position, not
/// any translation's wording.
pub enum TextRef {
    Bible(VerseRef),                      // canon-structural; N layers render it
    Concord(ConcordRef),                  // RESERVED: materializes when ingested
}

pub struct VerseRef { pub book: BookId, pub chapter: u16, pub verse: u16 }

/// Token indices — NECESSARILY tagged with the layer whose tokenization
/// they index: verse-level loci are layer-neutral, but a sub-unit span
/// only means something in one translation's wording. (Rules deferred;
/// the address space is reserved now.) None = the whole unit. A verse —
/// or a Concord paragraph — is not an atom; sub-unit nodes materialize
/// later WITHOUT changing this type or anything built on it.
pub struct TokenSpan { pub layer: TranslationId, pub start: u16, pub end: u16 }

/// The general text address: any corpus, any unit, optional sub-span.
pub struct TextLocus { pub at: TextRef, pub span: Option<TokenSpan> }

/// REFINEMENT where LAW demands Scripture specifically — typed to the
/// CORPUS, not to any one translation (owner, 2026-08-22: "parallel
/// accounts are necessarily scriptural... the KJV isn't the only
/// trustworthy translation and our types should reflect that").
/// BibleLocus is layer-neutral at verse level (the skeleton); which
/// translations are TRUSTWORTHY to render law-bearing text is registry
/// data on Translation nodes, with the KJV as the canonical layer
/// (inerrancy directive; citation integrity checks against it). So: an
/// extrabiblical ATTESTATION is a compile error (corpus bound), and an
/// attestation pinned to one translation is unrepresentable (the locus
/// carries no layer; renderings are per-trusted-layer) — while
/// `mentions`/citation edges accept the general TextLocus and may cross
/// corpora freely.
pub struct BibleLocus { pub verse: VerseRef, pub span: Option<TokenSpan> }
// = Locus<BibleTag>; the historical name KjvLocus is retired — it
// conflated the corpus with its canonical layer.
// impl From<BibleLocus> for TextLocus — widening is free; narrowing is
// a checked parse.

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
///   "CanonicalText Concord" or "Scripture attested by Concord" are
///   unrepresentable, not merely forbidden.
pub enum CorpusRole {
    NormaNormans,          // Scripture (kjv) — the norming norm; the attestation law lives here
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

/// LAW (owner, 2026-08-22): container content is CORPUS-HOMOGENEOUS BY
/// TYPE. "If we're getting a container for the BoC... the scripture
/// references/verse ids are distinguished as scripture and not mixed up
/// with the rest of it." A container groups text WITHIN one corpus (it
/// is part of that corpus's structural layer); Scripture appearing in
/// extrabiblical material is reachable ONLY through Scripture-typed
/// edges (quotes/cites, whose objects are BibleLocusRange) — never as
/// content. The general locus is parameterized by corpus, and BibleLocus
/// becomes an INSTANCE of it rather than a special case:
pub struct Locus<C: CorpusTag> { pub unit: C::Ref, pub span: Option<TokenSpan> }
//   pub type BibleLocus   = Locus<BibleTag>;    // the attestation-law refinement, unified
//   pub type ConcordLocus = Locus<ConcordTag>;
// (TextLocus remains the corpus-erased wire/UI form; narrowing is a
// checked parse, as with AnyNodeId.)

/// Container content: a SET of same-corpus loci (container algebra: ∅
/// is lawful identity; overlaps between containers are lawful; text
/// immutable). Contiguity is NOT assumed — multi-chapter and gapped
/// containers are first-class, which retires the same-chapter parse
/// limitation class.
pub struct LocusSet<C: CorpusTag>(BTreeSet<Locus<C>>);

/// If cross-corpus GROUPING is ever wanted (a study set pairing a psalm
/// with a catechism section), that is a DIFFERENT kind of thing: a
/// collection of node references (Pids) — kin to saved explorations —
/// never a text container. Containers hold text; collections hold
/// things.

/// ROOM RESERVED (owner, 2026-08-22, design directive — not an
/// implementation order): WORD-LEVEL ANALYSIS/EXPLORATION and
/// CROSS-CORPUS COMPUTATION AT ARBITRARY PRECISION. The addressing
/// already carries it (Locus<C> + layer-tagged TokenSpan reach any
/// word of any layer of any corpus); what is reserved beyond
/// addressing:
/// - A TOKENIZATION CONTRACT per (corpus, layer): when sub-unit
///   analysis materializes, each layer exposes
///     tokens : Unit -> [Token]
///   under pinned, versioned rules (rules themselves deferred), so any
///   analysis consumes tokens uniformly from any corpus — KJV wording
///   vs a Concord quotation, layer-vs-layer alignment between
///   translations, concordance and frequency studies.
/// - ANALYSES ARE CONTENT-ADDRESSED DERIVED THINGS: an analysis result
///   is pid'd by (analysis-kind, input pids, params) — so results
///   inherit the derive/verify law (recomputable from their id, never
///   authored, cacheable by construction) and are explorable/
///   commentable like everything else.
/// - CROSS-CORPUS ALIGNMENT lands as derived relations in the SAME
///   relations! manifest (span-precision alignment/wording-match
///   edges — the quotes edge's wording-divergence metadata is the
///   embryo). No schema change when any of this materializes: the
///   reserved room IS the contract shape, not new addressing.
```

## 2. Nodes

```rust
/// Closed enumeration TODAY; extending it is a deliberate act that the
/// compiler forces every match site to acknowledge (P5: a new node kind
/// is a record + compiler rule + display rule — the exhaustive match IS
/// the checklist).
pub enum NodeKind { Book, Chapter, Verse, Container, Event, Narrative,
                    Place, Person, Anchor, Era, Polity, CatechismItem,
                    Source, Translation }

pub struct Node { pub id: AnyNodeId, pub payload: NodePayload }

pub enum NodePayload {
    TextUnit { corpus: CorpusId, layer: TranslationId, text: String },
        // the ONLY store of ANY corpus text (P1), keyed by layer: a KJV
        // verse is the Bible corpus's canonical-layer unit at its
        // VerseRef; another translation's rendering of the same VerseRef
        // is another unit, same skeleton position, different layer.
        // Per-layer fidelity laws (each translation's adapter proves
        // bijection + reconstruction against ITS source).
    Translation { label: Label },                // explorable: provenance, coverage, license
    Container{ corpus: CorpusId, title: DecisiveTitle, content: ErasedLocusSet, provenance: SourceRef },
        // typed as Container<C>/LocusSet<C> in the authoring/compiler
        // layer; the payload records its corpus and its content is
        // homogeneous by construction — a verse locus inside a Concord
        // container cannot be built.
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

/// LAW (owner, 2026-08-22): "enforce edgekind pairs with types." The
/// dual pairing is not a function with a property test — it is the SHAPE
/// of EdgeKind. A kind is a relation plus a direction; the dual flips
/// the direction; flipping a two-variant enum is involutive by
/// construction. An unpaired kind cannot be written (every directed
/// relation has both readings automatically), a broken dual cannot be
/// written (there is nothing to get wrong), and a symmetric relation
/// lives in a separate id space that HAS no directions (so it can never
/// be given one, and a directed relation can never masquerade as
/// symmetric).
pub enum Direction { Forward, Inverse }
impl Direction { pub fn flip(self) -> Direction { /* two variants; involutive */ } }

pub enum EdgeKind {
    Directed(RelationId, Direction),
    Symmetric(SymRelationId),
}
pub fn dual(k: EdgeKind) -> EdgeKind {
    match k {
        EdgeKind::Directed(r, d) => EdgeKind::Directed(r, d.flip()),
        sym => sym,
    }
} // involution by construction; the old property test is DELETED as moot

/// The familiar names become derived LABELS, one per (relation,
/// direction) — display vocabulary, not identity:
///   (ContainsRel, Forward) -> "contains"     (ContainsRel, Inverse) -> "member-of"
///   (AttestsRel,  Forward) -> "attested-in"  (AttestsRel,  Inverse) -> "attests"
///   (ConfessRel,  Forward) -> "confesses"    (ConfessRel,  Inverse) -> "confessed-in"
///   (QuotesRel,   Forward) -> "quotes"       (QuotesRel,   Inverse) -> "quoted-by"
///   (CitesRel,    Forward) -> "cites"        (CitesRel,    Inverse) -> "cited-by"
///   (MentionsRel, Forward) -> "mentions"     (MentionsRel, Inverse) -> "mentioned-in"
///   (LocatedRel,  Forward) -> "located-at"   (LocatedRel,  Inverse) -> "site-of"
///   (DatedByRel,  Forward) -> "dated-by"     (DatedByRel,  Inverse) -> "dates"
///   (NamedRel,    Forward) -> "named"        (NamedRel,    Inverse) -> "name-of"
///   (SuccessionRel, Forward) -> "follows-in" (SuccessionRel, Inverse) -> "precedes-in"
///   Symmetric: "catechism-link", "parallel", "temporal-adjacency"

/// A relation declares its stored row, its endpoint TYPES, and its id —
/// the direction machinery comes for free from the shape above. Subject
/// and Object are typed, so a forward page's entries are Object-kinded
/// and an inverse page's are Subject-kinded, checked at compile time in
/// the query layer (erased only at the wire):
pub trait Relation {
    type Row;
    type Subject;                       // e.g. EventId for DatedByRel
    type Object;                        // e.g. AnchorId for DatedByRel
    const ID: RelationId;
    fn endpoints(r: &Self::Row) -> (Self::Subject, Self::Object);
}
pub trait SymRelation { type Row; type End; const ID: SymRelationId;
    fn ends(r: &Self::Row) -> (Self::End, Self::End); }

/// WHERE RELATIONIDS ARE DEFINED (owner question, 2026-08-22): in ONE
/// place — a declarative relation MANIFEST from which everything else
/// is generated by macro: the RelationId/SymRelationId closed enums,
/// each Relation impl's ID const, the (relation, direction) -> label
/// table, the Graph struct's per-relation tables, and the BiIndex
/// builders. A relation exists iff it has a manifest row; enum/impl/
/// table/label drift is unrepresentable because there is nothing to
/// keep in sync — one list, five expansions. (Adding a relation = one
/// manifest row + its Row type + a compiler rule + a display-policy
/// row: P5 made literal.)
relations! {
    // ---- authored, directed ----
    directed  Contains<C>   { row: Contains<C>,   subject: ContainerId<C>, object: Locus<C>,
                              labels: "contains" / "member-of" }
    directed  Attests       { row: Attests,       subject: EventId,        object: BibleLocusRange,
                              labels: "attested-in" / "attests" }
    directed  Confesses     { row: Confesses,     subject: ConcordLocus,   object: BibleLocusRange,
                              labels: "confesses" / "confessed-in" }
    directed  Succession    { row: Succession,    subject: EventId,        object: EventId,
                              labels: "follows-in" / "precedes-in",  tagged_by: NarrativeId }
    directed  DatedBy       { row: DatedBy,       subject: EventId,        object: AnchorId,
                              labels: "dated-by" / "dates" }
    directed  LocatedAt     { row: LocatedAt,     subject: EventId,        object: PlaceId,
                              labels: "located-at" / "site-of" }
    directed  Named         { row: Named,         subject: PlaceId,        object: Alias,
                              labels: "named" / "name-of" }
    // ---- imported, directed ----
    directed  Mentions      { row: Mentions,      subject: TextLocus,      object: PlaceOrPersonId,
                              labels: "mentions" / "mentioned-in" }
    directed  Cites         { row: CrossRef,      subject: TextLocus,      object: TextLocus,
                              labels: "cites" / "cited-by" }
    directed  Quotes        { row: Quotes,        subject: TextLocus,      object: BibleLocusRange,
                              labels: "quotes" / "quoted-by" }
    // ---- edge-position relations (edges are positions; §3 Grounds) ----
    directed  GroundedIn    { row: GroundRow,     subject: EdgeId,         object: GroundTarget,
                              labels: "grounded-in" / "grounds-for" }
    directed  DerivedFrom   { row: DerivationRow, subject: EdgeId,         object: EdgeId,
                              labels: "derived-from" / "derives" }
    // ---- authored, symmetric ----
    symmetric CatechismLink { row: CatechismLink, end: TextLocus,          label: "catechism-link" }
    // ---- derived, symmetric ----
    symmetric Corresponds<C>    { end: Locus<C>,  label: "corresponds-to" }
        // span-level alignment between layers of one corpus family;
        // same-type-ness enforced by C; versification maps land here;
        // authored OR analysis-derived
    symmetric Parallel          { derived, end: ContainerId,  label: "parallel" }
    symmetric TemporalAdjacency { derived, end: EventId,      label: "temporal-adjacency" }
}
// reading-order is NOT a relation row family — it is each corpus's
// spine (a total order per skeleton), queried by windows, not by pages
// of a relation. endpoints (Edge -> its two nodes) needs no manifest
// row either: it is structural, read off any row directly.

/// Built, never authored: both adjacency maps from one row table.
pub struct BiIndex { /* fwd: subject → [(EdgeId, object)], inv: object → [(EdgeId, subject)] */ }
impl BiIndex { pub fn build<R: Relation>(rows: &Table<R::Row>) -> BiIndex { /* one pass */ } }
```

```rust
pub struct Graph {
    pub nodes:       NodeTable,
    // -------- authored relations (each row carries provenance) --------
    pub contains:    Table<Contains>,     // Container → LocusSet entries
    pub attests:     Table<Attests>,      // Event → attesting Scripture range
    pub confesses:   Table<Confesses>,    // normed-norm accord toward Scripture
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
    // projection of contains/attests via their BiIndexes
    pub parallels:   ParallelIndex,       // via co-attestation of one Event
}

/// LAW (owner, 2026-08-22): "Edges also hold information that may be
/// explorable. i.e., justification for mapping a narrative to a
/// timerange would be derived from Scripture." Edges are POSITIONS: an
/// EdgeId can take focus, presents (Entry = the row; Card = the
/// justification view), and has a frontier of its own —
///   endpoints    Edge → its two nodes            (structural, derived)
///   grounded-in  Edge → loci / anchors / sources (from Grounds)
///   derived-from derived Edge → authored Edges   (compiler-emitted)
/// with duals grounds-for / derives — so a VERSE's frontier includes
/// "claims grounded in this verse": Scripture justifying the structure,
/// explorable from either end.
pub enum Position { Node(AnyNodeId), Edge(EdgeId) }   // Holdings = BTreeSet<Position>

/// GROUNDS ≠ PROVENANCE: provenance says WHO asserts (source+locator);
/// grounds say WHAT JUSTIFIES the claim's content — typically Scripture.
/// Authored rows: grounds optional-but-encouraged (populated
/// progressively through migration). Derived rows: derivation is
/// ALWAYS present, computed — a derived edge that cannot say what it
/// derives from is unrepresentable.
pub struct Grounds(Vec<Ground>);
pub enum Ground {
    Scripture(BibleLocusRange),   // the owner's example: sequence/date justified by the text
    Anchor(AnchorId),             // chronology grounds
    DerivedFrom(Vec<EdgeId>),     // derived edges: the exact rows the compiler used
    Source(SourceId),             // e.g., a Robertson section for a harmony ordering
}

pub trait EdgeData {
    fn id(&self) -> EdgeId;
    fn kind(&self) -> EdgeKind;
    fn endpoints(&self) -> (AnyNodeId, AnyNodeId);
    fn provenance(&self) -> ProvenanceId;
    fn grounds(&self) -> &Grounds;
}

/// A typed authored edge; the pattern for all of them (rows now carry
/// grounds alongside provenance). Contains is corpus-parameterized: the
/// container and its loci share C at the type level.
pub struct Contains<C: CorpusTag> {
    pub container: ContainerId<C>,
    pub content:   LocusSet<C>,
    pub provenance: ProvenanceId,
    pub grounds:   Grounds,
}

/// LAW (attestation law, nee witness canon): parallel accounts are
/// attestations on ONE event — so `parallel` is DERIVED from
/// co-attestation and cannot disagree with it.
/// TWO SENSES OF "WITNESS", DISAMBIGUATED (owner, 2026-08-22): the
/// Gospels witness the Gospels as ORIGINAL NARRATIVE TESTIMONY -- "there
/// are no narratives of Biblical events that are originally witnessed
/// in the book of concord" -- while the Concord witnesses the Gospel by
/// ACCORD -- "it does witness the Gospel in the sense that it accords
/// with them." Two relations, two names; the overloaded word "witness"
/// is RETIRED from the type vocabulary (legacy [[witness]] data maps
/// onto Attests at ingestion):
///   ATTESTS   -- original testimony to an event. Scripture-only by
///               type; layer-neutral; KJV canonical.
///   CONFESSES -- doctrinal accord: a normed-norm locus confesses what
///               Scripture teaches. Never the reverse (asymmetry by
///               construction). Distinct from the MECHANICAL relations
///               quotes (verbatim) and cites (explicit reference).
///               Today's catechism-link is this relation in embryo and
///               migrates into it when Concord becomes a corpus.
/// The Concord relates to EVENTS only derivably -- it confesses the
/// verses that attest the Crucifixion; it never attests the Crucifixion
/// itself. The two-hop path (confesses -> attests) is the honest shape
/// of "the confessions witness the Gospel," and it is ordinary
/// exploration.
pub struct Attests {
    pub event: EventId,
    pub attestation: BibleLocusRange,     // SCRIPTURE-typed (corpus bound; Concord can
                                          // never attest), layer-neutral: renderings
                                          // come from trusted translation layers, KJV
                                          // canonical (the legacy witness data's
                                          // translations->verse-set indirection was
                                          // this shape all along)
    pub provenance: ProvenanceId,
}

pub struct Confesses {
    pub confessing: ConcordLocus,         // normed-norm corpus loci only
    pub confessed:  BibleLocusRange,      // Scripture, by refinement
    pub provenance: ProvenanceId,
}

/// LAW (norma distinction): quotation points TOWARD the norming norm —
/// the subject is any corpus's span, the object is Scripture. The
/// reverse direction has no constructor; wording divergence between the
/// quoting corpus and the KJV is expected and carried as data, never
/// "corrected" in either corpus (both fidelity laws stand).
pub struct Quotes {
    pub quoting: TextLocus,               // e.g. a span within an SD paragraph
    pub quoted:  BibleLocusRange,         // Scripture, by refinement (layer-neutral; wording-match metadata may name a layer)
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
    Attestation(AttestationAssertion),
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
/// RELATIONSHIP TO §0 (Explore): Explorable is the CAPABILITY ("can be
/// explored" — per-node interface, paginated); Explore is the ACT (the
/// composition algebra of exploring). The bridge: for each edge kind k,
/// `step k n = union of ALL pages of n.edges(k)` — the trait's calls,
/// totaled, are the monad's generating Kleisli arrows. The trait is the
/// interface; the monad is its specification: a correct implementation
/// induces a lawful Explore, and the monad laws ARE the query layer's
/// property tests (left identity = popover semantics; right identity =
/// no phantom hops; associativity = incremental clicking agrees with
/// the one-shot deep query). Pagination exists only on the trait side —
/// pages are honest windows over the monad's total frontiers.
/// card/edge_summary belong to the VIEW side (Discovery → View,
/// rendered via Presentable), not to the composition algebra.
///
/// SPLIT (owner review, 2026-08-22: "why is card in the explorable
/// interface" — it was a popover-convenience smear). Two capabilities,
/// separately named, because they are separately true:
///
/// What a node IS — every node has this trivially:
pub trait NodeData {
    fn id(&self) -> AnyNodeId;
    fn payload(&self) -> &NodePayload;
    fn provenance(&self) -> ProvenanceId;
}

/// What EXPLORATION means — yielding frontiers, nothing else. This is
/// the capability the monad's arrows are generated from; its name now
/// claims exactly what it provides:
pub trait Explorable {
    fn edge_summary(&self, g: &Graph) -> EdgeSummary;        // kind → true count (honesty needs it)
    fn edges(&self, g: &Graph, q: EdgeQuery) -> EdgePage;    // one kind, one page
}

/// Card is NOT a trait method — it is a VIEW: assembled by the view
/// layer from NodeData plus law-computed selections (decisive label via
/// the container-algebra law; citation strings via CorpusScheme), then
/// rendered via Presentable(Card context). Laws stay server data; the
/// trait stack stays honest; the WIRE IS UNCHANGED (GET /api/node/{id}
/// still returns the assembled card view + edge summary — endpoint
/// shape identical, type story corrected).
pub fn card(n: &dyn NodeData, g: &Graph) -> Card;

### The Explore monad, realized (the carrier the §0 algebra specifies)

Rust has no higher-kinded Monad trait, so the monad is realized as one
concrete carrier in the query layer. This is server-side machinery: it
powers pooled multi-hop queries (map/timeline composites, deep
questions) and it is WHERE THE MONAD LAWS RUN AS PROPERTY TESTS over
the real compiled graph. The client never sees Holdings — it consumes
views and pages; simple navigation never builds one bigger than a
singleton.

```rust
/// Holdings: the exploration state. Set semantics — arrive at a node
/// once (edges never dedup; they live in pages with EdgeIds).
pub struct Holdings(BTreeSet<AnyNodeId>);

impl Holdings {
    /// return: hold one thing, frontier not yet consulted.
    pub fn focus(n: AnyNodeId) -> Holdings;

    /// bind, general form: follow-and-pool a continuation at every
    /// held position.
    pub fn bind(&self, f: impl Fn(AnyNodeId) -> Holdings) -> Holdings;

    /// bind at one generating arrow: union, over held positions, of the
    /// TOTAL frontier of kind k — i.e., the trait's edges(k) with all
    /// pages taken. `step` is the bridge equation made executable.
    pub fn step(&self, g: &Graph, k: EdgeKind) -> Holdings;

    /// Kleisli chain: steps(&[Cites, CatechismLink]) ≡ step >=> step.
    pub fn steps(&self, g: &Graph, path: &[EdgeKind]) -> Holdings;
}

/// THE LAWS, as committed property tests over the compiled graph
/// (sampled nodes, every edge kind):
///   left identity:   focus(n).bind(f)          == f(n)
///   right identity:  h.bind(Holdings::focus)   == h
///   associativity:   h.bind(f).bind(g2)        == h.bind(|x| f(x).bind(g2))
///   step/page agreement (trait ↔ monad bridge):
///                    focus(n).step(g, k)       == union of ALL pages of n.edges(k)
/// The last law is what makes lazy pagination trustworthy: pages are
/// windows over the same totals the monad composes.
```

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

## 7. Presentation algebra: focusability + frontier display

LAW (owner, 2026-08-22): "the frontier is part of the DAG but the whole
UI/UX design is basically deciding what elements of the graph are
focus-able and how we display a frontier for an explorable of a given
type." The presentation layer is therefore exactly two policy functions
over the graph — nothing else:

```rust
/// Which node kinds can take focus, per surface. "What can I click?"
/// has one answer: kinds focusable HERE. Enabling a new kind (Source
/// nodes at Batch S; Person at Batch P) is a policy-row edit, not UI work.
pub fn focusable(surface: Surface, kind: PositionKind) -> bool;  // PositionKind = NodeKind | EdgeKind:
                                                                 // edges take focus too ("why?" is a click)

/// How a focus of a given kind displays its frontier, per surface:
/// which edge kinds render, ordered how, clamped to how many initially,
/// in which style, through which renderer.
pub fn display(surface: Surface, kind: PositionKind) -> FrontierPresentation;

pub struct FrontierPresentation { pub sections: Vec<SectionSpec> }
pub struct SectionSpec {
    pub kind: EdgeKind,
    pub renderer: Renderer,         // entry-list | text-flow | map-pins | timeline-rows
    pub style: SectionStyle,        // standard | quiet | superscript-marker ...
    pub initial: u8,                // clamp; hidden remainder MUST be signaled
    pub order: SectionOrder,        // votes-ranked | chain | canonical | resolved-date
}
```

A frontier renderer is not always a list: map pins display a SiteOf
frontier within a time window; the mini-reader's text flow displays a
Contains frontier as prose; timeline rows display FollowsIn/
TemporalAdjacency. One graph fact, several renderers.

### Presentable — how a given THING is presented (owner, 2026-08-22)

`display` arranges collections; PRESENTABLE gives each thing its form,
indexed by the context it appears in. The same Verse presents as flowing
text (reader), a snippet-with-ref (popover entry), a superscript anchor,
or a heading link — one implementation per (kind, context), single-
sourced, so a thing cannot accidentally wear two different faces in the
same context.

```haskell
class Presentable a where
  present :: Context -> a -> Presentation
```

```rust
pub enum PresentationContext { Card, Entry, Inline, Heading, Pin, CitationRow, Marker }

pub trait Presentable {
    fn present(&self, ctx: PresentationContext, g: &Graph) -> Presentation;
}
/// Presentation is a small CLOSED vocabulary of renderable forms the
/// client knows how to draw — the client stays generic; per-kind visual
/// decisions live in Presentable impls, not scattered in surface code.
```

BOUNDARY: law-bearing selection is SERVER data, visual form is CLIENT
presentation. The decisive title (container-algebra law), the citation
string (CorpusScheme::cite), and clamped attestation text are computed
server-side and travel as data; Presentable implementations (client,
per kind × context) decide how that data looks. A law can never be
re-decided by a stylesheet.

LAYERING (the full stack, each layer only consuming the one below):
  1. Graph            — truth (nodes + bidirectional typed edges)
  2. frontier/edges   — derived truth (reach, per kind)
  3. focusable/display— selection policy (what shows, how arranged)
  4. Presentable      — form policy (how each shown thing looks, per context)
  5. Surface assembly — card(focus) presented + sections rendered

The honesty laws are DERIVED at the policy gap, no longer decreed:
- shown ⊆ frontier(focus) — the renderer can only draw entries returned
  by `edges`; fabricated affordances have no data path (affordance
  honesty).
- |shown| < |frontier section| ⇒ a visible signal with the true count
  (the "+N more" law, generalized to every surface).
- focusable(surface, kind) = false ⇒ rendered inert AND visually
  distinct (capability parity of styling).

## 8. Client contract (C#, Blazor)

```csharp
// One popover machinery (owner decree), typed by edge kind:
interface IExplorableClient {
    Task<NodeCard>  Card(AnyNodeId id);
    Task<EdgePage>  Edges(AnyNodeId id, EdgeKind kind, Cursor? cursor, int limit);
    Task<TextWindow> Reading(TextLocus from, int n, Direction dir);
}
// Presentable, client-side: one presenter per (NodeKind, Context) —
// the single place a kind's visual form is decided:
interface IPresentable { RenderFragment Present(PresentationContext ctx, NodeData d); }
IReadOnlyDictionary<(NodeKind, PresentationContext), IPresentable> Presenters { get; }
// Section registry: EdgeKind → renderer + SectionSpec; each entry inside
// a section is drawn via the presenter for (entry.kind, section renderer's
// context). Registering a kind is the ONLY act needed to surface a new
// relation anywhere popovers open.
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
  is one corpus; extrabiblical attestations are a type error, extrabiblical
  mentions/citations are not).
- The Book of Concord mistaken for Scripture (roles are asymmetric at the
  registry: canonical confidence derives from NormaNormans; quotes has no
  Scripture-quoting-Concord constructor; a quotation never promotes its
  text).
- Scripture smeared into an extrabiblical container's content (container
  content is corpus-homogeneous by type; Scripture in a Concord context
  is reachable only via Scripture-typed quotes/cites edges).
- A non-canonical translation displacing the KJV in a law-bearing
  position (attestation law, citation integrity, decisive canon-text
  titles evaluate against the canonical layer by type; other layers are
  explorable references).
- A sub-verse span outliving its translation (TokenSpan carries its
  layer; a span cannot be read against another layer's tokenization).
- A cross-skeleton pairing smuggled through Corresponds (both ends share
  the corpus parameter; Bible-Concord links exist only as
  quotes/cites/confesses).
- A second skeleton in the interior (deviant versification is normalized
  at the adapter with the map preserved as explorable correspondence
  data).
- A nested address (recursion is relational — flat loci, edges carry the
  structure).
- An inverse view that disagrees with its forward view (both are
  projections of one row; the shared EdgeId is the bijection witness).
- An unpaired edge kind, a broken dual, or a directed/symmetric mix-up
  (EdgeKind = relation × direction | symmetric; the dual is a direction
  flip — involutive by shape, nothing to test, nothing to get wrong).
- Relation-registry drift (RelationId enums, Relation impls, labels,
  Graph tables, and BiIndex builders are five expansions of ONE
  manifest; there is no second list to disagree with the first).
- One kind wearing two faces in the same context (Presentable is the
  single per-(kind, context) source of visual form; laws travel as
  server data and cannot be re-decided by styling).
- A "why?" without an answer (every edge is a position with provenance
  plus grounds or computed derivation; a derived edge that cannot name
  its derivation is unrepresentable; the grounds-for dual lets Scripture
  answer "what claims rest on me?").
- An unaddressable thing (every node, edge, and saved exploration IS its
  content hash; comments always have somewhere exact to land).
- An unverifiable fetch (derive's round-trip law: what comes back must
  hash to the pid that asked for it).
- Identical content stored twice (content addressing dedups by
  construction).
- A community citation rotting (change mints a new pid; supersession
  records carry the chain; nothing is recycled).

## 10. Open type questions (flagged, not hidden)

- `Interned` and store format (implementation planning).
- TokenSpan semantics await the tokenization rules (deferred by directive).
- Whether `SameTime` clusters paginate (large same-year antichains in dense eras).
- EdgeMeta variants per kind (finalized per surface during M-batches).
- ConcordRef structural scheme (Part/Article/Paragraph vs edition-keyed;
  decided when the Book of Concord corpus is ingested).
- The cross-layer frontier ("this verse in other translations") is
  DERIVED (skeleton position × available layers) — whether it warrants a
  stored edge or stays a computed section is an M-batch decision.
