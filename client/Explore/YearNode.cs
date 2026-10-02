using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class YearNode(TimeRange when, NodeRef @event) : IExplorable
{
    public string Title => when.Label;
    public string Kind => "Year";
    public NodeRef Identity => @event;

    public Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api) =>
        Task.FromResult<IReadOnlyList<Chip>>([new Chip("Show this time on the map", "popover-chip-map", new ChipTarget.NavigateWorld($"from={when.From.Value}&to={when.To.Value}"))]);

    public Task<RenderFragment> BodyAsync(AtlasClient api) => Task.FromResult<RenderFragment>(_ => { });

    public async Task<PopoverSection?> ResolveFrontierAsync(AtlasClient api, IPopoverSectionContext ctx)
    {
        Scene scene;
        try
        {
            scene = await api.SceneTime(when.From.Value, when.To.Value);
        }
        catch (Exception)
        {
            return null;
        }

        var events = DedupeAndOrder(scene.Places.SelectMany(p => p.Events));

        if (events.Count == 0)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "div");
            builder.AddAttribute(seq++, "class", "popover-year-chronology");
            builder.AddAttribute(seq++, "data-testid", "year-chronology");
            foreach (var e in events)
            {
                var id = e.Id;
                var label = e.Label;
                builder.OpenElement(seq++, "button");
                builder.AddAttribute(seq++, "type", "button");
                builder.AddAttribute(seq++, "class", "popover-event-row popover-event-row-button explorable");
                builder.AddAttribute(seq++, "data-testid", $"year-chronology-event-{id}");
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new PopoverOpening.Legacy(new EventNode(id, label)), EdgeKind.TemporalAdjacency)));
                builder.AddContent(seq++, label);
                builder.CloseElement();
            }
            builder.CloseElement();
        };
        return new PopoverSection("year-chronology", body);
    }

    public static List<SceneEvent> DedupeAndOrder(IEnumerable<SceneEvent> events) =>
        events
            .GroupBy(e => e.Id)
            .Select(g => g.First())
            .OrderBy(e => e.When.From.Value)
            .ThenBy(e => e.When.To.Value)
            .ThenBy(e => e.Label, StringComparer.Ordinal)
            .ToList();
}
