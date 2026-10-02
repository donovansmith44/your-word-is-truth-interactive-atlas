using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class NeighboursTests
{
    private static readonly NodeRef Hazor = new(id: "Place:hazor-1", kind: NodeKind.Place, label: "Hazor");
    private static readonly NodeRef Jabin = new(id: "Person:jabin_1", kind: NodeKind.Person, label: "Jabin");
    private static readonly EdgeRef ADating = new(id: "DatedBy:00ff", kind: EdgeKind.DatedBy, label: "A dating");

    private static EdgeEntry Leading(string edge, PositionRef to) =>
        new(edge: new EdgeRef(id: edge, kind: EdgeKind.Cites, label: edge), loci: null, narrative: null, neighbour: to, note: null, parentage: null, votes: null);

    [Fact]
    public void Nodes_keeps_every_node_neighbour_in_page_order_and_passes_over_an_edge()
    {
        // Arrange
        var page = new[] { Leading("e1", new NodePosition(Hazor)), Leading("e2", new EdgePosition(ADating)), Leading("e3", new NodePosition(Jabin)) };
        // Act
        var nodes = page.Nodes().ToList();
        // Assert
        Assert.Equal([Hazor, Jabin], nodes);
    }
}
