# Frontend–Backend Contract Architecture — design spec (DRAFT for owner review)

**Status:** DRAFT — awaiting owner review. Written from the owner's
2026-08-26 brainstorm directive (traveling, no detailed spec); every
assumption that fills a gap is marked **[ASSUMED]** and every question
that needs the owner is in §9. Nothing here is implemented.

**Owner directive (2026-08-26, near-verbatim):** contracts between
frontend and backend; the backend's contract is a semantically versioned
interface for querying the graph; consumer (frontend) driven;
property-based contract tests. Frontend type constructs: Views containing
a set of components; a component type presenting a Focus with a set of
escape hatches and a set of abstractions corresponding to the frontier
(the edges associated with the focus node); interfaces defining what on a
node's frontier is available for traversal, how we traverse, and how
presentation of focus/view/frontier/escape-hatches looks; a consumer
contract for the kinds of views we can ingest from our map app;
interfaces for everything, types for everything, composition over
inheritance, reusable components and shared interfaces to guarantee
uniformity.

**Owner amendments (2026-08-26, second directive):** views also need
escape hatches; we need ways to compose views (reader + world at the
same time is the example); hover vs click vs highlight interactions are
contractual on ALL components; and the governing law, verbatim:

## 0. THE TOTAL-CAPTURE LAW

**"There shall be nothing implemented which is not captured by an
interface and contract."** (Owner, 2026-08-26.) Every behavior traces to
an interface; every wire shape to the AQC (§2); every gesture to an
interaction contract (§4c); every view arrangement to a composition
contract (§4b); every exit to an escape hatch declaration. Anything
found implemented outside a contract is a defect by definition -- the
review rubric enforces this on every batch touching the client.

**THE CONTRACT-FIRST ORDER (owner ruling 2026-08-27, verbatim:
"henceforth you write the contracts first."):** for every batch that
adds or changes behavior, the CONTRACT ARTIFACTS land before the
implementation: the compiled types (the skeleton-first precedent, now
general), the declared capabilities/hatches/interaction contracts the
new surface will honor, and — for wire behavior — the Gherkin
scenarios. Order within a batch: contract → test → implementation;
where the contract needs controller/owner authority (compiled-
authority members, spec vocabulary), it lands as a preceding
controller commit before the batch dispatches. The conformance
corollary below then holds the implementation to what was written
first.

**THE CONFORMANCE COROLLARY (owner ruling 2026-08-26, verbatim: "if
contracts don't match implementation tests should fail"):** every
contract carries an enforcement mechanism that FAILS -- a compile
error, a failing test in a standing suite, or a fail-loud runtime
check -- the moment implementation drifts from it. A contract whose
violation nothing can fail on is documentation, not a contract, and
is itself a defect. The mechanisms, by contract kind: compiled types
(the client/Contracts skeleton, generated AQC types) fail at compile
time; wire behavior fails in the Gherkin corpus run by BOTH sides
(§3); state semantics fail in the law/effect property suites;
structural rules (atoms registered and named, effects only via the
registry, no undeclared gesture handling, no component-held shared
state) fail in CONFORMANCE TESTS -- reflection- or source-scan-based
tests in the standing client suite. Staging rule: each migration
batch that moves a construct onto a contract lands that construct's
conformance test IN THE SAME BATCH -- a migrated construct without
its drift-failing test is an incomplete batch, and the ST-4 sweep's
final rubric includes verifying the conformance suite covers every
contract then in force.

---

## 1. What this formalizes (and why it's the right time)

The client already contains an informal version of every construct the
owner named. This spec's job is to promote them from conventions to
typed, versioned, testable contracts:

| Owner's construct | Exists today as | Becomes |
|---|---|---|
| Focus | the node an `ExplorerPopover` shows (`IExplorable`) | `Focus` type: descriptor + payload |
| Frontier | `PopoverSectionProviders` / `EdgeSectionRegistry` sections | `IFrontierAbstraction` per typed edge family, generated from the crate's `relations!` manifest |
| Escape hatches | `ExplorationTarget`'s `NavigateReader` / `NavigateWorld` / `ShowMiniMap` + close/back | `IEscapeHatch` — the typed, closed set of non-traversal exits |
| Traversal | `ExplorerPopover._stack` Push/Back + G2's `ExplorationDescriptor {kind,key,title}` / `Reconstruct` | `ITraversal` — one interface; the popover is its canonical implementation |
| View | Reader page, World page, Split view, Sources page | `IView` = composition of `IViewComponent`s |
| Backend query surface | ad-hoc REST (`/api/verse/{vref}`, `/api/node/{id}`, `/api/scene`, `/api/text`, …) versioned only by the artifact's `FORMAT_VERSION` | the **Atlas Query Contract (AQC)** — consumer-owned, semver'd |

The timing is right because G2's descriptor seam (serializable node
references with a reconstruction factory) is exactly the wire-level
"focus reference" a contract needs, and it already shipped and is
tested.

