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

    [Fact]
    public void An_integer_property_named_like_its_record_is_named_Number()
    {
        // Arrange
        var verse = new JsonSchema { Type = JsonObjectType.Object };
        verse.Properties["verse"] = new JsonSchemaProperty { Type = JsonObjectType.Integer };
        var names = new PascalCasePropertyNames(new Dictionary<string, JsonSchema> { ["Verse"] = verse });

        // Act
        var generated = names.Generate(verse.Properties["verse"]);

        // Assert
        Assert.Equal("Number", generated);
    }

    [Fact]
    public void An_integer_property_named_like_its_record_stays_Number_for_a_differently_named_record()
    {
        // Arrange
        var chapter = new JsonSchema { Type = JsonObjectType.Object };
        chapter.Properties["chapter"] = new JsonSchemaProperty { Type = JsonObjectType.Integer };
        var names = new PascalCasePropertyNames(new Dictionary<string, JsonSchema> { ["Chapter"] = chapter });

        // Act
        var generated = names.Generate(chapter.Properties["chapter"]);

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
        Assert.Contains("Quote", exception.Message);
        Assert.Contains("quote", exception.Message);
    }
}
