using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class PolityDeltaNode : IExplorable
{
    public string PolityId { get; }
    public string PolityName { get; }
    public string DeltaKind { get; } // "transition" | "fall"
    public int FromYear { get; }
    public int ToYear { get; }
    public string? EventText { get; }
    public IReadOnlyList<string> Verses { get; }
    public string? RefNote { get; }

    public string Title { get; }
    public string Kind => "PolityDelta";

    public PolityDeltaNode(string polityId, string polityName, string deltaKind, int fromYear, int toYear, string? eventText, IReadOnlyList<string> verses, string? refNote)
    {
        PolityId = polityId;
        PolityName = polityName;
        DeltaKind = deltaKind;
        FromYear = fromYear;
        ToYear = toYear;
        EventText = eventText;
        Verses = verses;
        RefNote = refNote;
        Title = $"{polityName}, {YearText.Format(fromYear)} → {YearText.Format(toYear)}";
    }

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        IReadOnlyList<Exploration> list = new[]
        {
            new Exploration("Show on the map", "popover-chip-map", new ExplorationTarget.NavigateWorld($"from={FromYear}&to={ToYear}")),
        };
        return Task.FromResult(list);
    }

    public Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        RenderFragment fragment = _ => { };
        return Task.FromResult(fragment);
    }
}
