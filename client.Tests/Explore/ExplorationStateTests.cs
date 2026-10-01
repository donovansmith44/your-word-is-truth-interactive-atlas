using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests;

public sealed class ExplorationStateTests
{
    private const string Genesis1Id = "Container:bible-chapter-GEN-1";
    private const string Genesis2Id = "Container:bible-chapter-GEN-2";
    private const string PopoverOrigin = "popover";

    private static readonly Explorable Genesis1 = Resolved.Node(NodeKind.Container, Genesis1Id, "Genesis 1");
    private static readonly Explorable Genesis2 = Resolved.Node(NodeKind.Container, Genesis2Id, "Genesis 2");
    private static readonly Step ToGenesis2 = new(EdgeKind.FollowsIn, Genesis2);
    private static readonly Step BackToGenesis1 = new(EdgeKind.PrecedesIn, Genesis1);

    private static readonly ExplorationState Closed = new ExplorationState.Closed();
    private static readonly ExplorationState AtGenesis1 = new ExplorationState.Open(new Exploration(Genesis1, []));
    private static readonly ExplorationState AtGenesis2ViaGenesis1 = new ExplorationState.Open(new Exploration(Genesis1, [ToGenesis2]));

    [Fact]
    public void Open_on_a_closed_state_starts_an_exploration_at_that_node()
    {
        // Arrange
        var open = new ExplorationIntent.Open(Genesis1);

        // Act
        var opened = open.Apply(Closed);

        // Assert
        Assert.Equal(AtGenesis1, opened);
    }

    [Fact]
    public void Open_on_the_node_already_current_changes_nothing()
    {
        // Arrange
        var open = new ExplorationIntent.Open(Genesis2);

        // Act
        var again = open.Apply(AtGenesis2ViaGenesis1);

        // Assert
        Assert.Equal(AtGenesis2ViaGenesis1, again);
    }

    [Fact]
    public void Open_on_a_different_node_starts_afresh()
    {
        // Arrange
        var open = new ExplorationIntent.Open(Genesis1);

        // Act
        var fresh = open.Apply(AtGenesis2ViaGenesis1);

        // Assert
        Assert.Equal(AtGenesis1, fresh);
    }

    [Fact]
    public void Reset_closes_and_Reseed_replaces_verbatim()
    {
        // Arrange
        var snapshot = new Exploration(Genesis2, [BackToGenesis1]);
        var (reset, reseed) = (new ExplorationIntent.Reset(), new ExplorationIntent.Reseed(snapshot));

        // Act
        var results = (reset.Apply(AtGenesis2ViaGenesis1), reseed.Apply(AtGenesis2ViaGenesis1));

        // Assert
        Assert.Equal((Closed, new ExplorationState.Open(snapshot)), results);
    }

    [Fact]
    public void Every_intent_is_named_for_its_verb()
    {
        // Arrange
        IIntent<ExplorationState>[] intents =
        [
            new ExplorationIntent.Open(Genesis1),
            new ExplorationIntent.Arrive(new Exploration(Genesis1, []), new Exploration(Genesis1, [ToGenesis2])),
            new ExplorationIntent.Reset(),
            new ExplorationIntent.Reseed(new Exploration(Genesis1, [])),
        ];

        // Act
        var names = intents.Select(i => i.Name).ToList();

        // Assert
        Assert.Equal(["exploration-open", "exploration-arrive", "exploration-reset", "exploration-reseed"], names);
    }

    [Fact]
    public void The_intent_vocabulary_is_open_arrive_reset_reseed_and_nothing_else()
    {
        // Arrange
        var intent = typeof(ExplorationIntent);

        // Act
        var cases = intent.Assembly.GetTypes()
            .Where(t => t.IsSubclassOf(intent))
            .Select(t => (t.Name, Sealed: t.IsSealed, Nested: t.DeclaringType == intent))
            .OrderBy(c => c.Name)
            .ToList();

        // Assert
        Assert.Equal(
            [("Arrive", true, true), ("Open", true, true), ("Reseed", true, true), ("Reset", true, true)],
            cases);
    }

    [Fact]
    public void Dispatching_an_intent_on_the_atom_moves_its_value_and_records_the_origin()
    {
        // Arrange
        var atom = new StateAtom<ExplorationState>(AtomNames.Exploration, Closed);

        // Act
        atom.Dispatch(new ExplorationIntent.Open(Genesis1, PopoverOrigin));

        // Assert
        Assert.Equal((AtGenesis1, PopoverOrigin, AtomNames.Exploration), (atom.Value, atom.LastOrigin, atom.Name));
    }
}
