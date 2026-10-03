using NJsonSchema;
using NSwag;

namespace BibleAtlas.Client.ContractGenerator;

public static class IdentityTypes
{
    public static IReadOnlyDictionary<JsonSchema, string> Of(OpenApiDocument document)
    {
        var leaves = document.Definitions.Where(definition => IsLeaf(definition.Value) && !ContractGeneration.Unread.Contains(definition.Key)).ToDictionary(definition => definition.Value, definition => definition.Key, (IEqualityComparer<JsonSchema>)ReferenceEqualityComparer.Instance);
        var unions = document.Definitions.Where(definition => IsUnionOf(definition.Value, leaves) && !ContractGeneration.Unread.Contains(definition.Key)).ToDictionary(definition => definition.Value, definition => definition.Key, (IEqualityComparer<JsonSchema>)ReferenceEqualityComparer.Instance);
        return leaves.Concat(unions).ToDictionary(identity => identity.Key, identity => identity.Value, (IEqualityComparer<JsonSchema>)ReferenceEqualityComparer.Instance);
    }

    public static string Emit(OpenApiDocument document)
    {
        var identities = Of(document);
        var records = identities
            .OrderBy(identity => identity.Value, StringComparer.Ordinal)
            .Select(identity => Record(identity.Value, identity.Key, document, identities));
        return "\nnamespace BibleAtlas.Client.Contract\n{\n" + string.Concat(records) + "}\n";
    }

    public static IReadOnlySet<(string From, string To)> Widenings(OpenApiDocument document)
    {
        var identities = Of(document);
        var unions = identities.Where(identity => identity.Key.OneOf.Count > 0).ToDictionary(identity => identity.Value, identity => Members(identity.Key, document));
        var intoUnions = unions.SelectMany(union => union.Value.Select(member => (From: member, To: union.Key)));
        var betweenUnions = unions.SelectMany(narrower => unions.Where(wider => wider.Key != narrower.Key && narrower.Value.IsSubsetOf(wider.Value)).Select(wider => (From: narrower.Key, To: wider.Key)));
        return intoUnions.Concat(betweenUnions).ToHashSet();
    }

    private static bool IsLeaf(JsonSchema schema) =>
        (schema.Type.HasFlag(JsonObjectType.String) || schema.Type.HasFlag(JsonObjectType.Integer))
        && !schema.IsEnumeration
        && schema.Properties.Count == 0
        && schema.OneOf.Count == 0;

    private static bool IsUnionOf(JsonSchema schema, IReadOnlyDictionary<JsonSchema, string> leaves) =>
        schema.OneOf.Count > 0
        && schema.Type == JsonObjectType.None
        && schema.OneOf.All(member => member.HasReference && (leaves.ContainsKey(member.ActualSchema) || member.ActualSchema.IsEnumeration));

    private static HashSet<string> Members(JsonSchema union, OpenApiDocument document) =>
        union.OneOf.Select(member => NameOf(member.ActualSchema, document)).ToHashSet();

    private static string NameOf(JsonSchema schema, OpenApiDocument document) =>
        document.Definitions.Single(definition => ReferenceEquals(definition.Value, schema)).Key;

    private static string Record(string name, JsonSchema schema, OpenApiDocument document, IReadOnlyDictionary<JsonSchema, string> identities)
    {
        var integer = schema.Type.HasFlag(JsonObjectType.Integer);
        var primitive = integer ? "int" : "string";
        var read = integer ? "reader.TokenType == System.Text.Json.JsonTokenType.Number ? new " + name + "(reader.GetInt32())" : "reader.TokenType == System.Text.Json.JsonTokenType.String ? new " + name + "(reader.GetString()!)";
        var token = integer ? "number" : "string";
        var text = integer ? "Value.ToString(System.Globalization.CultureInfo.InvariantCulture)" : "Value";
        var write = integer ? "writer.WriteNumberValue(value.Value)" : "writer.WriteStringValue(value.Value)";
        var first = schema.Default is { } start ? $"\n        public static {name} First {{ get; }} = new({start});\n" : "";
        var widenings = Widenings(document).Where(widening => widening.To == name).OrderBy(widening => widening.From, StringComparer.Ordinal).Select(widening => Widening(widening.From, name, document, identities));
        return $$"""
                [System.Text.Json.Serialization.JsonConverter(typeof({{name}}.Json))]
                public sealed record {{name}}
                {
                    private {{name}}({{primitive}} value) => Value = value;

                    public {{primitive}} Value { get; }

                    public override string ToString() => {{text}};
            {{first}}{{string.Concat(widenings)}}
                    public sealed class Json : System.Text.Json.Serialization.JsonConverter<{{name}}>
                    {
                        public override {{name}} Read(ref System.Text.Json.Utf8JsonReader reader, System.Type typeToConvert, System.Text.Json.JsonSerializerOptions options) =>
                            {{read}}
                                : throw new System.Text.Json.JsonException("{{name}} is written as a JSON {{token}}");

                        public override void Write(System.Text.Json.Utf8JsonWriter writer, {{name}} value, System.Text.Json.JsonSerializerOptions options) =>
                            {{write}};
                    }
                }

            """;
    }

    private static string Widening(string from, string to, OpenApiDocument document, IReadOnlyDictionary<JsonSchema, string> identities)
    {
        var member = document.Definitions[from];
        var wire = identities.ContainsKey(member)
            ? "member.Value"
            : $"System.Text.Json.JsonSerializer.SerializeToElement(member, new System.Text.Json.JsonSerializerOptions {{ Converters = {{ new System.Text.Json.Serialization.JsonStringEnumConverter<{from}>() }} }}).GetString()!";
        return $"""

                    public static implicit operator {to}({from} member) => new({wire});

            """;
    }
}
