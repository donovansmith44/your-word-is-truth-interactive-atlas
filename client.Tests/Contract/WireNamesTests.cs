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
        var members = GeneratedEnumMembers();
        // Act
        var actual = members.Select(WireNameOf).ToList();
        var expected = members.Select(m => JsonSerializer.Serialize(m, m.GetType(), options).Trim('"')).ToList();
        // Assert
        Assert.Equal(expected, actual);
    }

    [Fact]
    public void Parse_inverts_WireName_for_every_member_of_every_generated_enum()
    {
        // Arrange
        var members = GeneratedEnumMembers();
        // Act
        var parsed = members.Select(m => ParseAs(m.GetType(), WireNameOf(m))).ToList();
        // Assert
        Assert.Equal(members, parsed);
    }

    [Fact]
    public void Parse_rejects_a_name_the_enum_does_not_declare()
    {
        // Arrange
        var undeclared = "cited";
        // Act
        Action act = () => WireNames.Parse<EdgeKind>(undeclared);
        // Assert
        Assert.Throws<FormatException>(act);
    }

    [Fact]
    public void WireName_of_an_enum_the_contract_does_not_declare_fails_naming_the_member()
    {
        // Arrange
        var member = NotInTheContract.Member;
        // Act
        Action act = () => member.WireName();
        // Assert
        var thrown = Assert.Throws<NoWireNameException>(act);
        Assert.Equal("NotInTheContract.Member has no wire name; WireNames serves only generated contract enums", thrown.Message);
    }

    private enum NotInTheContract
    {
        Member,
    }

    private static List<Enum> GeneratedEnumMembers() =>
        typeof(EdgeKind).Assembly.GetTypes()
            .Where(t => t.IsEnum && t.Namespace == typeof(EdgeKind).Namespace)
            .SelectMany(t => Enum.GetValues(t).Cast<Enum>())
            .ToList();

    private static Enum ParseAs(Type enumType, string name) =>
        (Enum)typeof(WireNames).GetMethod(nameof(WireNames.Parse))!
            .MakeGenericMethod(enumType)
            .Invoke(null, new object[] { name })!;

    private static string WireNameOf(Enum value) =>
        (string)typeof(WireNames).GetMethod(nameof(WireNames.WireName))!
            .MakeGenericMethod(value.GetType())
            .Invoke(null, new object[] { value })!;
}
