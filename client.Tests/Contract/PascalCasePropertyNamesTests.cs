using BibleAtlas.Client.ContractGenerator;
using NJsonSchema;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class PascalCasePropertyNamesTests
{
    [Fact]
    public async Task Snake_and_kebab_wire_names_become_PascalCase_property_names()
    {
        // Arrange
        var schema = await JsonSchema.FromJsonAsync("""
            {
              "type": "object",
              "properties": {
                "name": { "type": "string" },
                "edge_summary": { "type": "string" },
                "birth_year": { "type": "integer" },
                "also_called": { "type": "array" },
                "member-of": { "type": "string" }
              }
            }
            """);
        var names = new PascalCasePropertyNames();
        // Act
        var generated = schema.ActualProperties.Values.Select(names.Generate).ToArray();
        // Assert
        Assert.Equal(["Name", "EdgeSummary", "BirthYear", "AlsoCalled", "MemberOf"], generated);
    }
}
