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
        [EdgeKind.FollowsIn] = new Affordance.Arrows(),
        [EdgeKind.PrecedesIn] = new Affordance.Arrows(),
        [EdgeKind.Contains] = new Affordance.InlineChildren(),
        [EdgeKind.Shows] = new Affordance.InlineChildren(),
        [EdgeKind.MemberOf] = new Affordance.UpCrumb(),
        [EdgeKind.ShownOn] = new Affordance.UpCrumb(),
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
