using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>A whole book (spec node table row "Passage/Chapter/Book").</summary>
public sealed class BookNode : IExplorable
{
    private readonly string _bookCode;

    public BookNode(string bookCode) => _bookCode = bookCode;

    public string Title => _bookCode;
    public string Kind => "Book";

    // HATCH-DELIVERABLE-1 (2026-09-07): the SAME disclosed, not-fixed-this-
    // round gap as ChapterNode.ExploreAsync's own identical chip -- see that
    // file's own header comment for the full reasoning (BookMeta.WritePlace
    // can be absent, checking it here would need a new fetch this method
    // does not otherwise make, disclosed per the ticket's own instruction
    // rather than adding one).
    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        IReadOnlyList<Exploration> list = new[]
        {
            new Exploration("Show on /world", "popover-chip-map",
                new ExplorationTarget.NavigateWorld($"ref={Uri.EscapeDataString(_bookCode)}")),
            new Exploration("Read in context", "popover-chip-context",
                new ExplorationTarget.NavigateReader(_bookCode, 1, null)),
            new Exploration("About this book", "popover-chip-book",
                new ExplorationTarget.Push(new AuthorNode(_bookCode))),
        };
        return Task.FromResult(list);
    }

    public async Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        var toc = await api.Books();
        var entry = toc.FirstOrDefault(b => b.Code == _bookCode);
        var name = entry?.Name ?? _bookCode;
        var chapters = entry?.Chapters.Count ?? 0;
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-meta");
            builder.AddContent(2, $"{name} — {chapters} chapter{(chapters == 1 ? "" : "s")}.");
            builder.CloseElement();
        };
        return fragment;
    }
}
