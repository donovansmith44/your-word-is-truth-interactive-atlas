using System.Text.Json;
using System.Text.Json.Nodes;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.ContractTests.Steps;
using NJsonSchema;
using NSwag;

namespace BibleAtlas.Client.ContractTests;

public sealed class WireFixtureTests
{
    [Theory]
    [MemberData(nameof(RecordedBodies))]
    public void A_recorded_server_body_round_trips_through_the_generated_record_unchanged(string recorded, string json, Type record)
    {
        // Arrange
        var expected = Canonical(JsonDocument.Parse(json).RootElement);
        // Act
        var actual = Canonical(JsonSerializer.SerializeToElement(JsonSerializer.Deserialize(json, record), record, WithoutNulls));
        // Assert
        Assert.Equal((recorded, expected), (recorded, actual));
    }

    public static TheoryData<string, string, Type> RecordedBodies()
    {
        var document = OpenApiYamlDocument.FromFileAsync(Path.Combine(AqcSteps.RepoRoot, "contracts", "openapi.yaml")).GetAwaiter().GetResult();
        var pact = JsonDocument.Parse(File.ReadAllText(Path.Combine(AqcSteps.RepoRoot, "contracts", "pacts", "http.json"))).RootElement.GetProperty("entries");
        var pacted = pact.EnumerateObject()
            .Where(entry => entry.Name.StartsWith(Get, StringComparison.Ordinal) && entry.Value.GetProperty("status").GetInt32() == Ok)
            .Select(entry => (Recorded: entry.Name, Uri: entry.Name[Get.Length..], Body: entry.Value.GetProperty("body").GetRawText()));
        var index = JsonDocument.Parse(File.ReadAllText(Path.Combine(AqcSteps.RepoRoot, "contracts", QueryContract, "fixtures", "index.json"))).RootElement;
        var focused = index.EnumerateObject().Select(focus => (Recorded: focus.Value.GetString()!, Uri: $"/api/node/{Uri.EscapeDataString(focus.Name)}", Body: FixtureBody(QueryContract, $"{focus.Value.GetString()}.json")));
        var queried = Directory.GetFiles(Path.Combine(AqcSteps.RepoRoot, "contracts", QueryContract, "features"), "*.feature")
            .SelectMany(feature => System.Text.RegularExpressions.Regex.Matches(File.ReadAllText(feature), QueryStep).Select(step => step.Groups[1].Value))
            .Select(uri => (Fixture: AqcSteps.QueriedFixture(uri), Uri: uri))
            .Where(query => FixtureStatus(QueryContract, $"{query.Fixture}.json") == Ok)
            .Select(query => (Recorded: query.Fixture, query.Uri, Body: FixtureBody(QueryContract, $"{query.Fixture}.json")));
        var cases = new TheoryData<string, string, Type>();
        foreach (var (recorded, uri, body) in pacted.Concat(focused).Concat(queried).DistinctBy(recorded => recorded.Recorded))
        {
            cases.Add(recorded, body, ServedAs(document, uri));
        }

        return cases;
    }

    private const string Get = "GET ";
    private const string QueryStep = "I query \"([^\"]+)\"";
    private const int Ok = 200;

    private static Type ServedAs(OpenApiDocument document, string uri)
    {
        var path = uri.Split('?')[0];
        var operation = document.Paths.Single(served => Answers(served.Key, path)).Value.Single().Value;
        var schema = operation.ActualResponses["200"].Schema!.ActualSchema;
        return schema.Type.HasFlag(JsonObjectType.Array)
            ? typeof(List<>).MakeGenericType(Generated(document, schema.Item!.ActualSchema))
            : Generated(document, schema);
    }

    private static bool Answers(string template, string path)
    {
        var wanted = template.Split('/');
        var asked = path.Split('/');
        return wanted.Length == asked.Length && wanted.Zip(asked).All(segment => segment.First.StartsWith('{') || segment.First == segment.Second);
    }

    private static Type Generated(OpenApiDocument document, JsonSchema schema) =>
        typeof(NodeRecord).Assembly.GetType($"{typeof(NodeRecord).Namespace}.{document.Definitions.Single(definition => ReferenceEquals(definition.Value, schema)).Key}")!;

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

    private static int FixtureStatus(string contract, string name) =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AqcSteps.RepoRoot, "contracts", contract, "fixtures", name))).RootElement.GetProperty("status").GetInt32();

    private static string FixtureBody(string contract, string name)
    {
        var text = File.ReadAllText(Path.Combine(AqcSteps.RepoRoot, "contracts", contract, "fixtures", name));
        var root = JsonDocument.Parse(text).RootElement;
        return root.TryGetProperty("body", out var body) ? body.GetRawText() : text;
    }

    private static string Canonical(JsonElement element) => SameValueWhateverTheKeyOrderOrNumberSpellingOrAbsentNull(element)?.ToJsonString() ?? "null";

    private static JsonNode? SameValueWhateverTheKeyOrderOrNumberSpellingOrAbsentNull(JsonElement element) => element.ValueKind switch
    {
        JsonValueKind.Object => new JsonObject(element.EnumerateObject().Where(p => p.Value.ValueKind != JsonValueKind.Null).OrderBy(p => p.Name, StringComparer.Ordinal).Select(p => KeyValuePair.Create(p.Name, SameValueWhateverTheKeyOrderOrNumberSpellingOrAbsentNull(p.Value)))),
        JsonValueKind.Array => new JsonArray(element.EnumerateArray().Select(SameValueWhateverTheKeyOrderOrNumberSpellingOrAbsentNull).ToArray()),
        JsonValueKind.Number => JsonValue.Create(element.GetDouble()),
        _ => JsonNode.Parse(element.GetRawText()),
    };
}
