using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// A place's activity across ALL recorded history (spec node table row
/// "Place": "Its events across time (-&gt; TimeAndPlace nodes), Show on
/// /world"; Task 15's WORLD-8 opens this from World's PlaceCard title).
/// Title/Kind/body and the one "Show on /world" exploration are correct and
/// complete here; clicking an individual event row to push a
/// TimeAndPlaceNode is Task 15's own wiring (no CONTRACT testid exists yet
/// for that row), so the event list below is informational only for now.
/// </summary>
public sealed class PlaceNode : IExplorable
{
    private readonly string _placeId;
    private readonly string _placeName;
    private PlaceDetail? _cached;

    public PlaceNode(string placeId, string placeName)
    {
        _placeId = placeId;
        _placeName = placeName;
    }

    public string Title => _placeName;
    public string Kind => "Place";

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        // No single scripture ref or narrow time window is inherently "this
        // place's own" -- the whole-span default is the only /world query the
        // CONTRACT's from/to vocabulary can express for "everywhere this
        // place has ever appeared". Task 15 may narrow this once it wires
        // real per-event interaction here.
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
                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "popover-event-row");
                builder.OpenElement(seq++, "span");
                builder.AddAttribute(seq++, "class", "popover-event-label");
                builder.AddContent(seq++, e.Label);
                builder.CloseElement();
                builder.OpenElement(seq++, "span");
                builder.AddAttribute(seq++, "class", "popover-event-years");
                builder.AddContent(seq++, YearText.FormatRange(e.When.FromYear, e.When.ToYear));
                builder.CloseElement();
                builder.CloseElement();
            }
        };
        return fragment;
    }

    private async Task<PlaceDetail> Load(AtlasClient api) => _cached ??= await api.Place(_placeId);
}
