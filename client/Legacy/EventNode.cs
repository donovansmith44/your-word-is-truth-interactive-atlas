using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

public interface INarrativeAware
{
    Task<NarrativeEventPositions> NarrativePositionsAsync(AtlasClient api);
}

public sealed class EventNode : IExplorable, INarrativeAware
{
    private readonly AsyncMemo<EventPage> _detail = new();
    private readonly AsyncMemo<NodeRecord> _card = new();
    private readonly AsyncMemo<NarrativeEventPositions> _positions = new();

    public EventNode(string eventId, string title)
    {
        EventId = eventId;
        Title = title;
    }

    public string EventId { get; }
    public string Title { get; }
    public string Kind => "Event";
    public NodeRef Identity => new(id: NodeIds.Of(NodeKind.Event, EventId), kind: NodeKind.Event, label: Title);

    public async Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api)
    {
        var detail = await DetailAsync(api);

        if (detail.When is not { } when || detail.Places.Count == 0)
        {
            return Array.Empty<Chip>();
        }

        IReadOnlyList<Chip> list = new[]
        {
            new Chip("Show on the map", "popover-chip-map",
                new ChipTarget.NavigateWorld($"from={when.FromYear}&to={when.ToYear}")),
        };
        return list;
    }

    // Unreachable while the Kind == "Event" section providers are registered;
    // kept only as the interface's required fallback.
    public Task<RenderFragment> BodyAsync(AtlasClient api) => Task.FromResult<RenderFragment>(_ => { });

    public Task<EventPage> DetailAsync(AtlasClient api) => _detail.Get(() => api.Event(EventId));

    public Task<NodeRecord> CardAsync(AtlasClient api) => _card.Get(() => api.NodeRecord(Identity.Id));

    public Task<NarrativeEventPositions> NarrativePositionsAsync(AtlasClient api) =>
        _positions.Get(() => api.NarrativeEventPositions(EventId));
}
