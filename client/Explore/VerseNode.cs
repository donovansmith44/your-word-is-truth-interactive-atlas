using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class VerseNode : IExplorable
{
    private readonly string _vref;
    private readonly AsyncMemo<VerseDetail> _detail = new();

    public bool XrefEntryPoint { get; }

    public VerseNode(string vref, bool xrefEntryPoint = false)
    {
        _vref = vref;
        XrefEntryPoint = xrefEntryPoint;
    }

    public string Title => _vref;
    public string Kind => "Verse";
    public NodeRef Identity => new(id: NodeIds.Of(NodeKind.TextUnit, _vref), kind: PositionKind.TextUnit, label: _vref);

    public Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api)
    {
        var (book, chapter, verse) = CanonRef.ParseVerse(_vref);
        IReadOnlyList<Chip> list = new[]
        {
            new Chip("About this book", "popover-chip-book", new ChipTarget.Push(new AuthorNode(book), EdgeKind.MemberOf)),
            new Chip("Read in context", "popover-chip-context", new ChipTarget.NavigateReader(book, chapter, verse)),
        };
        return Task.FromResult(list);
    }

    public async Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        var detail = await DetailAsync(api);
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-verse-text");
            builder.AddContent(2, detail.Text);
            builder.CloseElement();
        };
        return fragment;
    }

    // AsyncMemo-backed rather than a value-memoizing `_cached ??= await`: up to six section
    // providers call this for the same Verse node via one Task.WhenAll, and each runs
    // synchronously up to its first await before the next starts, so a plain `??=` cache
    // would still be null for every one of them and each would fire its own independent
    // GET /api/verse/{vref}. AsyncMemo caches the in-flight task itself, assigned
    // synchronously, so concurrent callers share it.
    public Task<VerseDetail> DetailAsync(AtlasClient api) => _detail.Get(() => api.Verse(_vref));
}
