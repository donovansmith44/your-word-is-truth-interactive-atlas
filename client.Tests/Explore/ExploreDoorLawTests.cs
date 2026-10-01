using System.Text.RegularExpressions;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public sealed class ExploreDoorLawTests
{
    private static readonly Regex Traversal = new(@"\b_?[Ee]xplorer\.(Follow|Resolve)\(", RegexOptions.Compiled);
    private static readonly string[] TheMonad = ["Explore.cs"];

    [Fact]
    public void Only_the_explore_monad_follows_or_resolves_through_the_explorer()
    {
        // Arrange
        var sources = ConformanceTests.ClientSourceFiles().Where(file => !TheMonad.Contains(Path.GetFileName(file)));

        // Act
        var offenders = sources.SelectMany(file => File.ReadAllLines(file).Where(line => Traversal.IsMatch(line)).Select(line => $"{Path.GetFileName(file)}: {line.Trim()}"));

        // Assert
        Assert.Empty(offenders);
    }

    [Fact]
    public void The_law_sees_a_view_following_or_resolving_through_any_explorer_it_holds()
    {
        // Arrange
        var planted = new[]
        {
            "var next = await Explorer.Follow(link);",
            "var node = await explorer.Resolve(target);",
            "var next = await _explorer.Follow(link);",
            "var walk = Explore.Follow(link);",
        };

        // Act
        var seen = planted.Select(line => Traversal.IsMatch(line)).ToList();

        // Assert
        Assert.Equal([true, true, true, false], seen);
    }
}
