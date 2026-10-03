using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Legacy;

namespace BibleAtlas.Client.Tests;

public sealed class AuthorNodeTests
{
    private const string NehemiahsCard = """
        {"id":"Container:bible-book-NEH","kind":"Container","label":"Nehemiah","edge_summary":[],"provenance":{"id":"kjv","title":"The King James Version"},"version":"",
         "book":{"author":"Nehemiah","write_place":{"id":"Place:jerusalem","kind":"Place","label":"Jerusalem"},
                 "written":{"from":{"label":"445 BC","value":-445},"label":"445 – 432 BC","to":{"label":"432 BC","value":-432}}}}
        """;

    private static readonly Year Year445Bc = new(label: "445 BC", value: -445);
    private static readonly Year Year432Bc = new(label: "432 BC", value: -432);
    private static readonly TimeRange NehemiahsYears = new(from: Year445Bc, label: "445 – 432 BC", to: Year432Bc);
    private static readonly NodeRef Jerusalem = new(id: "Place:jerusalem", kind: NodeKind.Place, label: "Jerusalem");

    [Fact]
    public async Task A_book_shows_the_served_years_of_its_writing_on_the_map()
    {
        // Arrange
        var atlas = new StubbedAtlas(NehemiahsCard);

        // Act
        var explorations = await new AuthorNode("NEH").ExploreAsync(atlas.Client());

        // Assert
        Assert.Equal(
            (new Chip("Show on /world", "popover-chip-map", new ChipTarget.NavigateWorld("from=-445&to=-432")), "/api/node/Container%3Abible-book-NEH"),
            (explorations.Single(), string.Join(" ", atlas.Asked)));
    }

    [Fact]
    public void A_books_writing_reads_the_served_place_and_years()
    {
        // Arrange
        var books = new[]
        {
            new BookDetail(author: "Nehemiah", writePlace: Jerusalem, written: NehemiahsYears),
            new BookDetail(author: "Nehemiah", writePlace: Jerusalem, written: null),
            new BookDetail(author: "Nehemiah", writePlace: null, written: NehemiahsYears),
            new BookDetail(author: "Nehemiah", writePlace: null, written: null),
        };

        // Act
        var lines = books.Select(AuthorNode.WritingOf).ToArray();

        // Assert
        Assert.Equal(new string?[] { "Written from Jerusalem, 445 – 432 BC.", "Written from Jerusalem.", "Written 445 – 432 BC.", null }, lines);
    }
}
