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
    };
}
