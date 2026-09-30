using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

public class SelectionTests
{
    private const int PairedTogglesPerRun = 25;
    private const int DispatchesPerRun = 30;
    private const int FoldedDispatchesPerRun = 40;
    private const int ProjectionsWatching = 4;
    private const int PoolSize = 6;

    private static StateAtom<IReadOnlyList<NodeRef>> NewAtom(IReadOnlyList<NodeRef>? initial = null) =>
        new("selection", initial ?? Selection.Empty, SequenceEqualityComparer<NodeRef>.Instance);

    private static NodeRef Place(string id) => ServedGraph.Ref(NodeKind.Place, $"Place:{id}", id);

    private static List<NodeRef> Pool() => Enumerable.Range(0, PoolSize).Select(i => Place($"item-{i}")).ToList();

    [Fact]
    public void A_dispatch_that_reproduces_the_current_list_raises_no_change()
    {
        // Arrange
        var atom = NewAtom();
        var changes = 0;
        atom.Changed += () => changes++;

        // Act
        atom.Dispatch(new ClearSelection());

        // Assert
        Assert.Equal(0, changes);
    }

    [Fact]
    public void Two_distinct_lists_with_the_same_nodes_in_the_same_order_are_one_selection()
    {
        // Arrange
        var a = new List<NodeRef> { Place("x"), Place("y") };
        var b = new List<NodeRef> { Place("x"), Place("y") };

        // Act
        var same = SequenceEqualityComparer<NodeRef>.Instance.Equals(a, b);

        // Assert
        Assert.Equal((false, true), (ReferenceEquals(a, b), same));
    }

    [Fact]
    public void A_selection_is_ordered_so_the_same_nodes_in_another_order_are_another_selection()
    {
        // Arrange
        var a = new List<NodeRef> { Place("x"), Place("y") };
        var b = new List<NodeRef> { Place("y"), Place("x") };

        // Act
        var same = SequenceEqualityComparer<NodeRef>.Instance.Equals(a, b);

        // Assert
        Assert.False(same);
    }

    [Fact]
    public void Toggling_a_node_not_yet_selected_adds_it()
    {
        // Arrange
        var atom = NewAtom();

        // Act
        atom.Dispatch(new ToggleSelection(Place("place-1")));

        // Assert
        Assert.Equal([Place("place-1")], atom.Value);
    }

    [Fact]
    public void Toggling_a_selected_node_removes_it()
    {
        // Arrange
        var atom = NewAtom([Place("place-1")]);

        // Act
        atom.Dispatch(new ToggleSelection(Place("place-1")));

        // Assert
        Assert.Empty(atom.Value);
    }

    [Fact]
    public void Two_toggles_of_the_same_node_return_to_the_original_selection()
    {
        // Arrange
        var original = new List<NodeRef> { Place("existing") };
        var atom = NewAtom(original);

        // Act
        atom.Dispatch(new ToggleSelection(Place("place-1")));
        atom.Dispatch(new ToggleSelection(Place("place-1")));

        // Assert
        Assert.Equal(original, atom.Value);
    }

    [Theory]
    [InlineData(8001)]
    [InlineData(8002)]
    [InlineData(8003)]
    public void Paired_toggles_cancel_over_any_generated_sequence(int seed)
    {
        // Arrange
        var rng = new Random(seed);
        var pool = Pool();
        var atom = NewAtom();
        var afterEachPair = new List<IReadOnlyList<NodeRef>>();

        // Act
        for (var i = 0; i < PairedTogglesPerRun; i++)
        {
            var node = pool[rng.Next(pool.Count)];
            atom.Dispatch(new ToggleSelection(node));
            atom.Dispatch(new ToggleSelection(node));
            afterEachPair.Add(atom.Value);
        }

        // Assert
        Assert.All(afterEachPair, value => Assert.Empty(value));
    }

    [Fact]
    public void The_same_toggle_dispatched_twice_flips_twice()
    {
        // Arrange
        var atom = NewAtom();
        var changes = 0;
        atom.Changed += () => changes++;
        var toggle = new ToggleSelection(Place("place-1"));

        // Act
        atom.Dispatch(toggle);
        atom.Dispatch(toggle);

        // Assert
        Assert.Equal((2, 0), (changes, atom.Value.Count));
    }

