using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class ConcordUnitNode : IExplorable
{
    private readonly string? _givenText;
    private readonly AsyncMemo<string> _text = new();

    public ConcordUnitNode(string citation, string? text = null)
    {
        Title = citation;
        _givenText = text;
    }

    public string Title { get; }
    public string Kind => "ConcordUnit";

    public string NodeId => NodeIds.Of(NodeKind.TextUnit, Title);

    public Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api) =>
        Task.FromResult<IReadOnlyList<Chip>>(Array.Empty<Chip>());

    public Task<string> TextAsync(IExplorableClient graph) =>
        _givenText is { } given
            ? Task.FromResult(given)
            : _text.Get(async () => (await ParagraphAt(graph, Title))?.Text ?? string.Empty);

    public static async Task<TextUnit?> ParagraphAt(IExplorableClient graph, string citation) =>
        (await graph.Reading(citation, OneParagraph, WindowDir.Onward, Corpus.Concord)).Units.FirstOrDefault(u => u.Ref == citation);

    private const int OneParagraph = 1;

    // Unreachable while ConcordUnitTextSection is registered for Kind == "ConcordUnit"; the interface requires a fallback.
    public Task<RenderFragment> BodyAsync(AtlasClient api) => Task.FromResult<RenderFragment>(_ => { });
}
