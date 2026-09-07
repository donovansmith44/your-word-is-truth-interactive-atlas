//! THE FRONTIER CONTRACT — the cross-language authority (owner-ordered in
//! Rust, 2026-09-07: "write it in rust and make sure it compiles before
//! implementing").
//!
//! THE ARCHITECTURE PIVOT (owner verbatim, 2026-09-07): "if we're
//! implementing things twice in rust and c# then the front end is not
//! doing its job properly. the front end is basically a query engine +
//! presentation over a graph." The mirror/parity language this module
//! carried through FQ-0's design review is RETIRED along with it: this is
//! no longer mirrored into a C# copy for a parity test to hold in line.
//! The smart-frontier law below evaluates SERVER-SIDE (`compose_frontier`
//! in `server/atlas-graph`, behind `GET /api/focus/{id}` — FocusQuery,
//! FQ-1, not yet built); the client becomes a presenter of the wire's
//! already-composed frontier. This module is that policy's one compiled
//! source of truth.
//!
//! THE UNIFORM NODE MODEL (owner verbatim, 2026-09-07: "chapters and
//! books are nodes, though. chapters are nodes and their frontier
//! basically is the verses and event containers they contain, as well as
//! previous/next chapter navigation and whatever is associated on the map
//! side, etc... they're not special cases they just have their own
//! presentation."): every focus is, or WILL BE, a node; presentation is
//! the only per-kind variance. `FocusKind::backing()` below compiles that
//! ruling as an explicit, total classification rather than a silent
//! `None` (owner order: "if that's not in code, it needs to be.") — see
//! `FocusBacking`. Landing the actual Container/Anchor nodes is Batch
//! NODE-1; this batch (FQ-0) only compiles the honest cell.
//!
//! THE SMART-FRONTIER RULE (owner verbatim, spec §4e): "frontiers need to
//! be smart. I.e., they display instances of types of nodes on the
//! frontier iff both: 1) they exist for the focus node and 2) we have not
//! opted out of displaying that node type on the frontier of the focus
//! node."
//!
//!   display(section, focus) ⟺ exists(instances, focus)
//!                            ∧ allows(focus.kind, section.capability)
//!
//! THE UNITY RULING (owner verbatim, 2026-09-07): "1 and 2 ought to be
//! different expressions of the same thing. the visual frontier is how we
//! navigate the parts of the graph that are both available to us from a
//! focus and not opted out of." The graph frontier (edges leaving a
//! focus) and the visual frontier are ONE thing:
//!
//!   visual_frontier(focus) = graph_frontier(focus) ∩ allows(focus.kind)
//!
//! Compiled below as `Capability::edges()` — every capability anchors to
//! a real, DIRECTED `EdgeKind` from the edge manifest (relation +
//! direction, per the design review's B3: this is what lets a consumer
//! hand a capability straight to `Holdings::step`, the crate's own
//! frontier walker, with no re-derivation). A capability with no edges
//! behind it cannot exist (tested in this module); a capability whose
//! named relation has zero rows in the compiled artifact cannot exist
//! either (tested in `server/atlas-graph`, which can load the artifact —
//! see `atlas-graph/tests/frontier_falsifiability.rs`). This also settles
//! spec §9 Q5's direction: frontier abstractions ARE the relations,
//! filtered.
//!
//! `allows` below IS the opt-out registry. Rust buys us what no set-based
//! encoding can: the match is EXHAUSTIVE over both enums — adding a new
//! `FocusKind` (or a new `Capability`) refuses to compile until every cell
//! of the new row/column is an explicit decision. A cell edit is a
//! one-line change (the owner's standing acceptance bar).
//!
//! FQ-0 disposition (design review `frontier-design-review.md`, executed
//! per the owner's 2026-09-07 rulings, `progress.md` same date):
//! B1 (decorative relation cells) — fixed by keying every cell to a real
//! `EdgeKind` (B2/B3's merge) and re-deriving `Parallels`/
//! `PassageMembership` from the truth. B2 (relations() not a key) — fixed
//! by `FrontierEdge { kind: EdgeKind, target: Option<NodeKind> }`. B3 (no
//! `Direction`, doesn't compose with `Holdings::step`) — fixed by the
//! same struct. B4 (fictional `graph_kind()` cells) — fixed by
//! `FocusBacking` (`Author` → `ParameterizedView`; `Chapter`/`Book` →
//! `NodeDesignate(Container)`; `Passage`'s one-vs-many lossiness stays
//! documented, not solved — no set-valued return was ordered). B5 (CORE
//! never compiled) — fixed by `CoreSection`/`Section`, and the three
//! escapee sections (`PlaceEventsSection`, `PersonCardAndMentionsSection`,
//! `CatechismScripturesSection`) reclassified off the core catch-all.
//! I3 (marker interfaces double-encode the matrix) — contract-side part:
//! `Section` is now the ONE place a provider's classification can be
//! read from; collapsing the C# marker interfaces onto it is client-side
//! future work, out of this batch's scope wall. I5 (`exists` uncompiled)
//! — contract-side part: `SectionOutcome` below names the three-valued
//! result a future FocusQuery response carries; wiring
//! `ResolveAsync`/`IPopoverSectionProvider` to it is client-side future
//! work. C2 (`Capability::TimeAndPlace` name collision) — REJECTED by the
//! owner (`progress.md` 2026-09-07, "Owner red-flagged the
//! DatedAndLocated rename"): the shared name is deliberate semantic
//! unity — this capability's place-exploration YIELDS `TimeAndPlace` foci
//! (the owner's own `f(location, event-time)` formulation) — see the doc
//! comment on the variant itself.

