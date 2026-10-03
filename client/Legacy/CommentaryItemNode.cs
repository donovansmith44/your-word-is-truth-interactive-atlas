using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

public sealed class CommentaryItemNode : IExplorable
{
    private readonly string _id;

    public CommentaryItemNode(string id, string heading)
    {
        _id = id;
        Title = string.IsNullOrWhiteSpace(heading) ? "Commentary" : heading;
    }

    public string Title { get; }
    public string Kind => "CommentaryItem";
    public NodeRef Identity => new(id: LegacyNodeIds.Of(NodeKind.CommentaryItem, _id), kind: NodeKind.CommentaryItem, label: Title);

    public Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api) =>
        Task.FromResult<IReadOnlyList<Chip>>(Array.Empty<Chip>());

    // Unreachable while CommentaryItemProseSection is registered for Kind == "CommentaryItem";
    // kept only as the interface's required fallback.
    public Task<RenderFragment> BodyAsync(AtlasClient api) => Task.FromResult<RenderFragment>(_ => { });
}
