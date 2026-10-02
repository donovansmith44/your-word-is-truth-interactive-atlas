using BibleAtlas.Client.Tests.State;
using BibleAtlas.Client.Contract;
using System.Text.RegularExpressions;

namespace BibleAtlas.Client.Tests;

public class AsyncMemoConformanceTests
{
    private static string Normalize(string path) => path.Replace('\\', '/');

    private static string StripLineComments(string text) => Regex.Replace(text, "//[^\n]*", string.Empty);

    private static readonly Regex RawAwaitMemoizationPattern = new(
        @"(_\w+)\s*\?\?=\s*[^;\n]{0,80}?await\b",
        RegexOptions.Compiled);

    [Fact]
    public void PlantedDirectShapeViolation_IsCaught()
    {
        const string planted = "public Task<IReadOnlyList<CrossRef>> XrefsAsync(AtlasClient api) => _cachedXrefs ??= await api.Xrefs(_sref);";

        Assert.Matches(RawAwaitMemoizationPattern, planted);
    }

    [Fact]
    public void PlantedExtraOperandShapeViolation_IsCaught()
    {
        const string planted = "public async Task<Chapter> Load(AtlasClient api) => _cached ??= AlreadyLoaded ?? await api.Chapter(_book, _chapter);";

        Assert.Matches(RawAwaitMemoizationPattern, planted);
    }

    [Fact]
    public void PlantedJsInteropCacheShapeViolation_IsCaught()
    {
        const string planted = "_readerJs ??= await JS.InvokeAsync<IJSObjectReference>(\"import\", \"./js/reader.js\");";

        Assert.Matches(RawAwaitMemoizationPattern, planted);
    }

    [Fact]
    public void StripLineComments_ADocCommentQuotingTheBannedIdiomIsNotSelfFlagged()
    {
        const string commentedOut = "    // history: this used to be `_cached ??= await api.Xrefs(_sref)`, now AsyncMemo-backed.";

        Assert.Matches(RawAwaitMemoizationPattern, commentedOut);
        Assert.DoesNotMatch(RawAwaitMemoizationPattern, StripLineComments(commentedOut));
    }

    [Fact]
    public void NoRealSiteOutsideAsyncMemoItself()
    {
        var violations = new List<string>();
        foreach (var file in ConformanceTests.ClientSourceFiles())
        {
            var relative = Normalize(Path.GetRelativePath(ConformanceTests.RepoRoot(), file));
            if (relative == "client/Exploring/AsyncMemo.cs")
            {
                continue;
            }

            var text = StripLineComments(File.ReadAllText(file));
            foreach (Match m in RawAwaitMemoizationPattern.Matches(text))
            {
                violations.Add($"{relative}: {m.Value.Trim()}");
            }
        }

        Assert.True(violations.Count == 0,
            "Found a raw `_field ??= await ...` value-memoizing site outside AsyncMemo.cs -- " +
            "this is the exact bug class PERF-3 exists to kill (a concurrent Task.WhenAll dispatch " +
            "races the null-check ahead of the first caller's own await, firing duplicate fetches). " +
            "Migrate to a `private readonly AsyncMemo<T> _field = new();` field + `_field.Get(() => fetchCall())`, " +
            "the same fix every node in Explore/ (and AtlasClient's own five singleton caches, and " +
            "CompositionSplit/MiniReaderExpand's own JS-module-reference caches) already carries:\n" +
            string.Join("\n", violations));
    }
}
