using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class DeletionLawTests
{
    private static readonly IReadOnlyList<NodeKind> MigratedKinds = [];

    private const string AnyLocalId = "any";

    [Fact]
    public void No_kind_is_served_by_both_mechanisms()
    {
        // Arrange
        var migrated = MigratedKinds.Select(kind => Resolved.Node(kind, NodeIds.Of(kind, AnyLocalId), AnyLocalId)).ToList();

        // Act
        var legacyNodesForMigratedKinds = migrated.Select(node => (node.Kind, LegacyNodes.For(node))).ToList();

        // Assert
        Assert.Equal(MigratedKinds.Select(kind => (kind, (IExplorable?)null)).ToList(), legacyNodesForMigratedKinds);
    }
}
