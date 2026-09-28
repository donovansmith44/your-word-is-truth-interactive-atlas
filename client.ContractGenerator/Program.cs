using BibleAtlas.Client.ContractGenerator;
using NJsonSchema.CodeGeneration.CSharp;
using NSwag;
using NSwag.CodeGeneration.CSharp;

var (contract, output) = (args[0], args[1]);

var settings = new CSharpClientGeneratorSettings
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
        PropertyNameGenerator = new PascalCasePropertyNames(),
    },
};

var document = await OpenApiYamlDocument.FromFileAsync(contract);
Directory.CreateDirectory(Path.GetDirectoryName(output)!);
await File.WriteAllTextAsync(output, new CSharpClientGenerator(document, settings).GenerateFile());
