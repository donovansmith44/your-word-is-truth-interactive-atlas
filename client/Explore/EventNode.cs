using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch N ("narratives as first-class graph structure"), retargeted Batch T
/// (verse-level narrative-awareness retired -- see <see cref="VerseNode"/>'s
/// own doc comment): opted into by <see cref="EventNode"/>, the ONLY
/// remaining implementor. <see cref="ExplorerPopover"/> reads this directly
/// (a small, node-specific special case alongside its own pre-existing
/// ShowMiniMap one -- see that file's own header comment) purely for
/// MAP-FOCUS-SYNC: which narrative(s)' arrows to bring into particular
/// focus, and which event id is "the current leg." The SAME data also
/// drives the PRIOR/FOLLOWING section providers' own rendering
/// (Explore/PopoverSectionProviders.cs) -- one fetch (memoized on the node
/// instance), two consumers, never two derivations of "what narrative
/// position is this."
/// </summary>
public interface INarrativeAware
{
    Task<IReadOnlyList<NarrativePositionDto>> NarrativePositionsAsync(AtlasClient api);
}

/// <summary>
/// Batch T ("events as the narrative nodes"): an EVENT-kind PASSAGE --
/// "titled pericopes... explorable, part of the graph, have time and place
/// data, and you can traverse in time" (the owner's own words, verbatim).
/// Reached three ways: a reader-heading click (Reader.razor, req 5), a
/// verse popover's own "EVENT" membership row (req 3, PopoverSectionProviders.cs's
/// <c>VerseEventMembershipSection</c>), or a PRIOR/FOLLOWING traversal row
/// on THIS same node kind, recursively (req 2/4) -- the SAME "re-anchor the
/// popover onto the traversed event" mechanism Batch N's own (now retired)
/// <c>NarrativeEventNode</c> established, just richer: this node's own
/// popover carries title + date + place(s) + PARALLEL ACCOUNTS +
/// PRIOR/FOLLOWING + an Explore chip, not just a subject-text section.
///
/// Constructed with just <see cref="EventId"/> + <see cref="Title"/> (both
/// already in hand from wherever this node was reached -- a heading, an
/// event-membership row, an adjacent-event traversal row -- mirrors
/// <c>PlaceNode</c>/<c>CatechismNode</c>'s own "no fetch just to render the
/// header" discipline); <see cref="DetailAsync"/> lazily fetches the rest
/// (<c>GET /api/event/{id}</c>) once the popover actually opens.
/// </summary>
public sealed class EventNode : IExplorable, INarrativeAware
{
    private EventDetail? _cached;
    private IReadOnlyList<NarrativePositionDto>? _cachedPositions;

    public EventNode(string eventId, string title)
    {
        EventId = eventId;
        Title = title;
    }

    public string EventId { get; }
    public string Title { get; }
    public string Kind => "Event";

    /// <summary>
    /// Requirement 4's own "Explore (map -- the event's place(s)/scene;
    /// split view targets the right pane)" -- <see cref="ExplorationTarget.NavigateWorld"/>
    /// already gives split-mode-aware "apply to the atlas pane that's
    /// already showing, never a second full atlas" behavior for free
    /// (SPLIT-1's own NO-NESTED-POPUP note), same mechanism
    /// <c>TimeAndPlaceNode</c>/<c>PolityDeltaNode</c> already use for their
    /// own "Show on the map" chip. Bracketing window is this event's own
    /// `when` (a single year for most events; a genuine range for a few) --
    /// resolved lazily, alongside the popover body, via the SAME memoized
    /// <see cref="DetailAsync"/> every section provider reads.
    ///
    /// Batch T2: a `Kind == "general"` passage has no `When` (`null`, see
    /// <see cref="EventDetail"/>'s own doc comment) and, by construction,
    /// no places either -- there is no map scene to bracket, so this chip
    /// is simply absent (empty list, no error) for a general-kind passage.
    /// </summary>
    public async Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        EventDetail detail;
        try
        {
            detail = await DetailAsync(api);
        }
        catch (Exception)
        {
            return Array.Empty<Exploration>();
        }

        if (detail.When is not { } when)
        {
            return Array.Empty<Exploration>();
        }

        IReadOnlyList<Exploration> list = new[]
        {
            new Exploration("Show on the map", "popover-chip-map",
                new ExplorationTarget.NavigateWorld($"from={when.FromYear}&to={when.ToYear}")),
        };
        return list;
    }

    // Never actually invoked: the section-registry providers in
    // PopoverSectionProviders.cs (Kind == "Event") always apply and are
    // registered, so ExplorerPopover's own registry path renders this
    // node's body instead -- same "defensive, never-called-in-practice
    // fallback" shape TimeAndPlaceNode/PlaceNode/PolityDeltaNode already
    // document for their own registry-covered kinds.
    public Task<RenderFragment> BodyAsync(AtlasClient api) => Task.FromResult<RenderFragment>(_ => { });

    /// Memoized -- shared by every Kind=="Event" section provider (date,
    /// places, witnesses), mirrors <c>PlaceNode.DetailAsync</c>'s own "one
    /// fetch per node instance" reasoning.
    public async Task<EventDetail> DetailAsync(AtlasClient api) => _cached ??= await api.Event(EventId);

    /// Batch T requirement 2 (INarrativeAware): PRIOR/FOLLOWING now lives
    /// entirely on THIS node kind (Batch N's own verse-keyed half is
    /// retired -- see VerseDetail's own doc comment). Reuses the exact same
    /// event-id-keyed resolver <c>NarrativeEventNode</c> used to
    /// (`GET /api/narrative/event/{id}`, `atlas_core::narrative::positions_for_events`
    /// itself unchanged -- only this node replaces that one as its caller),
    /// so map-focus-sync (ExplorerPopover's own `SyncNarrativeFocusAsync`)
    /// keeps working unmodified, no ExplorerPopover change needed.
    public async Task<IReadOnlyList<NarrativePositionDto>> NarrativePositionsAsync(AtlasClient api) =>
        _cachedPositions ??= await api.NarrativeEventPositions(EventId);
}
