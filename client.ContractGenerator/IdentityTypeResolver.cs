using NJsonSchema;
using NJsonSchema.CodeGeneration.CSharp;
using NSwag;

namespace BibleAtlas.Client.ContractGenerator;

public sealed class IdentityTypeResolver(CSharpGeneratorSettings settings, IReadOnlyDictionary<JsonSchema, string> identities) : CSharpTypeResolver(settings)
{
    public static IdentityTypeResolver For(OpenApiDocument document, CSharpGeneratorSettings settings)
    {
        var identities = IdentityTypes.Of(document);
        var resolver = new IdentityTypeResolver(settings, identities);
        resolver.RegisterSchemaDefinitions(document.Definitions.Where(definition => !identities.ContainsKey(definition.Value) && !ContractGeneration.Unread.Contains(definition.Key)).ToDictionary());
        return resolver;
    }

    public override string Resolve(JsonSchema schema, bool isNullable, string? typeNameHint) =>
        IdentityOf(schema) is { } identity ? (isNullable ? identity + "?" : identity) : base.Resolve(schema, isNullable, typeNameHint);

    private string? IdentityOf(JsonSchema schema)
    {
        var named = schema.OneOf.Where(member => member.Type != JsonObjectType.Null).ToList() is [{ } only] && schema.OneOf.Count > 1 ? only : schema;
        return identities.TryGetValue(named.ActualSchema, out var identity) ? identity : null;
    }
}
