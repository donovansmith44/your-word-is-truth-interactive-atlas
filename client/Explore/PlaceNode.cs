using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// A place's activity across ALL recorded history (spec node table row
/// "Place": "Its events across time (-&gt; TimeAndPlace nodes), Show on
/// /world"; Task 15's WORLD-8 opens this from World's PlaceCard title).
/// Title/Kind/body and the one "Show on /world" exploration were already
/// correct as of Task 14; Task 15 wires the event rows themselves -- each is
/// a real button that pushes a fresh <see cref="TimeAndPlaceNode"/> for that
/// specific event via <see cref="OnSelectEvent"/>.
///
/// <see cref="OnSelectEvent"/> is a deliberate, narrow escape from
/// IExplorable's usual "ExplorerPopover renders every node identically"
/// promise -- documented here rather than hidden, and chosen over adding
/// another ExplorerPopover-side special case (alongside xrefs, and --
/// before O1, 2026-08-23, retired it -- ShowMiniMap): CONTRACT.md places no
/// fixed testid on "click an event row" the way it did for xrefs/mini-map,
/// so nothing forces this interaction out of the node's own BodyAsync the
/// way those were forced out. Also, unlike
/// xrefs (a toggleable, chip-gated list), WORLD-8 requires the event years
/// to be visible in the body IMMEDIATELY on open, with no chip click first
/// -- which only BodyAsync's own always-rendered content can satisfy.
/// ExplorerPopover.LoadCurrent sets this property immediately after making
/// this node Current, to a closure that pushes onto its own Stack.
/// </summary>
public sealed class PlaceNode : IExplorable
{
    private readonly string _placeId;
    private readonly string _placeName;
    private readonly int? _windowFrom;
    private readonly int? _windowTo;
    private PlaceDetail? _cached;

    /// <param name="windowFrom">
    /// Batch R requirement 3 (the PLACE section registry -- established/
    /// destroyed/blurb sections): the scene's own active time-mode window
    /// (null in scripture mode / when opened with no window in hand), so
    /// <see cref="DetailAsync"/>'s fetch resolves the SAME window-scoped
    /// blurb/display-name PlaceCard already shows for this place, per
    /// <c>AtlasClient.PlaceHistory</c>'s own doc comment. World.razor's
    /// OpenPlaceFromCard supplies this from the exact same
    /// <c>WindowFrom</c>/<c>WindowTo</c> values it already passes to
    /// PlaceCard. Established/destroyed dates are window-INDEPENDENT (still
    /// returned regardless), so a null window here only ever costs the blurb
    /// section, never the dates one.
    /// </param>
    public PlaceNode(string placeId, string placeName, int? windowFrom = null, int? windowTo = null)
    {
        _placeId = placeId;
        _placeName = placeName;
        _windowFrom = windowFrom;
        _windowTo = windowTo;
    }

    public string Title => _placeName;
    public string Kind => "Place";

    /// Batch G2 decision 3 (the descriptor seam): the minimal reconstruction
    /// key for this node -- <c>ExplorationDescriptor.Capture</c>'s own source
    /// for a Place descriptor, mirroring <see cref="EventNode.EventId"/>/
    /// <see cref="PersonNode.PersonId"/>'s own established "expose the raw id
    /// publicly, thread it through unchanged" pattern.
    public string PlaceId => _placeId;

    /// <summary>
    /// Wired by <c>ExplorerPopover.LoadCurrent</c> right after this node
    /// becomes Current; invoked by an event row's own onclick (see
    /// <see cref="BodyAsync"/>) with the <see cref="TimeAndPlaceNode"/> to
    /// push. Left null (a no-op click) if nobody ever wires it, e.g. a
    /// caller that renders this node's body outside a real ExplorerPopover.
    /// </summary>
    public Func<IExplorable, Task>? OnSelectEvent { get; set; }

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        // No single scripture ref or narrow time window is inherently "this
        // place's own" -- the whole-span default is the only /world query the
        // CONTRACT's from/to vocabulary can express for "everywhere this
        // place has ever appeared". Individual events get their OWN precise
        // NavigateWorld window via TimeAndPlaceNode's own chip once a row
        // below is clicked.
        IReadOnlyList<Exploration> list = new[]
        {
            new Exploration("Show on /world", "popover-chip-map",
                new ExplorationTarget.NavigateWorld("from=-4004&to=100")),
        };
        return Task.FromResult(list);
    }

    public async Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        var detail = await Load(api);
        var events = detail.Events;
        var placeName = _placeName;
        var select = OnSelectEvent;

        RenderFragment fragment = builder =>
        {
            var seq = 0;
            if (events.Count == 0)
            {
                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "popover-meta");
                builder.AddContent(seq++, "No recorded events.");
                builder.CloseElement();
                return;
            }

            foreach (var e in events)
            {
                var ev = e; // local copy -- captured per-row by the onclick closure below
                builder.OpenElement(seq++, "button");
                builder.AddAttribute(seq++, "type", "button");
                builder.AddAttribute(seq++, "class", "popover-event-row popover-event-row-button");
                builder.AddAttribute(seq++, "data-testid", $"place-event-{ev.Id}");
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(this, async () =>
                {
                    if (select is not null)
                    {
                        await select(new TimeAndPlaceNode(_placeId, placeName, ev.Id, ev.When, ev.Label, ev.VerseGroups));
                    }
                }));

                builder.OpenElement(seq++, "span");
                builder.AddAttribute(seq++, "class", "popover-event-label");
                builder.AddContent(seq++, ev.Label);
                builder.CloseElement();

                builder.OpenElement(seq++, "span");
                builder.AddAttribute(seq++, "class", "popover-event-years");
                builder.AddContent(seq++, YearText.FormatRange(ev.When.FromYear, ev.When.ToYear));
                builder.CloseElement();

                builder.CloseElement();
            }
        };
        return fragment;
    }

    // Public + memoized (mirrors VerseNode.DetailAsync/PassageNode.XrefsAsync's
    // own reasoning exactly) so Batch R's own PlaceDatesSection/
    // PlaceBlurbSection/PlaceEventsSection providers -- each of which need
    // THIS node's fetched PlaceDetail -- share one fetch rather than one each
    // (3-4x per popover open otherwise, for identical data). Batch R: reads
    // via AtlasClient.PlaceHistory (window-aware), not the older plain
    // AtlasClient.Place -- the SAME `/api/place/{id}` endpoint either way
    // (PlaceHistory just forwards the optional `?from=&to=`), so this is a
    // strict superset (adds `History`) of what the old call returned, not a
    // behavior change for `Events`.
    public async Task<PlaceDetail> DetailAsync(AtlasClient api) => _cached ??= await api.PlaceHistory(_placeId, _windowFrom, _windowTo);

    private Task<PlaceDetail> Load(AtlasClient api) => DetailAsync(api);
}
