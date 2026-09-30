using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class PersonNode : IExplorable
{
    private readonly string _personId;
    private readonly string _label;
    private readonly AsyncMemo<NodeCard> _card = new();

    public PersonNode(string personId, string label)
    {
        _personId = personId;
        _label = label;
    }

    public string PersonId => _personId;
    public string Title => _label;
    public string Kind => "Person";
    public Explorable Identity => new(NodeKind.Person, _personId, _label);

    public Task<NodeCard> CardAsync(Func<Task<NodeCard>> fetch) => _card.Get(fetch);

    public async Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api)
    {
        PersonLife? life;
        try
        {
            life = (await CardAsync(() => api.NodeCard(_personId))).Person;
        }
        catch (Exception)
        {
            return Array.Empty<Chip>();
        }

        if (life is null || life.Eternal)
        {
            return Array.Empty<Chip>();
        }

        var chips = new List<Chip>();
        if (life.Birth is { } born)
        {
            chips.Add(new Chip(Born(born), "popover-chip-year-born", new ChipTarget.NavigateWorld($"from={born.Value}&to={born.Value}")));
        }

        if (life.Death is { } died)
        {
            chips.Add(new Chip(Died(died), "popover-chip-year-died", new ChipTarget.NavigateWorld($"from={died.Value}&to={died.Value}")));
        }

        if (chips.Count == 0 && life.First is { } first && life.Last is { } last)
        {
            chips.Add(new Chip(MentionedAcross(first, last), "popover-chip-year-span", new ChipTarget.NavigateWorld($"from={first.Value}&to={last.Value}")));
        }

        return chips;
    }

    public Task<RenderFragment> BodyAsync(AtlasClient api) => Task.FromResult<RenderFragment>(_ => { });

    public static string Born(Year year) => $"Born c. {year.Label}";

    public static string Died(Year year) => $"Died c. {year.Label}";

    public static string MentionedAcross(Year first, Year last) => $"Mentioned across c. {first.Label} \u2013 {last.Label}";
}
