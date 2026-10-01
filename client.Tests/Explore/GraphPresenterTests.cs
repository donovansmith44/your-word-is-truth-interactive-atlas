using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class GraphPresenterTests
{
    private const string MosesId = "Person:moses_2108";
    private const string MosesLabel = "Moses";
    private const string Genesis1Id = "Container:bible-chapter-GEN-1";
    private const string Genesis1Label = "Genesis 1";

    private static readonly IPresenter Presenter = new GraphPresenter();
    private static readonly NodeRef ExodusEvent = ServedGraph.Ref(NodeKind.Event, "Event:red_sea", "The Red Sea parted");
    private static readonly NodeRef Exodus14 = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:EXO.14.21", "Exodus 14:21");
    private static readonly EdgeRef AttestedIn = ServedGraph.EdgeRef(EdgeKind.AttestedIn, "Attests:00aa", "The Red Sea parted · Attested in · Exodus 14:21");
    private static readonly EdgeRef Justification = ServedGraph.EdgeRef(EdgeKind.JustifiedBy, "JustifiedBy:00dd", "The Red Sea parted · Dated by · 1491 BC · Justified by · Ussher");

    [Fact]
    public async Task Presenting_an_edge_on_the_popover_yields_its_card_and_on_any_other_surface_nothing()
    {
        // Arrange
        var edge = Resolved.At(new ServedGraph().Serving(ServedGraph.EdgeRecordOf(AttestedIn, ExodusEvent, Exodus14)), ServedGraph.AtEdge(AttestedIn));

        // Act
        var presented = await Task.WhenAll(Enum.GetValues<Surface>().Select(surface => Presenter.Present(new PresentationRequest(edge, surface))));

        // Assert
        Assert.Equal(new Presentation?[] { null, null, new Presentation.Card(AttestedIn.Label, [new Presentation.Field("Provenance", ServedGraph.Provenance)]) }, presented);
    }

    [Fact]
    public async Task An_edge_derived_from_the_grounds_of_another_is_carded_without_a_provenance()
    {
        // Arrange
        var edge = Resolved.At(new ServedGraph().Serving(ServedGraph.EdgeRecordOf(Justification, ExodusEvent, Exodus14, provenance: null)), ServedGraph.AtEdge(Justification));

        // Act
        var presented = await Presenter.Present(new PresentationRequest(edge, Surface.Popover));

        // Assert
        Assert.Equal(new Presentation.Card(Justification.Label, []), presented);
    }

    [Fact]
    public async Task Presenting_a_node_on_the_popover_yields_the_generic_card_until_its_kind_is_migrated()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Person, MosesId, MosesLabel));
        var moses = Resolved.At(graph, ServedGraph.At(NodeKind.Person, MosesId, MosesLabel));

        // Act
        var presentation = await Presenter.Present(new PresentationRequest(moses, Surface.Popover));

        // Assert
        Assert.Equal(new Presentation.Card(MosesLabel, [new Presentation.Field("Provenance", ServedGraph.Provenance)]), presentation);
    }

    [Fact]
    public async Task Presenting_a_node_on_the_reader_yields_the_generic_card_until_its_form_is_built()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Container, Genesis1Id, Genesis1Label));
        var genesis1 = Resolved.At(graph, ServedGraph.At(NodeKind.Container, Genesis1Id, Genesis1Label));

        // Act
        var presentation = await Presenter.Present(new PresentationRequest(genesis1, Surface.Reader));

        // Assert
        Assert.Equal(new Presentation.Card(Genesis1Label, [new Presentation.Field("Provenance", ServedGraph.Provenance)]), presentation);
    }

    [Fact]
    public async Task A_node_has_no_presentation_on_a_surface_where_its_kind_has_no_form()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Person, MosesId, MosesLabel));
        var moses = Resolved.At(graph, ServedGraph.At(NodeKind.Person, MosesId, MosesLabel));

        // Act
        var presentation = await Presenter.Present(new PresentationRequest(moses, Surface.World));

        // Assert
        Assert.Null(presentation);
    }
}
