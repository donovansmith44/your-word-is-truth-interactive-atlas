using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// A person as a popover node. D5 (owner, 2026-09-15, verbatim: "when we
/// click on a person's name there's no point to just see every verse that
/// name is mentioned ... i want to see the years that person is alive --
/// years are explorable positions; the events (explorable) in which they
/// are mentioned; optionally a family tree whose names are all explorable;
/// the exception is God because he is eternal"): the chips row carries the
/// YEARS as explorable positions on the world (born / died when the source
/// gives them; else the corpus-mention span, said as such); the sections
/// (PersonLifeSection, PersonEventsSection, PersonFamilySection, then the
/// verse list collapsed last) carry the rest. Eternal: no years, ever.
/// </summary>
public sealed class PersonNode : IExplorable
{
    private readonly string _personId;
    private readonly string _label;
    private readonly AsyncMemo<NodeCardDto> _card = new();

    public PersonNode(string personId, string label)
    {
        _personId = personId;
        _label = label;
    }

    public string PersonId => _personId;
    public string Title => _label;
    public string Kind => "Person";

    /// <summary>The card, fetched once and shared by every section of this node.</summary>
    public Task<NodeCardDto> CardAsync(Func<Task<NodeCardDto>> fetch) => _card.Get(fetch);

    /// <summary>One year the way every other card says it (YearText.Format: "1575 BC" / "AD 64").</summary>
    public static string Year(int y) => YearText.Format(y);

    public async Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        PersonLifeDto? life;
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
