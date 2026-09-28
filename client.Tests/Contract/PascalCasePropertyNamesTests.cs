using BibleAtlas.Client.ContractGenerator;
using NJsonSchema;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class PascalCasePropertyNamesTests
{
    [Fact]
    public void Snake_and_kebab_wire_names_become_PascalCase_property_names()
    {
        // Arrange
        var record = new JsonSchema { Type = JsonObjectType.Object };
        record.Properties["name"] = new JsonSchemaProperty { Type = JsonObjectType.String };
        record.Properties["edge_summary"] = new JsonSchemaProperty { Type = JsonObjectType.String };
        record.Properties["birth_year"] = new JsonSchemaProperty { Type = JsonObjectType.Integer };
        record.Properties["also_called"] = new JsonSchemaProperty { Type = JsonObjectType.Array };
        record.Properties["member-of"] = new JsonSchemaProperty { Type = JsonObjectType.String };
        var names = new PascalCasePropertyNames(new Dictionary<string, JsonSchema> { ["Someone"] = record });

        // Act
        var generated = record.Properties.Values.Select(names.Generate).ToArray();

        // Assert
        Assert.Equal(["Name", "EdgeSummary", "BirthYear", "AlsoCalled", "MemberOf"], generated);
    }

    [Theory]
    [InlineData("Verse", "verse")]
    [InlineData("Chapter", "chapter")]
    public void An_integer_property_named_like_its_record_is_named_Number(string recordName, string propertyName)
    {
        // Arrange
        var record = new JsonSchema { Type = JsonObjectType.Object };
        record.Properties[propertyName] = new JsonSchemaProperty { Type = JsonObjectType.Integer };
        var names = new PascalCasePropertyNames(new Dictionary<string, JsonSchema> { [recordName] = record });

        // Act
        var generated = names.Generate(record.Properties[propertyName]);

        // Assert
        Assert.Equal("Number", generated);
    }

    [Fact]
    public void An_array_property_named_like_its_record_is_named_All()
    {
        // Arrange
        var polities = new JsonSchema { Type = JsonObjectType.Object };
        polities.Properties["polities"] = new JsonSchemaProperty { Type = JsonObjectType.Array };
        var names = new PascalCasePropertyNames(new Dictionary<string, JsonSchema> { ["Polities"] = polities });

        // Act
        var generated = names.Generate(polities.Properties["polities"]);

        // Assert
        Assert.Equal("All", generated);
    }

    [Fact]
    public void A_property_named_like_its_record_with_no_naming_rule_for_its_schema_fails_generation_loudly()
    {
        // Arrange
        var quote = new JsonSchema { Type = JsonObjectType.Object };
        quote.Properties["quote"] = new JsonSchemaProperty { Type = JsonObjectType.String };
        var names = new PascalCasePropertyNames(new Dictionary<string, JsonSchema> { ["Quote"] = quote });

        // Act
        var generate = () => names.Generate(quote.Properties["quote"]);

        // Assert
        var exception = Assert.Throws<InvalidOperationException>(generate);
        Assert.Equal(
            "Record 'Quote' has a property 'quote' whose generated name 'Quote' collides with the record's own name (schema type String); " +
            "C# forbids a member named like its type, and no naming rule covers this schema shape. Decide a name for it in PascalCasePropertyNames.",
            exception.Message);
    }
}
