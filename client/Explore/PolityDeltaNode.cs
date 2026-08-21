using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch M requirement 4 ("the DAG grows a node type"): one Scripture-mapped
/// era-boundary delta -- a polity's rise, an internal transition, or its
/// fall -- reached by clicking (or Enter-activating) a delta-eligible
/// border ring on the map (World.razor's own <c>OnPolityDeltaClick</c>,
/// map.js's <c>BorderLayer._makeDeltaHit</c>).
///
/// Every field is handed in directly at construction -- map.js's own
/// in-memory polity roster already resolved them all (no server-side
/// lookup-by-id needed), the same "never a fetch just to render the popover
/// header" discipline every other <see cref="IExplorable"/> node's
/// synchronous <see cref="Title"/>/<see cref="Kind"/> already follows (e.g.
/// <c>CatechismNode</c>, <c>TimeAndPlaceNode</c>).
///
/// <see cref="Title"/> is "{polity name}, {fromYear} -&gt; {toYear}" -- the
/// batch brief's own worked example, verbatim ("Israel, 931 BC -&gt; 722 BC"),
/// using a right arrow (a CONTRACT.md amendment, same-commit, disclosed in
/// the batch report: a delta describes a DIRECTIONAL change, not a static
/// span, so it deliberately does NOT reuse CONTRACT's general spaced-en-dash
/// Range format) -- ALWAYS present, even for the minimal-popover case (no
/// event/verses/ref_note authored): "an uneventful boundary stays visible
/// but gets the minimal popover" means minimal, not absent.
///
/// Registered content lives entirely in the section registry (three
/// providers in Explore/PopoverSectionProviders.cs, Kind == "PolityDelta"):
/// the event meta line, THE SCRIPTURES (via the shared PassageList
/// component), and the grounding note -- each independently conditional on
/// <see cref="EventText"/>/<see cref="Verses"/>/<see cref="RefNote"/> being
/// present. <see cref="BodyAsync"/> below is the interface's required
/// fallback, never actually reached (a PolityDelta node always resolves
/// through the registry) -- same "survives, unused" shape <c>PlaceNode</c>/
/// <c>CatechismNode</c> already establish.
/// </summary>
public sealed class PolityDeltaNode : IExplorable
{
    public string PolityName { get; }
    public string DeltaKind { get; } // "transition" | "fall"
    public int FromYear { get; }
    public int ToYear { get; }
    public string? EventText { get; }
    public IReadOnlyList<string> Verses { get; }
    public string? RefNote { get; }

    public string Title { get; }
    public string Kind => "PolityDelta";

    public PolityDeltaNode(string polityName, string deltaKind, int fromYear, int toYear, string? eventText, IReadOnlyList<string> verses, string? refNote)
    {
        PolityName = polityName;
        DeltaKind = deltaKind;
        FromYear = fromYear;
        ToYear = toYear;
        EventText = eventText;
        Verses = verses;
        RefNote = refNote;
        Title = $"{polityName}, {YearText.Format(fromYear)} → {YearText.Format(toYear)}";
    }

    /// <summary>
    /// Requirement 4's own "Explore (map affordance; in split view targets
    /// the right pane)" -- <see cref="ExplorationTarget.NavigateWorld"/>
    /// already gives split-mode-aware "apply to the atlas pane that's
    /// already showing, never a second full atlas" behavior for free
    /// (SPLIT-1's own NO-NESTED-POPUP note), same mechanism
    /// <c>TimeAndPlaceNode</c> already uses for its own "Show on /world"
    /// chip -- no special-casing needed here. Always offered, even for the
    /// minimal-popover case (the window itself is still worth jumping to).
    /// </summary>
    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        IReadOnlyList<Exploration> list = new[]
        {
            new Exploration("Show on the map", "popover-chip-map", new ExplorationTarget.NavigateWorld($"from={FromYear}&to={ToYear}")),
        };
        return Task.FromResult(list);
    }

    public Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        RenderFragment fragment = _ => { };
        return Task.FromResult(fragment);
    }
}
