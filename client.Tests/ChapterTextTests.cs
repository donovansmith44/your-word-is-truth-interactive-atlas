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
        Assert.Equivalent(
            new[] { new PassageListVerse("GEN.1.1", "In the beginning God created the heaven and the earth.", Anchors: [god], WordsOfChrist: []) },
            verses,
            strict: true);
    }
}
