using System.Text;
using System.Text.Json;
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

    public const string RelationsExtension = "x-atlas-relations";
    public const string EdgeKindSchema = "EdgeKind";

    public static readonly IReadOnlySet<string> Unread = new HashSet<string>();

    public static string EdgeKindLabels(OpenApiDocument document)
    {
        var relations = (IDictionary<string, object?>)document.ExtensionData![RelationsExtension]!;
        var names = Settings(document).CSharpGeneratorSettings.EnumNameGenerator;
        var edgeKind = document.Definitions[EdgeKindSchema];
        var arms = new StringBuilder();
        foreach (var (served, index) in ((object[])relations["kinds"]!).Cast<IDictionary<string, object?>>().Select((served, index) => (served, index)))
        {
            var kind = (string)served["kind"]!;
            arms.Append($"            {EdgeKindSchema}.{names.Generate(index, kind, kind, edgeKind)} => {JsonSerializer.Serialize((string)served["label"]!)},\n");
        }

        return $$"""

            namespace BibleAtlas.Client.Contract
            {
                public static class EdgeKindLabels
                {
                    public static string Label(this {{EdgeKindSchema}} kind) => kind switch
                    {
            {{arms}}        };
                }
            }

            """;
    }

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
