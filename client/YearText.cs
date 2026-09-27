using System.Text.RegularExpressions;

namespace BibleAtlas.Client;

// Mirrors tests/ux/lib/years.ts's formatYear/formatRange -- keep the two in sync, or the TS
// property helpers and this implementation will disagree on wire text.
public static class YearText
{
    private static readonly Regex BcYear = new(@"^(\d+) BC$", RegexOptions.Compiled);
    private static readonly Regex AdYear = new(@"^AD (\d+)$", RegexOptions.Compiled);

    private const string EnDashSeparator = " – ";
    private const string HyphenSeparator = " - ";

    public static string Format(int year) => year < 0 ? $"{-year} BC" : $"AD {year}";

    public static string FormatRange(int from, int to) =>
        from == to ? Format(from) : $"{Format(from)}{EnDashSeparator}{Format(to)}";

    public static string FormatClaim(int from, int to, string? note)
    {
        var range = FormatRange(from, to);
        return note is null ? range : $"c. {range}";
    }

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
