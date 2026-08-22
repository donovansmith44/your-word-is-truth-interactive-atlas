# The Explorable Graph — Bible Atlas domain-model design

Date: 2026-08-22
Status: approved in discussion; awaiting owner review of this written spec
Supersedes: the implicit domain model of Batches A–W5/HOTFIX-1..7 (which
remains running code until migration); Amendment F of batch-hotfix7-brief.md
(withdrawn — text order is not chronological order).

## 1. Goal (owner's words, binding)

"The goal of the application is to provide a way that anyone can educate
themselves on the Bible via exploration. So explorable things must adhere
to a contract where they implement an explorable typeclass."

Exploration modes: through TIME (narrative traversal, clickable or
represented on the map when a time range is chosen), through SPACE (the
map at points in time), and through TEXT/MEANING (verses linking to other
verses and to the Lutheran confessions).

"We build a massively interconnected graph, but we're disciplined and
selective about what kinds of things we show."

"Literally everything is explorable... we don't hard code a bunch of text
into the front end for the Bible... just make everything ultimately be a
query of the graph."

"We shouldn't arbitrarily assume that a verse is an atom. Verses are made
of words." (Design directive, not an implementation order.)

## 2. Principles (laws of the design)

P1. ONE ARTIFACT. The compiled output of ETL is the typed graph and
    nothing else. Every server response — reader text, map scenes,
    traversal rows, coverage numbers, the Sources page — is a query of
    the graph. Views may be regenerated from it; nothing is authored
    beside it. Parallel compiled stores are retired at the end of
    migration.
P2. TYPED EDGES; NO FABRICATED CONNECTIONS. Every connection between
    explorables is an edge with a declared kind. Each kind is either
    AUTHORED (a curated assertion) or DERIVED (computed by the compiler
    from assertions). Where no assertion or derivation speaks, no edge
    exists — absence is honest and renders as absence. A temporal link
    that is not a narrative link is a different kind of edge.
P3. PROOF AT THE BOUNDARY. Fidelity to sources is proven once, at
    ingestion, by fail-loud compile-time laws (e.g., the KJV adapter's
    bijection + reading-order reconstruction). After compilation there is
    one store, so drift between representations has nowhere to live.
P4. PRESENTATION = SELECTION. A surface (reader, map, popover, timeline)
    is an edge-kind selection plus a renderer. Discipline about "what we
    show" is expressed as selections and per-kind display rules, never as
    special-cased data.
P5. EXTENSIBILITY BY ADAPTER. New data enters as a new source adapter
    emitting typed assertions; new node/edge kinds are a record type plus
    compiler rule plus display rule. The contract is source-agnostic —
    TOML is one adapter among many, never a coupling.
P6. STANDING PRODUCT LAWS CARRY FORWARD, re-homed as graph laws:
    container algebra (verses immutable; containers over spans; ∅
    identity; overlaps lawful; reader display decisive), inerrancy
    doctrine (no textual-criticism register anywhere), KJV citation
    integrity, attestation law, nee witness canon (parallel accounts are
    attestations on one event; only Scripture attests; the Concord
    CONFESSES — accord, never original testimony), shared-title truth at every anchor, affordance honesty,
    single-feed chronology (dates resolve from the canonical anchor
    table), typed exemptions with reasons (never silent).

## 3. Node kinds

All nodes implement the Explorable contract (§5); none is special-cased
by the UI machinery.

- TextSpan family — Book ⊇ Chapter ⊇ Verse ⊇ Phrase ⊇ Word. Verse and
  above are materialized initially; Phrase/Word are RESERVED: the
  addressing scheme (locus = verse ref + optional intra-verse span) is
  defined from day one, so sub-verse nodes materialize later without any
  schema, edge, query, or law change. A verse's payload is its KJV text;
  when words materialize, verse text ≡ concatenation of its words (the
  fidelity law strengthens, P3).
- Container — titled passage over a set of text loci (pericopes, event
  containers, sections; the W-series product).
