using System.Reflection;

namespace BibleAtlas.Client.ContractTests;

/// <summary>
/// Batch AQC-1 fix round 1 -- THE COUNTING LAW (controller ruling): "the
/// dotnet line carries the parity check: client.Tests N/N |
/// client.ContractTests X/X -- where client.ContractTests's count MUST
/// EQUAL the AQC line's X ... any divergence means a feature file is being
/// executed by one side and not the other." This is that assertion, made
/// real rather than merely reported: it reflects over every
/// Reqnroll-generated scenario test THIS ASSEMBLY actually discovered
/// (`SkippableFactAttribute` per plain scenario, `SkippableTheoryAttribute`
/// + one `InlineDataAttribute` per Scenario Outline row -- confirmed by
/// direct inspection of the generated code-behind under `obj/`) and
/// compares that DISCOVERED count against the SAME count computed by
/// parsing the six committed `.feature` files directly (textually,
/// language-agnostic -- no Reqnroll/cucumber-specific logic). A feature
/// file that silently fails to link into this project (a stale
/// `ReqnrollFeatureFile` glob, a build-config gap) would make the
/// DISCOVERED count fall behind the DECLARED one; this test catches that
/// class of drift even though it cannot directly compare against the Rust
/// side's own live cucumber run in the same process.
/// </summary>
public class CorpusCountTests
{
    [Fact]
    public void DiscoveredScenarioCountMatchesTheCommittedFeatureFiles()
    {
        var declared = CountScenariosInFeatureFiles();
        var discovered = CountDiscoveredReqnrollScenarios();
        Assert.Equal(declared, discovered);
    }

    private static int CountDiscoveredReqnrollScenarios()
    {
        var assembly = typeof(Steps.AqcSteps).Assembly;
        var count = 0;
        foreach (var type in assembly.GetTypes())
        {
            foreach (var method in type.GetMethods(BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly))
            {
                var attrs = method.GetCustomAttributes(inherit: false);
                if (attrs.Any(a => a.GetType().Name == "SkippableFactAttribute"))
                {
                    count += 1;
                }
                count += attrs.Count(a => a.GetType().Name == "InlineDataAttribute");
            }
        }
        return count;
    }

    private static int CountScenariosInFeatureFiles()
    {
        var repoRoot = FindRepoRoot();
        var featuresDir = Path.Combine(repoRoot, "contracts", "atlas-query-contract", "features");
        var total = 0;
        foreach (var file in Directory.GetFiles(featuresDir, "*.feature"))
        {
            var inExamplesTable = false;
            var sawExamplesHeader = false;
            foreach (var rawLine in File.ReadAllLines(file))
            {
                var line = rawLine.Trim();
                if (line.StartsWith("Scenario Outline:", StringComparison.Ordinal))
                {
                    inExamplesTable = false;
                    sawExamplesHeader = false;
                    continue;
                }
                if (line.StartsWith("Scenario:", StringComparison.Ordinal))
                {
                    total += 1;
                    inExamplesTable = false;
                    sawExamplesHeader = false;
                    continue;
                }
                if (line.StartsWith("Examples:", StringComparison.Ordinal))
                {
                    inExamplesTable = true;
                    sawExamplesHeader = false;
                    continue;
                }
                if (inExamplesTable && line.StartsWith('|'))
                {
                    if (!sawExamplesHeader)
                    {
                        // First "|" row after "Examples:" is the header row
                        // (column names) -- not a data row.
                        sawExamplesHeader = true;
                    }
                    else
                    {
                        total += 1;
                    }
                }
            }
        }
        return total;
    }

    private static string FindRepoRoot([System.Runtime.CompilerServices.CallerFilePath] string here = "") =>
        Path.GetFullPath(Path.Combine(Path.GetDirectoryName(here)!, ".."));
}