/// THE focus-kind vocabulary — every EXPLORABLE kind the frontier serves.
/// NOT the graph's own `crate::id::NodeKind` (owner challenge 2026-09-07:
/// "we should just be reusing stuff that exists" — the answer: the graph
/// vocabulary has no Verse/Passage/Chapter/Book, which are LEVELS of
/// TextUnit/Container per id.rs's own doc, and Year/TimeAndPlace/
/// PolityDelta are client-synthesized views; the two vocabularies are
/// genuinely distinct, and the honest reuse is the type-checked BRIDGE
/// below, not a merged enum and not a colliding name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusKind {
    Verse,
    Passage,
    Chapter,
    Book,
    Author,
    Place,
    Person,
    Event,
    Catechism,
    Year,
    TimeAndPlace,
    PolityDelta,
    CommentaryItem,
    ConcordUnit,
}

/// The type-checked bridge to the graph's own vocabulary — TOTAL and
/// explicit by owner order ("if that's not in code, it needs to be"),
/// replacing a bare `Option<NodeKind>` that let "not a node today" and
/// "will never be a node" collapse into the same silent `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusBacking {
    /// A real graph node of this kind exists in the compiled artifact
    /// TODAY. Verified against the adapters (design review B4): Verse/
    /// Passage/ConcordUnit → TextUnit (`kjv_adapter.rs`/`concord_adapter.
    /// rs`); Place/Person/Event/Catechism/CommentaryItem → their own
    /// kind, one for one.
    Node(crate::id::NodeKind),
    /// Owner-ruled promotion (`progress.md` 2026-09-07, "chapters and
    /// books are nodes"): this focus WILL be backed by a real node of
    /// this kind — the vocabulary already has the kind, but no adapter
    /// emits one for this focus YET. `Chapter`/`Book` → `Container`
    /// (batch NODE-1: 66 books + ~1,189 chapters); `Year` → `Anchor`
    /// (NODE-1; NOTE: `NodeKind::Anchor` nodes already exist in the
    /// compiled graph today — `event_world::populate` emits one per
    /// `atlas.chronology_anchors` row — but those are chronology
    /// CALIBRATION points, not the client's calendar-`Year` focus; no
    /// adapter connects a `Year` focus id to a specific `Anchor` node, so
    /// this stays a designate, not `Node`, until NODE-1 does that mapping
    /// work honestly).
    NodeDesignate(crate::id::NodeKind),
    /// Genuinely a parameterized query over the graph, not a node walk —
    /// owner-acknowledged (`progress.md` 2026-09-07: "TimeAndPlace |
    /// PolityDelta stay None (parameterized views, owner-acknowledged)").
    /// `Author` joins this arm too (design review B4: `AuthorNode`
    /// fetches `VerseDetail.BookMeta` and projects three fields off it —
    /// it is not, and does not stand in for, a graph node; NODE-1 may
    /// investigate a real Person/Source mapping, "no guessing"
    /// (`progress.md` 2026-09-07) — so it stays here until that
    /// investigation rules one way or the other).
    ParameterizedView,
}

