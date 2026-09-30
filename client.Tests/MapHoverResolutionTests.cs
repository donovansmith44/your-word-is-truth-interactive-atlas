using BibleAtlas.Client.Tests.State;
using System.Text.RegularExpressions;

namespace BibleAtlas.Client.Tests;

public class MapHoverResolutionTests
{
    private static readonly Regex HoverCandidates = new(@"function collectHoverCandidates\(inst\) \{(?<body>[\s\S]*?)\n\}", RegexOptions.Compiled);

    [Fact]
    public void The_pointer_resolves_against_rendered_marker_positions_never_true_ones()
    {
        // Arrange
        var mapJs = File.ReadAllText(Path.Combine(ConformanceTests.ClientRoot, "wwwroot", "js", "map.js"));

        // Act
        var candidates = HoverCandidates.Match(mapJs);
        var truePositionsRead = string.Join(",", Regex.Matches(candidates.Groups["body"].Value, @"\btrue(Lat|Lon)\b").Select(m => m.Value));

        // Assert
        Assert.Equal((true, ""), (candidates.Success, truePositionsRead));
    }
}
