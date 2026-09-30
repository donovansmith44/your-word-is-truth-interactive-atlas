using System.Text.Json;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class TextRefWireTests
{
    private const string GenesisOneOneOnTheWire = """{"corpus":"bible","book":"GEN","chapter":1,"verse":1}""";
    private const string AugsburgArticleTwoOnTheWire = """{"corpus":"concord","article":2,"paragraph":1,"part":7}""";

    [Fact]
    public void A_text_ref_read_off_the_wire_is_the_unit_it_names()
    {
        // Arrange
        var served = new[] { GenesisOneOneOnTheWire, AugsburgArticleTwoOnTheWire };

        // Act
        var read = served.Select(json => JsonSerializer.Deserialize<TextRef>(json)).ToArray();

        // Assert
        Assert.Equal(new TextRef?[] { new BibleRef(BookId.GEN, 1, 1), new ConcordRef(article: 2, paragraph: 1, part: 7) }, read);
    }

    [Fact]
    public void A_text_ref_read_off_the_wire_writes_back_the_bytes_it_was_read_from()
    {
        // Arrange
        var served = new[] { GenesisOneOneOnTheWire, AugsburgArticleTwoOnTheWire };

        // Act
        var written = served.Select(json => JsonSerializer.Serialize(JsonSerializer.Deserialize<TextRef>(json))).ToArray();

        // Assert
        Assert.Equal(served, written);
    }
}
