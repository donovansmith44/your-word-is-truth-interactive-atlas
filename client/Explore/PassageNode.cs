using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class PassageNode : IExplorable
{
    private readonly string _sref;
    private readonly string _text;
    private readonly AsyncMemo<IReadOnlyList<CrossRef>> _xrefs = new();
    private readonly AsyncMemo<IReadOnlyList<CatechismRef>> _catechism = new();

    public PassageNode(string sref, string text)
    {
        _sref = sref;
        _text = text;
    }

    public string Title => _sref;
    public string Kind => "Passage";

    public string Text => _text;

    public Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api)
    {
        var (book, chapter, verse) = CanonRef.ParseVerse(CanonRef.FirstVerseOf(_sref));
        IReadOnlyList<Chip> list = new[]
        {
            new Chip("Read in context", "popover-chip-context",
                new ChipTarget.NavigateReader(book, chapter, verse)),
            new Chip("About this book", "popover-chip-book",
                new ChipTarget.Push(new AuthorNode(book))),
        };
        return Task.FromResult(list);
    }

    // AsyncMemo-backed rather than a value-memoizing `??= await`: the latter races when
    // ExplorerPopover.LoadCurrent's concurrent Task.WhenAll dispatch calls this more than once.
    public Task<IReadOnlyList<CrossRef>> XrefsAsync(AtlasClient api) => _xrefs.Get(() => api.Xrefs(_sref));

    public Task<IReadOnlyList<CatechismRef>> CatechismAsync(AtlasClient api) => _catechism.Get(() => api.Catechism(_sref));

    public Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-verse-text");
            builder.AddContent(2, _text);
            builder.CloseElement();
        };
        return Task.FromResult(fragment);
    }
}
