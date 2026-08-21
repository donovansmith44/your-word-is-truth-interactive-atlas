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

    /// Batch F2, requirement 6: a cross-reference TARGET's own same-chapter
    /// span bounds, mirroring the server's own `atlas_core::xrefs::target_span`
    /// three-shape parse (single verse / same-chapter range / cross-chapter-
    /// or-book range) -- used so <c>CrossRefsSection</c> can fetch each
    /// target's OWN full member-verse text (via the existing chapter
    /// fetch + LRU cache) rather than relying on <c>CrossRefOut.Preview</c>'s
    /// first-verse-only text. Returns null for a cross-chapter/book target
    /// (e.g. `"MAT.5.3-MAT.6.2"`) -- rare in practice, and this app's own
    /// "passage" concept is always single-chapter (PassageNode's own doc
    /// comment), so such a target has no single-chapter span to report;
    /// the caller falls back to the target's own first-verse preview text
    /// for that case (a disclosed, pre-existing limitation, not a
    /// regression -- see batch-f2-report.md).
    public static (string Book, int Chapter, int FromVerse, int ToVerse)? TargetSpan(string target)
    {
        var parts = target.Split('.');
        if (parts.Length != 3)
        {
            return null;
        }

        var book = parts[0];
        if (!int.TryParse(parts[1], out var chapter))
        {
            return null;
        }

        var versePart = parts[2];
        var dash = versePart.IndexOf('-');
        if (dash < 0)
        {
            return int.TryParse(versePart, out var v) ? (book, chapter, v, v) : null;
        }

        var fromOk = int.TryParse(versePart[..dash], out var fromVerse);
        var toOk = int.TryParse(versePart[(dash + 1)..], out var toVerse);
        return fromOk && toOk ? (book, chapter, fromVerse, toVerse) : null;
    }

    // Curated write_place/place-id slugs are kebab-case (e.g. "en-gedi",
    // "mount-sinai"). Used only as a fallback when a slug fails to resolve
    // via AtlasClient.Place, which normally supplies the real,
    // already-presentation-ready display Name.
    public static string Humanize(string slug) =>
        string.Join(' ', slug.Split('-', StringSplitOptions.RemoveEmptyEntries)
            .Select(w => char.ToUpperInvariant(w[0]) + w[1..]));
}
