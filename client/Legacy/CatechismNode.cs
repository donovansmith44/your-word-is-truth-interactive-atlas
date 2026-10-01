using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

public sealed class CatechismNode : IExplorable
{
    private readonly string _id;
    private readonly AsyncMemo<CatechismItem> _detail = new();

    public CatechismNode(string id, string name)
    {
        _id = id;
        Title = name;
    }

    public string Id => _id;
    public string Title { get; }
    public string Kind => "Catechism";
    public NodeRef Identity => new(id: NodeIds.Of(NodeKind.CatechismItem, _id), kind: NodeKind.CatechismItem, label: Title);

    public Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api) =>
        Task.FromResult<IReadOnlyList<Chip>>(Array.Empty<Chip>());

    public Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        RenderFragment fragment = _ => { };
        return Task.FromResult(fragment);
    }

    public Task<CatechismItem> DetailAsync(AtlasClient api) => _detail.Get(() => api.CatechismItem(_id));
}
