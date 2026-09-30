using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class ChapterTextTests
{
    private const string Genesis1Opening = """
        {"version":"v","units":[
          {"ref":"GEN.1.1","locus":{"corpus":"bible","book":"GEN","chapter":1,"verse":1},"text":"In the beginning God created the heaven and the earth.",
           "words_of_christ":[],"edge_summary":[],
           "anchors":[{"start":17,"end":20,"kind":"mentions","node":{"id":"Person:god_1324","kind":"Person","label":"God"}}]},
          {"ref":"GEN.1.2","locus":{"corpus":"bible","book":"GEN","chapter":1,"verse":2},"text":"And the earth was without form, and void.",
           "words_of_christ":[],"edge_summary":[],"anchors":[]}
        ]}
        """;

    private static readonly TextUnit Genesis1Verse2 = new(
        anchors: [],
        edgeSummary: [],
        heading: null,
        locus: new BibleRef(BookId.GEN, 1, 2),
        @ref: "GEN.1.2",
        text: "And the earth was without form, and void.",
        wordsOfChrist: []);

    [Fact]
    public async Task A_chapters_text_is_its_served_chapter_window_read_verse_by_verse()
    {
        // Arrange
        var atlas = new StubbedAtlas(Genesis1Opening);

        // Act
        var text = await atlas.Client().ChapterText("GEN", 1);

        // Assert
        Assert.Equivalent((Genesis1Verse2, "/api/text?ref=GEN.1&scope=chapter"), (text.Verse(2), string.Join(" ", atlas.Asked)), strict: true);
    }

    [Fact]
    public async Task A_chapter_already_read_is_not_asked_for_again()
    {
        // Arrange
        var atlas = new StubbedAtlas(Genesis1Opening);
        var client = atlas.Client();

        // Act
        await client.ChapterText("GEN", 1);
        await client.ChapterText("GEN", 1);

        // Assert
        Assert.Equal(["/api/text?ref=GEN.1&scope=chapter"], atlas.Asked);
    }

    [Fact]
    public async Task A_verse_read_for_a_passage_carries_the_names_anchored_in_it()
    {
        // Arrange
        var atlas = new StubbedAtlas(Genesis1Opening);
        var god = new Anchor(end: 20, kind: EdgeKind.Mentions, node: new NodeRef(id: "Person:god_1324", kind: PositionKind.Person, label: "God"), start: 17);

        // Act
        var verses = await VerseTextResolver.ResolveAsync(atlas.Client(), ["GEN.1.1"]);

        // Assert
        Assert.Equal(
            InOrder([new PassageListVerse("GEN.1.1", "In the beginning God created the heaven and the earth.", Anchors: [god], WordsOfChrist: [])]),
            InOrder(verses));
    }

    [Fact]
    public async Task A_span_across_chapters_reads_every_verse_from_its_first_through_its_last()
    {
        // Arrange
        var atlas = new StubbedAtlas(new Dictionary<string, string>
        {
            ["/api/text?ref=MAT.5&scope=chapter"] = Window(("MAT.5.47", "MAT", 5, 47), ("MAT.5.48", "MAT", 5, 48)),
            ["/api/text?ref=MAT.6&scope=chapter"] = Window(("MAT.6.1", "MAT", 6, 1), ("MAT.6.2", "MAT", 6, 2)),
            ["/api/text?ref=MAT.7&scope=chapter"] = Window(("MAT.7.1", "MAT", 7, 1), ("MAT.7.2", "MAT", 7, 2)),
        });
        var sermon = new TextSpan(from: new TextPoint(unit: new BibleRef(BookId.MAT, 5, 48), word: null), to: new TextPoint(unit: new BibleRef(BookId.MAT, 7, 1), word: null));

        // Act
        var verses = await VerseTextResolver.ResolveSpansAsync(atlas.Client(), [sermon]);

        // Assert
        Assert.Equal(
            InOrder(new[] { "MAT.5.48", "MAT.6.1", "MAT.6.2", "MAT.7.1" }.Select(vref => new PassageListVerse(vref, "", Anchors: [], WordsOfChrist: []))),
            InOrder(verses));
    }

    [Fact]
    public async Task A_span_whose_text_cannot_be_read_resolves_to_no_verses()
    {
        // Arrange
        var atlas = new StubbedAtlas("not a text window");
        var zion = new TextSpan(from: new TextPoint(unit: new BibleRef(BookId._2SA, 5, 7), word: null), to: new TextPoint(unit: new BibleRef(BookId._2SA, 5, 7), word: null));

        // Act
        var verses = await VerseTextResolver.ResolveSpansAsync(atlas.Client(), [zion]);

        // Assert
        Assert.Equal([], verses);
    }

    private static string InOrder(IEnumerable<PassageListVerse> verses) => System.Text.Json.JsonSerializer.Serialize(verses);

    private static string Window(params (string Ref, string Book, int Chapter, int Verse)[] units) =>
        $$"""{"version":"v","units":[{{string.Join(",", units.Select(u => $$"""{"ref":"{{u.Ref}}","locus":{"corpus":"bible","book":"{{u.Book}}","chapter":{{u.Chapter}},"verse":{{u.Verse}}},"text":"","words_of_christ":[],"edge_summary":[],"anchors":[]}"""))}}]}""";
}
