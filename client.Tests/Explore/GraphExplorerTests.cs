using System.Reflection;
using System.Text.RegularExpressions;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public sealed class GraphExplorerTests
{
    private const string Genesis1Id = "Container:bible-chapter-GEN-1";
    private const string Genesis2Id = "Container:bible-chapter-GEN-2";
    private const string ServedGenesis2Label = "Genesis 2";
    private const string LegacyGenesis2Label = "GEN.2";
    private const string MosesId = "Person:moses_2108";
    private const string MosesLabel = "Moses";
    private const string Genesis1Label = "Genesis 1";
    private const string TheOneConstructingFile = "Explorer.cs";
    private const string SourceId = "Source:ussher";
    private const string SourceLabel = "Ussher";
    private const string AbsentId = "Event:nowhere";

    private static readonly NodeRef ExodusEvent = ServedGraph.Ref(NodeKind.Event, "Event:red_sea", "The Red Sea parted");
    private static readonly NodeRef Exodus14 = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:EXO.14.21", "Exodus 14:21");
    private static readonly EdgeRef AttestedIn = ServedGraph.EdgeRef(EdgeKind.AttestedIn, "Attests:00aa", "The Red Sea parted · Attested in · Exodus 14:21");
    private static readonly EdgeRef JustifiedEdge = ServedGraph.EdgeRef(EdgeKind.DatedBy, "DatedBy:00cc", "The Red Sea parted · Dated by · 1491 BC");
    private static readonly EdgeRef Justification = ServedGraph.EdgeRef(EdgeKind.JustifiedBy, "JustifiedBy:00dd", "The Red Sea parted · Dated by · 1491 BC · Justified by · Ussher");

    private static readonly Regex BuildsAnExplorable = new(@"\bnew Explorable\(|\bExplorable\??\s+\w+\s*=>?\s*new\(", RegexOptions.Compiled);

    [Fact]
    public async Task Resolving_a_reference_fetches_its_card_and_yields_the_node_with_its_frontier()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Container, Genesis1Id, "Genesis 1", new FrontierGroup(EdgeKind.MemberOf, 1)));
        var explorer = new GraphExplorer(graph);

        // Act
        var genesis1 = await explorer.Resolve(ServedGraph.At(NodeKind.Container, Genesis1Id, "Genesis 1"));

        // Assert
        Assert.Equal(
            (new ElementKind.Node(NodeKind.Container) as ElementKind, Genesis1Id, "Genesis 1", new FrontierGroup(EdgeKind.MemberOf, 1)),
            (genesis1.Kind, genesis1.Id, genesis1.Label, genesis1.Groups.Single()));
    }

    [Fact]
    public async Task Following_a_link_resolves_its_target_so_the_node_carries_the_served_label_not_the_links()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Container, Genesis2Id, ServedGenesis2Label));
        var toGenesis2 = new Link(EdgeKind.FollowsIn, ServedGraph.At(NodeKind.Container, Genesis2Id, LegacyGenesis2Label));

        // Act
        var genesis2 = await new GraphExplorer(graph).Follow(toGenesis2);

        // Assert
        Assert.Equal((new ElementKind.Node(NodeKind.Container) as ElementKind, Genesis2Id, ServedGenesis2Label), (genesis2.Kind, genesis2.Id, genesis2.Label));
    }

    [Fact]
    public async Task Following_a_link_to_an_edge_resolves_it_with_its_two_ends_and_its_own_neighbour_groups()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.EdgeRecordOf(AttestedIn, ExodusEvent, Exodus14, new FrontierGroup(EdgeKind.JustifiedBy, 1)));
        var explorer = new GraphExplorer(graph);

        // Act
        var edge = await explorer.Follow(new Link(EdgeKind.Attests, ServedGraph.AtEdge(AttestedIn)));

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
        var edge = await explorer.Resolve(ServedGraph.AtEdge(ServedGraph.EdgeRef(EdgeKind.AttestedIn, AttestedIn.Id, LegacyGenesis2Label)));

        // Assert
        Assert.Equal(ServedGraph.AtEdge(AttestedIn), edge.Identity);
    }

    [Fact]
    public async Task A_node_has_no_ends()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Event, ExodusEvent.Id, ExodusEvent.Label));

        // Act
        var exodus = await new GraphExplorer(graph).Resolve(ServedGraph.At(ExodusEvent));

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
        var justifier = await new GraphExplorer(graph).Resolve(ServedGraph.At(NodeKind.Source, SourceId, SourceLabel));

        // Act
        var page = await justifier.Entries(EdgeKind.Justifies);

        // Assert
        Assert.Equal(new Page<Entry>([new Entry(new Link(EdgeKind.Justifies, ServedGraph.AtEdge(JustifiedEdge)), new Link(EdgeKind.Justifies, ServedGraph.AtEdge(Justification)))], null), page);
    }

    [Fact]
    public async Task Every_entry_offers_a_step_onto_its_served_edge()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.TextUnit, Exodus14.Id, Exodus14.Label, new FrontierGroup(EdgeKind.Attests, 1)))
            .Serving(Exodus14.Id, EdgeKind.Attests, null, ServedGraph.Page(EdgeKind.Attests, null, (AttestedIn, ServedGraph.At(ExodusEvent))));
        var verse = await new GraphExplorer(graph).Resolve(ServedGraph.At(Exodus14));

        // Act
        var page = await verse.Entries(EdgeKind.Attests);

        // Assert
        Assert.Equal(
            new Page<Entry>([new Entry(new Link(EdgeKind.Attests, ServedGraph.At(ExodusEvent)), new Link(EdgeKind.Attests, ServedGraph.AtEdge(AttestedIn)))], null),
            page);
    }

    [Fact]
    public async Task A_resolved_edge_pages_its_own_neighbours_from_the_neighbour_read_at_its_id()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.EdgeRecordOf(JustifiedEdge, ExodusEvent, Exodus14, new FrontierGroup(EdgeKind.JustifiedBy, 1)))
            .Serving(JustifiedEdge.Id, EdgeKind.JustifiedBy, null, ServedGraph.Page(EdgeKind.JustifiedBy, null, ServedGraph.Ref(NodeKind.Source, SourceId, SourceLabel)));
        var edge = await new GraphExplorer(graph).Resolve(ServedGraph.AtEdge(JustifiedEdge));

        // Act
        var page = await edge.Entries(EdgeKind.JustifiedBy);

        // Assert
        Assert.Equal(new Page<Entry>([ServedGraph.EntryTo(EdgeKind.JustifiedBy, ServedGraph.At(NodeKind.Source, SourceId, SourceLabel))], null), page);
    }

    [Fact]
    public async Task A_missing_element_is_a_contract_breach()
    {
        // Arrange
        var explorer = new GraphExplorer(new ServedGraph());

        // Act
        var breach = await Record.ExceptionAsync(() => explorer.Resolve(ServedGraph.At(NodeKind.Event, AbsentId, AbsentId)));

        // Assert
        Assert.Equal((typeof(ContractBreach), true), (breach?.GetType(), breach?.Message.Contains(AbsentId)));
    }

    [Fact]
    public async Task Resolving_many_targets_is_one_element_read_answered_in_the_order_asked()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.TextUnit, Exodus14.Id, Exodus14.Label))
            .Serving(ServedGraph.Card(NodeKind.Event, ExodusEvent.Id, ExodusEvent.Label))
            .Serving(ServedGraph.EdgeRecordOf(AttestedIn, ExodusEvent, Exodus14));
        var explorer = new GraphExplorer(graph);

        // Act
        var resolved = await explorer.Resolve([ServedGraph.At(Exodus14), ServedGraph.AtEdge(AttestedIn), ServedGraph.At(ExodusEvent)]);

        // Assert
        Assert.Equal((1, string.Join(" ", Exodus14.Id, AttestedIn.Id, ExodusEvent.Id)), (graph.ElementReads, string.Join(" ", resolved.Select(element => element.Id))));
    }

    [Fact]
    public async Task Presenting_an_edge_on_the_popover_yields_its_card_and_on_any_other_surface_nothing()
    {
        // Arrange
        var explorer = new GraphExplorer(new ServedGraph().Serving(ServedGraph.EdgeRecordOf(AttestedIn, ExodusEvent, Exodus14)));
        var edge = await explorer.Resolve(ServedGraph.AtEdge(AttestedIn));

        // Act
        var presented = await Task.WhenAll(Enum.GetValues<Surface>().Select(surface => explorer.Present(edge, surface)));

        // Assert
        Assert.Equal(new Presentation?[] { null, null, new Presentation.Card(AttestedIn.Label, [new Presentation.Field("Provenance", ServedGraph.Provenance)]) }, presented);
    }

    [Fact]
    public async Task An_edge_derived_from_the_grounds_of_another_is_carded_without_a_provenance()
    {
        // Arrange
        var explorer = new GraphExplorer(new ServedGraph().Serving(ServedGraph.EdgeRecordOf(Justification, ExodusEvent, Exodus14, provenance: null)));
        var edge = await explorer.Resolve(ServedGraph.AtEdge(Justification));

        // Act
        var presented = await explorer.Present(edge, Surface.Popover);

        // Assert
        Assert.Equal(new Presentation.Card(Justification.Label, []), presented);
    }

    [Fact]
    public async Task Presenting_a_node_on_the_popover_yields_the_generic_card_until_its_kind_is_migrated()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Person, MosesId, MosesLabel));
        var explorer = new GraphExplorer(graph);
        var moses = await explorer.Resolve(ServedGraph.At(NodeKind.Person, MosesId, MosesLabel));

        // Act
        var presentation = await explorer.Present(moses, Surface.Popover);

        // Assert
        Assert.Equal(new Presentation.Card(MosesLabel, [new Presentation.Field("Provenance", ServedGraph.Provenance)]), presentation);
    }

    [Fact]
    public async Task Presenting_a_node_on_the_reader_yields_the_generic_card_until_its_form_is_built()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Container, Genesis1Id, Genesis1Label));
        var explorer = new GraphExplorer(graph);
        var genesis1 = await explorer.Resolve(ServedGraph.At(NodeKind.Container, Genesis1Id, Genesis1Label));

        // Act
        var presentation = await explorer.Present(genesis1, Surface.Reader);

        // Assert
        Assert.Equal(new Presentation.Card(Genesis1Label, [new Presentation.Field("Provenance", ServedGraph.Provenance)]), presentation);
    }

    [Fact]
    public async Task A_node_has_no_presentation_on_a_surface_where_its_kind_has_no_form()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Person, MosesId, MosesLabel));
        var explorer = new GraphExplorer(graph);
        var moses = await explorer.Resolve(ServedGraph.At(NodeKind.Person, MosesId, MosesLabel));

        // Act
        var presentation = await explorer.Present(moses, Surface.World);

        // Assert
        Assert.Null(presentation);
    }

    [Fact]
    public void The_explorer_is_the_only_site_that_builds_an_explorable()
    {
        // Arrange
        var publicConstructors = typeof(Explorable).GetConstructors(BindingFlags.Public | BindingFlags.Instance);

        // Act
        var constructingFiles = string.Join(", ", ConformanceTests.ClientSourceFiles()
            .Where(file => BuildsAnExplorable.IsMatch(File.ReadAllText(file)))
            .Select(Path.GetFileName));

        // Assert
        Assert.Equal((0, TheOneConstructingFile), (publicConstructors.Length, constructingFiles));
    }
}
