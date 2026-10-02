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

    [Fact]
    public void The_law_recognises_a_switch_arm_and_a_type_test_over_a_closed_sum()
    {
        // Arrange
        var offending = """
            var a = kind switch
            {
                ElementKind.Node node => 1,
                case Frame.Bounded:
            };
            var b = opening is not PopoverOpening.Resume;
            var c = kind.Match(node: _ => 1, edge: _ => 2);
            """;

        // Act
        var found = PartialMatchOverAClosedSum.Matches(offending).Select(match => match.Value.Trim()).ToArray();

        // Assert
        Assert.Equal(["ElementKind.Node", "case Frame.Bounded", "is not PopoverOpening.Resume"], found);
    }
}
