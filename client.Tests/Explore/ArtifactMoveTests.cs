using BibleAtlas.Client.Components;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Legacy;
using BibleAtlas.Client.State;
using BibleAtlas.Client.Views;
using Bunit;
using Microsoft.Extensions.DependencyInjection;

namespace BibleAtlas.Client.Tests;

public sealed class ArtifactMoveTests
{
    private const string RootA = "root-a";
    private const string RootB = "root-b";
    private const int Children = 25;
    private const int RenewingRead = 1;

    private static readonly NodeRef Map = ServedGraph.Ref(NodeKind.Map, "Map:eden", "The world of Eden");
    private static readonly NodeRef Eden = ServedGraph.Ref(NodeKind.Place, "Place:eden", "Eden");
    private static readonly EdgeRef Shows = ServedGraph.EdgeRef(EdgeKind.Shows, "Shows:00aa", "The world of Eden · Shows · Eden");
    private static readonly PositionRef[] Elements = [ServedGraph.At(Map), ServedGraph.AtEdge(Shows)];

    [Fact]
    public async Task A_step_onto_a_new_root_renews_the_whole_trail_on_it_in_one_more_read()
    {
        // Arrange
        var graph = Graph().AtRoot(RootA);
        var explorer = new GraphExplorer(graph);
        var from = await Walked(explorer, new Link(EdgeKind.Shows, ServedGraph.AtEdge(Shows)), new Link(EdgeKind.Shows, ServedGraph.At(Eden)));
        graph.AtRoot(RootB);
        var reads = graph.ElementReads;

        // Act
        var outcome = await Explore.Follow(new Link(EdgeKind.ShownOn, ServedGraph.At(Map))).Run(explorer, from);

        // Assert
        Assert.Equal(
            (true, WholeValue.Of(Enumerable.Repeat(RootB, 4)), 1 + RenewingRead),
            (outcome is Outcome<(Explorable, Exploration)>.Arrived, WholeValue.Of(Roots(outcome)), graph.ElementReads - reads));
    }

    [Fact]
    public async Task Links_read_after_the_artifact_moved_renew_the_trail_and_read_the_new_root()
    {
        // Arrange
        var graph = Graph().AtRoot(RootA);
        var explorer = new GraphExplorer(graph);
        var from = new Exploration(await explorer.BeginAt(ServedGraph.At(Map)), []);
        graph.AtRoot(RootB);

        // Act
        var outcome = await Explore.Links(EdgeKind.Shows).Run(explorer, from);

        // Assert
        Assert.Equal(
            (new Page<Link>([new Link(EdgeKind.Shows, ServedGraph.At(Eden))], null), RootB),
            outcome.Match(arrived: walked => (walked.Value, walked.Trail.Current.Root), failed: () => default, superseded: () => default));
    }

    [Fact]
    public void A_trail_on_a_new_root_is_a_new_exploration_and_opening_it_replaces_the_old()
    {
        // Arrange
        var before = Resolved.At(Graph().AtRoot(RootA), ServedGraph.At(Map));
        var moved = Resolved.At(Graph().AtRoot(RootB), ServedGraph.At(Map));
        var open = new ExplorationState.Open(new Exploration(before, []));

        // Act
        var opened = new ExplorationIntent.Open(moved).Apply(open);

        // Assert
        Assert.Equal((false, RootB), (new Exploration(before, []) == new Exploration(moved, []), (opened as ExplorationState.Open)?.Exploration.Current.Root));
    }

    [Fact]
    public void Every_element_revealed_after_its_artifact_moved_shows_the_new_artifact_s_entries()
    {
        // Arrange
        var expected = Elements.Select(target => (Failures: 0, Children: WholeValue.Of(Moved(Positions.Of(target).Id).Select(child => $"popover-child-shows-{child.Id}"))));

        // Act
        var shown = Elements.Select(RevealedAfterMoving).ToList();

        // Assert
        Assert.Equal(expected, shown);
    }

    [Fact]
    public void Trying_again_on_an_element_whose_artifact_moved_asks_for_renewal_and_never_rereads_the_old_root()
    {
        // Arrange
        using var context = new BunitContext();
        context.Services.AddSingleton<IPresenter>(new GraphPresenter());
        var graph = Revealing(ServedGraph.At(Map)).AtRoot(RootA);
        var renewals = 0;
        var view = context.Render<FocusView>(p => p
            .Add(v => v.Node, Resolved.At(graph, ServedGraph.At(Map)))
            .Add(v => v.Surface, Surface.Popover)
            .Add(v => v.OnMoved, () => renewals++));
        graph.AtRoot(RootB);
        view.Find("[data-testid='popover-children-shows-more']").Click();
        var reads = graph.NeighbourReads;

        // Act
        view.WaitForElement("[data-testid='could-not-load-retry']").Click();

        // Assert
        Assert.Equal((2, reads), (renewals, graph.NeighbourReads));
    }

