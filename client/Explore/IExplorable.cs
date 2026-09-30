using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public interface IExplorable
{
    string Title { get; }
    string Kind { get; }
    Explorable Identity { get; }

    Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api);
    Task<RenderFragment> BodyAsync(AtlasClient api);
}
