using System.Text.RegularExpressions;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Small parsing/formatting helpers shared across Explore/* node types over
/// canonical scripture ref strings and place-id slugs. Implementation
/// plumbing only -- not part of the IExplorable/ExplorationTarget contract.
/// </summary>
internal static class CanonRef
{
    // The brief's own firstVerseOf definition, verbatim: the leading
    // canonical verse of either span form (GEN.1.1-5 -> GEN.1.1;
    // GEN.1.1-GEN.2.3 -> GEN.1.1).
    private static readonly Regex HeadPattern = new(@"^[A-Z0-9]{3}\.\d+\.\d+", RegexOptions.Compiled);

    public static string FirstVerseOf(string target)
    {
        var m = HeadPattern.Match(target);
        return m.Success ? m.Value : target;
    }

    public static (string Book, int Chapter, int Verse) ParseVerse(string vref)
    {
        var parts = vref.Split('.');
        return (parts[0], int.Parse(parts[1]), int.Parse(parts[2]));
    }

    // Curated write_place/place-id slugs are kebab-case (e.g. "en-gedi",
    // "mount-sinai"). Used only as a fallback when a slug fails to resolve
    // via AtlasClient.Place, which normally supplies the real,
    // already-presentation-ready display Name.
    public static string Humanize(string slug) =>
        string.Join(' ', slug.Split('-', StringSplitOptions.RemoveEmptyEntries)
            .Select(w => char.ToUpperInvariant(w[0]) + w[1..]));
}
