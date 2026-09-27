using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class CatechismNode : IExplorable
{
    private readonly string _id;
    private readonly AsyncMemo<CatechismItemDetail> _detail = new();

    public CatechismNode(string id, string name)
    {
        _id = id;
        Title = name;
    }

    public string Id => _id;
    public string Title { get; }
    public string Kind => "Catechism";

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api) =>
        Task.FromResult<IReadOnlyList<Exploration>>(Array.Empty<Exploration>());

    public Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        RenderFragment fragment = _ => { };
        return Task.FromResult(fragment);
    }

    public Task<CatechismItemDetail> DetailAsync(AtlasClient api) => _detail.Get(() => api.CatechismItem(_id));
}
