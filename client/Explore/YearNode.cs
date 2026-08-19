using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch E: a curated established/destroyed date claim as its own
/// explorable node -- user direction (2026-08-19): "timestamps/timeranges
/// are THEMSELVES explorable: hover a year -&gt; an 'Explore' button -&gt;
/// supporting verses first... this whole thing is a giant DAG where every
/// node is explorable." Constructed by <c>PlaceCard</c>, which already has
/// the place name and the fetched <see cref="DateClaimOut"/> in hand -- no
/// AtlasClient fetch needed here, same pattern as <see cref="TimeAndPlaceNode"/>.
/// Title mirrors the card's own instrument-face text exactly (both go
/// through <see cref="YearText.FormatClaim"/>), e.g. "Established c. 1003 BC".
/// </summary>
public sealed class YearNode : IExplorable
{
    private readonly TimeRangeDto _when;
    private readonly List<string> _verses;

    public YearNode(string label, TimeRangeDto when, List<string> verses, string? note)
    {
        Title = $"{label} {YearText.FormatClaim(when.FromYear, when.ToYear, note)}";
        _when = when;
        _verses = verses;
    }

    public string Title { get; }
    public string Kind => "Year";

    /// <summary>
    /// Leads with the curated SUPPORTING VERSES (one <see cref="VerseNode"/>
    /// push per verse, in curated order -- the order they best build the
    /// claim in, e.g. Jerusalem's established claim cites 2SA.5.6 then
    /// 2SA.5.7 then 2SA.5.9), then "Show this time on the map" last (DATE-1:
    /// "the card's date affordance opens the popover listing the curated
    /// supporting verses first").
    /// </summary>
    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        var list = new List<Exploration>();
        foreach (var vref in _verses)
        {
            var v = vref; // local copy -- captured per-chip by the Push target
            list.Add(new Exploration(v, $"popover-chip-verse-{v}", new ExplorationTarget.Push(new VerseNode(v))));
        }

        list.Add(new Exploration("Show this time on the map", "popover-chip-map",
            new ExplorationTarget.NavigateWorld($"from={_when.FromYear}&to={_when.ToYear}")));

        return Task.FromResult<IReadOnlyList<Exploration>>(list);
    }

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
}
