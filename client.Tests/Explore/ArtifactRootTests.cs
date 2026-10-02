using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Legacy;

namespace BibleAtlas.Client.Tests;

public sealed class ArtifactRootTests
{
    private const string RootA = "root-a";
    private const string RootB = "root-b";

    private static readonly NodeRef Map = ServedGraph.Ref(NodeKind.Map, "Map:eden", "The world of Eden");
    private static readonly NodeRef Eden = ServedGraph.Ref(NodeKind.Place, "Place:eden", "Eden");
    private static readonly EdgeRef Shows = ServedGraph.EdgeRef(EdgeKind.Shows, "Shows:00aa", "The world of Eden · Shows · Eden");
    private static readonly PositionRef[] Elements = [ServedGraph.At(Map), ServedGraph.AtEdge(Shows)];

    [Fact]
    public void Every_element_carries_the_root_of_the_artifact_it_was_served_from()
    {
        // Arrange
        var graph = Graph().AtRoot(RootA);

        // Act
        var roots = Elements.Select(target => Resolved.At(graph, target).Root);

        // Assert
        Assert.Equal([RootA, RootA], roots);
    }

    [Fact]
    public void Every_element_is_the_same_entity_on_any_root_and_the_same_presentation_only_on_its_own()
    {
        // Arrange
        var served = Elements.Select(target => (
            Before: Resolved.At(Graph().AtRoot(RootA), target),
            Again: Resolved.At(Graph().AtRoot(RootA), target),
            Moved: Resolved.At(Graph().AtRoot(RootB), target))).ToList();

        // Act
        var laws = served.Select(element => (
            Entity: element.Before == element.Moved,
            SameRoot: new PresentationRequest(element.Before, Surface.Popover) == new PresentationRequest(element.Again, Surface.Popover),
            NewRoot: new PresentationRequest(element.Before, Surface.Popover) == new PresentationRequest(element.Moved, Surface.Popover)));

        // Assert
        Assert.Equal([(true, true, false), (true, true, false)], laws);
    }

    [Fact]
    public void A_retained_legacy_view_is_kept_on_its_own_root_and_presented_afresh_on_a_new_one()
    {
        // Arrange
        var terah = LegacyViews.TerahLeavesUr;
        var views = new LegacyPresentations();
        var before = views.Present(new Exploration(Resolved.Node(LegacyEvent().AtRoot(RootA), terah), []));

        // Act
        var again = views.Present(new Exploration(Resolved.Node(LegacyEvent().AtRoot(RootA), terah), []));
        var moved = views.Present(new Exploration(Resolved.Node(LegacyEvent().AtRoot(RootB), terah), []));

        // Assert
        Assert.Equal((true, false, true), (ReferenceEquals(before, again), ReferenceEquals(again, moved), moved is not null));
    }

    [Fact]
    public async Task A_page_already_read_on_a_root_is_not_asked_for_again_on_that_root_but_is_on_a_new_one()
    {
        // Arrange
        var graph = Graph().AtRoot(RootA);
        var explorer = new GraphExplorer(graph);
        await (await explorer.BeginAt(ServedGraph.At(Map))).Entries(EdgeKind.Shows);
        await (await explorer.BeginAt(ServedGraph.At(Map))).Entries(EdgeKind.Shows);
        var sameRoot = graph.NeighbourReads;
        graph.AtRoot(RootB);

        // Act
        await (await explorer.BeginAt(ServedGraph.At(Map))).Entries(EdgeKind.Shows);

        // Assert
        Assert.Equal((1, 2), (sameRoot, graph.NeighbourReads));
    }

    [Fact]
    public async Task Paging_an_element_after_its_artifact_moved_fails_and_keeps_nothing_from_the_new_artifact()
    {
        // Arrange
        var graph = Graph().AtRoot(RootA);
        var map = await new GraphExplorer(graph).BeginAt(ServedGraph.At(Map));
        graph.AtRoot(RootB);

        // Act
        var first = await Record.ExceptionAsync(() => map.Entries(EdgeKind.Shows));
        var second = await Record.ExceptionAsync(() => map.Entries(EdgeKind.Shows));

        // Assert
        Assert.Equal((typeof(ArtifactMoved), typeof(ArtifactMoved), 2), (first?.GetType(), second?.GetType(), graph.NeighbourReads));
    }

    [Fact]
    public async Task The_page_store_keeps_only_its_resident_pages()
    {
        // Arrange
        var cursors = Enumerable.Range(0, ServedPages.Resident + 1).Select(cursor => (int?)cursor).ToList();
        var graph = cursors.Aggregate(Graph(), (served, cursor) => served.Serving(Map.Id, EdgeKind.Shows, cursor, ServedGraph.Page(EdgeKind.Shows, null, Eden)));
        var map = await new GraphExplorer(graph).BeginAt(ServedGraph.At(Map));
        foreach (var cursor in cursors)
        {
            await map.Entries(EdgeKind.Shows, cursor);
        }

        // Act
        await map.Entries(EdgeKind.Shows, cursors[^1]);
        var newest = graph.NeighbourReads;
        await map.Entries(EdgeKind.Shows, cursors[0]);

        // Assert
        Assert.Equal((cursors.Count, cursors.Count + 1), (newest, graph.NeighbourReads));
    }

    private static ServedGraph LegacyEvent() =>
        new ServedGraph().Serving(ServedGraph.Card(NodeKind.Event, LegacyViews.TerahLeavesUr.Id, LegacyViews.TerahLeavesUr.Label));

    private static ServedGraph Graph() =>
        new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Map, Map.Id, Map.Label, new FrontierGroup(EdgeKind.Shows, 1)))
            .Serving(ServedGraph.EdgeRecordOf(Shows, Map, Eden))
            .Serving(Map.Id, EdgeKind.Shows, null, ServedGraph.Page(EdgeKind.Shows, null, Eden));
}
