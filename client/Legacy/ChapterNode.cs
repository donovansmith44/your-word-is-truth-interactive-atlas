using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

public sealed class ChapterNode : IExplorable
{
    private readonly string _book;
    private readonly int _chapter;
    private readonly AsyncMemo<Chapter> _loaded = new();

    public int? TotalChapters { get; }

    public Chapter? AlreadyLoaded { get; init; }

    public string Book => _book;
    public int Chapter => _chapter;

    public ChapterNode(string book, int chapter, int? totalChapters = null)
    {
        _book = book;
        _chapter = chapter;
        TotalChapters = totalChapters;
    }

    public string Title => $"{_book}.{_chapter}";
    public string Kind => "Chapter";
    public NodeRef Identity => new(id: LegacyNodes.ChapterContainerId(_book, _chapter), kind: NodeKind.Container, label: Title);

    public Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api)
    {
        // From Donovan I think that, structurally, part of the problem is that we return a list of explorations, rather than a list of explorables (i.e., everything in the application is technically explorable; some things just might be sinks/sources. for instance Genesis 1 is a temporal source (you cannot go back in time before creation of the world. This seems like it could be a meaningful refactor.)
        // If we're returning explorations, the explorable thing is not actually recursive.
        IReadOnlyList<Chip> list = new[]
        {
            new Chip("Show on /world", "popover-chip-map",
                new ChipTarget.NavigateWorld($"ref={Uri.EscapeDataString(Title)}")),
            new Chip("Read in context", "popover-chip-context",
                new ChipTarget.NavigateReader(_book, _chapter, null)),
            new Chip("About this book", "popover-chip-book",
                new ChipTarget.Push(new AuthorNode(_book), EdgeKind.MemberOf)),
        };
        return Task.FromResult(list);
    }

    // Unreachable while ChapterCardSection is registered for Kind == "Chapter";
    // kept only as the interface's required fallback.
    public async Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        var chapter = await Load(api);
        var count = chapter.Verses.Count;
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-meta");
            builder.AddContent(2, $"{count} verse{(count == 1 ? "" : "s")}.");
            builder.CloseElement();
        };
        return fragment;
    }

    public Task<Chapter> Load(AtlasClient api) => _loaded.Get(() => AlreadyLoaded is { } already ? Task.FromResult(already) : api.Chapter(_book, _chapter));
}
