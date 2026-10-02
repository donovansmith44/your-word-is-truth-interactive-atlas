using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class ExplorationTests
{
    private const string Genesis1Id = "Container:bible-chapter-GEN-1";
    private const string Genesis2Id = "Container:bible-chapter-GEN-2";
    private const string Genesis3Id = "Container:bible-chapter-GEN-3";
    private const string GenesisId = "Container:bible-book-GEN";

    private static readonly Explorable Genesis1 = Resolved.Node(NodeKind.Container, Genesis1Id, "Genesis 1");
    private static readonly Explorable Genesis2 = Resolved.Node(NodeKind.Container, Genesis2Id, "Genesis 2");
    private static readonly Explorable Genesis3 = Resolved.Node(NodeKind.Container, Genesis3Id, "Genesis 3");
    private static readonly Explorable Genesis = Resolved.Node(NodeKind.Container, GenesisId, "Genesis");

    private static readonly Step ToGenesis2 = new(EdgeKind.FollowsIn, Genesis2);
    private static readonly Step ToGenesis3 = new(EdgeKind.FollowsIn, Genesis3);
    private static readonly Step BackToGenesis1 = new(EdgeKind.PrecedesIn, Genesis1);
    private static readonly Step BackToGenesis2 = new(EdgeKind.PrecedesIn, Genesis2);
    private static readonly Step UpToGenesis = new(EdgeKind.MemberOf, Genesis);

    [Fact]
    public void An_exploration_that_has_gone_nowhere_is_at_its_start()
    {
        // Arrange
        var exploration = new Exploration(Genesis1, []);

        // Act
        var current = exploration.Current;

        // Assert
        Assert.Equal(Genesis1, current);
    }

    [Fact]
    public void Following_a_step_appends_it_and_moves_the_current_node()
    {
        // Arrange
        var exploration = new Exploration(Genesis1, []);

        // Act
        var next = exploration.Follow(ToGenesis2);

        // Assert
        Assert.Equal((new Exploration(Genesis1, [ToGenesis2]), Genesis2), (next, next.Current));
    }

    [Fact]
    public void Going_back_follows_the_dual_of_the_last_step_to_the_node_it_left()
    {
        // Arrange
        var exploration = new Exploration(Genesis1, [ToGenesis2]);

        // Act
        var back = exploration.Back();

        // Assert
        Assert.Equal((new Exploration(Genesis1, [ToGenesis2, BackToGenesis1]), Genesis1), (back, back.Current));
    }

    [Fact]
    public void Going_back_twice_retraces_two_hops()
    {
        // Arrange
        var exploration = new Exploration(Genesis1, [ToGenesis2, ToGenesis3]);

        // Act
        var back = exploration.Back().Back();

        // Assert
        Assert.Equal(new Exploration(Genesis1, [ToGenesis2, ToGenesis3, BackToGenesis2, BackToGenesis1]), back);
    }

    [Fact]
    public void Going_back_from_the_start_changes_nothing()
    {
        // Arrange
        var exploration = new Exploration(Genesis1, []);

        // Act
        var back = exploration.Back();

        // Assert
        Assert.Equal(exploration, back);
    }

    [Fact]
    public void Going_back_once_every_hop_has_been_retraced_changes_nothing()
    {
        // Arrange
        var exploration = new Exploration(Genesis1, [ToGenesis2, BackToGenesis1]);

        // Act
        var back = exploration.Back();

        // Assert
        Assert.Equal(exploration, back);
    }

    [Fact]
    public void The_breadcrumb_collapses_a_step_followed_by_its_dual()
    {
        // Arrange
        var exploration = new Exploration(Genesis1, [ToGenesis2, BackToGenesis1, UpToGenesis]);

        // Act
        var breadcrumb = exploration.Breadcrumb;

        // Assert
        Assert.Equal([UpToGenesis], breadcrumb);
    }

    [Fact]
    public void The_breadcrumb_keeps_a_revisit_that_is_not_an_immediate_return()
    {
        // Arrange
        var downToGenesis1 = new Step(EdgeKind.Contains, Genesis1);
        var exploration = new Exploration(Genesis1, [ToGenesis2, UpToGenesis, downToGenesis1]);

        // Act
        var breadcrumb = exploration.Breadcrumb;

        // Assert
        Assert.Equal([ToGenesis2, UpToGenesis, downToGenesis1], breadcrumb);
    }

    [Fact]
    public void The_breadcrumb_keeps_a_return_to_the_right_node_under_the_wrong_kind()
    {
        // Arrange
        var upToGenesis1 = new Step(EdgeKind.MemberOf, Genesis1);
        var exploration = new Exploration(Genesis1, [ToGenesis2, upToGenesis1]);

        // Act
        var breadcrumb = exploration.Breadcrumb;

        // Assert
        Assert.Equal([ToGenesis2, upToGenesis1], breadcrumb);
    }

    [Fact]
    public void Stepping_onto_an_edge_and_back_out_of_the_end_it_was_entered_from_retraces()
    {
        // Arrange
        var kinds = Enum.GetValues<EdgeKind>();

        // Act
        var unretraced = kinds
            .SelectMany(kind => new[] { (kind, FromSubject: true), (kind, FromSubject: false) })
            .Where(entered => EnteredAndLeftByItsEnd(entered.kind, entered.FromSubject).Breadcrumb.Count != 0)
            .Select(entered => $"{entered.kind} entered from its {(entered.FromSubject ? "subject" : "object")}")
            .ToList();

        // Assert
        Assert.Empty(unretraced);
    }

    [Fact]
    public void Two_explorations_with_the_same_start_and_steps_are_equal_whatever_lists_hold_them()
    {
        // Arrange
        var (a, b) = (new Exploration(Genesis1, [ToGenesis2]), new Exploration(Genesis1, new List<Step> { ToGenesis2 }));

        // Act
        var (equal, sameHash) = (a == b, a.GetHashCode() == b.GetHashCode());

        // Assert
        Assert.Equal((true, true), (equal, sameHash));
    }

    [Fact]
    public void Two_explorations_with_different_steps_are_not_equal()
    {
        // Arrange
        var (a, b) = (new Exploration(Genesis1, [ToGenesis2]), new Exploration(Genesis1, []));

        // Act
        var equal = a == b;

        // Assert
        Assert.False(equal);
    }

    [Fact]
    public void Two_explorations_with_different_starts_are_not_equal()
    {
        // Arrange
        var (a, b) = (new Exploration(Genesis1, []), new Exploration(Genesis2, []));

        // Act
        var equal = a == b;

        // Assert
        Assert.False(equal);
    }

    [Fact]
    public void An_exploration_is_never_equal_to_nothing()
    {
        // Arrange
        Exploration? nothing = null;

        // Act
        var equal = new Exploration(Genesis1, []).Equals(nothing);

        // Assert
        Assert.False(equal);
    }

    private static Exploration EnteredAndLeftByItsEnd(EdgeKind kind, bool fromSubject)
    {
        var subject = ServedGraph.Ref(NodeKind.Event, "Event:subject", "Subject");
        var @object = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:GEN.1.1", "Object");
        var edgeRef = ServedGraph.EdgeRef(kind, $"{kind}:00aa", kind.ToString());
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(subject.Kind, subject.Id, subject.Label))
            .Serving(ServedGraph.Card(@object.Kind, @object.Id, @object.Label))
            .Serving(ServedGraph.EdgeRecordOf(edgeRef, subject, @object));
        var start = Resolved.Node(graph, fromSubject ? subject : @object);
        var edge = Resolved.At(graph, ServedGraph.AtEdge(edgeRef));
        var enteredBy = fromSubject ? kind : kind.Dual();
        var end = edge.Ends.Single(link => link.Target == start.Identity);
        return new Exploration(start, [new Step(enteredBy, edge), new Step(end.Kind, Resolved.At(graph, end.Target))]);
    }
}
