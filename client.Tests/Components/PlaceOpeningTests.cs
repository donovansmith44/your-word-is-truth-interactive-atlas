using BibleAtlas.Client.Components;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Legacy;
using Bunit;
using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Web;

namespace BibleAtlas.Client.Tests;

public sealed class PlaceOpeningTests : BunitContext
{
    private static readonly NodeRef Hazor = new(id: "Place:hazor-1", kind: NodeKind.Place, label: "Hazor");
    private const string Mentioned = "Hazor";
    private const int VerseNumber = 1;
    private const string ChapterBook = "JOS";
    private const int ChapterNumber = 11;

    [Fact]
    public void A_place_mentioned_in_the_text_opens_on_its_served_node()
    {
        // Arrange
        PopoverOpening? opened = null;
        var mention = Render<MentionScan>(p => p
            .Add(v => v.Pieces, [new TextPiece(Mentioned, false, new Anchor(end: Mentioned.Length, kind: EdgeKind.Mentions, node: Hazor, start: 0))])
            .Add(v => v.Verse, VerseNumber)
            .Add(v => v.ClassPrefix, "mention")
            .Add(v => v.OnExplore, (PopoverOpening opening) => opened = opening));

        // Act
        mention.Find("[data-testid='mention-1-hazor-1']").Click();

        // Assert
        Assert.Equal(new PopoverOpening.Explore(new NodePosition(Hazor)), opened);
    }

    [Fact]
    public void A_place_mentioned_in_the_text_toggles_its_served_node_into_the_selection()
    {
        // Arrange
        NodeRef? toggled = null;
        var mention = Render<MentionScan>(p => p
            .Add(v => v.Pieces, [new TextPiece(Mentioned, false, new Anchor(end: Mentioned.Length, kind: EdgeKind.Mentions, node: Hazor, start: 0))])
            .Add(v => v.Verse, VerseNumber)
            .Add(v => v.ClassPrefix, "mention")
            .Add(v => v.OnExplore, (PopoverOpening _) => { })
            .Add(v => v.OnToggleSelect, (NodeRef node) => toggled = node));

        // Act
        mention.Find("[data-testid='mention-1-hazor-1']").Click(new MouseEventArgs { CtrlKey = true });

        // Assert
        Assert.Equal(Hazor, toggled);
    }

    [Fact]
    public async Task A_place_on_the_chapter_card_is_pushed_as_its_served_node()
    {
        // Arrange
        var chapter = new Chapter(book: ChapterBook, number: ChapterNumber, @ref: $"{ChapterBook}.{ChapterNumber}", verses:
        [
            new Verse(heading: null, persons: [], places: [new PlaceRef(id: "hazor-1", name: Mentioned, node: Hazor)], text: Mentioned, number: VerseNumber, wordsOfChrist: [], xrefCount: 0),
        ]);
        var context = new RecordingContext();
        var section = await new ChapterCardSection().ResolveAsync(new ChapterNode(ChapterBook, ChapterNumber) { AlreadyLoaded = chapter }, new StubbedAtlas("{}").Client(), context);
        var card = Render(section!.Body);

        // Act
        card.Find("[data-testid='chapter-card-place-hazor-1']").Click();

        // Assert
        Assert.Equal([(new PopoverOpening.Explore(new NodePosition(Hazor)), EdgeKind.Mentions)], context.Pushed);
    }

    private sealed class RecordingContext : IPopoverSectionContext
    {
        public List<(PopoverOpening Opening, EdgeKind Via)> Pushed { get; } = [];

        public Task PushAsync(PopoverOpening opening, EdgeKind via)
        {
            Pushed.Add((opening, via));
            return Task.CompletedTask;
        }

        public Task ToggleSelectAsync(NodeRef node) => Task.CompletedTask;

        public int OtherContextSectionCount => 0;

        public bool XrefEntryPoint => false;

        public IExplorableClient Graph => throw new NotSupportedException();

        public Explorable Current => throw new NotSupportedException();

        public Task RenewAsync() => Task.CompletedTask;

        public Task NavigateWorldAsync(string query) => Task.CompletedTask;
    }
}
