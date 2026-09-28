using System.Text.Json;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.ContractTests.Steps;

namespace BibleAtlas.Client.ContractTests;

public sealed class WireFixtureTests
{
    [Theory]
    [InlineData("node-place-hazor-1.json", typeof(NodeCard))]
    [InlineData("node-event-ab-ur.json", typeof(NodeCard))]
    [InlineData("edges-hazor-1-site-of.json", typeof(EdgePage))]
    public void A_recorded_server_body_round_trips_through_the_generated_record_unchanged(string fixture, Type record)
    {
        // Arrange
        var json = FixtureBody(fixture);
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
        var json = FixtureBody("edges-hazor-1-site-of.json");
        // Act
        var page = JsonSerializer.Deserialize<EdgePage>(json)!;
        var kinds = (page.Kind, page.Entries.Select(e => e.Node.Kind).Distinct().ToArray());
        // Assert
        Assert.Equivalent((EdgeKind.SiteOf, new[] { PositionKind.Event }), kinds);
    }

    private static readonly JsonSerializerOptions WithoutNulls = new() { DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull };

    private static string FixtureBody(string name)
    {
        var text = File.ReadAllText(Path.Combine(AqcSteps.RepoRoot, "contracts", "atlas-graph-contract", "fixtures", name));
        var root = JsonDocument.Parse(text).RootElement;
        return root.TryGetProperty("body", out var body) ? body.GetRawText() : text;
    }

    private static string Canonical(JsonElement element) => JsonSerializer.Serialize(element);
}
