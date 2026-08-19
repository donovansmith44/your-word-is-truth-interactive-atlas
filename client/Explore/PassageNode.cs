using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// A verse range within a single chapter (READ-5/Task 15's shift-click
/// passage selection). The reader already has every verse's text on hand
/// (it just rendered the chapter to let the user shift-click it), so unlike
/// VerseNode/ChapterNode this node needs no AtlasClient fetch of its own for
/// its body -- the caller hands over the already-known sref and concatenated
/// text directly at construction, avoiding a second network round-trip for
/// content already in memory. Reader.razor owns the actual shift-click
/// wiring (Task 15).
///
/// The map chip is <see cref="ExplorationTarget.ShowMiniMap"/> (Task 15) --
/// "chips: map/context like VerseNode" per the brief, i.e. reveals the
/// <c>mini-map</c> IN PLACE the same way VerseNode's own map chip does,
/// rather than Task 14's placeholder <c>NavigateWorld</c> (which would have
/// left the popover entirely, breaking READ-5's own "chip -&gt; popover-chip-
/// map -&gt; mini-map, all within the SAME still-open passage popover" flow).
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
            new Exploration("Explore geo-temporally", "popover-chip-map", new ExplorationTarget.ShowMiniMap(_sref)),
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
