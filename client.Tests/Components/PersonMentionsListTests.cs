using BibleAtlas.Client.Components;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using Bunit;

namespace BibleAtlas.Client.Tests;

public sealed class PersonMentionsListTests : BunitContext
{
    private const string Abraham = "Person:abraham";
    private const string Mention = "[data-testid^='person-mention-']";
    private const int Mentions = 1_000;

    private static readonly int Step = Affordances.MentionedIn.InitialClamp;

    [Fact]
    public async Task Revealing_mentions_slides_a_bounded_window_and_fewer_slides_it_back()
    {
        // Arrange
        var graph = new MentionsGraph();
        var mentions = await Paging.Window(graph, Abraham, EdgeKind.MentionedIn);
        var view = Render<PersonMentionsList>(p => p.Add(v => v.Mentions, mentions).Add(v => v.OnExplore, _ => { }));
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
            (WholeValue.Of(Verses(Step, slid * Step + Step)), WholeValue.Of(Verses(0, slid * Step))),
            (WholeValue.Of(furthest), WholeValue.Of(Shown(view))));
    }

    [Fact]
    public async Task A_reveal_that_fails_offers_to_try_again_and_trying_again_reads_what_was_wanted()
    {
        // Arrange
        var graph = new MentionsGraph { Failures = 1 };
        var mentions = await Paging.Window(graph, Abraham, EdgeKind.MentionedIn);
        var view = Render<PersonMentionsList>(p => p.Add(v => v.Mentions, mentions).Add(v => v.OnExplore, _ => { }));
        await view.Find("[data-testid='person-mentions-more']").ClickAsync(new());
        var failed = view.FindAll("[data-testid='could-not-load-retry']").Count;

        // Act
        await view.Find("[data-testid='could-not-load-retry']").ClickAsync(new());

        // Assert
        Assert.Equal((1, WholeValue.Of(Verses(0, 2 * Step))), (failed, WholeValue.Of(Shown(view))));
    }

    private static List<string> Shown(IRenderedComponent<PersonMentionsList> view) =>
        view.FindAll(Mention).Select(row => row.GetAttribute("data-testid")!).ToList();

    private static List<string> Verses(int from, int to) =>
        Enumerable.Range(from, to - from).Select(n => $"person-mention-GEN.1.{n}").ToList();

    private sealed class MentionsGraph : IExplorableClient
    {
        public int Failures { get; init; }

        private int _failed;

        public Task<NodeRecord> Card(string id) => throw new NotSupportedException();

        public Task<ElementPage> Elements(IReadOnlyList<string> ids) => throw new NotSupportedException();

        public Task<EdgePage> Edges(string positionId, EdgeKind kind, int? cursor = null, int limit = IExplorableClient.DefaultPageSize)
        {
            if (cursor is not null && _failed++ < Failures)
            {
                return Task.FromException<EdgePage>(new HttpRequestException("offline"));
            }

            var from = cursor ?? 0;
            var to = Math.Min(from + limit, Mentions);
            var verses = Enumerable.Range(from, to - from).Select(n => ServedGraph.Ref(NodeKind.TextUnit, $"text-unit:GEN.1.{n}", $"GEN.1.{n}")).ToArray();
            return Task.FromResult(ServedGraph.Page(kind, to < Mentions ? to : null, verses));
        }

        public Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
            throw new NotSupportedException();
    }
}
