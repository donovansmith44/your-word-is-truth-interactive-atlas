using System.Text.RegularExpressions;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public sealed class ElementKindLawTests
{
    private static readonly Regex PartialMatchOverAClosedSum = new(
        @"(^\s*(case\s+)?|\bis\s+(not\s+)?)(ElementKind|Frame|Emphasis|PopoverOpening)\.[A-Z]\w*",
        RegexOptions.Compiled | RegexOptions.Multiline);

    [Fact]
    public void No_client_source_switches_over_a_closed_sum()
    {
        // Arrange
        var sources = ConformanceTests.ClientSourceFiles();

        // Act
        var offenders = sources
            .SelectMany(file => PartialMatchOverAClosedSum.Matches(File.ReadAllText(file)).Select(match => $"{Path.GetFileName(file)}: {match.Value.Trim()}"))
            .ToList();

        // Assert
        Assert.Empty(offenders);
    }
}
