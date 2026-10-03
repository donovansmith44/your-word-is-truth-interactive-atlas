using BibleAtlas.Client.Components;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using Bunit;

namespace BibleAtlas.Client.Tests;

public sealed class PersonMentionsListTests : BunitContext
{
    private const string Mention = "[data-testid^='person-mention-']";
    private const int Mentions = 1_000;
    private const string RootA = "root-a";
    private const string RootB = "root-b";

    private static readonly int Step = Affordances.PageSize;
    private static readonly Provenance Kjv = new(id: "kjv", title: "The King James Version");

    [Fact]
    public async Task Revealing_mentions_slides_a_bounded_window_and_fewer_slides_it_back()
    {
        // Arrange
        var view = Listed(await new RootedMentions(Mentions).Opened(), () => { });
        var slid = PageWindow.BlocksShown(Step);
        for (var more = 0; more < slid; more++)
        {
            await view.Find("[data-testid='person-mentions-more']").ClickAsync(new());
        }

        var furthest = Shown(view);

        // Act
        await view.Find("[data-testid='person-mentions-less']").ClickAsync(new());

        // Assert
        Assert.Equal(
            (WholeValue.Of(Verses(RootA, Step, slid * Step + Step)), WholeValue.Of(Verses(RootA, 0, slid * Step))),
            (WholeValue.Of(furthest), WholeValue.Of(Shown(view))));
    }

    [Fact]
    public async Task A_reveal_that_fails_offers_to_try_again_and_trying_again_reads_what_was_wanted()
    {
        // Arrange
        var graph = new RootedMentions(Mentions);
        var view = Listed(await graph.Opened(), () => { });
        graph.Failing(1);
        await view.Find("[data-testid='person-mentions-more']").ClickAsync(new());
        var failed = view.FindAll("[data-testid='could-not-load-retry']").Count;

        // Act
        await view.Find("[data-testid='could-not-load-retry']").ClickAsync(new());

        // Assert
        Assert.Equal((1, WholeValue.Of(Verses(RootA, 0, 2 * Step))), (failed, WholeValue.Of(Shown(view))));
    }

    [Fact]
    public async Task A_reveal_answered_from_a_new_artifact_shows_nothing_of_it_and_asks_for_renewal()
    {
        // Arrange
        var graph = new RootedMentions(Mentions);
        var renewals = 0;
        var view = Listed(await graph.Opened(), () => renewals++);
        graph.Root = RootB;

        // Act
        await view.Find("[data-testid='person-mentions-more']").ClickAsync(new());

        // Assert
        Assert.Equal((WholeValue.Of(Verses(RootA, 0, Step)), 1), (WholeValue.Of(Shown(view)), renewals));
    }

    [Fact]
    public async Task Each_mention_shows_its_verses_served_words_under_it()
    {
        // Arrange
        var mentions = await new RootedMentions(Mentions).Opened();

        // Act
        var view = Listed(mentions, () => { });

        // Assert
        Assert.Equal(
            WholeValue.Of(mentions.Shown.Select(entry => (Positions.Of(entry.Neighbour.Target).Label, entry.Words!.Text)).ToList()),
            WholeValue.Of(Verses(RootA, 0, Step).Select(row => row["person-mention-".Length..])
                .Select(vref => (vref, view.Find($"[data-testid='person-words-{vref}-text']").TextContent)).ToList()));
    }

    [Fact]
    public async Task A_persons_mentions_name_their_source_by_its_served_title()
    {
        // Arrange
        var mentions = await new RootedMentions(Mentions).Opened();

        // Act
        var view = Render<PersonMentionsList>(p => p.Add(v => v.Provenance, Kjv).Add(v => v.Mentions, mentions).Add(v => v.TotalCount, Mentions).Add(v => v.OnExplore, _ => { }).Add(v => v.OnMoved, () => { }));

        // Assert
        Assert.Equal("Source: The King James Version", view.Find("[data-testid='popover-person-provenance']").TextContent);
    }

    private IRenderedComponent<PersonMentionsList> Listed(PageWindow<Entry> mentions, Action moved) =>
        Render<PersonMentionsList>(p => p.Add(v => v.Mentions, mentions).Add(v => v.TotalCount, Mentions).Add(v => v.OnExplore, _ => { }).Add(v => v.OnMoved, moved));

    private static List<string> Shown(IRenderedComponent<PersonMentionsList> view) =>
        view.FindAll(Mention).Select(row => row.GetAttribute("data-testid")!).ToList();

    private static List<string> Verses(string root, int from, int to) =>
        Enumerable.Range(from, to - from).Select(n => $"person-mention-{RootedMentions.Label(root, n)}").ToList();
}
