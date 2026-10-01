using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

public interface IExplorable
{
    string Title { get; }
    string Kind { get; }
    NodeRef Identity { get; }

    Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api);
    Task<RenderFragment> BodyAsync(AtlasClient api);
}
