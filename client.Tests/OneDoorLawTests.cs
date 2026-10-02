using System.Text.RegularExpressions;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public sealed class OneDoorLawTests
{
    private static readonly Regex Catch = new(@"\bcatch\b", RegexOptions.Compiled);
    private static readonly Regex GuardedTry = new(@"\btry\s*\{", RegexOptions.Compiled);
    private static readonly Regex Awaited = new(@"await\s+(?:\(\s*await\s+)?(?<callee>[A-Za-z_][\w.]*)", RegexOptions.Compiled);
    private static readonly Regex NeighbourRead = new(@"\.Edges\(|\.Entries\(|\blimit\s*:", RegexOptions.Compiled);
    private static readonly Regex InteropCallee = new(@"^(_?[Jj][Ss]|_?readerJs|_lazyJs|_js|_?[mM]apInterop|\w+Ref)\.|^Task\.Delay$|^request\.Fetch$", RegexOptions.Compiled);
    private static readonly string[] TheMemoAndTheRequest = ["AsyncMemo.cs", "RequestSeries.cs"];
    private static readonly string[] ExplorablesAndSectionProviders = ["Exploring", "Legacy"];
    private static readonly string[] ThePagingDoor = ["Paging.cs", "Explorable.cs", "ServedPages.cs", "IExplorableClient.cs", "GraphExplorableClient.cs"];

    [Fact]
    public void No_explorable_or_section_provider_catches_a_failure_it_could_report()
    {
        // Arrange
        var explore = ConformanceTests.ClientSourceFiles().Where(file => ExplorablesAndSectionProviders.Contains(Path.GetFileName(Path.GetDirectoryName(file))));

        // Act
        var catching = explore.Where(file => Catch.IsMatch(File.ReadAllText(file))).Select(Path.GetFileName).Order();

        // Assert
        Assert.Equal(TheMemoAndTheRequest, catching);
    }

    [Fact]
    public void A_component_catches_only_interop_failures_and_hears_of_a_fetch_failure_as_an_outcome()
    {
        // Arrange
        var components = ConformanceTests.ClientSourceFiles().Where(file => file.EndsWith(".razor"));

        // Act
        var offenders = components.SelectMany(file => GuardedCallees(File.ReadAllText(file)).Where(callee => !Local(callee, file) && !InteropCallee.IsMatch(callee)).Select(callee => $"{Path.GetFileName(file)}: {callee}"));

        // Assert
        Assert.Empty(offenders);
    }

    [Fact]
    public void Neighbour_pages_are_read_through_the_one_paging_door()
    {
        // Arrange
        var sources = ConformanceTests.ClientSourceFiles().Where(file => !ThePagingDoor.Contains(Path.GetFileName(file)));

        // Act
        var reading = sources.SelectMany(file => File.ReadAllLines(file).Where(line => NeighbourRead.IsMatch(line)).Select(line => $"{Path.GetFileName(file)}: {line.Trim()}"));

        // Assert
        Assert.Empty(reading);
    }

    private static bool Local(string callee, string file) =>
        !callee.Contains('.') && Regex.IsMatch(File.ReadAllText(file), $@"\bTask\s+{Regex.Escape(callee)}\(");

    private static IEnumerable<string> GuardedCallees(string source)
    {
        foreach (Match opening in GuardedTry.Matches(source))
        {
            var end = opening.Index + opening.Length;
            for (var depth = 1; depth > 0; end++)
            {
                depth += source[end] switch
                {
                    '{' => 1,
                    '}' => -1,
                    _ => 0,
                };
            }

            if (Regex.IsMatch(source[end..], @"^\s*catch\b"))
            {
                foreach (Match call in Awaited.Matches(source[(opening.Index + opening.Length)..end]))
                {
                    yield return call.Groups["callee"].Value;
                }
            }
        }
    }
}
