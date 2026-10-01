using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class AnchoredTextTests
{
    private const string Joshua11Verse1 = "And it came to pass, when Jabin king of Hazor had heard those things";
    private const string Matthew4Verse19 = "And he saith unto them, Follow me, and I will make you fishers of men.";
    private const string Matthew28Verse10 = "Then said Jesus unto them, Be not afraid: go tell my brethren that they go into Galilee, and there shall they see me.";
    private const string WithAnAstralCharacter = "\U0001D4D6 Hazor";

    private static readonly NodeRef Jabin = new(id: "Person:jabin_1", kind: NodeKind.Person, label: "Jabin");
    private static readonly NodeRef Hazor = new(id: "Place:hazor-1", kind: NodeKind.Place, label: "Hazor");
    private static readonly NodeRef FirstPeter3Verse6 = new(id: "text-unit:1PE.3.6", kind: NodeKind.TextUnit, label: "1PE.3.6");

    private static readonly NodeRef Galilee = new(id: "Place:galilee", kind: NodeKind.Place, label: "Galilee");

    private static readonly Anchor JabinNamed = new(end: 31, kind: EdgeKind.Mentions, node: Jabin, start: 26);
    private static readonly Anchor HazorNamed = new(end: 45, kind: EdgeKind.Mentions, node: Hazor, start: 40);

    [Fact]
    public void A_verse_is_cut_at_every_name_the_graph_anchors_in_it()
    {
        // Act
        var runs = AnchoredText.Runs(Joshua11Verse1, [], [JabinNamed, HazorNamed]);

        // Assert
        Assert.Equal(
            [[
                new TextPiece("And it came to pass, when ", false, null),
                new TextPiece("Jabin", false, JabinNamed),
                new TextPiece(" king of ", false, null),
                new TextPiece("Hazor", false, HazorNamed),
                new TextPiece(" had heard those things", false, null),
            ]],
            runs);
    }

    [Fact]
    public void The_words_of_christ_are_one_run_of_their_own()
    {
        // Act
        var runs = AnchoredText.Runs(Matthew4Verse19, [new WordsOfChristSpan(end: 70, start: 24)], []);

        // Assert
        Assert.Equal(
            [
                [new TextPiece("And he saith unto them, ", false, null)],
                [new TextPiece("Follow me, and I will make you fishers of men.", true, null)],
            ],
            runs);
    }

    [Fact]
    public void A_name_inside_the_words_of_christ_is_anchored_within_their_run()
    {
        // Arrange
        var galilee = new Anchor(end: 87, kind: EdgeKind.Mentions, node: Galilee, start: 80);

        // Act
        var runs = AnchoredText.Runs(Matthew28Verse10, [new WordsOfChristSpan(end: 117, start: 27)], [galilee]);

        // Assert
        Assert.Equal(
            [
                [new TextPiece("Then said Jesus unto them, ", false, null)],
                [
                    new TextPiece("Be not afraid: go tell my brethren that they go into ", true, null),
                    new TextPiece("Galilee", true, galilee),
                    new TextPiece(", and there shall they see me.", true, null),
                ],
            ],
            runs);
    }

    [Fact]
    public void Offsets_count_unicode_scalars_not_utf16_code_units()
    {
        // Arrange
        var hazorAfterTheAstral = new Anchor(end: 7, kind: EdgeKind.Mentions, node: Hazor, start: 2);

        // Act
        var runs = AnchoredText.Runs(WithAnAstralCharacter, [], [hazorAfterTheAstral]);

        // Assert
        Assert.Equal([[new TextPiece("\U0001D4D6 ", false, null), new TextPiece("Hazor", false, hazorAfterTheAstral)]], runs);
    }

    [Fact]
    public void Text_with_nothing_anchored_is_one_plain_run_and_no_text_is_no_run()
    {
        // Act
        var runs = new[] { AnchoredText.Runs(Joshua11Verse1, [], []), AnchoredText.Runs("", [], []) };

        // Assert
        Assert.Equal([[[new TextPiece(Joshua11Verse1, false, null)]], []], runs);
    }

    [Fact]
    public void A_scalar_offset_is_read_back_from_a_utf16_index()
    {
        // Act
        var offsets = new[] { AnchoredText.ScalarOffsetOf(WithAnAstralCharacter, 3), AnchoredText.ScalarOffsetOf(Joshua11Verse1, 40) };

        // Assert
        Assert.Equal([2, 40], offsets);
    }

    [Fact]
    public void A_paragraph_that_cites_scripture_is_cut_at_each_citation()
    {
        // Arrange
        const string forWives = "as Sara obeyed Abraham (1 Pet. 3:6).";
        var cited = new Anchor(end: 34, kind: EdgeKind.Cites, node: FirstPeter3Verse6, start: 24);

        // Act
        var segments = AnchoredText.Segments(forWives, [cited]);

        // Assert
        Assert.Equal([new TextPiece("as Sara obeyed Abraham (", false, null), new TextPiece("1 Pet. 3:6", false, cited), new TextPiece(").", false, null)], segments);
    }
}