## 2. The Atlas Query Contract (AQC)

**Consumer-driven:** the contract lives with the CLIENT (the consumer
declares what it needs; the server proves it provides it — the Pact
direction, not provider-published-docs). One decisive home:

```
contracts/
  atlas-query-contract/
    aqc.schema.json        # the contract document (see format, §9 Q3)
    VERSION                # MAJOR.MINOR.PATCH
    CHANGELOG.md           # one line per version, semver-justified
```

**Contents of the contract document** — for each query the client
consumes, its request shape, response shape, and invariants:

- `FocusQuery(descriptor) → Focus` — node card payload + its Frontier
  (see §4) + its available EscapeHatches.
- `TraversalQuery(descriptor, frontierGroup, page?) → [Focus refs]` —
  expand one frontier abstraction into traversable targets.
- `TextWindowQuery(ref, radius) → verses` — including per-verse
  annotation spans (`words_of_christ` is the first; the shape is the
  general annotation-layer seam).
- `SceneQuery(window) → scene` — the map composition query.
- `ExplorationRoundTrip(descriptor)` — descriptor → node → descriptor
  identity (the G2 seam, now a contract law).
- **[ASSUMED]** the initial contract (v0.1.0) wraps the EXISTING
  endpoints' shapes as-is (a contract snapshot of today's truth), so it
  ships with zero server behavior change; redesigned query shapes come
  as later 0.x revisions. (§9 Q4.)

**Semver law (OWNER RULING 2026-08-26: v1.0.0 IS the production
release — we are not live, so the contract lives in 0.x until launch):**
- Pre-launch (0.x.y): initial contract = **0.1.0**; breaking changes
  bump MINOR (0.2.0, 0.3.0 …); additive/compatible changes bump PATCH.
  The 0.x line signals "interface still forming" per semver convention.
- **1.0.0 is minted at production launch** — the moment external
  consumers exist, and from then on:
  - MAJOR — any breaking change to a shape a consumer uses.
  - MINOR — additive: new queries, new optional response fields.
  - PATCH — documentation/invariant tightening with no shape change.
- The server advertises its supported contract range at
  `/api/contract` (new, tiny endpoint); the client checks at startup
  and fails LOUD on mismatch (fail-loud house law) rather than
  degrading mysteriously.
- The artifact's `FORMAT_VERSION` remains a separate, internal version
  (storage format ≠ query contract); the AQC version is what the
  frontend — and the map app, §6 — pins.

**Generated types on both sides:** the contract document generates the
C# consumer types and the Rust provider DTO conformance layer (build
step, drift = compile error). Hand-written duplicates of contract
shapes are forbidden once generation lands — one source of truth, the
LICENSES.md/sources-page discipline applied to API shapes.

## 3. Property-based contract tests

