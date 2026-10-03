using System.Text.RegularExpressions;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Legacy;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public sealed class DeletionLawTests
{
    private static readonly IReadOnlyList<NodeKind> MigratedKinds = [NodeKind.Place, NodeKind.Polity, NodeKind.Era, NodeKind.Map, NodeKind.TextUnit];

    private static readonly IReadOnlyDictionary<NodeKind, IReadOnlyList<string>> LegacyNames = new Dictionary<NodeKind, IReadOnlyList<string>>
    {
        [NodeKind.Place] = ["Place"],
        [NodeKind.Polity] = ["Polity"],
        [NodeKind.Era] = ["Era"],
        [NodeKind.Map] = ["Map"],
        [NodeKind.TextUnit] = ["Verse", "ConcordUnit"],
    };

    private static readonly IReadOnlyDictionary<Type, string> HeldUntil = new Dictionary<Type, string>
    {
        [typeof(PolityDeltaNode)] = "MAPS",
    };

    private const string AnyLocalId = "any";

    [Fact]
    public void Every_migrated_kind_names_its_legacy_names()
    {
        // Arrange
        var migrated = MigratedKinds;

        // Act
        var unnamed = migrated.Where(kind => !LegacyNames.TryGetValue(kind, out var names) || names.Count == 0).ToList();

        // Assert
        Assert.Empty(unnamed);
    }

    [Fact]
    public void No_kind_is_served_by_both_mechanisms()
    {
        // Arrange
        var migrated = MigratedKinds.Select(kind => Resolved.Node(kind, LegacyNodeIds.Of(kind, AnyLocalId), AnyLocalId)).ToList();

        // Act
        var legacyNodesForMigratedKinds = migrated.Select(node => (node.Kind, LegacyNodes.For(node))).ToList();

        // Assert
        Assert.Equal(MigratedKinds.Select(kind => ((ElementKind)new ElementKind.Node(kind), (IExplorable?)null)).ToList(), legacyNodesForMigratedKinds);
    }

    [Fact]
    public void No_section_provider_names_a_migrated_kind()
    {
        // Arrange
        var providers = File.ReadAllText(Path.Combine(ConformanceTests.ClientRoot, "Legacy", "PopoverSectionProviders.cs"));

        // Act
        var named = RetiredNames()
            .Where(name => Regex.IsMatch(providers, $@"AppliesTo\(IExplorable node\) => node\.Kind (==|is)[^;]*""{name}"""))
            .ToList();

        // Assert
        Assert.Empty(named);
    }

    [Fact]
    public void No_legacy_node_class_exists_for_a_migrated_kind()
    {
        // Arrange
        var legacyNodes = typeof(IExplorable).Assembly.GetTypes()
            .Where(type => typeof(IExplorable).IsAssignableFrom(type) && !type.IsInterface && !HeldUntil.ContainsKey(type));

        // Act
        var named = legacyNodes
            .Where(type => RetiredNames().Any(name => type.Name.StartsWith(name, StringComparison.Ordinal)))
            .Select(type => type.Name)
            .ToList();

        // Assert
        Assert.Empty(named);
    }

    private static IEnumerable<string> RetiredNames() => MigratedKinds.SelectMany(kind => LegacyNames[kind]);
}
