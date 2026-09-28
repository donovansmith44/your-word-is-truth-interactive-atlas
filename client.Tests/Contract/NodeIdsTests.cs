using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class NodeIdsTests
{
    [Theory]
    [InlineData(NodeKind.CatechismItem, "commandment-1", "CatechismItem:commandment-1")]
    [InlineData(NodeKind.Place, "hazor-1", "Place:hazor-1")]
    [InlineData(NodeKind.TextUnit, "JHN.3.16", "text-unit:JHN.3.16")]
    [InlineData(NodeKind.TextUnit, "BoC 7.2.1", "text-unit:BoC 7.2.1")]
    public void Of_writes_the_kind_then_the_local_part(NodeKind kind, string localPart, string expected)
    {
        // Arrange
        var local = localPart;
        // Act
        var id = NodeIds.Of(kind, local);
        // Assert
        Assert.Equal(expected, id);
    }

    [Theory]
    [InlineData("CatechismItem:commandment-1", PositionKind.CatechismItem, "commandment-1")]
    [InlineData("text-unit:BoC 7.2.1", PositionKind.TextUnit, "BoC 7.2.1")]
    public void LocalPart_is_the_id_after_its_kind(string id, PositionKind kind, string expected)
    {
        // Arrange
        var node = new NodeRef(id: id, kind: kind, label: id);
        // Act
        var local = NodeIds.LocalPart(node);
        // Assert
        Assert.Equal(expected, local);
    }

    [Fact]
    public void LocalPart_of_an_id_with_no_kind_fails_naming_the_id()
    {
        // Arrange
        var node = new NodeRef(id: "commandment-1", kind: PositionKind.CatechismItem, label: "commandment-1");
        // Act
        Action act = () => NodeIds.LocalPart(node);
        // Assert
        var thrown = Assert.Throws<FormatException>(act);
        Assert.Equal("'commandment-1' is not a node id: it has no 'Kind:' before its local part", thrown.Message);
    }
}
