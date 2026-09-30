using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public interface INarrativeAware
{
    Task<NarrativeEventPositions> NarrativePositionsAsync(AtlasClient api);
}

public sealed class EventNode : IExplorable, INarrativeAware
{
    private readonly AsyncMemo<EventPage> _detail = new();
    private readonly AsyncMemo<NodeCard> _card = new();
    private readonly AsyncMemo<NarrativeEventPositions> _positions = new();

    // Falls back to this caller-supplied kind when no fetch has resolved yet:
    // ExplorationDescriptor.Capture reads CachedKind synchronously, before
    // PushAsync's own await ever gets a chance to resolve DetailAsync, so a
    // freshly-clicked node would otherwise always report a null kind.
    private readonly EventKind? _knownKind;

    public EventNode(string eventId, string title, EventKind? knownKind = null)
    {
        EventId = eventId;
        Title = title;
        _knownKind = knownKind;
    }

    public string EventId { get; }
    public string Title { get; }
    public string Kind => "Event";

    public EventKind? CachedKind => _detail.CompletedValueOrDefault?.Kind ?? _knownKind;

    public async Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        EventPage detail;
        try
        {
            detail = await DetailAsync(api);
        }
        catch (Exception)
        {
            return Array.Empty<Exploration>();
        }

        if (detail.When is not { } when || detail.Places.Count == 0)
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

    // Unreachable while the Kind == "Event" section providers are registered;
    // kept only as the interface's required fallback.
    public Task<RenderFragment> BodyAsync(AtlasClient api) => Task.FromResult<RenderFragment>(_ => { });

    public Task<EventPage> DetailAsync(AtlasClient api) => _detail.Get(() => api.Event(EventId));

    public Task<NodeCard> CardAsync(AtlasClient api) => _card.Get(() => api.NodeCard(NodeIds.Of(NodeKind.Event, EventId)));

    public Task<NarrativeEventPositions> NarrativePositionsAsync(AtlasClient api) =>
        _positions.Get(() => api.NarrativeEventPositions(EventId));
}
