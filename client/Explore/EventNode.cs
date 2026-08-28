using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch N ("narratives as first-class graph structure"), retargeted Batch T
/// (verse-level narrative-awareness retired -- see <see cref="VerseNode"/>'s
/// own doc comment): opted into by <see cref="EventNode"/>, the ONLY
/// remaining implementor. <see cref="ExplorerPopover"/> reads this directly
/// (a small, node-specific special case -- see that file's own header
/// comment, which also covers the ShowMiniMap one this used to sit
/// alongside before O1, 2026-08-23, retired it) purely for
/// MAP-FOCUS-SYNC: which narrative(s)' arrows to bring into particular
/// focus, and which event id is "the current leg." The SAME data also
/// drives the PRIOR/FOLLOWING section providers' own rendering
/// (Explore/PopoverSectionProviders.cs) -- one fetch (memoized on the node
/// instance), two consumers, never two derivations of "what narrative
/// position is this."
/// </summary>
public interface INarrativeAware
{
    Task<NarrativeEventPositionsResult> NarrativePositionsAsync(AtlasClient api);
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
    private NarrativeEventPositionsResult? _cachedPositions;

    // PERI-1 fix round 1 (review S-1a/Q-1a, CRITICAL -- the review's own
    // recommended fix (b)): the FIRST draft of CachedKind (below) forwarded
    // ONLY `_cached?.Kind`, populated solely by DetailAsync -- but
    // ExplorationDescriptor.Capture runs SYNCHRONOUSLY, inside Visit.Apply,
    // BEFORE PushAsync's own `await LoadCurrent()` ever gets a chance to
    // resolve that fetch (ExplorerPopover.razor's own PushAsync: `Visit`
    // dispatches, THEN LoadCurrent awaits) -- so for a freshly-clicked
    // EventNode (every real owner-repro click), `_cached` is always still
    // null at the exact instant Capture reads it. That made the trail
    // badge fix a no-op in the common case, not a rare race -- see this
    // batch's fix-round report section for the full trace. Fixed here,
    // not by making Capture async (architecturally awkward -- Visit.Apply
    // is a pure, synchronous IIntent<FocusStack>): every construction site
    // that already KNOWS this event's own `Event::kind` at construction
    // time (a VerseEventDto/HeadingDto row -- the wire already carries
    // `kind` right there, no fetch needed) now passes it in as
    // `knownKind`, so CachedKind is correct from the FIRST synchronous
    // read, never dependent on a fetch racing a capture. A construction
    // site that genuinely does NOT know the kind yet (ArrowNav's own
    // PRIOR/FOLLOWING traversal reads a structurally-guaranteed-dated
    // `NarrativeAdjacentEventDto`, which carries no `Kind` field at all --
    // see that call site's own comment for why passing the literal
    // `"event"` there is a structural fact, not a guess) still falls back
    // to the old fetch-populated path once DetailAsync resolves.
    private readonly string? _knownKind;

    public EventNode(string eventId, string title, string? knownKind = null)
    {
        EventId = eventId;
        Title = title;
        _knownKind = knownKind;
    }

    public string EventId { get; }
    public string Title { get; }
    public string Kind => "Event";

    /// <summary>
    /// Batch PERI-1 (PRESENTATION CATEGORY LAW), fix round 1 (see
    /// <see cref="_knownKind"/>'s own doc comment above for why this reads
    /// TWO sources, not one): <see cref="_cached"/>'s own
    /// <see cref="EventDetail.Kind"/> once a fetch has actually resolved
    /// (authoritative -- always wins when present), falling back to
    /// <see cref="_knownKind"/> (the caller-supplied, already-on-the-wire
    /// value) when no fetch has happened yet. Either source is "event" |
    /// "general", the DATA's own classification -- distinct from this
    /// class's own <see cref="Kind"/> above, the CLIENT's structural
    /// node-type tag; see EVENT-1's own KIND-AGNOSTIC note, CONTRACT.md.
    /// Null only for the residual construction sites that neither know the
    /// kind up front NOR have resolved a fetch yet (e.g.
    /// <see cref="ExplorationDescriptor.Reconstruct"/>'s "Event" case,
    /// which DOES seed `knownKind` from the saved descriptor's own
    /// `IsGeneralKind` -- so this is genuinely rare in practice, not the
    /// deterministic gap fix round 1 closes).
    /// <see cref="ExplorationDescriptor.Capture"/> reads this to label a
    /// saved-trail row "Passage" instead of "Event" for a general-kind
    /// entry.
    /// </summary>
    public string? CachedKind => _cached?.Kind ?? _knownKind;

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
    /// keeps working unmodified, no ExplorerPopover change needed. Batch
    /// HOTFIX-4 requirement 1: the SAME single memoized fetch now also
    /// carries the global-timeline half (`.Timeline`) -- one network call,
    /// two consumers today (map-focus-sync reads `.Narrative`;
    /// EventDateAndPlacesSection reads `.Narrative`, EventChronologySection
    /// reads `.Timeline` -- TRAV-1, PopoverSectionProviders.cs).
    public async Task<NarrativeEventPositionsResult> NarrativePositionsAsync(AtlasClient api) =>
        _cachedPositions ??= await api.NarrativeEventPositions(EventId);
}
