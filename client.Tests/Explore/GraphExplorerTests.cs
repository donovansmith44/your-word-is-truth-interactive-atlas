using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class GraphExplorerTests
{
    private const string Genesis1Id = "Container:bible-chapter-GEN-1";
    private const string Genesis2Id = "Container:bible-chapter-GEN-2";
    private const string ServedGenesis2Label = "Genesis 2";
    private const string LegacyGenesis2Label = "GEN.2";
    private const string SourceId = "Source:ussher";
    private const string SourceLabel = "Ussher";
    private const string AbsentId = "Event:nowhere";
    private const string CitedWords = "Thus saith the LORD, which maketh a way in the sea";
    private static readonly ArtifactRoot MovedRoot = Wire.Root("moved");

    private static readonly NodeRef ExodusEvent = ServedGraph.Ref(NodeKind.Event, "Event:red_sea", "The Red Sea parted");
    private static readonly NodeRef Exodus14 = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:EXO.14.21", "Exodus 14:21");
    private static readonly NodeRef Cited = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:ISA.43.16", "ISA.43.16");
    private static readonly EdgeRef AttestedIn = ServedGraph.EdgeRef(EdgeKind.AttestedIn, "Attests:00aa", "The Red Sea parted · Attested in · Exodus 14:21");
    private static readonly EdgeRef JustifiedEdge = ServedGraph.EdgeRef(EdgeKind.DatedBy, "DatedBy:00cc", "The Red Sea parted · Dated by · 1491 BC");
    private static readonly EdgeRef Justification = ServedGraph.EdgeRef(EdgeKind.JustifiedBy, "JustifiedBy:00dd", "The Red Sea parted · Dated by · 1491 BC · Justified by · Ussher");

    [Fact]
    public async Task Resolving_a_reference_fetches_its_card_and_yields_the_node_with_its_frontier()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Container, Genesis1Id, "Genesis 1", new FrontierGroup(EdgeKind.MemberOf, 1)));
        var explorer = new GraphExplorer(graph);

        // Act
        var genesis1 = await explorer.BeginAt(ServedGraph.At(NodeKind.Container, Genesis1Id, "Genesis 1"));

        // Assert
        Assert.Equal(
            (new ElementKind.Node(NodeKind.Container) as ElementKind, Wire.Element(Genesis1Id), "Genesis 1", new FrontierGroup(EdgeKind.MemberOf, 1)),
            (genesis1.Kind, genesis1.Id, genesis1.Label, genesis1.Groups.Single()));
    }

    [Fact]
    public async Task Resolving_a_reference_yields_the_node_carrying_the_served_label_not_the_references()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Container, Genesis2Id, ServedGenesis2Label));
        var genesis2Reference = ServedGraph.At(NodeKind.Container, Genesis2Id, LegacyGenesis2Label);

        // Act
        var genesis2 = await new GraphExplorer(graph).BeginAt(genesis2Reference);

        // Assert
        Assert.Equal((new ElementKind.Node(NodeKind.Container) as ElementKind, Wire.Element(Genesis2Id), ServedGenesis2Label), (genesis2.Kind, genesis2.Id, genesis2.Label));
    }

    [Fact]
    public async Task Resolving_an_edge_yields_it_with_its_two_ends_and_its_own_neighbour_groups()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.EdgeRecordOf(AttestedIn, ExodusEvent, Exodus14, new FrontierGroup(EdgeKind.JustifiedBy, 1)));
        var explorer = new GraphExplorer(graph);

        // Act
        var edge = await explorer.BeginAt(ServedGraph.AtEdge(AttestedIn));

        // Assert
        Assert.Equal(
            (new ElementKind.Edge(EdgeKind.AttestedIn) as ElementKind, AttestedIn.Id, AttestedIn.Label,
                string.Join(" ", new FrontierGroup(EdgeKind.JustifiedBy, 1)),
                string.Join(" ", new Link(EdgeKind.Attests, ServedGraph.At(ExodusEvent)), new Link(EdgeKind.AttestedIn, ServedGraph.At(Exodus14)))),
            (edge.Kind, edge.Id, edge.Label, string.Join(" ", edge.Groups), string.Join(" ", edge.Ends)));
    }

    [Fact]
    public async Task A_resolved_edge_is_identified_by_the_position_that_names_it_as_served()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.EdgeRecordOf(AttestedIn, ExodusEvent, Exodus14));
        var explorer = new GraphExplorer(graph);

        // Act
        var edge = await explorer.BeginAt(ServedGraph.AtEdge(ServedGraph.EdgeRef(EdgeKind.AttestedIn, AttestedIn.Id, LegacyGenesis2Label)));

        // Assert
        Assert.Equal(ServedGraph.AtEdge(AttestedIn), edge.Identity);
    }

    [Fact]
    public async Task A_node_has_no_ends()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Event, ExodusEvent.Id, ExodusEvent.Label));

        // Act
        var exodus = await new GraphExplorer(graph).BeginAt(ServedGraph.At(ExodusEvent));

        // Assert
        Assert.Empty(exodus.Ends);
    }

    [Fact]
    public async Task A_page_that_leads_to_edges_yields_entries_leading_to_those_edges()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Source, SourceId, SourceLabel, new FrontierGroup(EdgeKind.Justifies, 1)))
            .Serving(SourceId, EdgeKind.Justifies, null, ServedGraph.Page(EdgeKind.Justifies, null, (Justification, ServedGraph.AtEdge(JustifiedEdge))));
        var justifier = await new GraphExplorer(graph).BeginAt(ServedGraph.At(NodeKind.Source, SourceId, SourceLabel));

        // Act
        var page = await justifier.Entries(EdgeKind.Justifies);

        // Assert
        Assert.Equal(new Page<Entry>([new Entry(new Link(EdgeKind.Justifies, ServedGraph.AtEdge(JustifiedEdge)), new Link(EdgeKind.Justifies, ServedGraph.AtEdge(Justification)), null)], null, null), page);
    }

    [Fact]
    public async Task Every_entry_offers_a_step_onto_its_served_edge()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.TextUnit, Exodus14.Id, Exodus14.Label, new FrontierGroup(EdgeKind.Attests, 1)))
            .Serving(Exodus14.Id, EdgeKind.Attests, null, ServedGraph.Page(EdgeKind.Attests, null, (AttestedIn, ServedGraph.At(ExodusEvent))));
        var verse = await new GraphExplorer(graph).BeginAt(ServedGraph.At(Exodus14));

        // Act
        var page = await verse.Entries(EdgeKind.Attests);

        // Assert
        Assert.Equal(
            new Page<Entry>([new Entry(new Link(EdgeKind.Attests, ServedGraph.At(ExodusEvent)), new Link(EdgeKind.Attests, ServedGraph.AtEdge(AttestedIn)), null)], null, null),
            page);
    }

    [Fact]
    public async Task An_entry_leading_to_a_text_unit_carries_its_served_words()
    {
        // Arrange
        var words = ServedGraph.WordsOf(CitedWords);
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.TextUnit, Exodus14.Id, Exodus14.Label, new FrontierGroup(EdgeKind.Cites, 1)))
            .Serving(ServedGraph.TextCard(Cited, words))
            .Serving(Exodus14.Id, EdgeKind.Cites, null, ServedGraph.Page(EdgeKind.Cites, null, Cited));
        var verse = await new GraphExplorer(graph).BeginAt(ServedGraph.At(Exodus14));

        // Act
        var page = await verse.Entries(EdgeKind.Cites);

        // Assert
        Assert.Same(words, Assert.Single(page.Items).Words);
    }

    [Fact]
    public async Task A_text_unit_answered_without_its_words_is_a_contract_breach_naming_it()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.TextUnit, Exodus14.Id, Exodus14.Label, new FrontierGroup(EdgeKind.Cites, 1)))
            .Serving(Exodus14.Id, EdgeKind.Cites, null, ServedGraph.Page(EdgeKind.Cites, null, Cited))
            .Serving(ServedGraph.Card(NodeKind.TextUnit, Cited.Id, Cited.Label));
        var verse = await new GraphExplorer(graph).BeginAt(ServedGraph.At(Exodus14));

        // Act
        var breach = await Assert.ThrowsAsync<ContractBreach>(() => verse.Entries(EdgeKind.Cites));

        // Assert
        Assert.Equal($"the element read answered {Cited.Id}, a text unit an edge page served, without its words", breach.Message);
    }

    [Fact]
    public async Task Words_answered_from_another_artifact_are_refused_as_the_artifact_moving()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.TextUnit, Exodus14.Id, Exodus14.Label, new FrontierGroup(EdgeKind.Cites, 1)))
            .Serving(Exodus14.Id, EdgeKind.Cites, null, ServedGraph.Page(EdgeKind.Cites, null, Cited));
        var verse = await new GraphExplorer(graph).BeginAt(ServedGraph.At(Exodus14));
        graph.ElementsAt(MovedRoot);

        // Act
        var moved = await Assert.ThrowsAsync<ArtifactMoved>(() => verse.Entries(EdgeKind.Cites));

        // Assert
        Assert.Equal((new ArtifactMoved(Wire.Root(ServedGraph.Version), MovedRoot).Message, true), (moved.Message, verse.Moved));
    }

    [Fact]
    public async Task A_resolved_edge_pages_its_own_neighbours_from_the_neighbour_read_at_its_id()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.EdgeRecordOf(JustifiedEdge, ExodusEvent, Exodus14, new FrontierGroup(EdgeKind.JustifiedBy, 1)))
            .Serving(JustifiedEdge.Id, EdgeKind.JustifiedBy, null, ServedGraph.Page(EdgeKind.JustifiedBy, null, ServedGraph.Ref(NodeKind.Source, SourceId, SourceLabel)));
        var edge = await new GraphExplorer(graph).BeginAt(ServedGraph.AtEdge(JustifiedEdge));

        // Act
        var page = await edge.Entries(EdgeKind.JustifiedBy);

        // Assert
        Assert.Equal(new Page<Entry>([ServedGraph.EntryTo(EdgeKind.JustifiedBy, ServedGraph.At(NodeKind.Source, SourceId, SourceLabel))], null, null), page);
    }

    [Fact]
    public async Task A_walk_that_begins_at_a_missing_element_fails()
    {
        // Arrange
        var explorer = new GraphExplorer(new ServedGraph());

        // Act
        var begun = await Explore.Begin(explorer, ServedGraph.At(NodeKind.Event, AbsentId, AbsentId));

        // Assert
        Assert.Equal(new Outcome<Exploration>.Failed(), begun);
    }
}
