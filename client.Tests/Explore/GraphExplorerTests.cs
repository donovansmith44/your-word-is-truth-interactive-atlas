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
    private const string TheOneConstructingFile = "Explorer.cs";

    private static readonly Regex BuildsAnExplorable = new(@"\bnew Explorable\(|\bExplorable\??\s+\w+\s*=>?\s*new\(", RegexOptions.Compiled);

    [Fact]
    public async Task Resolving_a_reference_fetches_its_card_and_yields_the_node_with_its_frontier()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Container, Genesis1Id, "Genesis 1", new FrontierGroup(EdgeKind.MemberOf, 1)));
        var explorer = new GraphExplorer(graph);

        // Act
        var genesis1 = await explorer.Resolve(ServedGraph.Ref(NodeKind.Container, Genesis1Id, "Genesis 1"));

        // Assert
        Assert.Equal(
            (NodeKind.Container, Genesis1Id, "Genesis 1", new FrontierGroup(EdgeKind.MemberOf, 1)),
            (genesis1.Kind, genesis1.Id, genesis1.Label, genesis1.Groups.Single()));
    }

    [Fact]
    public async Task Following_a_link_resolves_its_target_so_the_node_carries_the_served_label_not_the_links()
    {
        // Arrange
        var graph = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Container, Genesis2Id, ServedGenesis2Label));
        var toGenesis2 = new Link(EdgeKind.FollowsIn, ServedGraph.Ref(NodeKind.Container, Genesis2Id, LegacyGenesis2Label));

        // Act
        var genesis2 = await new GraphExplorer(graph).Follow(toGenesis2);

        // Assert
        Assert.Equal((NodeKind.Container, Genesis2Id, ServedGenesis2Label), (genesis2.Kind, genesis2.Id, genesis2.Label));
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
}
