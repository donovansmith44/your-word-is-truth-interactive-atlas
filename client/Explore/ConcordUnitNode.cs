using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch CORP-1 (R3): one Book of Concord TextUnit (one paragraph), reached
/// by clicking an explorable row in the Concord structure browser
/// (Pages/Concord.razor). Unlike <see cref="CommentaryItemNode"/>, this
/// node's own text arrives ALREADY resolved -- the SAME
/// <c>/api/text?corpus=concord</c> reading-window fetch that builds the
/// browser's own page of rows already carries each unit's full paragraph
/// text (<c>TextUnitDto.Text</c>), so no second fetch is needed here;
/// <see cref="BodyAsync"/> just wraps what it already has (contrast
/// <c>VerseNode.BodyAsync</c>'s own fetch, which exists precisely BECAUSE
/// the reader's own chapter fetch does not carry verse text a second time --
/// a disclosed, deliberate difference in shape, not an inconsistency).
///
/// <see cref="Title"/> is the unit's own citation (<c>"BoC 7.2.1"</c>) --
/// the same "Title IS the ref" convention <c>VerseNode.Title</c> already
/// establishes for its own vref. <see cref="Kind"/> is "ConcordUnit"
/// (distinct from VerseNode's "Verse") -- no popover section provider
/// claims it, so <see cref="BodyAsync"/>'s own fallback renders directly,
/// the same shape VerseNode/PersonNode already establish for a node kind
/// with no registered provider.
/// </summary>
public sealed class ConcordUnitNode : IExplorable
{
    private readonly string _text;

    public ConcordUnitNode(string citation, string text)
    {
        Title = citation;
        _text = text;
    }

    public string Title { get; }
    public string Kind => "ConcordUnit";

    // No further exploration chips this batch -- R3 offers the explorable
    // TextUnits themselves (ONE-RULE); a paragraph's own onward traversal
    // (e.g. the SC-overlap CatechismLink) is out of this batch's own scope,
    // the same "no chips at all" shape CatechismNode/PersonNode already
    // establish for a node kind with nothing extra to offer yet.
    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api) =>
        Task.FromResult<IReadOnlyList<Exploration>>(Array.Empty<Exploration>());

    public Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-concord-text");
            builder.AddContent(2, _text);
            builder.CloseElement();
        };
        return Task.FromResult(fragment);
    }
}
