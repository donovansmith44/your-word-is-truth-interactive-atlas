using System.Text.Json;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Tests.State;
using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class EdgeKindsTests
{
    [Fact]
    public void Every_generated_kind_has_a_dual()
    {
        // Arrange
        var kinds = Enum.GetValues<EdgeKind>();
        // Act
        var failure = Record.Exception(() => kinds.Select(k => k.Dual()).ToList());
        // Assert
        Assert.Null(failure);
    }

    [Fact]
    public void Dual_table_matches_x_atlas_relations()
    {
        // Arrange
        var relations = PublishedRelations();
        var expected = relations.Directed
            .SelectMany(r => new[]
            {
                KeyValuePair.Create(WireNames.Parse<EdgeKind>(r.Forward), WireNames.Parse<EdgeKind>(r.Inverse)),
                KeyValuePair.Create(WireNames.Parse<EdgeKind>(r.Inverse), WireNames.Parse<EdgeKind>(r.Forward)),
            })
            .Concat(relations.Symmetric.Select(s => KeyValuePair.Create(WireNames.Parse<EdgeKind>(s.Label), WireNames.Parse<EdgeKind>(s.Label))))
            .ToDictionary();
        // Act
        var duals = Enum.GetValues<EdgeKind>().ToDictionary(k => k, k => k.Dual());
        // Assert
        Assert.Equal(expected, duals);
    }

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
    public void The_symmetric_kinds_are_exactly_the_vocabularys_symmetric_relations()
    {
        // Arrange
        var declared = VocabularySymmetricLabels().Select(WireNames.Parse<EdgeKind>).ToHashSet();
        // Act
        var symmetric = Enum.GetValues<EdgeKind>().Where(k => k.IsSymmetric()).ToHashSet();
        // Assert
        Assert.Equal(declared, symmetric);
    }

    [Fact]
    public void Every_kind_is_shown_by_the_label_x_atlas_relations_serves_for_it()
    {
        // Arrange
        var served = PublishedRelations().Kinds.ToDictionary(k => WireNames.Parse<EdgeKind>(k.Kind), k => k.Label);
        // Act
        var labels = Enum.GetValues<EdgeKind>().ToDictionary(k => k, k => k.Label());
        // Assert
        Assert.Equal(served, labels);
    }

    private static Relations PublishedRelations() =>
        new DeserializerBuilder()
            .WithNamingConvention(CamelCaseNamingConvention.Instance)
            .IgnoreUnmatchedProperties()
            .Build()
            .Deserialize<PublishedDocument>(File.ReadAllText(Path.Combine(ConformanceTests.RepoRoot(), "contracts", "openapi.yaml")))
            .Relations;

    private static IEnumerable<string> VocabularySymmetricLabels()
    {
        var path = Path.Combine(ConformanceTests.RepoRoot(), "contracts", "atlas-graph-contract", "fixtures", "graph-vocabulary.json");
        using var vocabulary = JsonDocument.Parse(File.ReadAllText(path));
        return vocabulary.RootElement.GetProperty("symmetric").EnumerateArray().Select(s => s.GetProperty("label").GetString()!).ToList();
    }

    private sealed class PublishedDocument
    {
        [YamlMember(Alias = "x-atlas-relations", ApplyNamingConventions = false)]
        public Relations Relations { get; set; } = new();
    }

    private sealed class Relations
    {
        public List<DirectedRelation> Directed { get; set; } = [];

        public List<SymmetricRelation> Symmetric { get; set; } = [];

        public List<ServedKind> Kinds { get; set; } = [];
    }

    private sealed class ServedKind
    {
        public string Kind { get; set; } = "";

        public string Label { get; set; } = "";
    }

    private sealed class DirectedRelation
    {
        public string Forward { get; set; } = "";

        public string Inverse { get; set; } = "";
    }

    private sealed class SymmetricRelation
    {
        public string Label { get; set; } = "";
    }
}