impl FocusKind {
    /// THE bridge, total and pinned (see `FocusBacking`, and this
    /// module's `focus_backing_is_pinned_per_kind` test).
    pub const fn backing(self) -> FocusBacking {
        use crate::id::NodeKind as G;
        use FocusBacking as B;
        match self {
            FocusKind::Verse | FocusKind::Passage | FocusKind::ConcordUnit => B::Node(G::TextUnit),
            FocusKind::Place => B::Node(G::Place),
            FocusKind::Person => B::Node(G::Person),
            FocusKind::Event => B::Node(G::Event),
            FocusKind::Catechism => B::Node(G::CatechismItem),
            FocusKind::CommentaryItem => B::Node(G::CommentaryItem),
            FocusKind::Chapter | FocusKind::Book => B::NodeDesignate(G::Container),
            FocusKind::Year => B::NodeDesignate(G::Anchor),
            FocusKind::Author | FocusKind::TimeAndPlace | FocusKind::PolityDelta => {
                B::ParameterizedView
            }
        }
    }

    /// Convenience derived from `backing()`: `Some` only for a node kind
    /// that exists in the compiled artifact TODAY. Design review B4's
    /// literal ask — a consumer narrowing a focus to a graph kind and
    /// walking its edges must never land on a fictional cell — so
    /// `NodeDesignate` deliberately does NOT surface as `Some` here;
    /// treating a promotion as walkable before NODE-1 lands would be
    /// exactly the defect B4 found. Callers that need to distinguish
    /// "not a node" from "not a node YET" should match on `backing()`
    /// directly.
    pub const fn graph_kind(self) -> Option<crate::id::NodeKind> {
        match self.backing() {
            FocusBacking::Node(k) => Some(k),
            FocusBacking::NodeDesignate(_) | FocusBacking::ParameterizedView => None,
        }
    }
}

/// THE UNITY BRIDGE, keyed for real (design review B2/B3 merged): a
/// capability names the exact directed edge (or symmetric relation) it
/// walks, plus the target node kind where it is genuinely single-kinded.
/// `target: None` means "heterogeneous" or "not yet narrow-able" —
/// documented per cell, never guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrontierEdge {
    pub kind: crate::edge::EdgeKind,
    pub target: Option<crate::id::NodeKind>,
}