- Event — an occurrence; attested, possibly dated, possibly located.
- Narrative — a named doubly-linked list of Events.
- Place — with naming/alias data (KJV names).
- Person — planned (Batch P); enters as Theographic adapter output with
  zero machinery changes (the extensibility proof).
- Anchor — a canonical-chronology row (year, citation); explorable so a
  user can ask "why this date?" and see the primary-source paragraph.
- Era / Polity — time-range and border-bearing nodes for the map.
- CatechismItem — the Lutheran confessional units (Bente/Dau).
- Source — a provenance root (KJV text, Ussher's Annals, Theographic,
  Robertson, Crockett, ...); what Batch S renders; every assertion points
  at one.

## 4. Edge kinds

| Kind | From → To | Class | Notes |
|---|---|---|---|
| contains | Container → TextSpan loci | authored | container algebra; decisive-display law governs rendering |
| attests | Event → Scripture locus range | authored | original narrative testimony; Scripture-only; parallel accounts = multiple attestations on ONE event |
| confesses | Concord locus → Scripture range | authored | doctrinal accord (normed norm confessing the norming norm); catechism-link migrates into this family |
| succession(n) | Event ↔ Event, tagged by Narrative n | authored | the doubly-linked lists; the ONLY fine-grained sequence truth |
| reading-order | TextSpan ↔ TextSpan | derived (canon) | canon sequence at every granularity; the reader's spine |
| temporal-adjacency | Event ↔ Event | derived (anchors) | ONLY where resolved dates actually differ; never a tiebreak |
| dated-by | Event → Anchor (binding / offset / sequence / era form) | authored | single-feed chronology; salvages parked HOTFIX-7 resolution design |
| located-at | Event → Place | authored | |
| mentions | TextSpan locus → Place / Person | imported | Theographic verse refs today; refinable to word spans later |
| cross-ref | locus → locus | imported | votes-ranked; verse-level today, loci by design |
| catechism-link | locus ↔ CatechismItem | authored | |
| member-of | TextSpan → Event / Container | derived | inverse of contains/attests |
| parallel | Container ↔ Container | derived | via co-attestation of one Event |
| named | Place → alias set | authored | E3 KJV naming |

Per-kind invariants are compiler laws (fail-loud): succession chains are
well-formed doubly-linked lists; attestations resolve to real loci; no
an unbased ordering commitment (total temporal order stands on
TimePoint precision plus declared traditional chronology, basis and
optional justification hashed into the claim); contains respects span
addressing; exemptions are typed with reasons. The David's-death class of
lie becomes unrepresentable: nothing asserts an order, so no edge exists
to render one.

## 5. The Explorable runtime contract

Server: one trait — card() (id, kind, decisive label, provenance),
edge_summary() (kind → count), edges(kind, cursor, limit) (typed,
lazily-paged neighbors). Two generic endpoints serve the application:

  GET /api/node/{id}              → card + edge summary
  GET /api/node/{id}/edges?kind=&cursor=&limit= → one page of one kind

Wire shapes are uniform across all kinds; existing endpoints become views
over these queries during migration, then retire or remain as thin
conveniences.

Client: ONE composable popover machinery (owner decree), now typed — a
section registry keyed by edge kind, each kind carrying display rules
(ordering, clamp, quiet/standard styling, entry-point emphasis
parameter). Conditional presence and affordance honesty are consequences,
not disciplines: a section renders iff count > 0; a traversal affordance
exists iff the edge exists; there is no other data path to render from.

Surfaces (P4): reader = reading-order windows + decisive contains +
member-of loci; map = located-at + named + era/polity + dated-by
presence within the chosen range; time traversal = succession primary,
temporal-adjacency visibly distinct, anchors/eras as boundaries.

## 6. Reader = graph query; lazy loading

The reader issues "N nodes along reading-order from cursor C" — chapter
view, mini-reader, split view, and "+N more" are the same query with
different parameters. The client holds a sliding set of windows around
focus; popovers fetch per-section pages on expand (counts up front) —
honest truncation generalized. Windows are cacheable by (cursor, N) keyed
to a graph version stamp (the store is immutable between ETL runs).

