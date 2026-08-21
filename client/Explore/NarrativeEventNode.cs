using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch N ("narratives as first-class graph structure"): opted into by any
/// <see cref="IExplorable"/> node whose subject has a well-defined position
/// in the narrative graph (<see cref="VerseNode"/>, <see cref="NarrativeEventNode"/>).
/// <see cref="ExplorerPopover"/> reads this directly (a second, small,
/// node-specific special case alongside its own pre-existing ShowMiniMap
/// one -- see that file's own header comment) purely for MAP-FOCUS-SYNC
/// (requirement 3): which narrative(s)' arrows to bring into particular
/// focus, and which event id is "the current leg" (an arrow whose own
/// `from_event`/`to_event` matches gets the strongest emphasis). The SAME
/// data also drives the PRIOR/FOLLOWING section providers' own rendering
/// (Explore/PopoverSectionProviders.cs) -- one fetch (memoized on the node
/// instance), two consumers, never two derivations of "what narrative
/// position is this."
/// </summary>
public interface INarrativeAware
{
    Task<IReadOnlyList<NarrativePositionDto>> NarrativePositionsAsync(AtlasClient api);
}

/// <summary>
/// The traversal target for a PRIOR/FOLLOWING EVENT click -- "the popover
/// re-anchors to that event (its verses become the subject; its own
/// prior/following render) -- recursively, arbitrarily far, ending
/// naturally at the narrative's first/last event" (the brief, verbatim).
/// Constructed directly from a <see cref="NarrativeAdjacentEventDto"/> the
/// CURRENT node's own PRIOR/FOLLOWING section already fetched (no extra
/// round trip just to open one) -- carries just enough to render itself
/// (label + verse refs, resolved to full text and handed to the SAME
/// shared passage-list component every other verse list in this app uses)
/// and to resolve its OWN further prior/following, by EVENT ID
/// (<see cref="NarrativePositionsAsync"/>) -- requirement 1's own
/// "traversal steps resolve by event, not by re-searching verses" applies
/// here literally: some events carry zero verses at all (this app's own
/// server-side test fixtures do), so a verse-based re-lookup would have
/// nothing to click, but the event-id lookup always works regardless.
///
/// Locked to the ONE narrative it was reached through (<see cref="NarrativeId"/>)
/// -- if this same event also happens to be a leg of a DIFFERENT
/// narrative, that other narrative's own chain is not surfaced here, a
/// deliberate, disclosed scope choice (see the batch report's own
/// "one-graph equality" section): walking "PRIOR EVENT — The Exodus"
/// keeps walking the Exodus, never silently branching into an unrelated
/// narrative the event also happens to touch. The verse itself (clicking
/// one of this node's own passage entries) remains reachable to see EVERY
/// narrative it belongs to, same as opening any other verse.
/// </summary>
public sealed class NarrativeEventNode : IExplorable, INarrativeAware
{
    private readonly string _narrativeId;
    private readonly List<string> _vrefs;
    private List<NarrativePositionDto>? _cachedPositions;

    public NarrativeEventNode(string narrativeId, string narrativeName, string eventId, string eventLabel, IReadOnlyList<string> vrefs)
    {
        _narrativeId = narrativeId;
        NarrativeName = narrativeName;
        EventId = eventId;
        _vrefs = vrefs.ToList();
        Title = eventLabel;
    }

    public string Title { get; }
    public string Kind => "NarrativeEvent";
    public string NarrativeName { get; }
    public string EventId { get; }
    public IReadOnlyList<string> Vrefs => _vrefs;

    // No chips: this node has no single canonical ref/book/place of its own
    // to hang popover-chip-map/-book/-context off (its own verses may span
    // several books/chapters) -- same "conditional presence extends to
    // affordances too" reasoning CatechismNode.ExploreAsync already
    // established for the identical "no natural chip target" situation.
    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api) =>
        Task.FromResult<IReadOnlyList<Exploration>>(Array.Empty<Exploration>());

    // Never actually invoked: NarrativeEventTextSection (Explore/PopoverSectionProviders.cs)
    // always applies to Kind=="NarrativeEvent" and is registered, so
    // ExplorerPopover's own registry path renders this node's body instead
    // -- same "defensive, never-called-in-practice fallback" shape
    // TimeAndPlaceNode/PlaceNode's own BodyAsync already document for their
    // own registry-covered kinds.
    public Task<RenderFragment> BodyAsync(AtlasClient api) => Task.FromResult<RenderFragment>(_ => { });

    /// Memoized -- shared by NarrativeEventTextSection (the event's own main
    /// content) AND the Prior/Following section providers, mirrors
    /// <c>VerseNode.DetailAsync</c>'s own "one fetch per node instance"
    /// reasoning. Filtered down to THIS node's own locked narrative id (see
    /// this class's own header comment) -- the underlying event-id lookup
    /// may return positions for OTHER narratives too; this node only ever
    /// exposes its own.
    public async Task<IReadOnlyList<NarrativePositionDto>> NarrativePositionsAsync(AtlasClient api)
    {
        if (_cachedPositions is null)
        {
            var all = await api.NarrativeEventPositions(EventId);
            _cachedPositions = all.Where(p => p.NarrativeId == _narrativeId).ToList();
        }
        return _cachedPositions;
    }
}
