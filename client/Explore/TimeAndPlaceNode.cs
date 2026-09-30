using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class TimeAndPlaceNode : IExplorable
{
    private readonly TimeRange _when;
    private readonly string _label;
    private readonly IReadOnlyList<VerseGroup> _verseGroups;

    public TimeAndPlaceNode(string placeId, string placeName, string eventId, TimeRange when, string label, IReadOnlyList<VerseGroup> verseGroups)
    {
        PlaceId = placeId;
        EventId = eventId;
        Title = $"{placeName}, {when.Label}";
        _when = when;
        _label = label;
        _verseGroups = verseGroups;
    }

    public string PlaceId { get; }
    public string EventId { get; }
    public string Title { get; }
    public string Kind => "TimeAndPlace";
    public Explorable Identity => new(NodeKind.Event, NodeIds.Of(NodeKind.Event, EventId), _label);

    public Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api)
    {
        IReadOnlyList<Chip> list = new[]
        {
            new Chip("Show on /world", "popover-chip-map",
                new ChipTarget.NavigateWorld($"from={_when.From.Value}&to={_when.To.Value}")),
        };
        return Task.FromResult(list);
    }

    public Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        RenderFragment fragment = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "popover-meta");
            builder.AddContent(seq++, _label);
            builder.CloseElement();

            foreach (var g in _verseGroups)
            {
                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "popover-event-row");
                builder.AddContent(seq++, $"{g.Book} {g.Chapter} — {g.Count} verse{(g.Count == 1 ? "" : "s")}");
                builder.CloseElement();
            }
        };
        return Task.FromResult(fragment);
    }
}