/// The segregated capability families (spec §4e) — one per non-core
/// frontier section family. Core (chrome, hatches, back/trail, the focus
/// body) is NOT here — see `CoreSection`/`Section`: core is unconditional
/// law, never an opt-out decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    CrossReferences,
    /// B1/B2 fix: NOT the dead `SymRelationId::Parallel` (zero producers,
    /// zero consumers anywhere in the repo — the original cell named a
    /// relation that does not exist in the data). `VerseParallelsSection`
    /// actually walks `Attests` forward (verse → event) then `Attests`
    /// inverse (event → its OTHER witnesses) — a two-hop through the
    /// event, documented here as exactly that shape rather than deleted
    /// (owner-ratified recommendation, `progress.md` 2026-09-07).
    Parallels,
    /// Verse → event, forward. Shares `Attests` with `Accounts` — the
    /// SAME relation seen from opposite ends (the owner's "events are
    /// keys that map to sets of Biblical accounts" was already in the
    /// edge vocabulary; B3 makes the two directions distinct IN THE TYPE
    /// instead of in prose).
    EventMembership,
    /// B1/B2 fix: same `(focus=Verse, Attests, Forward)` triple as
    /// `EventMembership` — the two are, today, ONE relation with TWO
    /// presentations, distinguished only by a wire-payload discriminator
    /// (`VerseEventDto.Kind == "event"` vs `"general"`) the type system
    /// does not see. Promoting that split into the graph vocabulary
    /// itself (so the two become genuinely separable relations) is a
    /// PARKED ticket, not done here — this cell documents the honest
    /// shape rather than inventing a `Contains`-backed lie (the original
    /// cell named `Contains`, which has zero Bible-corpus rows; the only
    /// real `Contains` rows are Concord's, and `ConcordUnit` never
    /// claimed this capability).
    PassageMembership,
    /// Verse → person, forward, narrowed to `Person` (B1's lossiness fix:
    /// the raw `Mentions` relation also targets `Place`/`PeopleGroup`;
    /// `VersePersonsSection` filters to `Person` client-side today, so
    /// the cell now says so in the type, not just in the section's own
    /// filter).
    Persons,
    /// Symmetric `CatechismLink` — walked from either end. `Verse` claims
    /// it from the verse end (`VerseDetail.Catechism`); `Catechism`
    /// claims the SAME capability from the item end
    /// (`CatechismScripturesSection`, B5's third escapee) — one relation,
    /// two focus kinds, reused rather than given a second capability.
    CatechismSupport,
    /// `Succession` (chain membership; the "prev/next" navigation
    /// `EventChronologySection` and, per NODE-1's target state,
    /// `Chapter`/`Book` both read) + `TemporalAdjacency` (the timeline
    /// neighbor pairing), from one fetch — the design review's C4: "the
    /// one fully honest cell... the shape the other eight should match."
    Chronology,
    /// `DatedBy` + `LocatedAt`. Name KEPT despite colliding with
    /// `FocusKind::TimeAndPlace` — owner red-flag ruling
    /// (`progress.md` 2026-09-07, "Owner red-flagged the
    /// DatedAndLocated rename: REJECTED"): no technical collision exists
    /// (namespaced enums, both languages) and the shared name is
    /// deliberate SEMANTIC UNITY — an Event's place-exploration via this
    /// capability YIELDS `TimeAndPlace` foci (the owner's own
    /// `f(location, event-time)` formulation); a synonym would fork the
    /// vocabulary the way duplicate `NodeKind`s already warn against.
    /// `DatedBy`'s rows were verified (C4's open question, now closed):
    /// `graph.dated_by: Vec<chrono::DatedBy>` IS the placement authority
    /// row itself (`event_world::populate_dated_by` pushes
    /// `chrono::DatedBy` straight from `ChronologyDerivation`'s
    /// placements) — there is no second dating path; the covenant's own
    /// words hold.
    TimeAndPlace,
    /// Event → verse, `Attests` inverse. `EventDetail.Witnesses`.
    Accounts,
    /// ESCAPEE (design review B5, owner-ratified `progress.md`
    /// 2026-09-07): `LocatedAt` inverse, Place → event.
    /// `PlaceEventsSection` ("Jerusalem alone: 236 located-at events
    /// across the whole atlas") is a real, capped, paginated,
    /// edge-walking frontier section — not core focus-presentation. Was
    /// blanket-`false` under the old "body-only kinds" catch-all; now has
    /// its own governed cell.
    EventsAt,
    /// ESCAPEE (same ruling): `Mentions` inverse, Person → text unit.
    /// `PersonCardAndMentionsSection`'s mentions half — genuinely
    /// server-side paginated ("David at 896 mentions") — splits off the
    /// section's own card half, which stays `Section::Core(CoreSection::
    /// PersonCard)`.
    MentionsOf,
    /// ESCAPEE (same ruling): `CommentsOn` inverse — a commentary item
    /// comments-on a verse (forward); from the VERSE's side this is the
    /// inverse, "commented-on-by". Verified against the Kretzmann
    /// adapter: `CommentsOn.on` is a `BibleLocusRange` — i.e. every real
    /// `comments_on` row in the compiled graph is Verse-anchored, so
    /// `Verse` is the one focus kind this capability is verified live
    /// for today (per the owner's own instruction to check which focus
    /// kinds genuinely have comments-on rows before granting the cell).
    Commentary,
    /// NEW (owner ruling, `progress.md` 2026-09-07, NODE-1 target state
    /// — LANDED by NODE-1 + its NODE1-ROWS-1 fix round): `Contains`
    /// forward — "the things this container contains." `Chapter`/`Book`
    /// have real `Container` nodes and DECLARED Bible-corpus `Contains`
    /// rows now (chapter ⊃ verses as `ContainerContent::Loci`; book ⊃
    /// chapter as `ContainerContent::Container`, one row per child —
    /// `artifact.rs` serializes `contains_bible` since FORMAT_VERSION
    /// 11), so this capability's row-level falsifiability is LIVE, no
    /// longer gated; see
    /// `atlas-graph/tests/frontier_falsifiability.rs`.
    Members,
}

