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

    // The client never sends /api/text's optional `scope`, and nothing $refs TextScope.
    public static readonly IReadOnlySet<string> Unread = new HashSet<string> { "TextScope" };
}
