using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch R requirement 3 ("the popover becomes the composable content
/// platform"): the ExplorerPopover's own back-channel into ITS stack, handed
/// to every <see cref="IPopoverSectionProvider"/> so a section can act on the
/// shared popover (drill into a new node) without <c>ExplorerPopover.razor</c>
/// growing a bespoke callback per provider -- the same reason
/// <c>ExplorationTarget</c> exists for chips, one level down. Implemented by
/// <c>ExplorerPopover</c> itself.
/// </summary>
public interface IPopoverSectionContext
{
    /// Pushes <paramref name="node"/> onto the popover's own breadcrumb
    /// stack and loads it -- the exact same drill-in behavior an
    /// <see cref="ExplorationTarget.Push"/> chip already gives, just
    /// reachable from inside a section's own body instead of only from the
    /// chips row.
    Task PushAsync(IExplorable node);

    /// <summary>
    /// Batch F2 requirement 6 (cross-reference truncation, "no more than 3
    /// if xrefs are the ONLY kind of context... no more than 2 if there are
    /// OTHER types of context (small catechism, etc.)"): how many OTHER
    /// resolved sections (excluding the verse-text section itself -- "the
    /// subject, not context" -- and the xrefs section's own count of
    /// itself) are present for the CURRENT node, at the moment this is
    /// read. Read from a <c>RenderFragment</c> at RENDER time (not
    /// captured into a local at <c>ResolveAsync</c> time, when sibling
    /// providers may not have resolved yet -- concurrent resolution means
    /// no provider can know its own siblings' outcomes during its own
    /// <c>ResolveAsync</c> call) -- by the time ANY section's fragment
    /// actually renders, every provider has already finished (<c>LoadCurrent</c>
    /// awaits the whole batch before the render that shows any of them), so
    /// this always reflects the FINAL section list. Generic by construction
    /// -- counts whatever is actually in the resolved section list, never a
    /// hardcoded "is catechism present" check -- so this keeps working
    /// unchanged when Batch P (or any later batch) adds a new provider.
    /// </summary>
    int OtherContextSectionCount { get; }

    /// <summary>
    /// Batch M-D2 (owner's cross-reference superscript directive, entry-
    /// point parameter): <c>true</c> exactly when the CURRENT node is a
    /// <c>VerseNode</c> pushed via its own xref superscript cluster/many-
    /// marker (<see cref="Explore.VerseNode.XrefEntryPoint"/>) rather than
    /// the ordinary verse line/verse-num. Read from a <c>RenderFragment</c>
    /// at RENDER time, same discipline as <see cref="OtherContextSectionCount"/>
    /// immediately above (both are read INSIDE <see cref="CrossRefsSection"/>'s
    /// own returned closure, never captured during the concurrent
    /// <c>ResolveAsync</c> phase) -- house pattern, non-negotiable per this
    /// batch's own brief ("the per-chapter superscript lettering is exactly
    /// the shape that bit Batch N"). A parameter on the ONE shared
    /// abstraction (owner decree: never a parallel interface) -- this
    /// property, plus <see cref="OtherContextSectionCount"/>, are the ONLY
    /// two inputs <c>CrossRefsSection</c>'s own cap decision reads.
    /// </summary>
    bool XrefEntryPoint { get; }

    /// <summary>
    /// Batch P (M-D2 ruling: "the verse popover's PERSONS section... the
    /// FIRST IExplorableClient consumer"): the generic graph client, handed
    /// to every provider through this SAME back-channel <see cref="PushAsync"/>
    /// already uses -- so a section reads the generic contract without
    /// <c>ExplorerPopover.razor</c> growing a bespoke per-provider
    /// dependency, and without widening every OTHER provider's own
    /// <c>ResolveAsync(node, api, ctx)</c> signature (which would touch
    /// ~20 unrelated providers for one new consumer). Implemented by
    /// <c>ExplorerPopover</c> itself, backed by its own <c>@inject
    /// IExplorableClient</c>.
    /// </summary>
    IExplorableClient Graph { get; }
}

/// <summary>
/// One resolved, ready-to-render section of a popover body. <see cref="Testid"/>
/// becomes the section's own wrapper <c>data-testid="popover-section-{Testid}"</c>
/// -- a stable hook for tests/CSS that exists independent of whatever content
/// a provider happens to render this time.
/// </summary>
public sealed record PopoverSection(string Testid, RenderFragment Body);

/// <summary>
/// Batch R requirement 3: one pluggable chunk of ExplorerPopover content.
/// "A node type maps to an ordered list of section providers; each provider
/// answers (has content? -&gt; render section)" -- <see cref="AppliesTo"/> is
/// the cheap, synchronous first half of that question (does this provider
/// even consider this NODE KIND at all); <see cref="ResolveAsync"/> is the
/// second, possibly-fetching half, returning null for "no content, no
/// section" (conditional presence throughout -- a seam with nothing
/// registered, or a provider whose own data is empty for this node, renders
/// NOTHING, not an empty placeholder).
///
/// Registering (or removing) a provider is the ENTIRE integration surface
/// Batch F (the small catechism) and Batch P (Theographic place descriptions)
/// need -- see <see cref="PopoverSectionRegistry"/>'s own doc comment for
/// exactly where each one's own future provider slots in. Neither
/// <c>ExplorerPopover.razor</c> nor any <c>IExplorable</c> node needs to
/// change for either batch to land its own content.
/// </summary>
public interface IPopoverSectionProvider
{
    bool AppliesTo(IExplorable node);

    Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx);
}

