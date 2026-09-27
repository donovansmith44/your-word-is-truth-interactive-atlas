using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public interface IExplorable
{
    string Title { get; }
    string Kind { get; }
    
    // My main gripe is that this should return a set of explorables ; not a list of explorations... An exploration describes a DAG with >= 2 nodes and a connecting edge; it's actually a doubly linked list with no cycles.
    Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api);
    Task<RenderFragment> BodyAsync(AtlasClient api);
}
