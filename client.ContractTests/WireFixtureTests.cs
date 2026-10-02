using System.Text.Json;
using System.Text.Json.Nodes;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.ContractTests.Steps;

namespace BibleAtlas.Client.ContractTests;

public sealed class WireFixtureTests
{
    [Theory]
    [InlineData(GraphContract, "node-place-hazor-1.json", typeof(NodeCard))]
    [InlineData(GraphContract, "node-event-ab-ur.json", typeof(NodeCard))]
    [InlineData(GraphContract, "edges-hazor-1-site-of.json", typeof(EdgePage))]
    [InlineData(QueryContract, "scene-time.json", typeof(Scene))]
    [InlineData(QueryContract, "scene-scripture.json", typeof(Scene))]
    public void A_recorded_server_body_round_trips_through_the_generated_record_unchanged(string contract, string fixture, Type record)
    {
        // Arrange
        var json = FixtureBody(contract, fixture);
        var expected = Canonical(JsonDocument.Parse(json).RootElement);
        // Act
        var actual = Canonical(JsonSerializer.SerializeToElement(JsonSerializer.Deserialize(json, record), record, WithoutNulls));
        // Assert
        Assert.Equal(expected, actual);
    }

    [Fact]
    public void The_site_of_page_for_hazor_lists_events_under_a_typed_kind()
    {
        // Arrange
        var json = FixtureBody(GraphContract, "edges-hazor-1-site-of.json");
        // Act
        var page = JsonSerializer.Deserialize<EdgePage>(json)!;
        var kinds = (page.Kind, page.Entries.Nodes().Select(n => n.Kind).Distinct().ToArray());
        // Assert
        Assert.Equivalent((EdgeKind.SiteOf, new[] { NodeKind.Event }), kinds);
    }

    private const string GraphContract = "atlas-graph-contract";
    private const string QueryContract = "atlas-query-contract";

    private static readonly JsonSerializerOptions WithoutNulls = new() { DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull };

    private static string FixtureBody(string contract, string name)
    {
        var text = File.ReadAllText(Path.Combine(AqcSteps.RepoRoot, "contracts", contract, "fixtures", name));
        var root = JsonDocument.Parse(text).RootElement;
        return root.TryGetProperty("body", out var body) ? body.GetRawText() : text;
    }

    private static string Canonical(JsonElement element) => SameValueWhateverTheKeyOrderOrNumberSpelling(element)?.ToJsonString() ?? "null";

    private static JsonNode? SameValueWhateverTheKeyOrderOrNumberSpelling(JsonElement element) => element.ValueKind switch
    {
        JsonValueKind.Object => new JsonObject(element.EnumerateObject().OrderBy(p => p.Name, StringComparer.Ordinal).Select(p => KeyValuePair.Create(p.Name, SameValueWhateverTheKeyOrderOrNumberSpelling(p.Value)))),
        JsonValueKind.Array => new JsonArray(element.EnumerateArray().Select(SameValueWhateverTheKeyOrderOrNumberSpelling).ToArray()),
        JsonValueKind.Number => JsonValue.Create(element.GetDouble()),
        _ => JsonNode.Parse(element.GetRawText()),
    };
}
