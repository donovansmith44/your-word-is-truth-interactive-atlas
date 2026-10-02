using System.Text.RegularExpressions;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public enum Batch
{
    Focus2,
    Focus3,
    Focus4,
    Focus5,
    Focus7,
    Focus9,
    Maps,
}

public sealed record Retirement(Batch Batch, int Sites);

internal static class SourceRatchet
{
    internal static IReadOnlyList<string> OffencesOutside(Regex offence, IReadOnlyDictionary<string, Retirement> retiredBy) =>
        SitesOf(offence)
            .Where(file => !retiredBy.TryGetValue(file.Key, out var listed) || file.Value > listed.Sites)
            .Select(file => $"{file.Key}: {file.Value} sites")
            .Order()
            .ToList();

    internal static IReadOnlyList<string> ListedSitesGone(Regex offence, IReadOnlyDictionary<string, Retirement> retiredBy)
    {
        var sites = SitesOf(offence);
        return retiredBy
            .Where(listed => sites.GetValueOrDefault(listed.Key) < listed.Value.Sites)
            .Select(listed => $"{listed.Key}: {sites.GetValueOrDefault(listed.Key)} of {listed.Value.Sites} sites left, retired by {listed.Value.Batch}")
            .Order()
            .ToList();
    }

    private static IReadOnlyDictionary<string, int> SitesOf(Regex offence) =>
        ConformanceTests.ClientSourceFiles()
            .Select(file => (File: Path.GetRelativePath(ConformanceTests.ClientRoot, file).Replace('\\', '/'), Sites: offence.Matches(File.ReadAllText(file)).Count))
            .Where(file => file.Sites > 0)
            .ToDictionary(file => file.File, file => file.Sites);
}
