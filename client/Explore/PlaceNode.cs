using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class PlaceNode : IExplorable
{
    private readonly string _placeId;
    private readonly string _placeName;
    private readonly int? _windowFrom;
    private readonly int? _windowTo;
    private readonly AsyncMemo<PlacePage> _detail = new();
    private readonly AsyncMemo<NodeCard> _card = new();

    // A null window here only costs the blurb section; established/destroyed dates
    // are window-independent and are returned regardless.
    public PlaceNode(string placeId, string placeName, int? windowFrom = null, int? windowTo = null)
    {
        _placeId = placeId;
        _placeName = placeName;
        _windowFrom = windowFrom;
        _windowTo = windowTo;
    }

    public string Title => _placeName;
    public string Kind => "Place";

    public string PlaceId => _placeId;

    // Wired by ExplorerPopover.LoadCurrent right after this node becomes Current;
    // left null (a no-op click) if nobody wires it.
    public Func<IExplorable, Task>? OnSelectEvent { get; set; }

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
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
                var ev = e;
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
                builder.AddContent(seq++, ev.When.Label);
                builder.CloseElement();

                builder.CloseElement();
            }
        };
        return fragment;
    }

    // AsyncMemo-backed: shared by every Place-kind section provider so opening a
    // popover fires one /api/place/{id} request, not up to four.
    public Task<PlacePage> DetailAsync(AtlasClient api) => _detail.Get(() => api.PlaceHistory(_placeId, _windowFrom, _windowTo));

    public async Task<IReadOnlyList<PlaceDate>> DatesAsync(AtlasClient api) =>
        PlaceDates.Of((await CardAsync(api)).Place);

    private Task<NodeCard> CardAsync(AtlasClient api) => _card.Get(() => api.NodeCard(NodeIds.Of(NodeKind.Place, _placeId)));

    private Task<PlacePage> Load(AtlasClient api) => DetailAsync(api);
}
