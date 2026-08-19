using System.Text.RegularExpressions;

namespace BibleAtlas.Client;

/// <summary>
/// Formats and parses years and year-ranges in the exact CONTRACT text
/// formats (a year: <c>1447 BC</c> / <c>AD 30</c>; a range: the two years
/// joined by a spaced en dash U+2013, e.g. <c>1447 BC – 1400 BC</c>; a range
/// whose endpoints are equal collapses to the year alone). Mirrors
/// tests/ux/lib/years.ts's <c>formatYear</c>/<c>formatRange</c> so the TS
/// property helpers and this implementation always agree on wire text.
/// There is no year zero in this calendar (1 BC is immediately followed by
/// AD 1); <see cref="TryParse"/> rejects it explicitly, along with bare
/// digits (the CONTRACT format always carries a BC/AD marker) and inverted
/// ranges (the earlier year must come first).
/// </summary>
public static class YearText
{
    private static readonly Regex BcYear = new(@"^(\d+) BC$", RegexOptions.Compiled);
    private static readonly Regex AdYear = new(@"^AD (\d+)$", RegexOptions.Compiled);

    private const string EnDashSeparator = " – "; // canonical FormatRange separator
    private const string HyphenSeparator = " - ";       // what a keyboard actually types

    public static string Format(int year) => year < 0 ? $"{-year} BC" : $"AD {year}";

    public static string FormatRange(int from, int to) =>
        from == to ? Format(from) : $"{Format(from)}{EnDashSeparator}{Format(to)}";

    /// <summary>
    /// Batch E: formats a curated established/destroyed date claim, e.g.
    /// "c. 1003 BC" or "586 BC" -- a leading "c." appears exactly when
    /// <paramref name="note"/> is present (curated as a qualifier such as
    /// "traditional"), matching the brief's own worked example ("Established
    /// c. 1003 BC · Destroyed 586 BC" -- the first date is curated
    /// "traditional", the second is not). Shared by PlaceCard's
    /// instrument-face line and <see cref="Explore.YearNode"/>'s own title
    /// so the two always agree on exactly the same text.
    /// </summary>
    public static string FormatClaim(int from, int to, string? note)
    {
        var range = FormatRange(from, to);
        return note is null ? range : $"c. {range}";
    }

    /// <summary>
    /// Parses either a single year text (<c>1447 BC</c>, <c>AD 30</c>) or two
    /// such year texts joined by a spaced en dash or a spaced hyphen. Returns
    /// false (leaving <paramref name="from"/>/<paramref name="to"/> as 0)
    /// for anything that doesn't match, including "0 BC", unmarked digits,
    /// and a range whose first year is later than its second.
    /// </summary>
    public static bool TryParse(string text, out int from, out int to)
    {
        from = 0;
        to = 0;
        if (string.IsNullOrWhiteSpace(text))
        {
            return false;
        }

        var (firstText, secondText) = SplitRange(text.Trim());

        if (!TryParseYear(firstText, out var first))
        {
            return false;
        }

        if (secondText is null)
        {
            from = to = first;
            return true;
        }

        if (!TryParseYear(secondText, out var second) || first > second)
        {
            return false;
        }

        from = first;
        to = second;
        return true;
    }

    private static (string First, string? Second) SplitRange(string text)
    {
        var enDashIndex = text.IndexOf(EnDashSeparator, StringComparison.Ordinal);
        if (enDashIndex >= 0)
        {
            return (text[..enDashIndex], text[(enDashIndex + EnDashSeparator.Length)..]);
        }

        var hyphenIndex = text.IndexOf(HyphenSeparator, StringComparison.Ordinal);
        if (hyphenIndex >= 0)
        {
            return (text[..hyphenIndex], text[(hyphenIndex + HyphenSeparator.Length)..]);
        }

        return (text, null);
    }

    private static bool TryParseYear(string text, out int year)
    {
        var bc = BcYear.Match(text);
        if (bc.Success)
        {
            var magnitude = int.Parse(bc.Groups[1].Value);
            year = -magnitude;
            return magnitude != 0;
        }

        var ad = AdYear.Match(text);
        if (ad.Success)
        {
            var magnitude = int.Parse(ad.Groups[1].Value);
            year = magnitude;
            return magnitude != 0;
        }

        year = 0;
        return false;
    }
}