/// <summary>
/// The registry itself: one FLAT, ORDERED list of providers. ExplorerPopover
/// filters this list to whichever providers' own <see cref="IPopoverSectionProvider.AppliesTo"/>
/// says yes for the CURRENT node (preserving THIS list's own order), resolves
/// each, and renders every non-null result in that same order -- "registering/
/// removing a provider is the whole of adding/removing a kind of content,"
/// per the batch brief, verbatim.
///
/// Requirement 3's own explicit ordering, realized as registration order:
/// VERSE/PASSAGE get (a) the verse/passage itself with its own expand
/// affordance (requirement 4), (b) cross-references inline, (c) Batch F's
/// own "THE SMALL CATECHISM" seam (<see cref="CatechismSeamSection"/>,
/// filled -- see that class's own doc comment); PLACE gets (a) description
/// (Batch P fills this -- register its own provider REPLACING
/// PlaceDescriptionSection), (b) established/destroyed, (c) period blurb,
/// (d) events. CATECHISM (Batch F's own new node kind, reached by pushing a
/// citing item from the seam above) gets, in this order: (a) the item's own
/// text (conditional -- absent for Baptism/Confession/Sacrament-of-the-Altar
/// items, see <see cref="CatechismTextSection"/>), (b) the explanation under
/// Luther's own verbatim heading, (c) "Where is this written?" (conditional),
/// (d) "THE SCRIPTURES" -- the item's own proof verses, each explorable
/// (conditional). "Explore" (the map affordance) is deliberately NOT one of
/// these providers -- it stays the pre-existing <c>popover-chip-map</c>
/// chip, rendered by ExplorerPopover's own already-working chips row below
/// every section, unchanged by this batch (see ExplorerPopover.razor's own
/// comment on why) -- moot for CATECHISM nodes regardless, since
/// <c>CatechismNode.ExploreAsync</c> offers no chips at all (no geography).
///
/// A node kind no provider here claims at all (Chapter/Book/Author/
/// TimeAndPlace/Year) falls back to that node's own <c>BodyAsync</c> --
/// ExplorerPopover's pre-Batch-R rendering path, untouched -- so none of
/// those five kinds' popovers change shape from this batch.
///
/// Batch N ("narratives as first-class graph structure") originally added
/// two providers to VERSE's own list plus a NarrativeEvent traversal target
/// -- Batch T ("events as the narrative nodes") RETIRES both from Verse
/// (the owner, verbatim: "rather than putting the next/previous event on
/// every verse, add titles of events... traversal lives on event nodes")
/// and replaces NarrativeEventNode with a richer EVENT node kind. VERSE
/// gains ONE new provider instead -- "EVENT" membership
/// (<see cref="VerseEventMembershipSection"/>, conditional on the verse
/// touching >=1 titled EVENT-kind passage), each row explorable, opening a
/// fresh <see cref="EventNode"/>.
///
/// M-D3/U6, owner verbatim order (progress.md): "Header / Verse (focus) /
/// Event / Parallels / Small Catechism / cross references LAST." VERSE (and
/// PASSAGE, where the same provider also applies) sections now render in
/// EXACTLY this order: the verse's own text
/// (<see cref="VerseTextSectionProvider"/>, "focus"), "EVENT" membership
/// (<see cref="VerseEventMembershipSection"/>, Verse-only -- unchanged
/// scope note below), "PARALLELS" (<see cref="VerseParallelsSection"/>,
/// NEW this batch -- other witnesses of an event the verse belongs to, a
/// quick peek without a click into the EVENT node first; see that class's
/// own doc comment), "THE SMALL CATECHISM"
/// (<see cref="CatechismSeamSection"/>, now capped to 2 shown + U2's
/// shared reveal mechanic -- previously unconditional/uncapped), and
/// cross-references LAST (<see cref="CrossRefsSection"/>, moved from its
/// former 2nd slot). ("PERSONS" -- <see cref="VersePersonsSection"/> --
/// used to sit here too, between Parallels and Catechism, the controller's
/// own "Persons then Places" reconciliation; O4, 2026-08-23, owner
/// live-preview correction, unregistered it -- "remove persons from hover
/// menus for now," machinery retained, not deleted -- see the registry's
/// own comment below for the fuller story.) EVENT node sections, in order: date + place(s) +
/// narrative prior/following nav (<see cref="EventDateAndPlacesSection"/>
/// -- M-D3/U1 folds the narrative traversal arrows, formerly their own two
/// Batch N/T sections, directly into this one, "immediately below focus"
/// the owner's own words; see that class's own doc comment for the full
/// story), PARALLEL ACCOUNTS (<see cref="EventWitnessesSection"/>,
/// conditional presence: no "PARALLEL ACCOUNTS" framing at all when the
/// event has exactly one witness) -- recursion falls out of an EventNode's
/// own traversal arrow pushing ANOTHER EventNode, the SAME `AppliesTo`
/// clause matching it too, not a second mechanism.
/// </summary>
public static class PopoverSectionRegistry
{
    public static readonly IReadOnlyList<IPopoverSectionProvider> Providers = new IPopoverSectionProvider[]
    {
        // M-D3 (U4/B3): CHAPTER's own metadata-and-context card -- a
        // brand-new node kind (Kind == "Chapter"), so its registration
        // position doesn't interleave with any VERSE/PASSAGE/PLACE/EVENT/
        // CATECHISM/PolityDelta/Person provider's own AppliesTo above or
        // below it; listed first only because it is the FIRST node kind
        // alphabetically among this file's own section headers, not because
        // order matters here.
        new ChapterCardSection(),
        // M-D3/U6, owner verbatim order (progress.md): "Header / Verse
        // (focus) / Event / Parallels / Small Catechism / cross references
        // LAST." Every VERSE/PASSAGE-applicable provider below is now
        // listed in exactly this order (registration order IS render order
        // for same-kind providers, REGISTRY-1's own standing rule) --
        // REPLACES the pre-M-D3 order (xrefs 2nd, catechism 3rd, EVENT
        // membership appended at the very end). The PLACE/CATECHISM-DETAIL/
        // EVENT/PolityDelta/Person-card providers interleaved below never
        // apply to Verse or Passage, so their own position relative to
        // these six has no effect on VERSE/PASSAGE render order; they stay
        // where each was originally registered.
        new VerseTextSectionProvider(),
        // Batch T ("events as the narrative nodes"): VERSE-only (a
        // shift-click passage span's own per-verse event membership is
        // genuinely ambiguous in a way a single verse never is -- REGISTRY-1's
        // own standing scope note, unchanged).
        new VerseEventMembershipSection(),
        // M-D3/U6 (NEW this batch): "PARALLELS" -- other witnesses of an
        // event the current verse belongs to, a quick peek without a click
        // into the EVENT node first. Verse OR Passage (first-verse-anchored
        // for a Passage, the SAME convention VersePersonsSection immediately
        // below already establishes) -- see VerseParallelsSection's own doc
        // comment for the full conditional-presence rule.
        new VerseParallelsSection(),
        // Batch P: Persons -- the verse's own entry point INTO persons
        // mentioned at this locus (moved here from its former place far
        // below, near PersonCardAndMentionsSection -- U6's own "Persons
        // then Places slot AFTER Parallels BEFORE Catechism" reconciliation;
        // no VERSE-scoped "Places" section exists anywhere in this codebase
        // today to also reposition -- in-text place mentions are a
        // completely different mechanism, Explore/PlaceMentions.cs's own
        // hover-blink, not a popover section -- so only Persons actually
        // moves here).
        //
        // O4 (owner live-preview correction, 2026-08-23: "remove persons
        // from hover menus for now") UNREGISTERS VersePersonsSection --
        // `new VersePersonsSection()` used to sit on the very next line.
        // The class itself is NOT deleted: the ruling's own words ("removed
        // for now... machinery retained") are a deliberate, disclosed
        // exception to dead-code law, distinct from O1's own full removal a
        // few lines above this file's own history -- see
        // Explore/PopoverSectionProviders.cs's own VersePersonsSection for
        // the still-intact class. In-text person mentions (M-D3/U5,
        // Reader.razor's own verse-mention-person-* spans, PersonNode/
        // PersonCardAndMentionsSection below) are a DIFFERENT affordance,
        // not named by the order, and stay fully live -- see
        // reader-persons.spec.ts's own header comment for how its coverage
        // moved to that surviving entry path.
        new CatechismSeamSection(),
        // Cross-references LAST among Verse/Passage sections, per the
        // owner's own explicit ordering -- moved from its former 2nd slot.
        new CrossRefsSection(),
        new PlaceDescriptionSection(),
        new PlaceDatesSection(),
        new PlaceBlurbSection(),
        new PlaceEventsSection(),
        new CatechismTextSection(),
        new CatechismExplanationSection(),
        new CatechismWhereWrittenSection(),
        new CatechismScripturesSection(),
        // Batch T, M-D3/U1: EVENT node sections, in order -- date+places+
        // narrative-nav (one merged section now, see EventDateAndPlacesSection's
        // own doc comment), then PARALLEL ACCOUNTS.
        new EventDateAndPlacesSection(),
        new EventWitnessesSection(),
        // TRAV-1 (controller decisions 2+3): the GLOBAL-chronology
        // counterpart to the narrative nav folded into
        // EventDateAndPlacesSection above -- registered directly after the
        // sections above (narrative primacy preserved: the narrative-scoped
        // nav always renders first, same registration-order-is-render-order
        // rule HOTFIX-4 originally established here, unaffected by this
        // batch's own consolidation of what used to be its own TWO
        // providers), independent of narrative membership, so a
        // narrative-less dated event gets this ONE Chronology block
        // instead, and a narrative member gets BOTH blocks. RETIRES
        // HOTFIX-4's own EventTimelinePriorSection/EventTimelineFollowingSection
        // (this single line used to be two) -- see EventChronologySection's
        // own doc comment for the full "one Chronological block" story.
        new EventChronologySection(),
        // Batch M ("the DAG grows a node type"): PolityDelta's own three
        // sections, in order -- event text, THE SCRIPTURES, grounding note
        // -- appended at the end, same "later batches append below, never
        // disturb" convention. None of the three's own AppliesTo overlaps
        // any earlier provider's (Kind == "PolityDelta" is brand new), so
        // registration order here is ALSO render order for this node kind.
        new PolityDeltaEventSection(),
        new PolityDeltaScripturesSection(),
        new PolityDeltaGroundingSection(),
        // Batch P (the extensibility proof): the PERSON node's own card +
        // mentioned-in frontier -- appended at the end, same "later batches
        // append below, never disturb" convention every prior batch's own
        // new provider already followed. VersePersonsSection itself
        // (Batch P's OTHER half -- the verse's own entry point INTO
        // persons) no longer lives here -- M-D3/U6 moved it up to sit
        // between VerseParallelsSection and CatechismSeamSection, above.
        new PersonCardAndMentionsSection(),
    };
}
