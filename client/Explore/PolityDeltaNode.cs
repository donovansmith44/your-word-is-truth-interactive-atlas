using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class PolityDeltaNode : IExplorable
{
    public string PolityId { get; }
    public string PolityName { get; }
    public string DeltaKind { get; } // "transition" | "fall"
    public Year From { get; }
    public Year To { get; }
    public string? EventText { get; }
    public IReadOnlyList<string> Verses { get; }
    public string? RefNote { get; }

    public string Title { get; }
    public string Kind => "PolityDelta";

    public PolityDeltaNode(string polityId, string polityName, string deltaKind, Year from, Year to, string? eventText, IReadOnlyList<string> verses, string? refNote)
    {
        PolityId = polityId;
        PolityName = polityName;
        DeltaKind = deltaKind;
        From = from;
        To = to;
        EventText = eventText;
        Verses = verses;
        RefNote = refNote;
        Title = $"{polityName}, {from.Label} → {to.Label}";
    }

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        IReadOnlyList<Exploration> list = new[]
        {
            new Exploration("Show on the map", "popover-chip-map", new ExplorationTarget.NavigateWorld($"from={From.Value}&to={To.Value}")),
        };
        return Task.FromResult(list);
    }

    public Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        RenderFragment fragment = _ => { };
        return Task.FromResult(fragment);
    }
}
