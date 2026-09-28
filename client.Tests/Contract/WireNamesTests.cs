using System.Text.Json;
using System.Text.Json.Serialization;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class WireNamesTests
{
    [Fact]
    public void WireName_agrees_with_System_Text_Json_for_every_member_of_every_generated_enum()
    {
        // Arrange
        var options = new JsonSerializerOptions { Converters = { new JsonStringEnumConverter() } };
        var members = typeof(EdgeKind).Assembly.GetTypes()
            .Where(t => t.IsEnum && t.Namespace == typeof(EdgeKind).Namespace)
            .SelectMany(t => Enum.GetValues(t).Cast<Enum>())
            .ToList();
        // Act
        var actual = members.Select(WireNameOf).ToList();
        var expected = members.Select(m => JsonSerializer.Serialize(m, m.GetType(), options).Trim('"')).ToList();
        // Assert
        Assert.Equal(expected, actual);
    }

    private static string WireNameOf(Enum value) =>
        (string)typeof(WireNames).GetMethod(nameof(WireNames.WireName))!
            .MakeGenericMethod(value.GetType())
            .Invoke(null, new object[] { value })!;
}
