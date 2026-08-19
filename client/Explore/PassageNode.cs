using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// A verse range within a single chapter (READ-5/Task 15's shift-click
/// passage selection). The reader already has every verse's text on hand
/// (it just rendered the chapter to let the user shift-click it), so unlike
/// VerseNode/ChapterNode this node needs no AtlasClient fetch of its own for
/// its body -- the caller hands over the already-known sref and concatenated
/// text directly at construction, avoiding a second network round-trip for
/// content already in memory. Task 15 owns the actual shift-click wiring in
/// Reader.razor; this class exists now with the Title/Kind/body/explorations
/// the spec's node table gives for "Passage/Chapter/Book".
/// </summary>
public sealed class PassageNode : IExplorable
{
    private readonly string _sref;
    private readonly string _text;

    public PassageNode(string sref, string text)
    {
        _sref = sref;
        _text = text;
    }

    public string Title => _sref;
    public string Kind => "Passage";

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        var (book, chapter, verse) = CanonRef.ParseVerse(CanonRef.FirstVerseOf(_sref));
        IReadOnlyList<Exploration> list = new[]
        {
            new Exploration("Show on /world", "popover-chip-map",
                new ExplorationTarget.NavigateWorld($"ref={Uri.EscapeDataString(_sref)}")),
            new Exploration("Read in context", "popover-chip-context",
                new ExplorationTarget.NavigateReader(book, chapter, verse)),
            new Exploration("About this book", "popover-chip-book",
                new ExplorationTarget.Push(new AuthorNode(book))),
        };
        return Task.FromResult(list);
    }

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
