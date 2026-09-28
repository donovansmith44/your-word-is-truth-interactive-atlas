using NJsonSchema;
using NJsonSchema.CodeGeneration;

namespace BibleAtlas.Client.ContractGenerator;

public sealed class PascalCasePropertyNames : IPropertyNameGenerator
{
    private static readonly char[] WordSeparators = ['_', '-'];

    private readonly IReadOnlyDictionary<JsonSchema, string> declaringRecordNames;

    public PascalCasePropertyNames(IDictionary<string, JsonSchema> definitions) =>
        declaringRecordNames = definitions.ToDictionary(
            entry => entry.Value,
            entry => entry.Key,
            (IEqualityComparer<JsonSchema>)ReferenceEqualityComparer.Instance);

    public string Generate(JsonSchemaProperty property)
    {
        var pascalName = string.Concat(property.Name.Split(WordSeparators, StringSplitOptions.RemoveEmptyEntries).Select(Capitalised));
        return NameForItsSchema(property, pascalName) ?? pascalName;
    }

    private string? NameForItsSchema(JsonSchemaProperty property, string pascalName)
    {
        if (property.ParentSchema is not { } declaringRecord
            || !declaringRecordNames.TryGetValue(declaringRecord, out var recordName)
            || recordName != pascalName)
        {
            return null;
        }

        if (property.Type.HasFlag(JsonObjectType.Integer))
        {
            return "Number";
        }

        if (property.IsArray)
        {
            return "All";
        }

        throw new InvalidOperationException(
            $"Record '{recordName}' has a property '{property.Name}' whose generated name '{pascalName}' " +
            $"collides with the record's own name (schema type {property.Type}); C# forbids a member named " +
            "like its type, and no naming rule covers this schema shape. Decide a name for it in PascalCasePropertyNames.");
    }

    private static string Capitalised(string word) => char.ToUpperInvariant(word[0]) + word[1..];
}
