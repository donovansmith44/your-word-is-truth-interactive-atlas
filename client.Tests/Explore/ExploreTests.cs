using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class ExploreTests
{
    private const string AbsentId = "Container:bible-chapter-GEN-99";

    private static readonly NodeRef Genesis1Ref = ServedGraph.Ref(NodeKind.Container, "Container:bible-chapter-GEN-1", "Genesis 1");
    private static readonly NodeRef Genesis2Ref = ServedGraph.Ref(NodeKind.Container, "Container:bible-chapter-GEN-2", "Genesis 2");
    private static readonly NodeRef Genesis3Ref = ServedGraph.Ref(NodeKind.Container, "Container:bible-chapter-GEN-3", "Genesis 3");
    private static readonly NodeRef GenesisRef = ServedGraph.Ref(NodeKind.Container, "Container:bible-book-GEN", "Genesis");
    private static readonly NodeRef AbsentRef = ServedGraph.Ref(NodeKind.Container, AbsentId, AbsentId);

    private static readonly Link ToGenesis2 = new(EdgeKind.FollowsIn, ServedGraph.At(Genesis2Ref));
    private static readonly Link ToGenesis3 = new(EdgeKind.FollowsIn, ServedGraph.At(Genesis3Ref));
    private static readonly Link UpToGenesis = new(EdgeKind.MemberOf, ServedGraph.At(GenesisRef));
    private static readonly Link ToTheAbsent = new(EdgeKind.FollowsIn, ServedGraph.At(AbsentRef));

    private static readonly IReadOnlyList<(string Name, Explore<Explorable> Walk)> Primitives =
    [
        ("here", Explore.Here),
        ("back", Explore.Back),
        ("follow to Genesis 2", Explore.Follow(ToGenesis2)),
        ("follow to Genesis 3", Explore.Follow(ToGenesis3)),
        ("follow up to Genesis", Explore.Follow(UpToGenesis)),
        ("follow to the absent", Explore.Follow(ToTheAbsent)),
        ("follow the first follows-in link", Explore.Links(EdgeKind.FollowsIn).SelectMany(page => page.Items is [var first, ..] ? Explore.Follow(first) : Explore.Here)),
    ];

    private static readonly IReadOnlyList<(string Name, Func<Explorable, Explore<Explorable>> Bind)> Continuations =
    [
        .. Primitives.Select(primitive => ($"then {primitive.Name}", (Func<Explorable, Explore<Explorable>>)(_ => primitive.Walk))),
        ("then return it", Explore.Return),
        ("then follow what mentions it", node => Explore.Follow(new Link(EdgeKind.Mentions, node.Identity))),
    ];

    private static readonly IReadOnlyList<(string Name, Explore<Explorable> Walk)> Walks =
    [
        ("here", Explore.Here),
        .. Primitives,
        .. Primitives.SelectMany(first => Primitives.Select(second => ($"{first.Name}, {second.Name}", first.Walk.SelectMany(_ => second.Walk)))),
    ];

    [Fact]
    public void Returning_a_value_then_binding_is_the_bound_walk_itself()
    {
        // Arrange
        var (explorer, from) = Graph();

        // Act
        var broken = Continuations
            .Where(f => Run(Explore.Return(Genesis(explorer, Genesis2Ref)).SelectMany(f.Bind), explorer, from) != Run(f.Bind(Genesis(explorer, Genesis2Ref)), explorer, from))
            .Select(f => f.Name)
            .ToList();

        // Assert
        Assert.Empty(broken);
    }

    [Fact]
    public void Binding_a_walk_to_return_is_the_walk_itself()
    {
        // Arrange
        var (explorer, from) = Graph();

        // Act
        var broken = Walks
            .Where(m => Run(m.Walk.SelectMany(Explore.Return), explorer, from) != Run(m.Walk, explorer, from))
            .Select(m => m.Name)
            .ToList();

        // Assert
        Assert.Empty(broken);
    }

    [Fact]
    public void Binding_is_associative()
    {
        // Arrange
        var (explorer, from) = Graph();

        // Act
        var broken = Walks
            .SelectMany(m => Continuations.SelectMany(f => Continuations.Select(g => (Name: $"{m.Name} / {f.Name} / {g.Name}", m.Walk, F: f.Bind, G: g.Bind))))
            .Where(law => Run(law.Walk.SelectMany(law.F).SelectMany(law.G), explorer, from) != Run(law.Walk.SelectMany(x => law.F(x).SelectMany(law.G)), explorer, from))
            .Select(law => law.Name)
            .ToList();

        // Assert
        Assert.Empty(broken);
    }

    [Fact]
    public void The_laws_are_measured_over_walks_that_arrive_and_walks_that_fail()
    {
        // Arrange
        var (explorer, from) = Graph();

        // Act
        var outcomes = Walks.Select(m => Run(m.Walk, explorer, from).Match(arrived: _ => "arrived", failed: () => "failed", superseded: () => "superseded")).Distinct().Order().ToList();

        // Assert
        Assert.Equal(["arrived", "failed"], outcomes);
    }

    [Fact]
    public async Task A_failed_request_ends_the_walk_before_any_later_step_runs_or_asks()
    {
        // Arrange
        var (graph, explorer, from) = Served();
        var readsBefore = graph.ElementReads;
        var laterStepRan = false;
        var walk = Explore.Follow(ToTheAbsent).SelectMany(_ =>
        {
            laterStepRan = true;
            return Explore.Follow(ToGenesis2);
        });

        // Act
        var outcome = await walk.Run(explorer, from);

        // Assert
        Assert.Equal(((Outcome<(Explorable, Exploration)>)new Outcome<(Explorable, Exploration)>.Failed(), false, 1), (outcome, laterStepRan, graph.ElementReads - readsBefore));
    }

    [Fact]
    public async Task A_query_walk_records_each_step_and_its_trail_is_its_breadcrumb()
    {
        // Arrange
        var (explorer, from) = Graph();
        var walk =
            from chapter in Explore.Here
            from next in Explore.Follow(ToGenesis2)
            from book in Explore.Follow(UpToGenesis)
            select (chapter.Label, next.Label, book.Label);

        // Act
        var outcome = await walk.Run(explorer, from);

        // Assert
        var trail = new Exploration(from.Start, [new Step(EdgeKind.FollowsIn, Genesis(explorer, Genesis2Ref)), new Step(EdgeKind.MemberOf, Genesis(explorer, GenesisRef))]);
        Assert.Equal(
            (new Outcome<((string, string, string), Exploration)>.Arrived(((Genesis1Ref.Label, Genesis2Ref.Label, GenesisRef.Label), trail)), true),
            (outcome, outcome.Match(arrived: walked => walked.Trail.Steps.SequenceEqual(walked.Trail.Breadcrumb), failed: () => false, superseded: () => false)));
    }

    [Fact]
    public async Task Links_pages_the_current_node_s_links_of_a_kind_without_moving()
    {
        // Arrange
        var (explorer, from) = Graph();

        // Act
        var outcome = await Explore.Links(EdgeKind.FollowsIn).Run(explorer, from);

        // Assert
        Assert.Equal(new Outcome<(Page<Link>, Exploration)>.Arrived((new Page<Link>([ToGenesis2], null, null), from)), outcome);
    }

    [Fact]
    public async Task Links_from_a_cursor_read_the_page_the_cursor_names()
    {
        // Arrange
        const int SecondPage = 1;
        var (graph, explorer, from) = Served();
        graph.Serving(Genesis1Ref.Id, EdgeKind.FollowsIn, SecondPage, ServedGraph.Page(EdgeKind.FollowsIn, null, Genesis3Ref));

        // Act
        var outcome = await Explore.Links(EdgeKind.FollowsIn, SecondPage).Run(explorer, from);

        // Assert
        Assert.Equal(new Outcome<(Page<Link>, Exploration)>.Arrived((new Page<Link>([ToGenesis3], null, null), from)), outcome);
    }

    [Fact]
    public async Task Following_a_link_records_one_step_to_its_target()
    {
        // Arrange
        var (explorer, from) = Graph();

        // Act
        var outcome = await Explore.Follow(ToGenesis2).Run(explorer, from);

        // Assert
        var genesis2 = Genesis(explorer, Genesis2Ref);
        Assert.Equal(new Outcome<(Explorable, Exploration)>.Arrived((genesis2, from.Follow(new Step(EdgeKind.FollowsIn, genesis2)))), outcome);
    }

    [Fact]
    public async Task Beginning_resolves_the_start_into_an_exploration_that_has_gone_nowhere()
    {
        // Arrange
        var (explorer, _) = Graph();

        // Act
        var begun = await Explore.Begin(explorer, ServedGraph.At(Genesis1Ref));

        // Assert
        Assert.Equal(new Outcome<Exploration>.Arrived(new Exploration(Genesis(explorer, Genesis1Ref), [])), begun);
    }

    [Fact]
    public async Task Beginning_at_a_position_the_graph_cannot_resolve_fails()
    {
        // Arrange
        var (explorer, _) = Graph();

        // Act
        var begun = await Explore.Begin(explorer, ServedGraph.At(AbsentRef));

        // Assert
        Assert.Equal(new Outcome<Exploration>.Failed(), begun);
    }

    [Fact]
    public async Task Going_back_follows_the_dual_of_the_last_hop_to_the_node_it_left()
    {
        // Arrange
        var (explorer, from) = Graph();
        var walk = from before in Explore.Here from _ in Explore.Follow(ToGenesis2) from back in Explore.Back select (Before: before, Back: back);

        // Act
        var outcome = await walk.Run(explorer, from);

        // Assert
        var genesis2 = Genesis(explorer, Genesis2Ref);
        Assert.Equal(
            new Outcome<((Explorable, Explorable), Exploration)>.Arrived(((from.Start, from.Start), new Exploration(from.Start, [new Step(EdgeKind.FollowsIn, genesis2), new Step(EdgeKind.PrecedesIn, from.Start)]))),
            outcome);
    }

    [Fact]
    public void Going_back_after_any_follow_returns_to_the_node_before_it()
    {
        // Arrange
        var (explorer, from) = Graph();
        var links = new[] { ToGenesis2, ToGenesis3, UpToGenesis };

        // Act
        var strayed = links
            .Where(link => Run(from before in Explore.Here from _ in Explore.Follow(link) from back in Explore.Back select before == back, explorer, from).Equals(Arrived(true, explorer, from, link)) is false)
            .ToList();

        // Assert
        Assert.Empty(strayed);
    }

    [Fact]
    public async Task Going_back_twice_retraces_two_hops()
    {
        // Arrange
        var (explorer, from) = Graph();
        var walk = Explore.Follow(ToGenesis2).SelectMany(_ => Explore.Follow(ToGenesis3)).SelectMany(_ => Explore.Back).SelectMany(_ => Explore.Back);

        // Act
        var outcome = await walk.Run(explorer, from);

        // Assert
        var (genesis2, genesis3) = (Genesis(explorer, Genesis2Ref), Genesis(explorer, Genesis3Ref));
        Assert.Equal(
            new Outcome<(Explorable, Exploration)>.Arrived((from.Start, new Exploration(from.Start, [new Step(EdgeKind.FollowsIn, genesis2), new Step(EdgeKind.FollowsIn, genesis3), new Step(EdgeKind.PrecedesIn, genesis2), new Step(EdgeKind.PrecedesIn, from.Start)]))),
            outcome);
    }

    [Fact]
    public async Task Going_back_from_the_start_changes_nothing()
    {
        // Arrange
        var (explorer, from) = Graph();

        // Act
        var outcome = await Explore.Back.Run(explorer, from);

        // Assert
        Assert.Equal(new Outcome<(Explorable, Exploration)>.Arrived((from.Start, from)), outcome);
    }

    [Fact]
    public async Task Going_back_once_every_hop_is_retraced_never_goes_forward()
    {
        // Arrange
        var (explorer, from) = Graph();
        var walk = Explore.Follow(ToGenesis2).SelectMany(_ => Explore.Back).SelectMany(_ => Explore.Back);

        // Act
        var outcome = await walk.Run(explorer, from);

        // Assert
        Assert.Equal(
            new Outcome<(Explorable, Exploration)>.Arrived((from.Start, new Exploration(from.Start, [new Step(EdgeKind.FollowsIn, Genesis(explorer, Genesis2Ref)), new Step(EdgeKind.PrecedesIn, from.Start)]))),
            outcome);
    }

    [Fact]
    public void Replaying_the_steps_of_any_arrived_trail_from_its_start_reproduces_it()
    {
        // Arrange
        var (explorer, from) = Graph();
        var trails = Walks.Select(m => Run(m.Walk, explorer, from)).OfType<Outcome<(Explorable Value, Exploration Trail)>.Arrived>().Select(arrived => arrived.Value.Trail).ToList();

        // Act
        var unreproduced = trails
            .Where(trail => Run(Explore.Replay(trail.Steps.Select(step => new Link(step.Kind, step.Target.Identity)).ToList()), explorer, new Exploration(trail.Start, [])).Equals(new Outcome<(Unit, Exploration)>.Arrived((default, trail))) is false)
            .ToList();

        // Assert
        Assert.Equal((true, 0), (trails.Any(trail => trail.Steps.Count > 1), unreproduced.Count));
    }

    [Fact]
    public async Task Replaying_links_resolves_every_target_in_one_element_read_then_walks_them()
    {
        // Arrange
        var (graph, explorer, from) = Served();
        var readsBefore = graph.ElementReads;

        // Act
        var outcome = await Explore.Replay([ToGenesis2, ToGenesis3, UpToGenesis]).Run(explorer, from);
        var reads = graph.ElementReads - readsBefore;

        // Assert
        var trail = new Exploration(from.Start, [new Step(EdgeKind.FollowsIn, Genesis(explorer, Genesis2Ref)), new Step(EdgeKind.FollowsIn, Genesis(explorer, Genesis3Ref)), new Step(EdgeKind.MemberOf, Genesis(explorer, GenesisRef))]);
        Assert.Equal(((Outcome<(Unit, Exploration)>)new Outcome<(Unit, Exploration)>.Arrived((default, trail)), 1), (outcome, reads));
    }

    [Fact]
    public async Task Replaying_no_links_asks_the_graph_nothing()
    {
        // Arrange
        var (graph, explorer, from) = Served();
        var readsBefore = graph.ElementReads;

        // Act
        var outcome = await Explore.Replay([]).Run(explorer, from);

        // Assert
        Assert.Equal(((Outcome<(Unit, Exploration)>)new Outcome<(Unit, Exploration)>.Arrived((default, from)), 0), (outcome, graph.ElementReads - readsBefore));
    }

    [Fact]
    public async Task Replaying_a_link_the_graph_cannot_resolve_fails_the_replay()
    {
        // Arrange
        var (explorer, from) = Graph();

        // Act
        var outcome = await Explore.Replay([ToGenesis2, ToTheAbsent]).Run(explorer, from);

        // Assert
        Assert.Equal(new Outcome<(Unit, Exploration)>.Failed(), outcome);
    }

    [Fact]
    public async Task Resuming_resolves_the_start_and_every_trail_target_in_one_element_read()
    {
        // Arrange
        var (graph, explorer, _) = Served();
        var readsBefore = graph.ElementReads;

        // Act
        var outcome = await Explore.Resume(explorer, ServedGraph.At(Genesis1Ref), [ToGenesis2, ToGenesis3, UpToGenesis]);
        var reads = graph.ElementReads - readsBefore;

        // Assert
        var trail = new Exploration(Genesis(explorer, Genesis1Ref), [new Step(EdgeKind.FollowsIn, Genesis(explorer, Genesis2Ref)), new Step(EdgeKind.FollowsIn, Genesis(explorer, Genesis3Ref)), new Step(EdgeKind.MemberOf, Genesis(explorer, GenesisRef))]);
        Assert.Equal(((Outcome<Exploration>)new Outcome<Exploration>.Arrived(trail), 1), (outcome, reads));
    }

    [Fact]
    public async Task Resuming_with_no_trail_is_one_element_read()
    {
        // Arrange
        var (graph, explorer, _) = Served();
        var readsBefore = graph.ElementReads;

        // Act
        var outcome = await Explore.Resume(explorer, ServedGraph.At(Genesis1Ref), []);
        var reads = graph.ElementReads - readsBefore;

        // Assert
        Assert.Equal(((Outcome<Exploration>)new Outcome<Exploration>.Arrived(new Exploration(Genesis(explorer, Genesis1Ref), [])), 1), (outcome, reads));
    }

    [Fact]
    public async Task Resuming_through_a_link_the_graph_cannot_resolve_fails()
    {
        // Arrange
        var (explorer, _) = Graph();

        // Act
        var outcome = await Explore.Resume(explorer, ServedGraph.At(Genesis1Ref), [ToGenesis2, ToTheAbsent]);

        // Assert
        Assert.Equal(new Outcome<Exploration>.Failed(), outcome);
    }

    private static Outcome<(T Value, Exploration Trail)> Run<T>(Explore<T> walk, IExplorer explorer, Exploration from) =>
        walk.Run(explorer, from).GetAwaiter().GetResult();

    private static Outcome<(bool, Exploration)> Arrived(bool value, IExplorer explorer, Exploration from, Link link)
    {
        var target = explorer.BeginAt(link.Target).GetAwaiter().GetResult();
        return new Outcome<(bool, Exploration)>.Arrived((value, from.Follow(new Step(link.Kind, target)).Follow(new Step(link.Kind.Dual(), from.Start))));
    }

    private static Explorable Genesis(IExplorer explorer, NodeRef node) => explorer.BeginAt(ServedGraph.At(node)).GetAwaiter().GetResult();

    private static (IExplorer Explorer, Exploration From) Graph()
    {
        var (_, explorer, from) = Served();
        return (explorer, from);
    }

    private static (ServedGraph Graph, IExplorer Explorer, Exploration From) Served()
    {
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Container, Genesis1Ref.Id, Genesis1Ref.Label, new FrontierGroup(EdgeKind.FollowsIn, 2), new FrontierGroup(EdgeKind.MemberOf, 1)))
            .Serving(ServedGraph.Card(NodeKind.Container, Genesis2Ref.Id, Genesis2Ref.Label, new FrontierGroup(EdgeKind.FollowsIn, 1), new FrontierGroup(EdgeKind.PrecedesIn, 1)))
            .Serving(ServedGraph.Card(NodeKind.Container, Genesis3Ref.Id, Genesis3Ref.Label, new FrontierGroup(EdgeKind.PrecedesIn, 1)))
            .Serving(ServedGraph.Card(NodeKind.Container, GenesisRef.Id, GenesisRef.Label))
            .Serving(Genesis1Ref.Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, Genesis2Ref))
            .Serving(Genesis2Ref.Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, Genesis3Ref));
        var explorer = new GraphExplorer(graph);
        return (graph, explorer, new Exploration(Resolved.Node(graph, Genesis1Ref), []));
    }
}