    [Fact]
    public void A_node_is_the_same_selection_by_kind_and_id_whatever_its_label()
    {
        // Arrange
        var atom = NewAtom([ServedGraph.Ref(NodeKind.Place, "Place:p1", "Old Name")]);

        // Act
        atom.Dispatch(new ToggleSelection(ServedGraph.Ref(NodeKind.Place, "Place:p1", "New Name")));

        // Assert
        Assert.Empty(atom.Value);
    }

    [Fact]
    public void Removing_a_node_that_is_not_selected_changes_nothing()
    {
        // Arrange
        var atom = NewAtom([Place("a")]);
        var changes = 0;
        atom.Changed += () => changes++;

        // Act
        atom.Dispatch(new RemoveSelection(Place("not-there")));

        // Assert
        Assert.Equal(WholeValue.Of(new { Changes = 0, Value = new[] { Place("a") } }), WholeValue.Of(new { Changes = changes, atom.Value }));
    }

    [Fact]
    public void Removing_matches_by_kind_and_id_whatever_the_label()
    {
        // Arrange
        var atom = NewAtom([ServedGraph.Ref(NodeKind.Place, "Place:p1", "Old Name"), Place("b")]);

        // Act
        atom.Dispatch(new RemoveSelection(ServedGraph.Ref(NodeKind.Place, "Place:p1", "New Name")));

        // Assert
        Assert.Equal([Place("b")], atom.Value);
    }

    [Fact]
    public void Redispatching_the_same_remove_is_a_no_op()
    {
        // Arrange
        var atom = NewAtom([Place("a"), Place("b")]);
        var remove = new RemoveSelection(Place("a"));
        atom.Dispatch(remove);
        var changes = 0;
        atom.Changed += () => changes++;

        // Act
        atom.Dispatch(remove);

        // Assert
        Assert.Equal(WholeValue.Of(new { Changes = 0, Value = new[] { Place("b") } }), WholeValue.Of(new { Changes = changes, atom.Value }));
    }

    [Fact]
    public void Clearing_an_already_empty_selection_is_a_no_op()
    {
        // Arrange
        var atom = NewAtom([Place("a")]);
        atom.Dispatch(new ClearSelection());
        var changes = 0;
        atom.Changed += () => changes++;

        // Act
        atom.Dispatch(new ClearSelection());

        // Assert
        Assert.Equal((0, 0), (changes, atom.Value.Count));
    }

    [Theory]
    [InlineData(8101)]
    [InlineData(8102)]
    public void Every_projection_agrees_with_the_atom_after_every_dispatch(int seed)
    {
        // Arrange
        var rng = new Random(seed);
        var pool = Pool();
        var atom = NewAtom();
        var projections = Enumerable.Range(0, ProjectionsWatching).Select(_ => new Projection<IReadOnlyList<NodeRef>>(atom)).ToList();
        var disagreements = 0;

        // Act
        for (var i = 0; i < DispatchesPerRun; i++)
        {
            atom.Dispatch(AnyIntent(rng, pool));
            disagreements += projections.Count(projection => !ReferenceEquals(projection.Value, atom.Value));
        }

        // Assert
        Assert.Equal(0, disagreements);
    }

    [Theory]
    [InlineData(8111)]
    public void The_final_selection_is_a_pure_fold_of_the_dispatched_intents(int seed)
    {
        // Arrange
        var rng = new Random(seed);
        var pool = Pool();
        var intents = Enumerable.Range(0, FoldedDispatchesPerRun).Select(_ => AnyIntent(rng, pool)).ToList();
        var atom = NewAtom();

        // Act
        foreach (var intent in intents)
        {
            atom.Dispatch(intent);
        }

        // Assert
        Assert.Equal(intents.Aggregate(Selection.Empty, (acc, intent) => intent.Apply(acc)), atom.Value, SequenceEqualityComparer<NodeRef>.Instance);
    }

    private static IIntent<IReadOnlyList<NodeRef>> AnyIntent(Random rng, List<NodeRef> pool)
    {
        var node = pool[rng.Next(pool.Count)];
        return rng.Next(3) switch
        {
            0 => new ToggleSelection(node),
            1 => new RemoveSelection(node),
            _ => new ClearSelection(),
        };
    }
}
