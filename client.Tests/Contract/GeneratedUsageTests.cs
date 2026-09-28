using System.Text.Json;
using System.Text.RegularExpressions;
using BibleAtlas.Client.ContractGenerator;
using BibleAtlas.Client.Tests.State;
using NSwag;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class GeneratedUsageTests
{
    private static readonly string[] ProjectsThatReadGeneratedTypes = ["client", "client.ContractTests"];

    [Fact]
    public void The_declared_unread_set_matches_every_type_no_reader_reaches()
    {
        // Arrange
        var schemas = ContractSchemas();
        var generatedTypeNames = schemas.Keys.Where(name => !IsInlinedArray(schemas[name])).ToHashSet();
        var readerSources = ProjectsThatReadGeneratedTypes.Select(ConformanceTests.SourceUnder).ToList();
        var roots = generatedTypeNames.Where(name => readerSources.Any(source => NamesType(name, source))).ToHashSet();
        var computedUnread = generatedTypeNames.Except(ReachableFrom(roots, schemas)).ToHashSet();
        // Act
        var declaredUnread = ContractGeneration.Unread;
        // Assert
        Assert.Equal(computedUnread, declaredUnread);
    }

    private static Dictionary<string, JsonElement> ContractSchemas()
    {
        using var document = JsonDocument.Parse(File.ReadAllText(Path.Combine(ConformanceTests.RepoRoot(), "contracts", "atlas-query-contract", "aqc.schema.json")));
        return document.RootElement.GetProperty("$defs").EnumerateObject().ToDictionary(p => p.Name, p => p.Value.Clone());
    }

    private static bool IsInlinedArray(JsonElement schema) =>
        GeneratorInlinesNamedArrays && IsArraySchema(schema);

    private static readonly bool GeneratorInlinesNamedArrays =
        ContractGeneration.Settings(new OpenApiDocument()).CSharpGeneratorSettings.InlineNamedArrays;

    private static bool IsArraySchema(JsonElement schema) =>
        schema.TryGetProperty("type", out var type) && type.ValueKind == JsonValueKind.String && type.GetString() == "array";

    private static bool NamesType(string typeName, string source) =>
        typeName == ContractTypeSharingItsNamespaceName
            ? NamesTheContractTypeRatherThanItsNamespace(source)
            : Regex.IsMatch(source, $@"\b{Regex.Escape(typeName)}\b");

    private const string ContractTypeSharingItsNamespaceName = "Contract";

    private static bool NamesTheContractTypeRatherThanItsNamespace(string source) =>
        Regex.IsMatch(source, @"\bContract\.Contract\b")
        || Regex.Matches(source, @"\bContract\b")
            .Any(m => (m.Index == 0 || source[m.Index - 1] != '.')
                      && (m.Index + m.Length == source.Length || source[m.Index + m.Length] != '.'));

    private static HashSet<string> ReachableFrom(IReadOnlySet<string> roots, IReadOnlyDictionary<string, JsonElement> schemas)
    {
        var reachable = new HashSet<string>(roots);
        var frontier = new Queue<string>(roots);
        while (frontier.TryDequeue(out var name))
        {
            foreach (var next in DirectRefs(schemas[name]))
            {
                if (reachable.Add(next))
                {
                    frontier.Enqueue(next);
                }
            }
        }

        return reachable;
    }

    private const string DefinitionRefPrefix = "#/$defs/";

    private static IEnumerable<string> DirectRefs(JsonElement schema)
    {
        switch (schema.ValueKind)
        {
            case JsonValueKind.Object:
                foreach (var property in schema.EnumerateObject())
                {
                    if (property.Name == "$ref" && property.Value.ValueKind == JsonValueKind.String
                        && property.Value.GetString() is { } target && target.StartsWith(DefinitionRefPrefix, StringComparison.Ordinal))
                    {
                        yield return target[DefinitionRefPrefix.Length..];
                    }
                    else
                    {
                        foreach (var nested in DirectRefs(property.Value))
                        {
                            yield return nested;
                        }
                    }
                }

                break;
            case JsonValueKind.Array:
                foreach (var item in schema.EnumerateArray())
                {
                    foreach (var nested in DirectRefs(item))
                    {
                        yield return nested;
                    }
                }

                break;
        }
    }
}