Fidelity (P3): the KJV adapter law — every source verse becomes exactly
one Verse node (bijection), and the full reading-order walk reproduces
the source text byte-for-byte — runs at ETL, red-then-green at adoption.
A windowing law pins that any partition of the walk concatenates
identically, so lazy loading can never change what the Bible says.

## 7. Ingestion contract

Typed assertion records (Rust types; serde adapters), one per fact kind,
each wrapped in a uniform provenance envelope: source id, locator
(file/paragraph/URL), verification info, confidence class
(canonical-text | curated | imported | derived). Adapters today: KJV
text, curated TOML fact files, Theographic JSON, cross-references,
catechism. Compiler passes, in order: normalize → merge/alias (event and
place merge tables become assertion-level rules) → derive
(reading-order, temporal-adjacency, member-of, parallel) → index
(per-edge-kind adjacency + cursors) → validate (adapter-fidelity
bijections; E1–E4 chronology properties over dated-by/Anchor; per-kind
invariants; typed exemptions). Everything fail-loud at compile time.

## 8. Migration (strangler; suite green throughout; batch-sized)

- M-A: graph core + KJV adapter + reading-order + the two generic
  endpoints; reader on windowed queries; fidelity law red-then-green;
  old endpoints become views.
- M-B: events / attestations / narratives / anchors into the graph —
  salvaging the parked HOTFIX-7 date_resolve work as dated-by
  resolution; timeline + popovers on generic queries; bespoke resolvers
  retire; chronology properties re-homed.
- M-C: cross-refs, catechism, places, eras, mentions; map on graph
  queries; parallel compiled artifacts DELETED (P1 fully realized).
- M-D onward: the existing queue re-scoped ON the contract — HOTFIX-5
  presentation fixes as query/render rules; X superscripts as
  member-of/cross-ref at loci; P (Persons) as the extensibility proof:
  new node kind + mentions adapter, zero machinery changes; C3/G2/S as
  planned, S rendering Source nodes.

Parked work: batch-hotfix7-parked-state.md documents the uncommitted
date_resolve tree; M-B is its salvage point. The three parked duplicate
pairs and remaining polish items carry into the re-scoped queue.

## 9. Acceptance of the design itself

- The Bible reconstructs from the graph byte-identically (§6 law).
- David's-death-class ordering lies are unrepresentable (§4).
- A new source populates the graph through an adapter with no downstream
  changes (Batch P is the live proof).
- Every UI affordance corresponds to a queried edge (affordance honesty
  by construction).
- Sub-verse exploration requires no schema change when materialized
  (locus addressing already carries it).

## 9b. Community addressability (owner, 2026-08-22)

The project is headed for open-source, community-driven development:
people comment on nodes, edges, and explorations, and those comments
inform direction ("This edge has heretical implications. Fix it!"). The
identity contract serving that community: EVERY thing's id is the hash
of its canonical content, and the id is a key from which the thing is
derivable (content-addressed store; self-verifying fetches; Merkle
references; GraphVersion = root hash). Change mints a new id; recompiles
emit supersession records so citations resolve to the exact version
discussed and chain forward — nothing recycled, nothing rotting. Human
handles are a mutable name->id ref layer (the git model). The community
layer (GitHub issues first; in-app annotation later) consumes ids and is
never part of compiled truth; the governance loop is comment ->
curation -> assertion change -> recompile -> supersession.

## 10. Deferred (deliberately)

- Phrase/Word materialization trigger and KJV tokenization rules
  (implementation-time; the design only reserves the space).
- Graph store physical format and index structures (implementation
  planning).
- DECIDED (owner, 2026-08-22): no same-year clusters — traversal order
  is TOTAL, as granular as necessary (day-level where sources or
  tradition give it), with adopted-traditional placements carrying an
  optionally filled justification inside the content-addressed identity.
