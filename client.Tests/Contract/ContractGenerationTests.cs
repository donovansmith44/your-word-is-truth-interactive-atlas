using BibleAtlas.Client.ContractGenerator;
using NJsonSchema;
using NJsonSchema.CodeGeneration.CSharp;
using NSwag;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class ContractGenerationTests
{
    [Fact]
    public void Settings_configures_the_generator_for_the_clients_wire_shapes_and_naming_rule()
    {
        // Arrange
        var document = new OpenApiDocument();

        // Act
        var settings = ContractGeneration.Settings(document);
        var csharp = settings.CSharpGeneratorSettings;

        // Assert
        Assert.False(settings.GenerateClientClasses);
        Assert.True(settings.GenerateDtoTypes);
        Assert.Equal("BibleAtlas.Client.Contract", csharp.Namespace);
        Assert.Equal(CSharpClassStyle.Record, csharp.ClassStyle);
        Assert.True(csharp.GenerateNativeRecords);
        Assert.Equal(CSharpJsonLibrary.SystemTextJson, csharp.JsonLibrary);
        Assert.Equal(9.0m, csharp.JsonLibraryVersion);
        Assert.Equal("System.Collections.Generic.IReadOnlyList", csharp.ArrayType);
        Assert.Equal("System.Collections.Generic.List", csharp.ArrayInstanceType);
        Assert.True(csharp.InlineNamedArrays);
        Assert.True(csharp.GenerateNullableReferenceTypes);
        Assert.True(csharp.GenerateOptionalPropertiesAsNullable);
        Assert.False(csharp.GenerateDataAnnotations);
        Assert.True(csharp.GenerateDefaultValues);
        Assert.False(csharp.GenerateJsonMethods);
        Assert.IsType<PascalCasePropertyNames>(csharp.PropertyNameGenerator);
        Assert.Equal(ContractGeneration.Unread, csharp.ExcludedTypeNames);
    }

    [Fact]
    public void A_discriminated_union_and_its_subtypes_own_parts_are_generated_closed_and_nothing_else_is_touched()
    {
        // Arrange
        var union = new JsonSchema { Type = JsonObjectType.Object, AllowAdditionalProperties = true, DiscriminatorObject = new OpenApiDiscriminator { PropertyName = "corpus" } };
        var subtypesOwnPart = new JsonSchema { Type = JsonObjectType.Object, AllowAdditionalProperties = true };
        var subtype = new JsonSchema();
        subtype.AllOf.Add(new JsonSchema { Reference = union });
        subtype.AllOf.Add(subtypesOwnPart);
        union.DiscriminatorObject.Mapping["bible"] = subtype;
        var unrelated = new JsonSchema { Type = JsonObjectType.Object, AllowAdditionalProperties = true };
        var document = new OpenApiDocument();
        document.Definitions["TextRef"] = union;
        document.Definitions["BibleRef"] = subtype;
        document.Definitions["Unrelated"] = unrelated;

        // Act
        ContractGeneration.CloseDiscriminatedUnions(document);

        // Assert
        Assert.Equal([false, false, true], new[] { union, subtypesOwnPart, unrelated }.Select(schema => schema.AllowAdditionalProperties));
    }
}
