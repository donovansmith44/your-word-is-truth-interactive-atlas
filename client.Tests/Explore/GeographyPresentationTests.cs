using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class GeographyPresentationTests
{
    private const string Conquest = "Map:era-conquest";
    private const string ConquestLabel = "The Conquest";
    private const string Patriarchs = "Map:era-patriarchs";
    private const string PatriarchsLabel = "The Patriarchs";
    private const string Judges = "Map:era-judges";
    private const string JudgesLabel = "The Judges";
    private const string Kingdom = "Era:united-kingdom";
    private const string KingdomLabel = "The United Kingdom";
    private const string Bethel = "Place:bethel-1";
    private const string BethelLabel = "Bethel";
    private const string BethelCanonical = "Bethel 1";
    private const string BethelBlurb = "Where Jacob dreamed of a ladder.";
    private const double BethelLat = 31.93;
    private const double BethelLon = 35.22;
    private const string Philistia = "Polity:philistia";
    private const string PhilistiaLabel = "Philistia";
    private const string Moses = "Person:moses_2108";
    private const string MosesLabel = "Moses";
    private const string AttestedInId = "Attests:00aa";
    private const string AttestedInLabel = "The Red Sea parted · Attested in · Exodus 14:21";
    private const string WindowField = "Window";
    private const string ReignField = "Reign";
    private const string CanonicalNameField = "Canonical name";
    private const string EstablishedField = "Established";
    private const string DestroyedField = "Destroyed";
    private const string BlurbField = "Blurb";
    private const string ProvenanceField = "Provenance";

    private static readonly TimeRange ConquestWindow = ServedGraph.Range(new Year(label: "1451 BC", value: -1450), new Year(label: "1400 BC", value: -1399), "1451 BC – 1400 BC");
    private static readonly TimeRange KingdomWindow = ServedGraph.Range(new Year(label: "1095 BC", value: -1094), new Year(label: "975 BC", value: -974), "1095 BC – 975 BC");
    private static readonly TimeRange PhilistiaReign = ServedGraph.Range(new Year(label: "1200 BC", value: -1199), new Year(label: "604 BC", value: -603), "1200 BC – 604 BC");
    private static readonly DateClaim BethelEstablished = new(@event: null, label: "c. 2000 BC", note: null, verses: [], when: ServedGraph.Range(new Year(label: "2000 BC", value: -1999), new Year(label: "2000 BC", value: -1999), "c. 2000 BC"));
    private static readonly NodeRef PatriarchsMap = ServedGraph.Ref(NodeKind.Map, Patriarchs, PatriarchsLabel);
    private static readonly NodeRef JudgesMap = ServedGraph.Ref(NodeKind.Map, Judges, JudgesLabel);

    [Fact]
    public async Task A_map_on_the_world_is_bounded_by_its_window_with_its_neighbouring_maps_at_either_end()
    {
        // Arrange
        var (explorer, conquest) = Serve(
            ServedGraph.Card(NodeKind.Map, Conquest, ConquestLabel, new FrontierGroup(EdgeKind.PrecedesIn, 1), new FrontierGroup(EdgeKind.FollowsIn, 1)) with { Map = ServedGraph.MapWindow(ConquestWindow) },
            graph => graph
                .Serving(Conquest, EdgeKind.PrecedesIn, null, ServedGraph.Page(EdgeKind.PrecedesIn, null, PatriarchsMap))
                .Serving(Conquest, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, JudgesMap)));

        // Act
        var presented = await explorer.Present(new PresentationRequest(conquest, Surface.World));

        // Assert
        Assert.Equal(
            new Presentation.Geography(
                new Frame.Bounded(ConquestWindow, new Link(EdgeKind.PrecedesIn, ServedGraph.At(PatriarchsMap)), new Link(EdgeKind.FollowsIn, ServedGraph.At(JudgesMap))),
                new Emphasis.None()),
            presented);
    }

    [Fact]
    public async Task An_era_on_the_world_is_bounded_like_its_map()
    {
        // Arrange
        var judgesEra = ServedGraph.Ref(NodeKind.Era, Judges, JudgesLabel);
        var (explorer, kingdom) = Serve(
            ServedGraph.Card(NodeKind.Era, Kingdom, KingdomLabel, new FrontierGroup(EdgeKind.PrecedesIn, 1)) with { Era = ServedGraph.EraWindow(KingdomWindow) },
            graph => graph.Serving(Kingdom, EdgeKind.PrecedesIn, null, ServedGraph.Page(EdgeKind.PrecedesIn, null, judgesEra)));

        // Act
        var presented = await explorer.Present(new PresentationRequest(kingdom, Surface.World));

        // Assert
        Assert.Equal(
            new Presentation.Geography(new Frame.Bounded(KingdomWindow, new Link(EdgeKind.PrecedesIn, ServedGraph.At(judgesEra)), null), new Emphasis.None()),
            presented);
    }

    [Fact]
    public async Task A_first_map_has_no_previous_end()
    {
        // Arrange
        var (explorer, patriarchs) = Serve(
            ServedGraph.Card(NodeKind.Map, Patriarchs, PatriarchsLabel, new FrontierGroup(EdgeKind.FollowsIn, 1)) with { Map = ServedGraph.MapWindow(ConquestWindow) },
            graph => graph.Serving(Patriarchs, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, JudgesMap)));

        // Act
        var presented = await explorer.Present(new PresentationRequest(patriarchs, Surface.World));

        // Assert
        Assert.Equal(
            new Presentation.Geography(new Frame.Bounded(ConquestWindow, null, new Link(EdgeKind.FollowsIn, ServedGraph.At(JudgesMap))), new Emphasis.None()),
            presented);
    }

    [Fact]
    public async Task A_place_on_the_world_keeps_the_current_frame_and_marks_its_site()
    {
        // Arrange
        var (explorer, bethel) = Serve(ServedGraph.Card(NodeKind.Place, Bethel, BethelLabel) with { Place = ServedGraph.PlaceAt(BethelLabel, BethelLat, BethelLon) });

        // Act
        var presented = await explorer.Present(new PresentationRequest(bethel, Surface.World));

        // Assert
        Assert.Equal(
            new Presentation.Geography(new Frame.Current(), new Emphasis.Site(ServedGraph.Ref(NodeKind.Place, Bethel, BethelLabel), BethelLat, BethelLon)),
            presented);
    }

    [Fact]
    public async Task A_polity_on_the_world_keeps_the_current_era_and_marks_its_territory_with_its_reign()
    {
        // Arrange
        var (explorer, philistia) = Serve(ServedGraph.Card(NodeKind.Polity, Philistia, PhilistiaLabel) with { Polity = ServedGraph.Reign(PhilistiaReign) });

        // Act
        var presented = await explorer.Present(new PresentationRequest(philistia, Surface.World));

        // Assert
        Assert.Equal(
            new Presentation.Geography(new Frame.Current(), new Emphasis.Territory(ServedGraph.Ref(NodeKind.Polity, Philistia, PhilistiaLabel), PhilistiaReign)),
            presented);
    }

    [Fact]
    public async Task A_geographic_node_served_without_its_detail_is_a_contract_breach()
    {
        // Arrange
        var bare = new[] { NodeKind.Map, NodeKind.Era, NodeKind.Place, NodeKind.Polity }
            .Select(kind => Serve(ServedGraph.Card(kind, $"{kind}:bare", kind.ToString())))
            .ToList();

        // Act
        var breaches = await Task.WhenAll(bare.Select(served => Assert.ThrowsAsync<ContractBreach>(() => served.Explorer.Present(new PresentationRequest(served.Element, Surface.World)))));

        // Assert
        Assert.Equal(
            ["Map:bare", "Era:bare", "Place:bare", "Polity:bare"],
            breaches.Select(breach => bare.Select(served => served.Element.Id).Single(breach.Message.Contains)).ToArray());
    }

    [Fact]
    public async Task A_place_card_lists_only_the_served_fields()
    {
        // Arrange
        var (explorer, bethel) = Serve(ServedGraph.Card(NodeKind.Place, Bethel, BethelLabel) with
        {
            Place = ServedGraph.PlaceAt(BethelLabel, BethelLat, BethelLon) with { CanonicalName = BethelCanonical, Blurb = BethelBlurb },
        });

        // Act
        var presented = await explorer.Present(new PresentationRequest(bethel, Surface.Popover));

        // Assert
        Assert.Equal(
            new Presentation.Card(BethelLabel, [new Presentation.Field(CanonicalNameField, BethelCanonical), new Presentation.Field(BlurbField, BethelBlurb), new Presentation.Field(ProvenanceField, ServedGraph.Provenance)]),
            presented);
    }

    [Fact]
    public async Task A_place_card_lists_every_served_field_in_order()
    {
        // Arrange
        var destroyed = BethelEstablished with { Label = "722 BC" };
        var (explorer, bethel) = Serve(ServedGraph.Card(NodeKind.Place, Bethel, BethelLabel) with
        {
            Place = ServedGraph.PlaceAt(BethelLabel, BethelLat, BethelLon) with { CanonicalName = BethelCanonical, Established = BethelEstablished, Destroyed = destroyed, Blurb = BethelBlurb },
        });

        // Act
        var presented = await explorer.Present(new PresentationRequest(bethel, Surface.Popover));

        // Assert
        Assert.Equal(
            new Presentation.Card(
                BethelLabel,
                [
                    new Presentation.Field(CanonicalNameField, BethelCanonical),
                    new Presentation.Field(EstablishedField, BethelEstablished.Label),
                    new Presentation.Field(DestroyedField, destroyed.Label),
                    new Presentation.Field(BlurbField, BethelBlurb),
                    new Presentation.Field(ProvenanceField, ServedGraph.Provenance),
                ]),
            presented);
    }

    [Fact]
    public async Task A_polity_card_shows_its_reign_label()
    {
        // Arrange
        var (explorer, philistia) = Serve(ServedGraph.Card(NodeKind.Polity, Philistia, PhilistiaLabel) with { Polity = ServedGraph.Reign(PhilistiaReign) });

        // Act
        var presented = await explorer.Present(new PresentationRequest(philistia, Surface.Popover));

        // Assert
        Assert.Equal(
            new Presentation.Card(PhilistiaLabel, [new Presentation.Field(ReignField, PhilistiaReign.Label), new Presentation.Field(ProvenanceField, ServedGraph.Provenance)]),
            presented);
    }

    [Fact]
    public async Task A_map_card_shows_its_window_label()
    {
        // Arrange
        var (explorer, conquest) = Serve(ServedGraph.Card(NodeKind.Map, Conquest, ConquestLabel) with { Map = ServedGraph.MapWindow(ConquestWindow) });

        // Act
        var presented = await explorer.Present(new PresentationRequest(conquest, Surface.Popover));

        // Assert
        Assert.Equal(
            new Presentation.Card(ConquestLabel, [new Presentation.Field(WindowField, ConquestWindow.Label), new Presentation.Field(ProvenanceField, ServedGraph.Provenance)]),
            presented);
    }

    [Fact]
    public async Task An_era_card_shows_its_window_label()
    {
        // Arrange
        var (explorer, kingdom) = Serve(ServedGraph.Card(NodeKind.Era, Kingdom, KingdomLabel) with { Era = ServedGraph.EraWindow(KingdomWindow) });

        // Act
        var presented = await explorer.Present(new PresentationRequest(kingdom, Surface.Popover));

        // Assert
        Assert.Equal(
            new Presentation.Card(KingdomLabel, [new Presentation.Field(WindowField, KingdomWindow.Label), new Presentation.Field(ProvenanceField, ServedGraph.Provenance)]),
            presented);
    }

    [Fact]
    public async Task A_non_geographic_node_is_not_presented_on_the_world()
    {
        // Arrange
        var (explorer, moses) = Serve(ServedGraph.Card(NodeKind.Person, Moses, MosesLabel));

        // Act
        var presented = await explorer.Present(new PresentationRequest(moses, Surface.World));

        // Assert
        Assert.Null(presented);
    }

    [Fact]
    public async Task An_edge_is_never_presented_on_the_world()
    {
        // Arrange
        var attestedIn = ServedGraph.EdgeRef(EdgeKind.AttestedIn, AttestedInId, AttestedInLabel);
        var graph = new ServedGraph().Serving(ServedGraph.EdgeRecordOf(attestedIn, PatriarchsMap, JudgesMap));
        var explorer = new GraphExplorer(graph);
        var edge = await explorer.Resolve(ServedGraph.AtEdge(attestedIn));

        // Act
        var presented = await explorer.Present(new PresentationRequest(edge, Surface.World));

        // Assert
        Assert.Null(presented);
    }

    private static (GraphExplorer Explorer, Explorable Element) Serve(NodeRecord record, Func<ServedGraph, ServedGraph>? neighbours = null)
    {
        var graph = (neighbours ?? (graph => graph))(new ServedGraph().Serving(record));
        var explorer = new GraphExplorer(graph);
        return (explorer, explorer.Resolve(ServedGraph.At(record.Kind, record.Id, record.Label)).GetAwaiter().GetResult());
    }
}
