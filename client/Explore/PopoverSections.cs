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
/// (<see cref="VerseEventMembershipSection"/>, appended at the end, same
/// "later batches append below" convention, conditional on the verse
/// touching >=1 titled EVENT-kind passage), each row explorable, opening a
/// fresh <see cref="EventNode"/>. EVENT node sections, in order: date +
/// place(s) (<see cref="EventDateAndPlacesSection"/>), PARALLEL ACCOUNTS
/// (<see cref="EventWitnessesSection"/>, conditional presence: no "PARALLEL
/// ACCOUNTS" framing at all when the event has exactly one witness),
/// PRIOR EVENT (<see cref="EventPriorSection"/>) / FOLLOWING EVENT
/// (<see cref="EventFollowingSection"/>, both retargeted from Batch N's own
/// Verse/NarrativeEvent onto Event only -- recursion falls out of an
/// EventNode's own traversal row pushing ANOTHER EventNode, the SAME
/// `AppliesTo` clause matching it too, not a second mechanism).
/// </summary>
public static class PopoverSectionRegistry
{
    public static readonly IReadOnlyList<IPopoverSectionProvider> Providers = new IPopoverSectionProvider[]
    {
        new VerseTextSectionProvider(),
        new CrossRefsSection(),
        new CatechismSeamSection(),
        new PlaceDescriptionSection(),
        new PlaceDatesSection(),
        new PlaceBlurbSection(),
        new PlaceEventsSection(),
        new CatechismTextSection(),
        new CatechismExplanationSection(),
        new CatechismWhereWrittenSection(),
        new CatechismScripturesSection(),
        // Batch T ("events as the narrative nodes"): VERSE's own new "EVENT"
        // membership section (appended at the end, same "later batches
        // append below, never disturb" layering PlaceCard.razor's own G1
        // narrative-traversal section already established) -- lands
        // strictly after cross-references/catechism (none of the
        // PLACE/CATECHISM-only providers above apply to Verse).
        new VerseEventMembershipSection(),
        // Batch T: EVENT node sections, in order -- date+places, PARALLEL
        // ACCOUNTS, then PRIOR/FOLLOWING (unchanged registration slot from
        // Batch N, just retargeted onto Event instead of Verse/NarrativeEvent).
        new EventDateAndPlacesSection(),
        new EventWitnessesSection(),
        new EventPriorSection(),
        new EventFollowingSection(),
        // Batch HOTFIX-4 requirement 1 ("whole-DAG chronological traversal"):
        // the GLOBAL-timeline counterparts to the two rows immediately
        // above -- registered directly after them (narrative primacy: the
        // narrative-scoped rows always render first, requirement 1
        // verbatim), independent of narrative membership, so a
        // narrative-less dated event gets these two instead, and a
        // narrative member gets BOTH pairs.
        new EventTimelinePriorSection(),
        new EventTimelineFollowingSection(),
        // Batch M ("the DAG grows a node type"): PolityDelta's own three
        // sections, in order -- event text, THE SCRIPTURES, grounding note
        // -- appended at the end, same "later batches append below, never
        // disturb" convention. None of the three's own AppliesTo overlaps
        // any earlier provider's (Kind == "PolityDelta" is brand new), so
        // registration order here is ALSO render order for this node kind.
        new PolityDeltaEventSection(),
        new PolityDeltaScripturesSection(),
        new PolityDeltaGroundingSection(),
        // Batch P (the extensibility proof): appended at the end, same
        // "later batches append below, never disturb" convention every
        // prior batch's own new provider already followed (Batch T's
        // VerseEventMembershipSection, Batch M's three PolityDelta
        // providers immediately above) -- registration order here is
        // BATCH-chronological, not grouped by node kind (VerseEventMembershipSection,
        // a VERSE-only provider, already sits far from the other
        // Verse/Passage providers above for the identical reason).
        // VersePersonsSection first (the verse's own entry point INTO
        // persons), PersonCardAndMentionsSection second (the destination
        // node's own popover) -- matching the brief's own framing order.
        new VersePersonsSection(),
        new PersonCardAndMentionsSection(),
    };
}
