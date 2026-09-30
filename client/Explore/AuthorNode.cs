using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

public sealed class AuthorNode : IExplorable
{
    private const string BookContainerPrefix = "bible-book-";

    private readonly string _bookCode;
    private readonly AsyncMemo<NodeCard> _card = new();

    public AuthorNode(string bookCode) => _bookCode = bookCode;

    public string Title => _bookCode;
    public string Kind => "Author";

    public async Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        if ((await Load(api)).Book is not { WritePlace: not null, Written: { } written })
        {
            return Array.Empty<Exploration>();
        }

        return new[]
        {
            new Exploration("Show on /world", "popover-chip-map",
                new ExplorationTarget.NavigateWorld($"from={written.From.Value}&to={written.To.Value}")),
        };
    }

    public async Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        if ((await Load(api)).Book is not { } book)
        {
            return _ => { };
        }

        var writing = WritingOf(book);
        RenderFragment fragment = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "popover-meta");
            builder.AddContent(seq++, $"By {book.Author}.");
            builder.CloseElement();

            if (writing is not null)
            {
                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "popover-meta");
                builder.AddContent(seq++, writing);
                builder.CloseElement();
            }
        };
        return fragment;
    }

    public static string? WritingOf(BookDetail book) => (book.WritePlace?.Label, book.Written?.Label) switch
    {
        ({ } place, { } years) => $"Written from {place}, {years}.",
        ({ } place, null) => $"Written from {place}.",
        (null, { } years) => $"Written {years}.",
        (null, null) => null,
    };

    private Task<NodeCard> Load(AtlasClient api) =>
        _card.Get(() => api.NodeCard(NodeIds.Of(NodeKind.Container, $"{BookContainerPrefix}{_bookCode}")));
}
