using System.Text.RegularExpressions;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public sealed class NodesResolveByIdConformanceTests
{
    private static readonly Regex FoundByLabel = new(@"FirstOrDefault\([^)]*\.(Label|Name|Title|DisplayName)\s*==", RegexOptions.Compiled);

    [Fact]
    public void No_client_code_finds_a_node_by_its_label()
    {
        // Arrange
        var lines = ConformanceTests.ClientSourceFiles()
            .SelectMany(file => File.ReadAllLines(file).Select((text, i) => (File: Path.GetFileName(file), Line: i + 1, Text: text)));

        // Act
        var offenders = string.Join(", ", lines.Where(l => FoundByLabel.IsMatch(l.Text)).Select(l => $"{l.File}:{l.Line}"));

        // Assert
        Assert.Equal("", offenders);
    }

    [Fact]
    public void A_lookup_by_label_or_name_is_what_the_law_catches()
    {
        // Arrange
        var planted = new[]
        {
            "var date = dates.FirstOrDefault(d => d.Label == label)",
            "polities.All.FirstOrDefault(e => e.Name == polityName && e.From == fromYear)",
            "var place = scene.Places.FirstOrDefault(p => p.Id == id);",
        };

        // Act
        var caught = planted.Select(line => FoundByLabel.IsMatch(line)).ToList();

        // Assert
        Assert.Equal([true, true, false], caught);
    }
}
