using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests;

public sealed class ExplorationInterleavingTests
{
    private static readonly NodeRef Genesis1 = ServedGraph.Ref(NodeKind.Container, "Container:bible-chapter-GEN-1", "Genesis 1");
    private static readonly NodeRef Genesis2 = ServedGraph.Ref(NodeKind.Container, "Container:bible-chapter-GEN-2", "Genesis 2");
    private static readonly NodeRef Genesis = ServedGraph.Ref(NodeKind.Container, "Container:bible-book-GEN", "Genesis");

    [Fact]
    public async Task A_walk_arriving_after_another_navigation_landed_does_not_overwrite_it()
    {
        // Arrange
        var (graph, from, atom) = Hosting();
        var explorer = new GraphExplorer(graph);
        var slow = new GatedGraph(graph);
        var first = Explore.Follow(new Link(EdgeKind.FollowsIn, ServedGraph.At(Genesis2))).Run(new GraphExplorer(slow), from);
        var second = await Trail(Explore.Follow(new Link(EdgeKind.MemberOf, ServedGraph.At(Genesis))), explorer, from);

        // Act
        atom.Dispatch(new ExplorationIntent.Arrive(from, second));
        slow.Open();
        atom.Dispatch(new ExplorationIntent.Arrive(from, await Trail(first)));

        // Assert
        Assert.Equal(new ExplorationState.Open(second), atom.Value);
    }

    [Fact]
    public async Task A_walk_arriving_on_the_exploration_it_started_from_lands()
    {
        // Arrange
        var (graph, from, atom) = Hosting();
        var explorer = new GraphExplorer(graph);
        var walked = await Trail(Explore.Follow(new Link(EdgeKind.FollowsIn, ServedGraph.At(Genesis2))), explorer, from);

        // Act
        atom.Dispatch(new ExplorationIntent.Arrive(from, walked));

        // Assert
        Assert.Equal(new ExplorationState.Open(walked), atom.Value);
    }

    [Fact]
    public async Task A_walk_arriving_after_the_exploration_closed_stays_closed()
    {
        // Arrange
        var (graph, from, atom) = Hosting();
        var explorer = new GraphExplorer(graph);
        var walked = await Trail(Explore.Follow(new Link(EdgeKind.FollowsIn, ServedGraph.At(Genesis2))), explorer, from);
        atom.Dispatch(new ExplorationIntent.Reset());

        // Act
        atom.Dispatch(new ExplorationIntent.Arrive(from, walked));

        // Assert
        Assert.Equal(new ExplorationState.Closed(), atom.Value);
    }

    private static async Task<Exploration> Trail<T>(Explore<T> walk, IExplorer explorer, Exploration from) => await Trail(walk.Run(explorer, from));

    private static async Task<Exploration> Trail<T>(Task<Outcome<(T Value, Exploration Trail)>> walking) =>
        await walking is Outcome<(T Value, Exploration Trail)>.Arrived { Value.Trail: var trail } ? trail : throw new InvalidOperationException("the walk did not arrive");

    private static (ServedGraph Graph, Exploration From, StateAtom<ExplorationState> Atom) Hosting()
    {
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(Genesis1.Kind, Genesis1.Id, Genesis1.Label))
            .Serving(ServedGraph.Card(Genesis2.Kind, Genesis2.Id, Genesis2.Label))
            .Serving(ServedGraph.Card(Genesis.Kind, Genesis.Id, Genesis.Label));
        var from = new Exploration(Resolved.Node(graph, Genesis1), []);
        return (graph, from, new StateAtom<ExplorationState>(AtomNames.Exploration, new ExplorationState.Open(from)));
    }

    private sealed class GatedGraph(IExplorableClient graph) : IExplorableClient
    {
        private readonly TaskCompletionSource _gate = new();

        public void Open() => _gate.SetResult();

        public Task<NodeRecord> Card(NodeId id) => graph.Card(id);

        public async Task<ElementPage> Elements(IReadOnlyList<ElementId> ids)
        {
            await _gate.Task;
            return await graph.Elements(ids);
        }

        public Task<EdgePage> Edges(ElementId positionId, EdgeKind kind, EdgePageCursor? cursor = null, int limit = BibleAtlas.Client.Exploring.Affordances.PageSize) =>
            graph.Edges(positionId, kind, cursor, limit);

        public Task<TextWindow> Reading(TextWindowReference fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
            graph.Reading(fromRef, n, dir, corpus);
    }
}