impl Capability {
    /// Every compiled capability, for exhaustive iteration in tests (and
    /// in the falsifiability sweep this module cannot itself run — see
    /// `atlas-graph/tests/frontier_falsifiability.rs`).
    pub const ALL: &'static [Capability] = &[
        Capability::CrossReferences,
        Capability::Parallels,
        Capability::EventMembership,
        Capability::PassageMembership,
        Capability::Persons,
        Capability::CatechismSupport,
        Capability::Chronology,
        Capability::TimeAndPlace,
        Capability::Accounts,
        Capability::EventsAt,
        Capability::MentionsOf,
        Capability::Commentary,
        Capability::Members,
    ];

    /// THE UNITY BRIDGE, executable (design review B3): returns real
    /// `EdgeKind`s (relation + direction), the exact type
    /// `crate::explore::Holdings::step` takes and `Graph`'s own indexes
    /// are keyed by — so a consumer holding `allows(kind, cap) == true`
    /// and `cap.edges()` composes straight into a graph walk with no
    /// re-derived direction. A capability with no edges behind it cannot
    /// exist (`capability_edges_are_never_empty` below).
    pub const fn edges(self) -> &'static [FrontierEdge] {
        use crate::edge::{Direction as D, EdgeKind as EK, RelationId as R, SymRelationId as S};
        use crate::id::NodeKind as G;
        match self {
            Capability::CrossReferences => {
                &[FrontierEdge { kind: EK::Directed(R::Cites, D::Forward), target: Some(G::TextUnit) }]
            }
            Capability::Parallels => &[
                FrontierEdge { kind: EK::Directed(R::Attests, D::Forward), target: Some(G::Event) },
                FrontierEdge { kind: EK::Directed(R::Attests, D::Inverse), target: Some(G::TextUnit) },
            ],
            Capability::EventMembership => {
                &[FrontierEdge { kind: EK::Directed(R::Attests, D::Forward), target: Some(G::Event) }]
            }
            Capability::PassageMembership => {
                &[FrontierEdge { kind: EK::Directed(R::Attests, D::Forward), target: Some(G::Event) }]
            }
            Capability::Persons => {
                &[FrontierEdge { kind: EK::Directed(R::Mentions, D::Forward), target: Some(G::Person) }]
            }
            Capability::CatechismSupport => {
                &[FrontierEdge { kind: EK::Symmetric(S::CatechismLink), target: Some(G::CatechismItem) }]
            }
            Capability::Chronology => &[
                // Chain membership, not a simple pairwise endpoint --
                // target intentionally `None` (a `Succession` row names a
                // whole ordered chain, per `Narrative`; the prev/next
                // PAIR a consumer wants is derived from position within
                // that chain, not from this edge's endpoint alone).
                FrontierEdge { kind: EK::Directed(R::Succession, D::Forward), target: None },
                FrontierEdge { kind: EK::Symmetric(S::TemporalAdjacency), target: Some(G::Event) },
            ],
            Capability::TimeAndPlace => &[
                // `DatedBy`'s target is heterogeneous (`ChronoTarget`:
                // Anchor | Prior(Event) | Era) -- `None` is the honest
                // cell, not a guess.
                FrontierEdge { kind: EK::Directed(R::DatedBy, D::Forward), target: None },
                FrontierEdge { kind: EK::Directed(R::LocatedAt, D::Forward), target: Some(G::Place) },
            ],
            Capability::Accounts => {
                &[FrontierEdge { kind: EK::Directed(R::Attests, D::Inverse), target: Some(G::TextUnit) }]
            }
            Capability::EventsAt => {
                &[FrontierEdge { kind: EK::Directed(R::LocatedAt, D::Inverse), target: Some(G::Event) }]
            }
            Capability::MentionsOf => {
                &[FrontierEdge { kind: EK::Directed(R::Mentions, D::Inverse), target: Some(G::TextUnit) }]
            }
            Capability::Commentary => {
                &[FrontierEdge { kind: EK::Directed(R::CommentsOn, D::Inverse), target: Some(G::CommentaryItem) }]
            }
            Capability::Members => {
                &[FrontierEdge { kind: EK::Directed(R::Contains, D::Forward), target: Some(G::TextUnit) }]
            }
        }
    }
}

