using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class KretzmannCitationScanTests
{
    private static readonly IReadOnlyList<CanonBook> Toc =
    [
        new(chapters: [16], code: "ROM", name: "Romans"),
        new(chapters: [21], code: "JHN", name: "John"),
    ];

    [Fact]
    public void A_citation_in_commentary_prose_is_a_cites_anchor_on_the_verse_it_names()
    {
        // Arrange
        const string prose = "All have sinned, Rom. 3, 23; for God so loved the world, John 3, 16-18.";

        // Act
        var anchors = KretzmannCitationScan.Anchors(prose, Toc);

        // Assert
        Assert.Equal(
            [
                new Anchor(end: 27, kind: EdgeKind.Cites, node: new NodeRef(id: "text-unit:ROM.3.23", kind: NodeKind.TextUnit, label: "ROM.3.23"), start: 17),
                new Anchor(end: 70, kind: EdgeKind.Cites, node: new NodeRef(id: "text-unit:JHN.3.16", kind: NodeKind.TextUnit, label: "JHN.3.16"), start: 57),
            ],
            anchors);
    }

    [Fact]
    public void An_abbreviation_that_could_name_two_books_cites_nothing()
    {
        // Act
        var anchors = KretzmannCitationScan.Anchors("compare Cor. 5, 7 and Phil. 2, 5", Toc);

        // Assert
        Assert.Equal([], anchors);
    }

    [Fact]
    public void Offsets_after_an_astral_character_count_unicode_scalars()
    {
        // Act
        var anchors = KretzmannCitationScan.Anchors("\U0001D4D6 Rom. 3, 23", Toc);

        // Assert
        Assert.Equal([new Anchor(end: 12, kind: EdgeKind.Cites, node: new NodeRef(id: "text-unit:ROM.3.23", kind: NodeKind.TextUnit, label: "ROM.3.23"), start: 2)], anchors);
    }
}
