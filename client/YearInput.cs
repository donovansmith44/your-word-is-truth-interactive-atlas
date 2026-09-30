using System.Text.RegularExpressions;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public static class YearInput
{
    private const string FirstEnd = "first";
    private const string LastEnd = "last";
    private const string Circa = @"(?:c\. )?";
    private const string Dash = @"\s*[–-]\s*";

    private static readonly Regex Label = new($"^{Circa}{EndPattern(FirstEnd)}(?:{Dash}{EndPattern(LastEnd)})?$", RegexOptions.Compiled);

    public static YearSpan? Read(string text)
    {
        var match = Label.Match(text.Trim());
        if (!match.Success)
        {
            return null;
        }

        var first = EndOf(match, FirstEnd);
        var last = match.Groups[LastEnd].Success ? EndOf(match, LastEnd) : first;
        if (first is not { } start || last is not { } end)
        {
            return null;
        }

        // A served range in one era names it once ("1450 – 1400 BC", "AD 1 – 100"), so an end
        // written without an era is reckoned in the other end's.
        return (start.YearIn(end.NamedEra), end.YearIn(start.NamedEra)) is ({ } from, { } to) && from <= to
            ? new YearSpan(from, to)
            : null;
    }

    private static string EndPattern(string end) =>
        $"(?<{EraGroup(end, Era.AnnoDomini)}>AD )?(?<{end}>[0-9]+)(?<{EraGroup(end, Era.BeforeChrist)}> BC)?";

    private static End? EndOf(Match match, string end)
    {
        if (!int.TryParse(match.Groups[end].Value, out var magnitude) || magnitude == 0)
        {
            return null;
        }

        return (Names(match, end, Era.AnnoDomini), Names(match, end, Era.BeforeChrist)) switch
        {
            (true, true) => null,
            (true, false) => new End(magnitude, Era.AnnoDomini),
            (false, true) => new End(magnitude, Era.BeforeChrist),
            (false, false) => new End(magnitude, null),
        };
    }

    private static bool Names(Match match, string end, Era era) => match.Groups[EraGroup(end, era)].Success;

    private static string EraGroup(string end, Era era) => $"{end}{era}";

    private enum Era
    {
        BeforeChrist = -1,
        AnnoDomini = 1,
    }

    private readonly record struct End(int Magnitude, Era? NamedEra)
    {
        public int? YearIn(Era? otherEndsEra) => (NamedEra ?? otherEndsEra) is { } era ? (int)era * Magnitude : null;
    }
}
