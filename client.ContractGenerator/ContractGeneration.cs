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

    public static string PagedReads(OpenApiDocument document)
    {
        var defaults = document.Paths
            .SelectMany(path => path.Value.Select(operation => (Operation: $"{operation.Key} {path.Key}", operation.Value)))
            .SelectMany(operation => operation.Value.ActualParameters
                .Where(parameter => parameter.Name == "cursor" && parameter.Kind == OpenApiParameterKind.Query)
                .Select(cursor => (operation.Operation, Default: (cursor.Schema ?? cursor).Default)))
            .ToList();
        var published = defaults.Select(cursor => cursor.Default?.ToString()).Distinct().ToList();
        return published is [{ } first] && int.TryParse(first, out var firstPage)
            ? $$"""

                namespace BibleAtlas.Client.Contract
                {
                    public static class PagedReads
                    {
                        public const int FirstPage = {{firstPage}};
                    }
                }

                """
            : throw new InvalidOperationException(
                $"every paged read must publish one cursor default, and the document publishes {string.Join(", ", defaults.Select(cursor => $"{cursor.Operation}: {cursor.Default ?? "none"}"))}");
    }

    public static readonly IReadOnlySet<string> Unread = new HashSet<string> { "BookMeta", "VerseDetail", "VerseEvent" };

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
