using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class CrossingTests
{
    private const int Start = -1450;
    private const int End = -1399;
    private const int Before = -1500;
    private const int Inside = -1420;
    private const int After = -1350;

    private static readonly TimeRange Bounds = ServedGraph.Range(new Year(label: "1451 BC", value: Start), new Year(label: "1400 BC", value: End), "1451 BC – 1400 BC");

    [Fact]
    public void A_window_inside_the_bounds_crosses_nothing()
    {
        // Act
        var crossed = Crossing.Of(Bounds, Inside, Inside);

        // Assert
        Assert.Null(crossed);
    }

    [Fact]
    public void A_window_reaching_past_the_end_crosses_to_the_next()
    {
        // Act
        var crossed = Crossing.Of(Bounds, Inside, After);

        // Assert
        Assert.Equal(ArrowDirection.Next, crossed);
    }

    [Fact]
    public void A_window_reaching_before_the_start_crosses_to_the_previous()
    {
        // Act
        var crossed = Crossing.Of(Bounds, Before, Inside);

        // Assert
        Assert.Equal(ArrowDirection.Previous, crossed);
    }

    [Fact]
    public void Touching_a_bound_is_not_crossing_it()
    {
        // Act
        var crossed = (Crossing.Of(Bounds, Start, Inside), Crossing.Of(Bounds, Inside, End), Crossing.Of(Bounds, Start, End));

        // Assert
        Assert.Equal(((ArrowDirection?)null, (ArrowDirection?)null, (ArrowDirection?)null), crossed);
    }

    [Fact]
    public async Task Crossing_follows_the_first_neighbour_of_its_kind_and_records_the_hop()
    {
        // Arrange
        var (explorer, from) = Eras();

        // Act
        var outcome = await Crossing.Walk(EdgeKind.FollowsIn).Run(explorer, from);

        // Assert
        var judges = await explorer.BeginAt(ServedGraph.At(Judges));
        Assert.Equal(new Outcome<(Explorable, Exploration)>.Arrived((judges, from.Follow(new Step(EdgeKind.FollowsIn, judges)))), outcome);
    }

    [Fact]
    public async Task Crossing_where_no_neighbour_of_its_kind_is_served_stays_put()
    {
        // Arrange
        var (explorer, from) = Eras();

        // Act
        var outcome = await Crossing.Walk(EdgeKind.PrecedesIn).Run(explorer, from);

        // Assert
        Assert.Equal(new Outcome<(Explorable, Exploration)>.Arrived((from.Start, from)), outcome);
    }

    private static readonly NodeRef Conquest = ServedGraph.Ref(NodeKind.Map, "Map:era-conquest", "The Conquest");
    private static readonly NodeRef Judges = ServedGraph.Ref(NodeKind.Map, "Map:era-judges", "The Judges");

    private static (IExplorer Explorer, Exploration From) Eras()
    {
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Map, Conquest.Id, Conquest.Label, new FrontierGroup(EdgeKind.FollowsIn, 1)))
            .Serving(ServedGraph.Card(NodeKind.Map, Judges.Id, Judges.Label))
            .Serving(Conquest.Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, Judges));
        return (new GraphExplorer(graph), new Exploration(Resolved.Node(graph, Conquest), []));
    }

    [Fact]
    public async Task Crossing_after_the_frame_is_presented_reads_only_the_next_element()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Map, Conquest.Id, Conquest.Label, new FrontierGroup(EdgeKind.FollowsIn, 1)) with { Map = ServedGraph.MapWindow(Bounds) })
            .Serving(ServedGraph.Card(NodeKind.Map, Judges.Id, Judges.Label))
            .Serving(Conquest.Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, Judges));
        var explorer = new GraphExplorer(graph);
        var from = new Exploration(Resolved.Node(graph, Conquest), []);
        await new GraphPresenter().Present(new PresentationRequest(from.Current, Surface.World));
        var (elementsBefore, neighboursBefore) = (graph.ElementReads, graph.NeighbourReads);

        // Act
        await Crossing.Walk(EdgeKind.FollowsIn).Run(explorer, from);

        // Assert
        Assert.Equal((1, 0), (graph.ElementReads - elementsBefore, graph.NeighbourReads - neighboursBefore));
    }

    [Fact]
    public async Task An_explorable_reads_a_page_of_its_neighbours_once()
    {
        // Arrange
        var (graph, conquest) = Paged();

        // Act
        var pages = (await conquest.Entries(EdgeKind.FollowsIn), await conquest.Entries(EdgeKind.FollowsIn));

        // Assert
        Assert.Equal((pages.Item1, 1), (pages.Item2, graph.NeighbourReads));
    }

    [Fact]
    public async Task An_explorable_reads_each_cursor_and_limit_as_its_own_page()
    {
        // Arrange
        const int Limit = 1;
        var (graph, conquest) = Paged();

        // Act
        await conquest.Entries(EdgeKind.FollowsIn);
        await conquest.Entries(EdgeKind.FollowsIn, limit: Limit);
        await conquest.Entries(EdgeKind.FollowsIn, cursor: Limit);

        // Assert
        Assert.Equal(3, graph.NeighbourReads);
    }

    [Fact]
    public async Task A_page_whose_read_failed_is_read_again()
    {
        // Arrange
        var (graph, conquest) = Paged();
        graph.Failing(Conquest.Id, EdgeKind.FollowsIn);
        await Record.ExceptionAsync(() => conquest.Entries(EdgeKind.FollowsIn));
        graph.Serving(Conquest.Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, Judges));

        // Act
        var page = await conquest.Entries(EdgeKind.FollowsIn);

        // Assert
        Assert.Equal((new Page<Entry>([ServedGraph.EntryTo(EdgeKind.FollowsIn, ServedGraph.At(Judges))], null, null), 2), (page, graph.NeighbourReads));
    }

    private static (ServedGraph Graph, Explorable Conquest) Paged()
    {
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Map, Conquest.Id, Conquest.Label, new FrontierGroup(EdgeKind.FollowsIn, 1)))
            .Serving(Conquest.Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, Judges))
            .Serving(Conquest.Id, EdgeKind.FollowsIn, 1, ServedGraph.Page(EdgeKind.FollowsIn, null, Judges));
        return (graph, Resolved.Node(graph, Conquest));
    }
}
