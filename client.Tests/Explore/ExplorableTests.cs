using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class ExplorableTests
{
    private const string Genesis1Id = "Container:bible-chapter-GEN-1";
    private const string Genesis1Label = "Genesis 1";
    private const string Genesis2Id = "Container:bible-chapter-GEN-2";
    private const string Genesis2Label = "Genesis 2";
    private const string LegacyGenesis1Label = "GEN.1";
    private const int VersesInGenesis1 = 31;
    private const int SecondPageCursor = 20;

    private static readonly FrontierGroup[] Genesis1Groups =
    [
        new(EdgeKind.Contains, VersesInGenesis1),
        new(EdgeKind.MemberOf, 1),
        new(EdgeKind.FollowsIn, 1),
    ];

    private static readonly Explorable Genesis1 = Resolved.Node(NodeKind.Container, Genesis1Id, Genesis1Label, Genesis1Groups);
    private static readonly NodeRef Genesis2Ref = ServedGraph.Ref(NodeKind.Container, Genesis2Id, Genesis2Label);
    private static readonly NodeRef LegacyGenesis2Ref = ServedGraph.Ref(NodeKind.Container, Genesis2Id, "GEN.2");
    private static readonly PositionRef ADating = ServedGraph.AtEdge(ServedGraph.EdgeRef(EdgeKind.DatedBy, "DatedBy:00ff", "A dating"));

    [Fact]
    public void A_resolved_node_carries_the_served_kind_id_and_label()
    {
        // Arrange
        var genesis1 = Genesis1;

        // Act
        var identity = (genesis1.Kind, genesis1.Id, genesis1.Label);

        // Assert
        Assert.Equal((new ElementKind.Node(NodeKind.Container) as ElementKind, Genesis1Id, Genesis1Label), identity);
    }

    [Fact]
    public void A_resolved_node_s_identity_is_the_reference_that_names_it_as_served()
    {
        // Arrange
        var genesis1 = Genesis1;

        // Act
        var identity = genesis1.Identity;

        // Assert
        Assert.Equal(ServedGraph.At(NodeKind.Container, Genesis1Id, Genesis1Label), identity);
    }

    [Fact]
    public void The_frontier_groups_are_the_cards_edge_summary_in_declaration_order()
    {
        // Arrange
        var genesis1 = Genesis1;

        // Act
        var groups = genesis1.Groups;

        // Assert
        Assert.Equal(Genesis1Groups, groups);
    }

    [Fact]
    public async Task A_frontier_group_pages_to_entries_in_server_order()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Container, Genesis1Id, Genesis1Label, Genesis1Groups))
            .Serving(Genesis1Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, SecondPageCursor, new NodePosition(Genesis2Ref)));
        var genesis1 = Resolved.Node(graph, ServedGraph.Ref(NodeKind.Container, Genesis1Id, Genesis1Label));

        // Act
        var page = await genesis1.Entries(EdgeKind.FollowsIn);

        // Assert
        Assert.Equal(new Page<Entry>([ServedGraph.EntryTo(EdgeKind.FollowsIn, ServedGraph.At(Genesis2Ref))], null, SecondPageCursor), page);
    }

    [Fact]
    public async Task An_edge_position_on_a_frontier_page_is_an_entry_leading_to_that_edge()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Container, Genesis1Id, Genesis1Label, Genesis1Groups))
            .Serving(Genesis1Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, ADating, ServedGraph.At(Genesis2Ref)));
        var genesis1 = Resolved.Node(graph, ServedGraph.Ref(NodeKind.Container, Genesis1Id, Genesis1Label));

        // Act
        var page = await genesis1.Entries(EdgeKind.FollowsIn);

        // Assert
        Assert.Equal(new Page<Entry>([ServedGraph.EntryTo(EdgeKind.FollowsIn, ADating), ServedGraph.EntryTo(EdgeKind.FollowsIn, ServedGraph.At(Genesis2Ref))], null, null), page);
    }

    [Fact]
    public async Task Entries_of_a_kind_the_frontier_does_not_declare_are_an_empty_page_without_asking_the_graph()
    {
        // Arrange
        var genesis1 = Genesis1;

        // Act
        var page = await genesis1.Entries(EdgeKind.Mentions);

        // Assert
        Assert.Equal(new Page<Entry>([], null, null), page);
    }

    [Fact]
    public async Task Entries_are_paged_by_the_clients_one_default_page_size()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Container, Genesis1Id, Genesis1Label, Genesis1Groups))
            .Serving(Genesis1Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, new NodePosition(Genesis2Ref)));
        var genesis1 = Resolved.Node(graph, ServedGraph.Ref(NodeKind.Container, Genesis1Id, Genesis1Label));

        // Act
        await genesis1.Entries(EdgeKind.FollowsIn);

        // Assert
        Assert.Equal(IExplorableClient.DefaultPageSize, graph.LimitAsked);
    }

    [Fact]
    public void Two_explorables_with_the_same_kind_and_id_are_equal_whatever_their_labels()
    {
        // Arrange
        var legacyGenesis1 = Resolved.Node(NodeKind.Container, Genesis1Id, LegacyGenesis1Label);

        // Act
        var (equal, sameHash) = (Genesis1 == legacyGenesis1, Genesis1.GetHashCode() == legacyGenesis1.GetHashCode());

        // Assert
        Assert.Equal((true, true), (equal, sameHash));
    }

    [Fact]
    public void Two_explorables_with_different_ids_are_not_equal()
    {
        // Arrange
        var genesis2 = Resolved.Node(NodeKind.Container, Genesis2Id, Genesis2Label);

        // Act
        var equal = Genesis1 == genesis2;

        // Assert
        Assert.False(equal);
    }

    [Fact]
    public void Two_explorables_with_different_kinds_are_not_equal()
    {
        // Arrange
        var asTextUnit = Resolved.Node(NodeKind.TextUnit, Genesis1Id, Genesis1Label);

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
        var (equalsNothing, isNothing, differsFromNothing) = (Genesis1.Equals(nothing), Genesis1 == nothing, Genesis1 != nothing);

        // Assert
        Assert.Equal((false, false, true), (equalsNothing, isNothing, differsFromNothing));
    }

    [Fact]
    public void Two_links_to_the_same_node_under_the_same_kind_are_equal_whatever_the_labels()
    {
        // Arrange
        var (a, b) = (new Link(EdgeKind.FollowsIn, ServedGraph.At(Genesis2Ref)), new Link(EdgeKind.FollowsIn, ServedGraph.At(LegacyGenesis2Ref)));

        // Act
        var (equal, sameHash) = (a == b, a.GetHashCode() == b.GetHashCode());

        // Assert
        Assert.Equal((true, true), (equal, sameHash));
    }

    [Fact]
    public void Two_links_to_the_same_node_under_different_kinds_are_not_equal()
    {
        // Arrange
        var (a, b) = (new Link(EdgeKind.FollowsIn, ServedGraph.At(Genesis2Ref)), new Link(EdgeKind.Contains, ServedGraph.At(Genesis2Ref)));

        // Act
        var equal = a == b;

        // Assert
        Assert.False(equal);
    }

    [Fact]
    public void A_link_to_a_node_and_a_link_to_an_edge_sharing_its_id_are_not_equal()
    {
        // Arrange
        var (toNode, toEdge) = (new Link(EdgeKind.FollowsIn, ServedGraph.At(Genesis2Ref)), new Link(EdgeKind.FollowsIn, ServedGraph.AtEdge(ServedGraph.EdgeRef(EdgeKind.FollowsIn, Genesis2Id, Genesis2Label))));

        // Act
        var equal = toNode == toEdge;

        // Assert
        Assert.False(equal);
    }

    [Fact]
    public void Two_pages_with_the_same_links_and_next_cursor_are_equal_whatever_lists_hold_them()
    {
        // Arrange
        var toGenesis2 = new Link(EdgeKind.FollowsIn, ServedGraph.At(Genesis2Ref));

        // Act
        var (a, b) = (new Page<Link>([toGenesis2], null, SecondPageCursor), new Page<Link>(new List<Link> { toGenesis2 }, null, SecondPageCursor));

        // Assert
        Assert.Equal((true, true), (a == b, a.GetHashCode() == b.GetHashCode()));
    }

    [Fact]
    public void Two_pages_with_different_links_are_not_equal()
    {
        // Arrange
        var toGenesis2 = new Link(EdgeKind.FollowsIn, ServedGraph.At(Genesis2Ref));

        // Act
        var (a, b) = (new Page<Link>([toGenesis2], null, null), new Page<Link>([], null, null));

        // Assert
        Assert.NotEqual(a, b);
    }
}
