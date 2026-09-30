using NJsonSchema.CodeGeneration.CSharp;
using NSwag;
using NSwag.CodeGeneration.CSharp;

namespace BibleAtlas.Client.ContractGenerator;

public static class ContractGeneration
{
    public static CSharpClientGeneratorSettings Settings(OpenApiDocument document) => new()
    {
        GenerateClientClasses = false,
        GenerateDtoTypes = true,
        CSharpGeneratorSettings =
        {
            Namespace = "BibleAtlas.Client.Contract",
            ClassStyle = CSharpClassStyle.Record,
            GenerateNativeRecords = true,
            JsonLibrary = CSharpJsonLibrary.SystemTextJson,
            JsonLibraryVersion = 9.0m,
            ArrayType = "System.Collections.Generic.IReadOnlyList",
            ArrayInstanceType = "System.Collections.Generic.List",
            InlineNamedArrays = true,
            GenerateNullableReferenceTypes = true,
            GenerateOptionalPropertiesAsNullable = true,
            GenerateDataAnnotations = false,
            GenerateDefaultValues = true,
            GenerateJsonMethods = false,
            PropertyNameGenerator = new PascalCasePropertyNames(document.Definitions),
            ExcludedTypeNames = Unread.ToArray(),
        },
    };

    public static readonly IReadOnlySet<string> Unread = new HashSet<string> { "Contract" };

    // A discriminator base is open in the document only so that JSON Schema lets its allOf
    // subtypes' own properties through, and each subtype is closed there by
    // `unevaluatedProperties`, which the generator does not read. In C# a subtype carries its
    // properties as typed members, so a property bag on either would only ever catch the
    // discriminator itself -- breaking record equality (a bag compares by reference) and
    // writing the discriminator twice.
    public static void CloseDiscriminatedUnions(OpenApiDocument document)
    {
        foreach (var union in document.Definitions.Values)
        {
            if (union.DiscriminatorObject is not { } discriminator)
            {
                continue;
            }

            union.AllowAdditionalProperties = false;
            foreach (var part in discriminator.Mapping.Values.SelectMany(subtype => subtype.ActualSchema.AllOf))
            {
                part.AllowAdditionalProperties = false;
            }
        }
    }
}
