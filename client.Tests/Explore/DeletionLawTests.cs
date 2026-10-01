using System.Text.RegularExpressions;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public sealed class DeletionLawTests
{
    private static readonly IReadOnlyList<NodeKind> MigratedKinds = [NodeKind.Place, NodeKind.Polity, NodeKind.Era, NodeKind.Map];

    private static readonly IReadOnlyList<Type> HeldForTheMapsMigration = [typeof(PolityDeltaNode)];

    private const string AnyLocalId = "any";

    [Fact]
    public void No_kind_is_served_by_both_mechanisms()
    {
        // Arrange
        var migrated = MigratedKinds.Select(kind => Resolved.Node(kind, NodeIds.Of(kind, AnyLocalId), AnyLocalId)).ToList();

        // Act
        var legacyNodesForMigratedKinds = migrated.Select(node => (node.Kind, LegacyNodes.For(node))).ToList();

        // Assert
        Assert.Equal(MigratedKinds.Select(kind => ((ElementKind)new ElementKind.Node(kind), (IExplorable?)null)).ToList(), legacyNodesForMigratedKinds);
    }

    [Fact]
    public void No_section_provider_names_a_migrated_kind()
    {
        // Arrange
        var providers = File.ReadAllText(Path.Combine(ConformanceTests.ClientRoot, "Explore", "PopoverSectionProviders.cs"));

        // Act
        var named = MigratedKinds
            .Where(kind => Regex.IsMatch(providers, $@"AppliesTo\(IExplorable node\) => node\.Kind (==|is)[^;]*""{kind}"""))
            .ToList();

        // Assert
        Assert.Empty(named);
    }

    [Fact]
    public void No_legacy_node_class_exists_for_a_migrated_kind()
    {
        // Arrange
        var legacyNodes = typeof(IExplorable).Assembly.GetTypes()
            .Where(type => typeof(IExplorable).IsAssignableFrom(type) && !type.IsInterface && !HeldForTheMapsMigration.Contains(type));

        // Act
        var named = legacyNodes
            .Where(type => MigratedKinds.Any(kind => type.Name.StartsWith(kind.ToString(), StringComparison.Ordinal)))
            .Select(type => type.Name)
            .ToList();

        // Assert
        Assert.Empty(named);
    }
}