**LAW (owner ruling 2026-08-26; clarified same day: "something like
gherkin BDD"):** contract tests are language-agnostic — the binding
expression of every contract test is **Gherkin** (Given/When/Then
`.feature` files) in the contract directory. No test that binds the
contract may exist only as code in one language's test framework. Each
side executes the SAME feature files through its own thin step-
definition layer:

```
contracts/
  atlas-query-contract/
    aqc.schema.json        # shapes (§2) — referenced by steps
    features/              # THE contract test corpus (Gherkin)
      focus-query.feature
      traversal.feature
      text-window.feature      # incl. annotation-spans law (RED-1)
      scene-query.feature
      exploration-roundtrip.feature
      versioning.feature       # /api/contract advertisement + fail-loud
```

- **Scenarios ARE the invariants**, readable by the owner without any
  code: "Given a node of every kind sampled from the graph / When I run
  FocusQuery / Then every frontier group is a relations! family / And
  every traversal target resolves to a live node / And every escape
  hatch is defined for the view context." Descriptor round-trip (the G2
  seam) and spans-within-verse-length (RED-1's alignment law) each get
  a scenario the same way.
- **Property depth via Scenario Outlines:** a provider-side exporter
  draws descriptors from the REAL graph (every kind; seeded,
  deterministic — never wall-clock random) and emits/refreshes the
  committed `Examples:` tables. The generator is code; its OUTPUT is
  Gherkin both sides execute verbatim. Steps phrased over "every kind"/
  "each sampled node" iterate internally so tables stay readable.
- **Step definitions are thin glue, not contract knowledge:** Rust
  binds steps with the `cucumber` crate against live handlers; C# binds
  the same phrases (Reqnroll/SpecFlow lineage) through the generated
  types. A phrase means ONE thing; both bindings implement the same
  glossary (kept beside the features). Adding a contract test = writing
  a scenario; a scenario either binds on both sides or the suite fails
  loud with the unbound phrase.
- Native property tests (proptest etc.) may explore deeper, but any
  contract behavior they enforce must also be stated as a scenario — a
  native-only contract assertion is a drift bug under this law.
  (§9 Q3 accordingly resolves: Gherkin features + JSON Schema for
  shapes.)

**The gate:** contract tests are part of the standing suite (they join
the canonical count); a provider change that breaks a consumer
invariant fails BEFORE it ships, and the failure names the contract
version that would have to bump.

## 4. Frontend type system (composition over inheritance)

> **COMPILED AUTHORITY (owner order 2026-08-26: "write a compilable
> code skeleton first and work directly from that so that there is no
> drift from our agreed upon types"):** these types are materialized at
> `client/Contracts/{Views,Interactions,State,Focus}.cs` — implementers
> work from those FILES, extend-only, member changes route through the
> controller (the atlas-graph-types discipline). Members the spec names
> but does not declare are resolved in-code under `SKELETON RESOLUTION`
> markers. Where this prose and the compiled skeleton disagree, the
> skeleton wins pending an owner ruling.

All interfaces; no inheritance hierarchies — capabilities compose:

```csharp
// A View composes components AND declares its own escape hatches
// (owner amendment 2026-08-26): Reader declares "open-world"/
// "enter-split", World the mirror, Split its collapses.
interface IView {
    string Name { get; }
    IReadOnlyList<IViewComponent> Components { get; }
    IReadOnlyList<IEscapeHatch> EscapeHatches { get; }
}
interface IViewComponent { }             // marker + lifecycle seam

// 4b. View composition -- a composition IS a view (so compositions
// nest, and get hatches/components like any other view). Today's
// split view becomes the first IViewComposition:
// Members=[Reader, World], Layout=split-h (bible left, atlas right).
//
// OWNER RULING (2026-08-26, verbatim): "your reader world split
// interface is too specific we should support split for any view."
// This interface was always view-agnostic; the RULING binds the
// IMPLEMENTATION to honor it: no view is a privileged split host, no
// view a privileged guest. Reader.razor's hard-wired World embed and
// any arrangement vocabulary shaped like ReaderOnly|WorldOnly|Split
// are transitional and must generalize to: ViewArrangement's value =
// (ordered Members over ANY views, a Layout kind, per-member state
// such as divider fraction); one generic composition host mounts the
// members; "enter-split" becomes a declared escape hatch any view can
// offer with any partner view. Mode links (follow-text) activate on
// arrangement CONTENT ("a locus-bearing member and a window-bearing
// member are both present"), never on hard-coded page identity.
// (Supersedes ST-2 ruling R4's "Reader remains the only split HOST".)
interface IViewComposition : IView {
    IReadOnlyList<IView> Members { get; }
    ICompositionLayout Layout { get; }
}
interface ICompositionLayout {
    string Kind { get; }   // "split-h" | "overlay" | "pinned-panel" | ...
    // constraints (min widths, focus rules) are contract DATA
}

// 4c. Interaction contracts -- on ALL components (owner amendment).
// A component MAY NOT respond to a gesture it has not declared; two
// components declaring the same (Gesture, Semantic) must behave
// identically -- uniformity as a testable guarantee, not a convention.
interface IInteractive {
    IReadOnlyList<IInteractionContract> Interactions { get; }
}
interface IInteractionContract {
    Gesture Gesture { get; }     // Hover | Click | CtrlClick | ShiftClick | Highlight
    string Semantic { get; }     // "peek"|"explore"|"pin"|"select"|"range-extend"|"emphasize"
    TimingDiscipline Timing { get; }  // grace/debounce constants as contract data
}
// Seed vocabulary = the already-litigated gesture laws: Hover=peek
// (grace-timed), Click=explore/pin (ONE-RULE/PIN-1), CtrlClick=select
// (tray ruling), ShiftClick=range-extend, Highlight=emphasize.

// 4d. STATE CONTRACTS -- semantics + algebra (owner amendment
// 2026-08-26: "we have severe state management issues... we need to
// create semantics and algebra around this problem"; motivating
// defect SYNC-1: the map-side chapter picker and the reader-side
// chapter picker disagree in follow-text mode).
//
// SEMANTICS: all shared state lives in named, typed ATOMS, one
// canonical store each. Components never hold copies -- they render
// PROJECTIONS and emit INTENTS. Desync is impossible by construction.
interface IStateAtom<T> {
    string Name { get; }              // "locus" | "time-window" | "selection" | ...
    T Value { get; }
    void Dispatch(IIntent<T> intent); // the ONLY write path
}
interface IProjection<T> {            // what components consume; pure, read-only
    IStateAtom<T> Source { get; }
}
// ALGEBRA: atoms compose via LINKS -- declared derivations making
// modes (follow-text) first-class testable objects.
interface IStateLink<A, B> {
    IStateAtom<A> Source { get; }
    IStateAtom<B> Target { get; }
    B Derive(A source, B current);    // pure
    bool Active { get; }              // links toggle = modes
}
// Follow-text IS Link(Locus -> TimeWindow), active in split view;
// both chapter boxes are projections of the ONE Locus atom.
//
// LAWS (property-tested over generated intent sequences):
// 1. Single-writer: atoms mutate only via Dispatch.
// 2. Idempotence: same intent twice = once.
// 3. No-echo: link-derived updates carry origin, never re-derive
//    their source (bidirectional links cannot oscillate).
// 4. Confluence: same intent sequence => same final state regardless
//    of render timing.
// 5. Agreement: all projections of one atom are equal ALWAYS.
//
// Seed atoms: Locus, TimeWindow, FocusStack, Selection,
// ViewArrangement. ViewStateService remains the PERSISTENCE layer
// beneath atoms (it persists state; atoms OWN it); URLs/deep links
// are projections under the same agreement law.
//
// OWNER RULING (2026-08-26, verbatim): "no we're not going to track
// down the state bugs one by one. we're going to structure our front
// end so that they cannot occur." State defects are retired BY
// CONSTRUCTION, never by targeted patches -- the state layer (this
// section) is APPROVED FOR IMPLEMENTATION ahead of the rest of this
// spec, staged migrations with the standing review discipline.

// 4d-EFFECTS [CONTROLLER ADDITION 2026-08-26, forced by ST-2's review;
// owner may veto]: async materialization of atom values (fetches, JS
// interop) is an OWNED EFFECT: IStateEffect<T> {Name, Source,
// AppliesTo, Materialize} + IEffectRegistry.Claim — one owner per
// effect name, claim-on-mount / release-on-dispose / latest-claim-
// wins / RECONCILE-on-claim (the effect runs against the current
// value even when idempotence fired no Changed). Retires by
// construction the two defect classes ST-2 hit live: stale-instance
// effects against shared atoms, and blank-on-remount when the atom
// was already converged. Compiled in client/Contracts/State.cs.

// The focus component — the owner's named construct.
interface IFocusComponent : IViewComponent {
    Focus Focus { get; }                              // descriptor + payload
    IReadOnlyList<IFrontierAbstraction> Frontier { get; }
    IReadOnlyList<IEscapeHatch> EscapeHatches { get; }
}

// One frontier abstraction per typed edge family on the focus node.
interface IFrontierAbstraction {
    string EdgeFamily { get; }            // a relations! name: "comments-on", "spoken-by", …
    string Label { get; }                 // presentation label
    int? Cardinality { get; }             // known count where cheap, null where lazy
    ITraversal Traversal { get; }         // how this abstraction is traversed
}

// Traversal: what Push/Back/expand MEAN. One implementation rules
// (the popover); saved explorations and the selection tray consume the
// same descriptors.
interface ITraversal {
    Task<IReadOnlyList<Focus>> Expand(int page);
    ExplorationDescriptor Describe(Focus target);     // the G2 seam
}

// Escape hatches: typed exits that LEAVE graph traversal.
interface IEscapeHatch {
    string Kind { get; }                  // "read-in-context" | "show-on-world" | "mini-map" | …
    Task Invoke();
}

// Presentation: how (view, focus, frontier, hatches) LOOKS — the ONE
// render rule generalized. Components implement presentation via
// composition (a presenter per surface), never by subclassing a base
// component.
interface IPresentation<TContext> { RenderFragment Render(TContext ctx); }
```

**Uniformity guarantee:** every focus, on every surface (popover, peek,
future panes), renders through the same `IFocusComponent` +
`IPresentation` pipeline — the existing ONE-RULE (verse text renders the
same everywhere) promoted to an architectural invariant. New node kinds
and new edge families acquire frontier presentation by REGISTRATION
(the existing section-registry pattern), never by editing a switch in a
component.

**The data structure does the work:** frontier abstractions are
GENERATED from the crate's `relations!` manifest — a new typed edge in
the graph vocabulary automatically surfaces as a (default-presented)
frontier abstraction on nodes that carry it. This honors the halted
Explorable-typeclass brainstorm's direction without pre-empting the
owner's popover-content decisions: WHAT is traversable is generated;
WHAT a given popover chooses to show remains governed by POPOVER-LAW-1
and the deferred brainstorm. [ASSUMED — §9 Q5.]

## 5. Escape hatches — definition

**[ASSUMED]** An escape hatch is any exit from graph traversal into a
view context: "Read in context" (→ Reader at the locus), "Show on
world" (→ World at the window), mini-map, and close/back-to-surface.
They are per-(focus-kind, view) sets declared in the contract, so a
FocusQuery answers not only "what is this node and what edges leave it"
but "what are the sanctioned ways OUT of here" — making hatches
testable (a hatch that navigates nowhere is a contract violation) and
uniform (the same hatch kind renders identically everywhere). (§9 Q1.)

**THE FOLLOW-RELEASE LAW (owner ruling 2026-08-27, verbatim: "on every
thing sharing state with the reader state, we need an escape hatch to
stop following the reader as well"):** every view that shares the
reader's state — today, any view declaring the locus-bearing
capability and rendering from the shared Locus atom (Kretzmann; the
world map's follow chip is the precedent instance for its window) —
MUST declare a "toggle-follow" escape hatch: one affordance, same
semantics everywhere (following = render from the shared atom;
released = the view navigates independently from its own view-local
position, seeded from the shared value at release; re-follow =
reconverge to the shared atom, the SyncToken re-sync discipline).
Conformance (per §0's corollary): a registered view that declares the
locus-bearing capability without declaring the toggle-follow hatch
fails a standing conformance test.

## 6. Map-app view consumer contract

The peer map system already consumes our data exports under pinned
versions (gazetteer/chronology contracts). This adds the reverse-
direction, view-level contract the owner named: **the kinds of views we
can ingest FROM the map app.**

- Consumer-driven again — WE declare the view shapes we accept:
  `contracts/map-view-contract/` with its own semver.
- **[ASSUMED]** initial (0.1.0) ingestible view kinds: a `SceneView` (window +
  camera + lit places + overlays — what our World pane renders today)
  and a `MiniSceneView` (the mini-map's contract), so the map app can
  hand us a fully-specified view and our `IView` composition mounts it
  as a component. Richer kinds (comparative scenes, route/journey
  views) become later 0.x revisions after the owner names them. (§9 Q2.)
- Property tests: every ingested view definition validates and mounts
  (headless render smoke) or is rejected loudly with the contract
  clause it violates.

## 7. Migration path (behavior-preserving; no big-bang)

1. **AQC v0.1.0** — contract document snapshotting today's consumed
   shapes + `/api/contract` endpoint + generated types + property
   contract tests. Zero behavior change; the Playwright suite is the
   proof.
2. **Refactor to interfaces** — `ExplorerPopover`/section registry
   re-expressed as `IFocusComponent`/`IFrontierAbstraction`/
   `IEscapeHatch`/`ITraversal` implementations behind unchanged
   behavior (suite green throughout; CONTRACT.md untouched).
3. **Frontier generation** — the relations!-manifest-driven frontier,
   replacing per-kind hand lists where they exist.
4. **Map-view contract v0.1.0** — the two ingestible view kinds, wired to
   the existing World/mini-map rendering.
5. New features (selection verbs, new popover surfaces, new corpora
   frontiers) build ON the contract from then on; contract version
   bumps become the visible, reviewable record of interface evolution.

Each step is its own batch with the standing review discipline.

## 8. What this spec deliberately does NOT decide

- Popover CONTENT per node kind (POPOVER-LAW-1 + the owner's deferred
  frontend-elegance brainstorm own that).
- The selection tray's verb set (its own future brief).
- Any change to graph vocabulary, artifact format, or data.
- Backend storage/scale topics (separately settled: split+compress
  direction; LAN/VPS deployment).

## 9. OPEN QUESTIONS for the owner

1. **Escape hatches** — RESOLVED (owner, 2026-08-26): "yes" to §5's
   definition, extended: VIEWS carry escape hatches too (§4's amended
   IView).
2. **Map-app views** — which view kinds do you want ingestible first?
   §6 assumes SceneView + MiniSceneView.
3. **Contract format** — JSON Schema (broad tooling, verbose) vs a
   small hand-rolled TOML/IDL we generate from (tighter, house-styled)?
   Recommendation: JSON Schema for the initial contract (generation
   tooling exists on
   both sides), revisit if it fights us.
4. **Initial scope** — snapshot today's endpoint shapes as AQC v0.1.0 (zero
   behavior change, recommended) or redesign the query surface
   immediately in the first contract?
5. **Frontier generation** — comfortable with frontier abstractions
   being generated from the relations! manifest (new edge kinds surface
   automatically, presentation-gated), or do you want every frontier
   family owner-approved before it appears anywhere?

---

*Next step per the house process: owner reviews this spec; on approval
it goes to writing-plans for a task-by-task implementation plan
(migration steps §7.1–7.4 are the natural task seams). Until then this
document is inert.*