/// THE COMPILED CORE (design review B5): the section families present on
/// every frontier of their kind WITHOUT exception — unconditional law,
/// never a `Capability` opt-out decision. Named exhaustively so
/// `allows() == false` stops being ambiguous between "opted out" (a
/// `Capability` policy decision — the registry's whole purpose) and
/// "core, not governed here" (a `CoreSection` classification): two
/// different facts that used to share one encoding.
///
/// Derived from `client/Explore/PopoverSectionProviders.cs`'s own
/// self-labelled `body-only/core` providers (owner-ordered list, grouped
/// per focus kind). NOT exhaustive over every core provider in that file
/// (e.g. `YearFrontierSection` is core too but was not named in the
/// ruling) — this compiles the taxonomy the design review discussed;
/// building the full provider→`Section` registry is client-side future
/// work.
///
/// Three sections the ORIGINAL "body-only kinds" catch-all wrongly parked
/// here were reclassified OUT (B5): `PlaceEventsSection` →
/// `Capability::EventsAt`; `PersonCardAndMentionsSection` SPLIT — its
/// card half stays `PersonCard` below, its mentions half →
/// `Capability::MentionsOf`; `CatechismScripturesSection` →
/// `Capability::CatechismSupport` (reused from the verse end, not given a
/// fourth escapee capability — see that variant's own doc comment).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoreSection {
    /// Verse/Chapter/Book's own text presentation.
    ChapterCard,
    CatechismText,
    CatechismExplanation,
    CatechismWhereWritten,
    PlaceDescription,
    PlaceDates,
    PlaceBlurb,
    PolityDeltaEvent,
    PolityDeltaScriptures,
    PolityDeltaGrounding,
    /// The card half of `PersonCardAndMentionsSection` — see this enum's
    /// own doc comment for the split.
    PersonCard,
    CommentaryItemProse,
}

/// A frontier section is EITHER core (unconditional) or a governed
/// capability — never both, never neither (design review I3's
/// contract-side fix: this is now the ONE place a provider's
/// classification is declared; a future client-side registry reflecting
/// over a provider's own `Section` retires the hand-written provider→set
/// map I3 flagged, and the vestigial `IHasCrossReferences`-style marker
/// interfaces with it — that refactor itself is out of this batch's
/// scope wall).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Core(CoreSection),
    Capability(Capability),
}

/// I5's contract-side part: `exists` — the other half of the
/// smart-frontier biconditional — compiled here so a future FocusQuery
/// response (FQ-1) has a real three-valued type to carry, instead of the
/// client's ad hoc "return null" convention the design review found
/// indistinguishable from a fetch failure (`CrossRefsSection`'s
/// `catch (Exception) { return null; }` and `if (xrefs.Count == 0)
/// { return null; }` produce the identical observable outcome today).
/// `Empty` and `Unavailable` are meant to RENDER identically (correct UX,
/// unchanged) — the distinction is what makes `exists` testable at all.
/// Wiring `IPopoverSectionProvider`/`ResolveAsync` to this vocabulary is
/// client-side future work (FQ-1), out of this batch's scope wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionOutcome {
    Content,
    Empty,
    Unavailable,
}

