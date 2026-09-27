using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class CommentaryItemNode : IExplorable
{
    private readonly string _id;

    public CommentaryItemNode(string id, string heading)
    {
        _id = id;
        Title = string.IsNullOrWhiteSpace(heading) ? "Commentary" : heading;
    }

    public string Id => _id;

    public string Title { get; }
    public string Kind => "CommentaryItem";

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api) =>
        Task.FromResult<IReadOnlyList<Exploration>>(Array.Empty<Exploration>());

    // Unreachable while CommentaryItemProseSection is registered for Kind == "CommentaryItem";
    // kept only as the interface's required fallback.
    public Task<RenderFragment> BodyAsync(AtlasClient api) => Task.FromResult<RenderFragment>(_ => { });
}
