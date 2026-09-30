using BibleAtlas.Client.Tests.State;
using System.Text.RegularExpressions;

namespace BibleAtlas.Client.Tests;

public class MapHoverResolutionTests
{
    private static readonly Regex Resolver = new(@"function resolveHoverTarget\(inst, pt, fallbackId\) \{(?<body>[\s\S]*?)\n\}", RegexOptions.Compiled);
    private static readonly Regex DistanceAtRenderedPosition = new(@"const p = map\.latLngToContainerPoint\(\[c\.lat, c\.lon\]\);[\s\S]*?dist: Math\.hypot\(p\.x - pt\.x, p\.y - pt\.y\)", RegexOptions.Compiled);
    private static readonly Regex TheElementUnderThePointerWins = new(@"const routed = fallbackId \? candidates\.find\(c => c\.id === fallbackId\) : undefined;[\s\S]*?const winner = routed \?\? within\[0\];", RegexOptions.Compiled);
    private static readonly Regex CoincidenceJudgedInTruth = new(@"Math\.hypot\(c\.tx - winner\.tx, c\.ty - winner\.ty\) <= AMBIGUITY_RADIUS_PX", RegexOptions.Compiled);

    [Fact]
    public void The_pointer_resolves_to_the_marker_it_is_over_measured_where_markers_are_drawn_and_coincidence_is_judged_in_truth()
    {
        // Arrange
        var mapJs = File.ReadAllText(Path.Combine(ConformanceTests.ClientRoot, "wwwroot", "js", "map.js"));

        // Act
        var resolver = Resolver.Match(mapJs).Groups["body"].Value;

        // Assert
        Assert.Equal(
            (true, true, true),
            (DistanceAtRenderedPosition.IsMatch(resolver), TheElementUnderThePointerWins.IsMatch(resolver), CoincidenceJudgedInTruth.IsMatch(resolver)));
    }
}