/// THE OPT-OUT REGISTRY. `false` = opted out for that kind (or the
/// capability's edges are `Section::Core` for that kind instead — see
/// `CoreSection`). Exhaustive over both enums by construction — the
/// compiler enforces the whole matrix. One line per row; a cell edit is
/// a one-line diff.
pub const fn allows(kind: FocusKind, cap: Capability) -> bool {
    use Capability as C;
    use FocusKind as K;
    match (kind, cap) {
        // Verse — the richest frontier. Commentary joins the true row
        // here (B5's escapee, verified live for Verse only).
        (K::Verse, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::CatechismSupport | C::Commentary) => true,
        (K::Verse, C::Chronology | C::TimeAndPlace | C::Accounts
            | C::EventsAt | C::MentionsOf | C::Members) => false,

        // Passage — verse-family, minus the verse-only memberships, and
        // minus Commentary (Kretzmann rows are verified Verse-anchored
        // only; no Passage-level consumer today — B1 discipline: no
        // capability without a verified row/consumer).
        (K::Passage, C::CrossReferences | C::Parallels | C::Persons
            | C::CatechismSupport) => true,
        (K::Passage, C::EventMembership | C::PassageMembership
            | C::Chronology | C::TimeAndPlace | C::Accounts | C::Commentary
            | C::EventsAt | C::MentionsOf | C::Members) => false,

        // Event — the owner's calibration row: NO cross references until
        // event-level xref data exists ("we are not yet at the point of
        // being able ot provide lots of cross references for events").
        (K::Event, C::Chronology | C::TimeAndPlace | C::Accounts) => true,
        (K::Event, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::CatechismSupport | C::Commentary
            | C::EventsAt | C::MentionsOf | C::Members) => false,

        // Place — the EventsAt escapee (B5).
        (K::Place, C::EventsAt) => true,
        (K::Place, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::CatechismSupport | C::Chronology
            | C::TimeAndPlace | C::Accounts | C::Commentary | C::MentionsOf | C::Members) => false,

        // Person — the MentionsOf escapee (B5); the card half is
        // `Section::Core(CoreSection::PersonCard)`, not a capability.
        (K::Person, C::MentionsOf) => true,
        (K::Person, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::CatechismSupport | C::Chronology
            | C::TimeAndPlace | C::Accounts | C::Commentary | C::EventsAt | C::Members) => false,

        // Catechism — reuses CatechismSupport from the item end (B5's
        // third escapee, see that variant's doc comment).
        (K::Catechism, C::CatechismSupport) => true,
        (K::Catechism, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::Chronology | C::TimeAndPlace
            | C::Accounts | C::Commentary | C::EventsAt | C::MentionsOf | C::Members) => false,

        // Chapter/Book — NODE-1's target state (owner ruling: "chapters
        // and books are nodes... their frontier basically is the verses
        // and event containers they contain, as well as previous/next
        // chapter navigation"): Members (what this container holds) +
        // Chronology (prev/next). The POLICY is decided now; the
        // underlying `Container`/Bible-`Contains` rows land with NODE-1
        // (see `atlas-graph/tests/frontier_falsifiability.rs`'s
        // TODO(NODE-1)-gated cells).
        (K::Chapter | K::Book, C::Members | C::Chronology) => true,
        (K::Chapter | K::Book, C::CrossReferences | C::Parallels | C::EventMembership
            | C::PassageMembership | C::Persons | C::CatechismSupport | C::TimeAndPlace
            | C::Accounts | C::Commentary | C::EventsAt | C::MentionsOf) => false,

        // Not-yet-earned kinds: every capability opted out. `Author`,
        // `Year`, `TimeAndPlace`, `PolityDelta` are `ParameterizedView`/
        // `NodeDesignate` (see `FocusBacking`) with no capability
        // governing them yet; `CommentaryItem`'s own prose is
        // `Section::Core(CoreSection::CommentaryItemProse)`, not a
        // capability (it is the TARGET end of `Commentary`, not a
        // frontier of its own); `ConcordUnit` is unchanged from the
        // pre-FQ-0 table (its real `Contains` rows are Concord-corpus,
        // not named by any capability here).
        (
            K::Author | K::Year | K::TimeAndPlace | K::PolityDelta
            | K::CommentaryItem | K::ConcordUnit,
            _,
        ) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE UNITY LAW: no capability exists without edges behind it — the
    /// visual frontier is the graph frontier filtered, never an invented
    /// sibling.
    #[test]
    fn capability_edges_are_never_empty() {
        for cap in Capability::ALL {
            assert!(!cap.edges().is_empty(), "capability {cap:?} has no edges behind it");
        }
    }

    /// `FocusBacking` is total by construction (the match is exhaustive);
    /// this PINS today's assignment per kind so a drift shows up here
    /// first, not as a silent behavior change downstream (owner order:
    /// "if that's not in code, it needs to be").
    #[test]
    fn focus_backing_is_pinned_per_kind() {
        use crate::id::NodeKind as G;
        use FocusBacking as B;
        let expected: &[(FocusKind, FocusBacking)] = &[
            (FocusKind::Verse, B::Node(G::TextUnit)),
            (FocusKind::Passage, B::Node(G::TextUnit)),
            (FocusKind::ConcordUnit, B::Node(G::TextUnit)),
            (FocusKind::Chapter, B::NodeDesignate(G::Container)),
            (FocusKind::Book, B::NodeDesignate(G::Container)),
            (FocusKind::Author, B::ParameterizedView),
            (FocusKind::Place, B::Node(G::Place)),
            (FocusKind::Person, B::Node(G::Person)),
            (FocusKind::Event, B::Node(G::Event)),
            (FocusKind::Catechism, B::Node(G::CatechismItem)),
            (FocusKind::Year, B::NodeDesignate(G::Anchor)),
            (FocusKind::TimeAndPlace, B::ParameterizedView),
            (FocusKind::PolityDelta, B::ParameterizedView),
            (FocusKind::CommentaryItem, B::Node(G::CommentaryItem)),
        ];
        assert_eq!(expected.len(), 14, "every FocusKind must be pinned exactly once -- totality check on the test itself");
        for (kind, want) in expected {
            assert_eq!(kind.backing(), *want, "{kind:?}'s backing() drifted from the pinned classification");
        }
    }

    /// `graph_kind()` must never surface a `NodeDesignate` or
    /// `ParameterizedView` as `Some` -- that was B4's exact defect
    /// (fictional cells a consumer could walk today).
    #[test]
    fn graph_kind_is_none_for_designates_and_parameterized_views() {
        for kind in [FocusKind::Chapter, FocusKind::Book, FocusKind::Year, FocusKind::Author,
                     FocusKind::TimeAndPlace, FocusKind::PolityDelta] {
            assert_eq!(kind.graph_kind(), None, "{kind:?} must not claim a real graph node yet");
        }
    }

    /// The owner's calibration row, pinned: an event frontier does not
    /// implement cross references (until event-level data exists — the
    /// day it does, this test is UPDATED alongside the one-line cell).
    #[test]
    fn event_has_no_cross_references() {
        assert!(!allows(FocusKind::Event, Capability::CrossReferences));
    }

    /// EVT-3's three event capabilities, pinned.
    #[test]
    fn event_implements_chronology_time_and_place_accounts() {
        assert!(allows(FocusKind::Event, Capability::Chronology));
        assert!(allows(FocusKind::Event, Capability::TimeAndPlace));
        assert!(allows(FocusKind::Event, Capability::Accounts));
    }

    /// Verse always carries its seven (six original + Commentary); never
    /// the event-family three, never the three kind-specific capabilities
    /// that belong to other focus kinds.
    #[test]
    fn verse_row_matches_the_owner_reviewed_matrix() {
        use Capability as C;
        for cap in [C::CrossReferences, C::Parallels, C::EventMembership,
                    C::PassageMembership, C::Persons, C::CatechismSupport, C::Commentary] {
            assert!(allows(FocusKind::Verse, cap), "Verse must allow {cap:?}");
        }
        for cap in [C::Chronology, C::TimeAndPlace, C::Accounts,
                    C::EventsAt, C::MentionsOf, C::Members] {
            assert!(!allows(FocusKind::Verse, cap), "Verse must not allow {cap:?}");
        }
    }

    /// FQ-0's three escapees (design review B5), pinned one per kind.
    #[test]
    fn escapee_capabilities_land_on_their_owner_kind_only() {
        assert!(allows(FocusKind::Place, Capability::EventsAt));
        assert!(allows(FocusKind::Person, Capability::MentionsOf));
        assert!(allows(FocusKind::Catechism, Capability::CatechismSupport));
        for kind in [FocusKind::Verse, FocusKind::Event, FocusKind::Chapter, FocusKind::Book] {
            assert!(!allows(kind, Capability::EventsAt), "{kind:?} must not claim EventsAt");
            assert!(!allows(kind, Capability::MentionsOf), "{kind:?} must not claim MentionsOf");
        }
    }

    /// NODE-1's target state (owner ruling), pinned now as policy even
    /// though the underlying rows are TODO(NODE-1) -- see
    /// `atlas-graph/tests/frontier_falsifiability.rs`.
    #[test]
    fn chapter_and_book_get_members_and_chronology() {
        for kind in [FocusKind::Chapter, FocusKind::Book] {
            assert!(allows(kind, Capability::Members), "{kind:?} must allow Members");
            assert!(allows(kind, Capability::Chronology), "{kind:?} must allow Chronology");
            assert!(!allows(kind, Capability::EventsAt), "{kind:?} must not allow EventsAt");
        }
    }
}
