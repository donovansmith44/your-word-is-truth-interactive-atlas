using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>A whole chapter (spec node table row "Passage/Chapter/Book").</summary>
public sealed class ChapterNode : IExplorable
{
    private readonly string _book;
    private readonly int _chapter;
    private ChapterOut? _cached;

    public ChapterNode(string book, int chapter)
    {
        _book = book;
        _chapter = chapter;
    }

    public string Title => $"{_book}.{_chapter}";
    public string Kind => "Chapter";

    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        IReadOnlyList<Exploration> list = new[]
        {
            new Exploration("Show on /world", "popover-chip-map",
                new ExplorationTarget.NavigateWorld($"ref={Uri.EscapeDataString(Title)}")),
            new Exploration("Read in context", "popover-chip-context",
                new ExplorationTarget.NavigateReader(_book, _chapter, null)),
            new Exploration("About this book", "popover-chip-book",
                new ExplorationTarget.Push(new AuthorNode(_book))),
        };
        return Task.FromResult(list);
    }

    public async Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        var chapter = await Load(api);
        var preview = chapter.Verses.Count > 0 ? chapter.Verses[0].Text : "";
        var count = chapter.Verses.Count;
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-verse-text");
            builder.AddContent(2, preview);
            builder.CloseElement();
            builder.OpenElement(3, "p");
            builder.AddAttribute(4, "class", "popover-meta");
            builder.AddContent(5, $"{count} verse{(count == 1 ? "" : "s")}.");
            builder.CloseElement();
        };
        return fragment;
    }

    private async Task<ChapterOut> Load(AtlasClient api) => _cached ??= await api.Chapter(_book, _chapter);
}
