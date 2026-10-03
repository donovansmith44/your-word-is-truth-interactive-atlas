using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class LegacyNodeIdsTests
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
        var id = LegacyNodeIds.Of(kind, local);
        // Assert
        Assert.Equal(Wire.Node(expected), id);
    }

    [Fact]
    public void Of_refuses_a_local_part_that_is_already_an_id_of_its_kind()
    {
        // Arrange
        var id = "CommentaryItem:kretzmann/0.1.0";
        // Act
        Action act = () => LegacyNodeIds.Of(NodeKind.CommentaryItem, id);
        // Assert
        var thrown = Assert.Throws<FormatException>(act);
        Assert.Equal("'CommentaryItem:kretzmann/0.1.0' is already a node id; Of wants its local part", thrown.Message);
    }

    [Theory]
    [InlineData("CatechismItem:commandment-1", NodeKind.CatechismItem, "commandment-1")]
    [InlineData("text-unit:BoC 7.2.1", NodeKind.TextUnit, "BoC 7.2.1")]
    public void LocalPart_is_the_id_after_its_kind(string id, NodeKind kind, string expected)
    {
        // Arrange
        var node = new NodeRef(id: Wire.Node(id), kind: kind, label: id);
        // Act
        var local = LegacyNodeIds.LocalPart(node);
        // Assert
        Assert.Equal(expected, local);
    }

    [Fact]
    public void LocalPart_of_an_id_whose_kind_prefix_is_empty_is_the_text_after_the_colon()
    {
        // Arrange
        var node = new NodeRef(id: Wire.Node(":foo"), kind: NodeKind.TextUnit, label: ":foo");
        // Act
        var local = LegacyNodeIds.LocalPart(node);
        // Assert
        Assert.Equal("foo", local);
    }

    [Fact]
    public void LocalPart_of_an_id_with_no_kind_fails_naming_the_id()
    {
        // Arrange
        var node = new NodeRef(id: Wire.Node("commandment-1"), kind: NodeKind.CatechismItem, label: "commandment-1");
        // Act
        Action act = () => LegacyNodeIds.LocalPart(node);
        // Assert
        var thrown = Assert.Throws<FormatException>(act);
        Assert.Equal("'commandment-1' is not a node id: it has no 'Kind:' before its local part", thrown.Message);
    }
}
