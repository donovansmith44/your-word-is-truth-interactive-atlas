using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// The Explorable abstraction (design spec S6.1): every entity the app can
/// show a popover for -- a verse, a passage, a chapter, a book, a place, a
/// place-in-time, an author -- implements this one interface, so
/// <c>ExplorerPopover</c> can render any of them identically. <see cref="Title"/>
/// and <see cref="Kind"/> are synchronous and must be derivable from the
/// node's own constructor arguments alone (never from a fetch) -- the
/// popover needs a header the instant a node is pushed, before any network
/// round-trip resolves. <see cref="ExploreAsync"/>/<see cref="BodyAsync"/>
/// are the async halves that actually hit <see cref="AtlasClient"/>.
/// </summary>
public interface IExplorable
{
    string Title { get; }
    string Kind { get; }
    Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api);
    Task<RenderFragment> BodyAsync(AtlasClient api);
}
