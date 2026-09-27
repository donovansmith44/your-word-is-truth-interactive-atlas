using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class YearNode : IExplorable
{
    private readonly TimeRangeDto _when;
    private readonly List<string> _verses;
    private readonly bool _isEventTime;

    public YearNode(string placeId, string label, TimeRangeDto when, List<string> verses, string? note)
    {
        PlaceId = placeId;
        Label = label;
        Title = $"{label} {YearText.FormatClaim(when.FromYear, when.ToYear, note)}";
        _when = when;
        _verses = verses;
        _isEventTime = false;
    }

    public YearNode(TimeRangeDto when)
    {
        PlaceId = "";
        Label = "";
        Title = YearText.FormatRange(when.FromYear, when.ToYear);
        _when = when;
        _verses = new List<string>();
        _isEventTime = true;
    }

    public string PlaceId { get; }
    public string Label { get; }
    public string Title { get; }
    public string Kind => "Year";

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        var list = new List<Exploration>();
        foreach (var vref in _verses)
        {
            var v = vref;
            list.Add(new Exploration(v, $"popover-chip-verse-{v}", new ExplorationTarget.Push(new VerseNode(v))));
        }

        list.Add(new Exploration("Show this time on the map", "popover-chip-map",
            new ExplorationTarget.NavigateWorld($"from={_when.FromYear}&to={_when.ToYear}")));

        return Task.FromResult<IReadOnlyList<Exploration>>(list);
    }

    // Never actually invoked once YearFrontierSection is registered for Kind == "Year" --
    // ExplorerPopover only falls back to BodyAsync when no registered provider claims the
    // node's Kind. Kept intact as a defensive fallback rather than deleted as dead code.
    public Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        var count = _verses.Count;
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-meta");
            builder.AddContent(2, count == 1 ? "1 supporting verse." : $"{count} supporting verses.");
            builder.CloseElement();
        };
        return Task.FromResult(fragment);
    }

    public Task<PopoverSection?> ResolveFrontierAsync(AtlasClient api, IPopoverSectionContext ctx)
    {
        if (_isEventTime)
        {
            return ResolveChronologyAsync(api, ctx);
        }

        var count = _verses.Count;
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-meta");
            builder.AddContent(2, count == 1 ? "1 supporting verse." : $"{count} supporting verses.");
            builder.CloseElement();
        };
        return Task.FromResult<PopoverSection?>(new PopoverSection("year-body", fragment));
    }

    private async Task<PopoverSection?> ResolveChronologyAsync(AtlasClient api, IPopoverSectionContext ctx)
    {
        Scene scene;
        try
        {
            scene = await api.SceneTime(_when.FromYear, _when.ToYear);
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
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new EventNode(id, label, "event"))));
                builder.AddContent(seq++, label);
                builder.CloseElement();
            }
            builder.CloseElement();
        };
        return new PopoverSection("year-chronology", body);
    }

    // Dedupes by id (an event attested at more than one place in this same window is one row,
    // never one per place), then orders by When.FromYear/ToYear with label as the final
    // tiebreak. Two events sharing the identical When range have no finer wire signal to
    // order by (SceneEvent carries no day/month/sequence field), so those still fall back
    // to alphabetical -- a real improvement over pure-alphabetical, not true intra-year
    // sequencing.
    public static List<SceneEvent> DedupeAndOrder(IEnumerable<SceneEvent> events) =>
        events
            .GroupBy(e => e.Id)
            .Select(g => g.First())
            .OrderBy(e => e.When.FromYear)
            .ThenBy(e => e.When.ToYear)
            .ThenBy(e => e.Label, StringComparer.Ordinal)
            .ToList();
}
