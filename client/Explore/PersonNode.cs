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

    public Task<NodeCard> CardAsync(Func<Task<NodeCard>> fetch) => _card.Get(fetch);

    public static string Year(int y) => YearText.Format(y);

    public async Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        PersonLife? life;
        try
        {
            life = (await CardAsync(() => api.NodeCard(_personId))).Person;
        }
        catch (Exception)
        {
            return Array.Empty<Exploration>();
        }

        if (life is null || life.Eternal)
        {
            return Array.Empty<Exploration>();
        }

        var chips = new List<Exploration>();
        if (life.BirthYear is int born)
        {
            chips.Add(new Exploration($"Born c. {Year(born)}", "popover-chip-year-born", new ExplorationTarget.NavigateWorld($"from={born}&to={born}")));
        }

        if (life.DeathYear is int died)
        {
            chips.Add(new Exploration($"Died c. {Year(died)}", "popover-chip-year-died", new ExplorationTarget.NavigateWorld($"from={died}&to={died}")));
        }

        if (chips.Count == 0 && life.FirstYear is int first && life.LastYear is int last)
        {
            chips.Add(new Exploration($"Mentioned across c. {Year(first)} - {Year(last)}", "popover-chip-year-span", new ExplorationTarget.NavigateWorld($"from={first}&to={last}")));
        }

        return chips;
    }

    public Task<RenderFragment> BodyAsync(AtlasClient api) => Task.FromResult<RenderFragment>(_ => { });
}
