using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class ExplorationTests
{
    private const string Genesis1Id = "Container:bible-chapter-GEN-1";
    private const string Genesis2Id = "Container:bible-chapter-GEN-2";
    private const string Genesis3Id = "Container:bible-chapter-GEN-3";
    private const string GenesisId = "Container:bible-book-GEN";

    private static readonly Explorable Genesis1 = new(NodeKind.Container, Genesis1Id, "Genesis 1");
    private static readonly Explorable Genesis2 = new(NodeKind.Container, Genesis2Id, "Genesis 2");
    private static readonly Explorable Genesis3 = new(NodeKind.Container, Genesis3Id, "Genesis 3");
    private static readonly Explorable Genesis = new(NodeKind.Container, GenesisId, "Genesis");

    private static readonly Link ToGenesis2 = new(EdgeKind.FollowsIn, Genesis2);
    private static readonly Link ToGenesis3 = new(EdgeKind.FollowsIn, Genesis3);
    private static readonly Link BackToGenesis1 = new(EdgeKind.PrecedesIn, Genesis1);
    private static readonly Link BackToGenesis2 = new(EdgeKind.PrecedesIn, Genesis2);
    private static readonly Link UpToGenesis = new(EdgeKind.MemberOf, Genesis);

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
    public void Following_a_link_appends_it_and_moves_the_current_node()
    {
        // Arrange
        var exploration = new Exploration(Genesis1, []);

        // Act
        var next = exploration.Follow(ToGenesis2);

        // Assert
        Assert.Equal((new Exploration(Genesis1, [ToGenesis2]), Genesis2), (next, next.Current));
    }

    [Fact]
    public void Going_back_follows_the_dual_of_the_last_link_to_the_node_it_left()
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
    public void The_breadcrumb_collapses_a_link_followed_by_its_dual()
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
        var downToGenesis1 = new Link(EdgeKind.Contains, Genesis1);
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
        var upToGenesis1 = new Link(EdgeKind.MemberOf, Genesis1);
        var exploration = new Exploration(Genesis1, [ToGenesis2, upToGenesis1]);

        // Act
        var breadcrumb = exploration.Breadcrumb;

        // Assert
        Assert.Equal([ToGenesis2, upToGenesis1], breadcrumb);
    }

    [Fact]
    public void Two_explorations_with_the_same_start_and_links_are_equal_whatever_lists_hold_them()
    {
        // Arrange
        var (a, b) = (new Exploration(Genesis1, [ToGenesis2]), new Exploration(Genesis1, new List<Link> { ToGenesis2 }));

        // Act
        var (equal, sameHash) = (a == b, a.GetHashCode() == b.GetHashCode());

        // Assert
        Assert.Equal((true, true), (equal, sameHash));
    }

    [Fact]
    public void Two_explorations_with_different_links_are_not_equal()
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
}
