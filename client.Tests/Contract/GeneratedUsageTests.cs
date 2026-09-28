using System.Text.Json;
using System.Text.RegularExpressions;
using BibleAtlas.Client.ContractGenerator;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class GeneratedUsageTests
{
    private static readonly string RepoRoot = Path.GetFullPath(Path.Combine(ConformanceTests.ClientRoot, ".."));

    [Fact]
    public void The_declared_unread_set_matches_every_type_no_reader_reaches()
    {
        // Arrange
        var schemas = ContractSchemas();
        var generatedTypeNames = schemas.Keys.Where(name => !IsInlinedArray(schemas[name])).ToHashSet();
        // The pact harness deserialises into generated types, so it is a reader too.
        var roots = generatedTypeNames
            .Where(name => NamesType(name, ConformanceTests.SourceUnder("client")) || NamesType(name, ConformanceTests.SourceUnder("client.ContractTests")))
            .ToHashSet();
        var computedUnread = generatedTypeNames.Except(ReachableFrom(roots, schemas)).ToHashSet();

        // Act
        var declaredUnread = ContractGeneration.Unread;

        // Assert
        Assert.Equal(computedUnread, declaredUnread);
    }

    private static Dictionary<string, JsonElement> ContractSchemas()
    {
        using var document = JsonDocument.Parse(File.ReadAllText(Path.Combine(RepoRoot, "contracts", "atlas-query-contract", "aqc.schema.json")));
        return document.RootElement.GetProperty("$defs").EnumerateObject().ToDictionary(p => p.Name, p => p.Value.Clone());
    }

    // An array-shaped schema is inlined by the generator and never becomes a type.
    private static bool IsInlinedArray(JsonElement schema) =>
        schema.TryGetProperty("type", out var type) && type.ValueKind == JsonValueKind.String && type.GetString() == "array";

    private static bool NamesType(string typeName, string source)
    {
        if (typeName != "Contract")
        {
            return Regex.IsMatch(source, $@"\b{Regex.Escape(typeName)}\b");
        }

        // `Contract` is also a namespace segment; only an undotted token or `Contract.Contract` names the type.
        if (Regex.IsMatch(source, @"\bContract\.Contract\b"))
        {
            return true;
        }

        return Regex.Matches(source, @"\bContract\b")
            .Any(m => (m.Index == 0 || source[m.Index - 1] != '.')
                      && (m.Index + m.Length == source.Length || source[m.Index + m.Length] != '.'));
    }

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
