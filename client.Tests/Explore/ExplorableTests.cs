using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class ExplorableTests
{
    private const string Genesis1Id = "Container:bible-chapter-GEN-1";
    private const string Genesis1Label = "Genesis 1";
    private const string Genesis2Id = "Container:bible-chapter-GEN-2";
    private const string LegacyGenesis1Label = "GEN.1";

    private static readonly Explorable Genesis1 = new(NodeKind.Container, Genesis1Id, Genesis1Label);

    [Fact]
    public void An_explorable_made_from_a_node_card_carries_its_kind_id_and_label()
    {
        // Arrange
        var card = new NodeCard(
            book: null, catechism: null, description: null, edgeSummary: [], @event: null,
            id: Genesis1Id, kind: NodeKind.Container, label: Genesis1Label,
            person: null, place: null, provenance: "kjv", version: "v");

        // Act
        var explorable = Explorable.From(card);

        // Assert
        Assert.Equal((NodeKind.Container, Genesis1Id, Genesis1Label), (explorable.Kind, explorable.Id, explorable.Label));
    }

    [Fact]
    public void Two_explorables_with_the_same_kind_and_id_are_equal_whatever_their_labels()
    {
        // Arrange
        var legacyGenesis1 = new Explorable(NodeKind.Container, Genesis1Id, LegacyGenesis1Label);

        // Act
        var (equal, sameHash) = (Genesis1 == legacyGenesis1, Genesis1.GetHashCode() == legacyGenesis1.GetHashCode());

        // Assert
        Assert.Equal((true, true), (equal, sameHash));
    }

    [Fact]
    public void Two_explorables_with_different_ids_are_not_equal()
    {
        // Arrange
        var genesis2 = new Explorable(NodeKind.Container, Genesis2Id, Genesis1Label);

        // Act
        var equal = Genesis1 == genesis2;

        // Assert
        Assert.False(equal);
    }

    [Fact]
    public void Two_explorables_with_different_kinds_are_not_equal()
    {
        // Arrange
        var asTextUnit = new Explorable(NodeKind.TextUnit, Genesis1Id, Genesis1Label);

        // Act
        var equal = Genesis1 == asTextUnit;

        // Assert
        Assert.False(equal);
    }

    [Fact]
    public void An_explorable_is_never_equal_to_nothing()
    {
        // Arrange
        Explorable? nothing = null;

        // Act
        var equal = Genesis1.Equals(nothing);

        // Assert
        Assert.False(equal);
    }

    [Fact]
    public void Two_links_to_the_same_node_under_the_same_kind_are_equal_whatever_the_labels()
    {
        // Arrange
        var genesis2 = new Explorable(NodeKind.Container, Genesis2Id, "Genesis 2");
        var legacyGenesis2 = new Explorable(NodeKind.Container, Genesis2Id, "GEN.2");

        // Act
        var (a, b) = (new Link(EdgeKind.FollowsIn, genesis2), new Link(EdgeKind.FollowsIn, legacyGenesis2));

        // Assert
        Assert.Equal(a, b);
    }

    [Fact]
    public void Two_pages_with_the_same_links_and_next_cursor_are_equal()
    {
        // Arrange
        var toGenesis2 = new Link(EdgeKind.FollowsIn, new Explorable(NodeKind.Container, Genesis2Id, "Genesis 2"));
        const int nextCursor = 20;

        // Act
        var (a, b) = (new Page<Link>([toGenesis2], nextCursor), new Page<Link>([toGenesis2], nextCursor));

        // Assert
        Assert.Equal((true, true), (a == b, a.GetHashCode() == b.GetHashCode()));
    }

    [Fact]
    public void Two_pages_with_different_links_are_not_equal()
    {
        // Arrange
        var toGenesis2 = new Link(EdgeKind.FollowsIn, new Explorable(NodeKind.Container, Genesis2Id, "Genesis 2"));

        // Act
        var (a, b) = (new Page<Link>([toGenesis2], null), new Page<Link>([], null));

        // Assert
        Assert.NotEqual(a, b);
    }

    [Fact]
    public async Task A_frontier_pages_its_links_by_the_clients_one_default_page_size_when_no_limit_is_given()
    {
        // Arrange
        var frontier = new RecordingFrontier(Genesis1, [new FrontierGroup(EdgeKind.Contains, 31)]);

        // Act
        await frontier.Links(EdgeKind.Contains);

        // Assert
        Assert.Equal(IExplorableClient.DefaultPageSize, frontier.LimitAsked);
    }

    private sealed class RecordingFrontier(Explorable node, IReadOnlyList<FrontierGroup> groups) : IFrontier
    {
        public int? LimitAsked { get; private set; }

        public Explorable Node => node;

        public IReadOnlyList<FrontierGroup> Groups => groups;

        public Task<Page<Link>> Links(EdgeKind kind, int? cursor = null, int limit = IExplorableClient.DefaultPageSize)
        {
            LimitAsked = limit;
            return Task.FromResult(new Page<Link>([], null));
        }
    }
}
