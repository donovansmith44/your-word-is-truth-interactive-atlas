using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class YearNode : IExplorable
{
    private readonly TimeRange _when;
    private readonly IReadOnlyList<TextSpan> _verses;
    private readonly bool _isEventTime;

    public YearNode(string placeId, PlaceDate date)
    {
        PlaceId = placeId;
        Label = date.Label;
        Title = $"{date.Label} {date.Claim.Label}";
        _when = date.Claim.When;
        _verses = date.Claim.Verses;
        _isEventTime = false;
    }

    public YearNode(TimeRange when)
    {
        PlaceId = "";
        Label = "";
        Title = when.Label;
        _when = when;
        _verses = [];
        _isEventTime = true;
    }

    public string PlaceId { get; }
    public string Label { get; }
    public string Title { get; }
    public string Kind => "Year";

    public Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api)
    {
        var list = new List<Chip>();
        foreach (var span in _verses)
        {
            var reference = CanonRef.SpanOf(span);
            list.Add(new Chip(reference, $"popover-chip-verse-{reference}", new ChipTarget.Push(new VerseNode(CanonRef.VerseOf(CanonRef.FirstVerseOf(span))))));
        }

        list.Add(new Chip("Show this time on the map", "popover-chip-map",
            new ChipTarget.NavigateWorld($"from={_when.From.Value}&to={_when.To.Value}")));

        return Task.FromResult<IReadOnlyList<Chip>>(list);
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
            scene = await api.SceneTime(_when.From.Value, _when.To.Value);
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
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new EventNode(id, label, EventKind.Event))));
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
