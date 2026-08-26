using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// One event at a place-and-time (spec node table row "TimeAndPlace": "The
/// verses there-and-then (-&gt; Verse nodes), Show on /world"). Task 15's own
/// brief text pins the shape exactly: Title = "{place name}, {YearText
/// range}", body = the event's verse list, exploration = NavigateWorld to
/// that event's own window. Constructed by whoever already has the event in
/// hand (Task 15's PlaceNode row click) -- no AtlasClient fetch needed here.
///
/// Batch G2 decision 3: <see cref="PlaceId"/>/<see cref="EventId"/> are a
/// small, additive constructor widening -- neither was stored anywhere
/// before this batch (the live-click callers already had both in hand, just
/// never threaded through) -- so <c>ExplorationDescriptor.Capture</c> has a
/// stable composite key ("{placeId}|{eventId}") to reconstruct this node
/// from later, via <c>AtlasClient.PlaceHistory(placeId)</c> + a lookup of
/// the matching <see cref="SceneEvent.Id"/> in its own <c>Events</c> list --
/// see that class's own doc comment. Both call sites (<c>PlaceNode.BodyAsync</c>'s
/// own event-row click, <c>PlaceEventsList.razor</c>'s identical one) already
/// have the enclosing place's own id and the row's own <see cref="SceneEvent.Id"/>
/// on hand, so this costs neither call site a new fetch.
/// </summary>
public sealed class TimeAndPlaceNode : IExplorable
{
    private readonly TimeRangeDto _when;
    private readonly string _label;
    private readonly List<VerseGroup> _verseGroups;

    public TimeAndPlaceNode(string placeId, string placeName, string eventId, TimeRangeDto when, string label, List<VerseGroup> verseGroups)
    {
        PlaceId = placeId;
        EventId = eventId;
        Title = $"{placeName}, {YearText.FormatRange(when.FromYear, when.ToYear)}";
        _when = when;
        _label = label;
        _verseGroups = verseGroups;
    }

    public string PlaceId { get; }
    public string EventId { get; }
    public string Title { get; }
    public string Kind => "TimeAndPlace";

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        IReadOnlyList<Exploration> list = new[]
        {
            new Exploration("Show on /world", "popover-chip-map",
                new ExplorationTarget.NavigateWorld($"from={_when.FromYear}&to={_when.ToYear}")),
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
