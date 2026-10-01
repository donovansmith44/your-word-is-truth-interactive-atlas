using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class AffordancesTests
{
    private const int CitesShownBeforeReveal = 3;
    private const int MentionsShownBeforeReveal = 50;
    private const int MentionedInShownBeforeReveal = 12;
    private const int ListedLinksShownBeforeReveal = 20;

    private static readonly IReadOnlyDictionary<EdgeKind, Affordance> NamedRows = new Dictionary<EdgeKind, Affordance>
    {
        [EdgeKind.FollowsIn] = new Affordance.Arrows(ArrowDirection.Next),
        [EdgeKind.PrecedesIn] = new Affordance.Arrows(ArrowDirection.Previous),
        [EdgeKind.Contains] = new Affordance.InlineChildren(),
        [EdgeKind.Shows] = new Affordance.InlineChildren(),
        [EdgeKind.MemberOf] = new Affordance.UpCrumb(),
        [EdgeKind.ShownOn] = new Affordance.UpCrumb(),
        [EdgeKind.From] = new Affordance.UpCrumb(),
        [EdgeKind.To] = new Affordance.UpCrumb(),
        [EdgeKind.SourceOf] = new Affordance.EntryEnd(),
        [EdgeKind.TargetOf] = new Affordance.EntryEnd(),
        [EdgeKind.Cites] = new Affordance.SectionList(SectionStyle.Quiet, CitesShownBeforeReveal, SectionOrder.VotesRanked),
        [EdgeKind.Mentions] = new Affordance.SectionList(SectionStyle.Standard, MentionsShownBeforeReveal, SectionOrder.Canonical),
        [EdgeKind.MentionedIn] = new Affordance.SectionList(SectionStyle.Standard, MentionedInShownBeforeReveal, SectionOrder.Canonical),
        [EdgeKind.CommentedOnBy] = new Affordance.SectionList(SectionStyle.Standard, ListedLinksShownBeforeReveal, SectionOrder.Canonical),
    };

    private static readonly Affordance EveryOtherRow = new Affordance.SectionList(SectionStyle.Standard, ListedLinksShownBeforeReveal, SectionOrder.Canonical);

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
    public void An_edge_shows_its_two_ends_as_crumbs()
    {
        // Arrange
        var ends = new[] { EdgeKind.From, EdgeKind.To };
        // Act
        var affordances = ends.Select(Affordances.Of).ToArray();
        // Assert
        Assert.Equal(new Affordance[] { new Affordance.UpCrumb(), new Affordance.UpCrumb() }, affordances);
    }

    [Fact]
    public void Stepping_onto_an_edge_is_an_entry_control_not_a_section()
    {
        // Arrange
        var steps = new[] { EdgeKind.SourceOf, EdgeKind.TargetOf };
        // Act
        var affordances = steps.Select(Affordances.Of).ToArray();
        // Assert
        Assert.Equal(new Affordance[] { new Affordance.EntryEnd(), new Affordance.EntryEnd() }, affordances);
    }

    [Fact]
    public void The_listed_rows_a_section_reads_by_name_are_the_rows_the_table_serves()
    {
        // Arrange
        var expected = new Dictionary<EdgeKind, Affordance.SectionList>
        {
            [EdgeKind.Cites] = Affordances.Cites,
            [EdgeKind.Mentions] = Affordances.Mentions,
            [EdgeKind.MentionedIn] = Affordances.MentionedIn,
        };
        // Act
        var served = expected.Keys.ToDictionary(k => k, Affordances.Of);
        // Assert
        Assert.Equal(expected.ToDictionary(e => e.Key, e => (Affordance)e.Value), served);
    }
}
