using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class AffordancesTests
{
    private const int ListPage = 20;
    private const int ArrowsShown = 1;
    private const int CrumbsShown = 3;

    private static readonly IReadOnlyDictionary<EdgeKind, Affordance> NamedRows = new Dictionary<EdgeKind, Affordance>
    {
        [EdgeKind.FollowsIn] = new Affordance.Arrows(ArrowDirection.Next),
        [EdgeKind.PrecedesIn] = new Affordance.Arrows(ArrowDirection.Previous),
        [EdgeKind.Contains] = new Affordance.InlineChildren(),
        [EdgeKind.Shows] = new Affordance.InlineChildren(),
        [EdgeKind.MemberOf] = new Affordance.UpCrumb(),
        [EdgeKind.ShownOn] = new Affordance.UpCrumb(),
        [EdgeKind.Cites] = new Affordance.SectionList(SectionStyle.Quiet, SectionOrder.VotesRanked),
    };

    private static readonly Affordance EveryOtherRow = new Affordance.SectionList(SectionStyle.Standard, SectionOrder.Canonical);

    [Fact]
    public void Every_list_shows_one_page_of_twenty_and_arrows_and_crumbs_keep_their_own_counts()
    {
        // Arrange
        var kinds = Enum.GetValues<EdgeKind>();
        // Act
        var shown = kinds.ToDictionary(k => k, k => Affordances.Of(k).InitialClamp);
        // Assert
        Assert.Equal(
            kinds.ToDictionary(k => k, k => Affordances.Of(k) is Affordance.Arrows ? ArrowsShown : Affordances.Of(k) is Affordance.UpCrumb ? CrumbsShown : ListPage),
            shown);
    }

    [Fact]
    public void Affordances_are_total_over_every_generated_edge_kind()
    {
        // Arrange
        var kinds = Enum.GetValues<EdgeKind>();
        // Act
        var failure = Record.Exception(() => kinds.Select(Affordances.Of).ToList());
        // Assert
        Assert.Null(failure);
    }

    [Fact]
    public void The_named_kinds_get_their_named_affordances_and_every_other_kind_is_a_listed_group()
    {
        // Arrange
        var kinds = Enum.GetValues<EdgeKind>();
        var expected = kinds.ToDictionary(k => k, k => NamedRows.GetValueOrDefault(k, EveryOtherRow));
        // Act
        var table = kinds.ToDictionary(k => k, Affordances.Of);
        // Assert
        Assert.Equal(expected, table);
    }

    [Fact]
    public void Following_is_the_next_arrow_and_preceding_the_previous()
    {
        // Arrange
        var succession = new[] { EdgeKind.FollowsIn, EdgeKind.PrecedesIn };
        // Act
        var arrows = succession.Select(Affordances.Of).ToArray();
        // Assert
        Assert.Equal(new Affordance[] { new Affordance.Arrows(ArrowDirection.Next), new Affordance.Arrows(ArrowDirection.Previous) }, arrows);
    }

    [Fact]
    public void The_listed_rows_a_section_reads_by_name_are_the_rows_the_table_serves()
    {
        // Arrange
        var expected = new Dictionary<EdgeKind, Affordance.SectionList>
        {
            [EdgeKind.Cites] = Affordances.Cites,
            [EdgeKind.Mentions] = Affordances.DefaultList,
            [EdgeKind.MentionedIn] = Affordances.DefaultList,
        };
        // Act
        var served = expected.Keys.ToDictionary(k => k, Affordances.Of);
        // Assert
        Assert.Equal(expected.ToDictionary(e => e.Key, e => (Affordance)e.Value), served);
    }
}
