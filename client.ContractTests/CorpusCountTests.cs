using System.Reflection;

namespace BibleAtlas.Client.ContractTests;

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
                if (attrs.Any(a => a.GetType().Name == "SkippableTheoryAttribute"))
                {
                    count += attrs.Count(a => a.GetType().Name == "InlineDataAttribute");
                }
            }
        }
        return count;
    }

    private static int CountScenariosInFeatureFiles()
    {
        var repoRoot = Steps.AqcSteps.RepoRoot;
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
}