    private static (int Failures, string Children) RevealedAfterMoving(PositionRef target)
    {
        using var context = new BunitContext();
        var id = Positions.Of(target).Id;
        var graph = Revealing(target).AtRoot(RootA);
        Hosting(context, graph);
        var popover = context.Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Explore(target)));
        var more = popover.WaitForElement("[data-testid='popover-children-shows-more']");
        graph.AtRoot(RootB).Serving(id, EdgeKind.Shows, null, ServedGraph.Page(EdgeKind.Shows, null, Moved(id)));
        more.Click();
        popover.WaitForAssertion(() => Assert.Equal(Moved(id).Length, popover.FindAll(".focus-child").Count));
        return (popover.FindAll("[data-testid='could-not-load']").Count, WholeValue.Of(popover.FindAll(".focus-child").Select(child => child.GetAttribute("data-testid"))));
    }

    private static NodeRef[] Moved(string id) => [ServedGraph.Ref(NodeKind.Person, $"Person:{id}-moved", "Moved")];

    private static NodeRef[] Original(int from, int to) =>
        Enumerable.Range(from, to - from).Select(n => ServedGraph.Ref(NodeKind.Person, $"Person:{n}", $"Person {n}")).ToArray();

    private static ServedGraph Revealing(PositionRef target)
    {
        var id = Positions.Of(target).Id;
        var clamp = Affordances.PageSize;
        return new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Map, Map.Id, Map.Label, new FrontierGroup(EdgeKind.Shows, Children)))
            .Serving(ServedGraph.EdgeRecordOf(Shows, Map, Eden, new FrontierGroup(EdgeKind.Shows, Children)))
            .Serving(id, EdgeKind.Shows, null, ServedGraph.Page(EdgeKind.Shows, clamp, Original(0, clamp)))
            .Serving(id, EdgeKind.Shows, clamp, ServedGraph.Page(EdgeKind.Shows, null, Original(clamp, Children)));
    }

    private static IEnumerable<string> Roots(Outcome<(Explorable Value, Exploration Trail)> outcome) =>
        outcome.Match(
            arrived: walked => walked.Trail.Steps.Select(step => step.Target.Root).Prepend(walked.Trail.Start.Root).ToList(),
            failed: () => [],
            superseded: () => []);

    private static async Task<Exploration> Walked(IExplorer explorer, params Link[] links)
    {
        var start = new Exploration(await explorer.BeginAt(ServedGraph.At(Map)), []);
        return (await Explore.Replay(links).Run(explorer, start)).Match(arrived: walked => walked.Trail, failed: () => start, superseded: () => start);
    }

    private static ServedGraph Graph() =>
        new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Map, Map.Id, Map.Label, new FrontierGroup(EdgeKind.Shows, 1)))
            .Serving(ServedGraph.Card(NodeKind.Place, Eden.Id, Eden.Label))
            .Serving(ServedGraph.EdgeRecordOf(Shows, Map, Eden))
            .Serving(Map.Id, EdgeKind.Shows, null, ServedGraph.Page(EdgeKind.Shows, null, Eden));

    private static void Hosting(BunitContext context, IExplorableClient graph)
    {
        context.JSInterop.Mode = JSRuntimeMode.Loose;
        context.Services.AddSingleton(new AtlasClient(new HttpClient { BaseAddress = new Uri("http://unserved.invalid") }));
        context.Services.AddSingleton<IExplorableClient>(graph);
        context.Services.AddSingleton<IExplorer>(new GraphExplorer(graph));
        context.Services.AddSingleton<IPresenter>(new GraphPresenter());
        context.Services.AddSingleton(new SavedExplorationsService(new InMemoryLocalStorage()));
        context.Services.AddSingleton<OwnershipRegistry>();
        context.Services.AddSingleton(new StateAtom<IReadOnlyList<NodeRef>>(AtomNames.Selection, Selection.Empty, SequenceEqualityComparer<NodeRef>.Instance));
        AppServices.AddStateAtoms(context.Services);
    }
}
