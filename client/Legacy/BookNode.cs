using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

public sealed class BookNode : IExplorable
{
    private readonly string _bookCode;

    public BookNode(string bookCode) => _bookCode = bookCode;

    public string Title => _bookCode;
    public string Kind => "Book";
    public NodeRef Identity => new(id: LegacyNodes.BookContainerId(_bookCode), kind: NodeKind.Container, label: _bookCode);

    public Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api)
    {
        IReadOnlyList<Chip> list = new[]
        {
            new Chip("Show on /world", "popover-chip-map",
                new ChipTarget.NavigateWorld($"ref={Uri.EscapeDataString(_bookCode)}")),
            new Chip("Read in context", "popover-chip-context",
                new ChipTarget.NavigateReader(_bookCode, 1, null)),
            new Chip("About this book", "popover-chip-book",
                new ChipTarget.Push(new AuthorNode(_bookCode), EdgeKind.AuthoredBy)),
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
