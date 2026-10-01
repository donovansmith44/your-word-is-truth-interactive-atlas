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
    private const string HazorId = "Place:hazor-1";
    private const string HazorLabel = "Hazor 1";
    private const string TheOneConstructingFile = "Explorer.cs";
    private const string AttestsEdge = "Attests:00aa";
    private const string AttestsLabel = "The Red Sea parted attested in EXO.14.21";
    private const string SourceId = "Source:ussher";
    private const string SourceLabel = "Ussher";
    private const string JustifiedEdge = "Mentions:00cc";
    private const string JustifiedLabel = "EXO.14.21 mentions the Red Sea";
    private const string JustifiesEdge = "JustifiedBy:00dd";

    private static readonly NodeRef ExodusEvent = ServedGraph.Ref(NodeKind.Event, "Event:red_sea", "The Red Sea parted");
    private static readonly NodeRef Exodus14 = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:EXO.14.21", "EXO.14.21");

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
    public async Task Following_a_link_to_an_edge_resolves_the_edge_with_its_ends_as_its_frontier()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.EdgeCardOf(EdgeKind.AttestedIn, AttestsEdge, AttestsLabel, ServedGraph.At(ExodusEvent), ServedGraph.At(Exodus14)))
            .Serving(ServedGraph.Card(NodeKind.TextUnit, Exodus14.Id, Exodus14.Label, new FrontierGroup(EdgeKind.Attests, 1)));
        var explorer = new GraphExplorer(graph);

        // Act
        var edge = await explorer.Follow(new Link(EdgeKind.TargetOf, ServedGraph.AtEdge(AttestsEdge, EdgeKind.AttestedIn, AttestsLabel)));

        // Assert
        Assert.Equal(
            (new ElementKind.Edge(EdgeKind.AttestedIn) as ElementKind, AttestsEdge, AttestsLabel, string.Join(" ", new FrontierGroup(EdgeKind.From, 1), new FrontierGroup(EdgeKind.To, 1))),
            (edge.Kind, edge.Id, edge.Label, string.Join(" ", edge.Groups)));
    }

    [Fact]
    public async Task A_page_that_leads_to_edges_yields_links_to_those_edges()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Source, SourceId, SourceLabel, new FrontierGroup(EdgeKind.Justifies, 1)))
            .Serving(SourceId, EdgeKind.Justifies, null, ServedGraph.Page(EdgeKind.Justifies, null, (JustifiesEdge, EdgeEnd.To, ServedGraph.AtEdge(JustifiedEdge, EdgeKind.Mentions, JustifiedLabel))));
        var justifier = await new GraphExplorer(graph).Resolve(ServedGraph.At(NodeKind.Source, SourceId, SourceLabel));

        // Act
        var page = await justifier.Links(EdgeKind.Justifies);

        // Assert
        Assert.Equal(new Page<Link>([new Link(EdgeKind.Justifies, ServedGraph.AtEdge(JustifiedEdge, EdgeKind.Mentions, JustifiedLabel))], null), page);
    }

    [Fact]
    public async Task Following_is_total_over_every_position_the_contract_declares()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.EdgeCardOf(EdgeKind.AttestedIn, AttestsEdge, AttestsLabel, ServedGraph.At(ExodusEvent), ServedGraph.At(Exodus14)))
            .Serving(ServedGraph.Card(NodeKind.TextUnit, Exodus14.Id, Exodus14.Label));
        var explorer = new GraphExplorer(graph);
        var onePerDeclaredPosition = new Dictionary<Type, PositionRef>
        {
            [typeof(NodePosition)] = ServedGraph.At(Exodus14),
            [typeof(EdgePosition)] = ServedGraph.AtEdge(AttestsEdge, EdgeKind.AttestedIn, AttestsLabel),
        };

        // Act
        var followed = await Task.WhenAll(DeclaredPositions().Select(async type => (type, (await explorer.Follow(new Link(EdgeKind.Mentions, onePerDeclaredPosition[type]))).Id)));

        // Assert
        Assert.Equal(new[] { (typeof(EdgePosition), AttestsEdge), (typeof(NodePosition), Exodus14.Id) }, followed);
    }

    [Fact]
    public async Task Presenting_an_edge_on_the_popover_yields_its_card_and_on_any_other_surface_nothing()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.EdgeCardOf(EdgeKind.AttestedIn, AttestsEdge, AttestsLabel, ServedGraph.At(ExodusEvent), ServedGraph.At(Exodus14)));
        var explorer = new GraphExplorer(graph);
        var edge = await explorer.Resolve(ServedGraph.AtEdge(AttestsEdge, EdgeKind.AttestedIn, AttestsLabel));

        // Act
        var presented = await Task.WhenAll(Enum.GetValues<Surface>().Select(surface => explorer.Present(edge, surface)));

        // Assert
        Assert.Equal(new Presentation?[] { null, null, new Presentation.Card(AttestsLabel, [new Presentation.Field("Provenance", ServedGraph.Provenance)]) }, presented);
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
    public async Task Presenting_a_node_on_its_home_surface_yields_the_generic_card_until_its_kind_is_migrated()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Place, HazorId, HazorLabel));
        var explorer = new GraphExplorer(graph);
        var hazor = await explorer.Resolve(ServedGraph.At(NodeKind.Place, HazorId, HazorLabel));

        // Act
        var presentation = await explorer.Present(hazor, Surface.World);

        // Assert
        Assert.Equal(new Presentation.Card(HazorLabel, [new Presentation.Field("Provenance", ServedGraph.Provenance)]), presentation);
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

    private const string DeclaredSubtype = "JsonInheritanceAttribute";
    private const string DeclaredSubtypeType = "Type";

    private static IEnumerable<Type> DeclaredPositions() =>
        typeof(PositionRef).GetCustomAttributes(inherit: false)
            .Where(attribute => attribute.GetType().Name == DeclaredSubtype)
            .Select(attribute => (Type)attribute.GetType().GetProperty(DeclaredSubtypeType)!.GetValue(attribute)!);
}
