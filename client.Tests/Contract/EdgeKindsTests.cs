using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class EdgeKindsTests
{
    private const int DeclaredSymmetricKinds = 6;

    [Fact]
    public void Dual_is_an_involution()
    {
        // Arrange
        var kinds = Enum.GetValues<EdgeKind>();
        // Act
        var twice = kinds.Select(k => k.Dual().Dual()).ToArray();
        // Assert
        Assert.Equal(kinds, twice);
    }

    [Fact]
    public void Member_of_and_contains_are_each_others_dual()
    {
        // Arrange
        var contains = EdgeKind.Contains;
        // Act
        var dual = contains.Dual();
        // Assert
        Assert.Equal(EdgeKind.MemberOf, dual);
        Assert.Equal(contains, dual.Dual());
    }

    [Fact]
    public void Symmetric_kinds_are_their_own_dual()
    {
        // Arrange
        var symmetric = Enum.GetValues<EdgeKind>().Where(k => k.IsSymmetric()).ToArray();
        // Act
        var duals = symmetric.Select(k => k.Dual()).ToArray();
        // Assert
        Assert.Equal(DeclaredSymmetricKinds, symmetric.Length);
        Assert.Equal(symmetric, duals);
    }

    [Fact]
    public void Label_round_trips_through_Parse_for_every_kind()
    {
        // Arrange
        var kinds = Enum.GetValues<EdgeKind>();
        // Act
        var back = kinds.Select(k => EdgeKinds.Parse(k.Label())).ToArray();
        // Assert
        Assert.Equal(kinds, back);
        Assert.Equal("member-of", EdgeKind.MemberOf.Label());
    }

    [Fact]
    public void Parse_rejects_an_undeclared_label()
    {
        // Arrange
        var label = "cited";
        // Act
        Action act = () => EdgeKinds.Parse(label);
        // Assert
        Assert.Throws<FormatException>(act);
    }
}
