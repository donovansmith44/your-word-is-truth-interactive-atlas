using System.Text.RegularExpressions;

namespace BibleAtlas.Client.Tests;

/// <summary>
/// PERF-3 re-review, fix round 2 (ruling 3 -- "THE TRIPWIRE, so this class
/// can never return"): a conformance-style scan, same discipline as
/// <c>State/ConformanceTests.cs</c>'s own established precedent (its
/// <c>.Changed +=</c> allowlist scan and its several planted-line-proven
/// regex sweeps) -- forbids the raw, pre-batch `_field ??= await ...`
/// value-memoizing idiom ANYWHERE in <c>client/</c> outside
/// <c>Explore/AsyncMemo.cs</c> itself. That idiom is the exact bug class
/// PERF-3 exists to kill: under any concurrent <c>Task.WhenAll</c> dispatch
/// (the popover section-registry path, or the <c>ExploreAsync</c>+<c>BodyAsync</c>
/// fallback path the re-review's own AuthorNode finding proved is a SECOND
/// reachable shape of the identical race), the null-check races ahead of
/// the first caller's own await, producing duplicate fetches -- and, if
/// ever "fixed" naively by task-caching without <see cref="Explore.AsyncMemo{T}"/>'s
/// own reset-on-fault discipline, a transient failure permanently poisons
/// that field for the rest of its owner's lifetime (fix round 1's own
/// Q-2 finding).
///
/// This batch's own fix-round-2 sweep found and migrated FIVE more real
/// sites this exact scan would have caught on day one:
/// <c>AuthorNode.Load</c> (a live race, re-review Major finding),
/// <c>ChapterNode.Load</c>, <c>CompositionSplit._js</c>,
/// <c>MiniReaderExpand._readerJs</c>, and <c>AtlasClient</c>'s own five
/// fetch-once-forever singletons (<c>Books</c>/<c>Eras</c>/<c>Landmarks</c>/
/// <c>LandMask</c>/<c>Sources</c>) -- confirmed live-race-capable too:
/// AtlasClient is a DI singleton, and its own cached lists are reachable
/// from more than one page/component's concurrent startup fetch, the same
/// race shape scoped to app-init rather than a per-click hot path. As of
/// this fix round, the real-tree scan below finds ZERO remaining sites --
/// no allowlist entry is needed (the strongest, simplest outcome: nothing
/// to justify, nothing to keep accurate against drift).
/// </summary>
public class AsyncMemoConformanceTests
{
    private static string RepoRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null && !Directory.Exists(Path.Combine(dir.FullName, "client")))
        {
            dir = dir.Parent;
        }

        Assert.NotNull(dir);
        return dir!.FullName;
    }

    private static IEnumerable<string> ClientSourceFiles()
    {
        var clientDir = Path.Combine(RepoRoot(), "client");
        return Directory.EnumerateFiles(clientDir, "*.cs", SearchOption.AllDirectories)
            .Concat(Directory.EnumerateFiles(clientDir, "*.razor", SearchOption.AllDirectories))
            .Where(f => !f.Contains($"{Path.DirectorySeparatorChar}obj{Path.DirectorySeparatorChar}")
                        && !f.Contains($"{Path.DirectorySeparatorChar}bin{Path.DirectorySeparatorChar}"));
    }

    private static string Normalize(string path) => path.Replace('\\', '/');

    // Same disclosed-limitation stripper as ConformanceTests.cs's own
    // StripLineComments (does not understand string literals containing
    // "//" -- irrelevant here, no real site in this codebase has ever put
    // a URL on the same line as a `??=` memoization expression) -- applied
    // so this file's OWN doc comments (which quote the banned idiom
    // verbatim as prose, e.g. the header above and every migrated node's
    // own "was: `_cached ??= await ...`" explanation) never self-trigger
    // the scan they document.
    private static string StripLineComments(string text) => Regex.Replace(text, "//[^\n]*", string.Empty);

    // Matches `_field ??= <up to 80 chars> await` -- the lazy, bounded
    // window (not a bare `\?\?=\s*await\b`) is deliberate: the real,
    // historical ChapterNode.Load shape was `_cached ??= AlreadyLoaded ??
    // await api.Chapter(...)`, an extra `?? AlreadyLoaded` operand sitting
    // between `??=` and `await` -- a bare adjacency match would have missed
    // it. 80 chars comfortably covers every real shape this batch has
    // found (verified: the widest, AsyncMemo's own `_task ??= FetchAsync(fetch)`-shaped
    // callers, RE-WRITTEN via AsyncMemo now so none remain -- see the
    // planted-line tests below for the exact retired shapes this proves
    // against) without being so wide it would start matching an unrelated,
    // later statement's own incidental "await" token.
    private static readonly Regex RawAwaitMemoizationPattern = new(
        @"(_\w+)\s*\?\?=\s*[^;\n]{0,80}?await\b",
        RegexOptions.Compiled);

    [Fact]
    public void PlantedDirectShapeViolation_IsCaught()
    {
        // The retired VerseNode/AuthorNode/PlaceNode/EventNode/CatechismNode
        // shape (pre-fix-round-1/2) -- `_field ??= await <fetch>` directly.
        const string planted = "public Task<VerseDetail> DetailAsync(AtlasClient api) => _cachedDetail ??= await api.Verse($\"{_bookCode}.1.1\");";

        Assert.Matches(RawAwaitMemoizationPattern, planted);
    }

    [Fact]
    public void PlantedExtraOperandShapeViolation_IsCaught()
    {
        // The retired ChapterNode.Load shape specifically -- an extra `??`
        // operand (AlreadyLoaded) sits between `??=` and `await`, proving
        // the bounded lazy window catches this, not just the bare-adjacency
        // case the OTHER planted test above already covers.
        const string planted = "public async Task<ChapterOut> Load(AtlasClient api) => _cached ??= AlreadyLoaded ?? await api.Chapter(_book, _chapter);";

        Assert.Matches(RawAwaitMemoizationPattern, planted);
    }

    [Fact]
    public void PlantedJsInteropCacheShapeViolation_IsCaught()
    {
        // The retired CompositionSplit/MiniReaderExpand shape -- caching an
        // IJSObjectReference, not a fetched DTO; proves the pattern isn't
        // accidentally scoped to "await api.*" calls specifically.
        const string planted = "_readerJs ??= await JS.InvokeAsync<IJSObjectReference>(\"import\", \"./js/reader.js\");";

        Assert.Matches(RawAwaitMemoizationPattern, planted);
    }

    [Fact]
    public void StripLineComments_ADocCommentQuotingTheBannedIdiomIsNotSelfFlagged()
    {
        // This file's own header comment (and every migrated node's own
        // "was: ..." doc comment) quotes the banned shape verbatim as
        // prose -- proving the stripper correctly removes it before the
        // scan ever sees it, the same "explain the rule without tripping
        // it" property ConformanceTests.cs's own allowlist-adjacent
        // comments rely on throughout.
        const string commentedOut = "    // history: this used to be `_cached ??= await api.Verse(_vref)`, now AsyncMemo-backed.";

        Assert.Matches(RawAwaitMemoizationPattern, commentedOut); // sanity: the raw text DOES match before stripping
        Assert.DoesNotMatch(RawAwaitMemoizationPattern, StripLineComments(commentedOut));
    }

    [Fact]
    public void NoRealSiteOutsideAsyncMemoItself()
    {
        var violations = new List<string>();
        foreach (var file in ClientSourceFiles())
        {
            var relative = Normalize(Path.GetRelativePath(RepoRoot(), file));
            if (relative == "client/Explore/AsyncMemo.cs")
            {
                continue; // the one sanctioned definition site -- AsyncMemo<T>.Get itself uses no `??=` at all (a plain `if (_task is { } current)` check), but excluded explicitly per the ruling's own wording regardless
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
