using System.Text.RegularExpressions;

namespace BibleAtlas.Client.Explore;

public readonly record struct ScriptureRefMatch(int Start, int Length, string Sref);

public static class ScriptureRefScan
{
    // Codes verified live against this app's own /api/books response, not copied from an ETL
    // source file (an earlier draft's copy disagreed with this app's canonical codes for
    // several books: John is "JHN" here not "JOH", Ezekiel "EZK" not "EZE", Mark "MRK" not
    // "MAR", James "JAS" not "JAM", Jude "JUD" not "JDE", Nahum "NAM" not "NAH", the Johannine
    // epistles "1JN"/"2JN"/"3JN" not "1JO"/"2JO"/"3JO").
    //
    // Deliberately has NO entry for a bare "Cor"/"Pet"/"Phil": each is genuinely ambiguous
    // (a bare "Cor." is at least as often an elided 2 Corinthians as 1; "Phil" is ambiguous
    // between Philippians and Philemon), and an unlisted abbreviation is meant to be a silent
    // miss, never a guessed misattribution. Do not add these back without verifying real
    // corpus usage first.
    private static readonly (string Alias, string Code)[] CitationAliases =
    {
        ("Gen", "GEN"), ("Exod", "EXO"), ("Exo", "EXO"), ("Lev", "LEV"), ("Num", "NUM"),
        ("Deut", "DEU"), ("Deu", "DEU"), ("Josh", "JOS"), ("Judg", "JDG"),
        ("1 Sam", "1SA"), ("2 Sam", "2SA"), ("1 Kings", "1KI"), ("1 Kgs", "1KI"),
        ("2 Kings", "2KI"), ("2 Kgs", "2KI"), ("1 Chron", "1CH"), ("1 Chr", "1CH"),
        ("2 Chron", "2CH"), ("2 Chr", "2CH"), ("Neh", "NEH"), ("Esth", "EST"),
        ("Ps", "PSA"), ("Pss", "PSA"), ("Psalm", "PSA"), ("Prov", "PRO"),
        ("Eccl", "ECC"), ("Isa", "ISA"), ("Jer", "JER"), ("Lam", "LAM"),
        ("Ezek", "EZK"), ("Eze", "EZK"), ("Dan", "DAN"), ("Hos", "HOS"),
        ("Obad", "OBA"), ("Mic", "MIC"), ("Nah", "NAM"), ("Hab", "HAB"),
        ("Zeph", "ZEP"), ("Hag", "HAG"), ("Zech", "ZEC"), ("Mal", "MAL"),
        ("Matt", "MAT"), ("Mat", "MAT"), ("Mk", "MRK"), ("Lk", "LUK"),
        ("Rom", "ROM"), ("1 Cor", "1CO"), ("2 Cor", "2CO"),
        ("Gal", "GAL"), ("Eph", "EPH"), ("Php", "PHP"),
        ("Col", "COL"), ("1 Thess", "1TH"), ("1 Thes", "1TH"), ("2 Thess", "2TH"),
        ("2 Thes", "2TH"), ("1 Tim", "1TI"), ("2 Tim", "2TI"), ("Tit", "TIT"),
        ("Philem", "PHM"), ("Heb", "HEB"), ("Jas", "JAS"),
        ("1 Pet", "1PE"), ("2 Pet", "2PE"),
        ("1 John", "1JN"), ("2 John", "2JN"), ("3 John", "3JN"),
        ("Jude", "JUD"), ("Rev", "REV"),
    };

    // Longest-alias-first: an alternation tries left-to-right, so "1 Corinthians" must be
    // offered before "1 Cor" or the shorter alias would win and strand " inthians" as plain text.
    private static Regex BuildPattern(IReadOnlyList<BookTocEntry> toc)
    {
        var tokens = new List<(string Alias, string Code)>();
        foreach (var book in toc)
        {
            tokens.Add((book.Name, book.Code));
        }
        tokens.AddRange(CitationAliases);

        var ordered = tokens.OrderByDescending(t => t.Alias.Length).ToList();
        var bookAlternation = string.Join('|', ordered.Select(t => Regex.Escape(t.Alias)));

        return new Regex(
            @"\b(?<book>" + bookAlternation + @")\.?\s+(?<chapter>\d{1,3})[,:]\s*(?<verse>\d{1,3})(?:-(?<verse2>\d{1,3}))?",
            RegexOptions.Compiled);
    }

    // Cached by reference equality against the caller's own toc list: AtlasClient.Books()
    // hands out one singleton-cached list for the app's whole lifetime, so this never goes
    // stale, and rebuilding it (compiling a ~130-branch regex) on every render was measured
    // as a real perf cost across this app's commentary/reading surfaces.
    private static IReadOnlyList<BookTocEntry>? _cachedToc;
    private static Regex? _cachedPattern;
    private static Dictionary<string, string>? _cachedCodeByAlias;

    private static (Regex Pattern, Dictionary<string, string> CodeByAlias) GetOrBuild(IReadOnlyList<BookTocEntry> toc)
    {
        if (_cachedPattern is not null && ReferenceEquals(_cachedToc, toc))
        {
            return (_cachedPattern, _cachedCodeByAlias!);
        }

        var pattern = BuildPattern(toc);
        var codeByAlias = new Dictionary<string, string>();
        foreach (var b in toc)
        {
            codeByAlias[b.Name] = b.Code;
        }
        foreach (var (alias, code) in CitationAliases)
        {
            codeByAlias.TryAdd(alias, code);
        }

        _cachedToc = toc;
        _cachedPattern = pattern;
        _cachedCodeByAlias = codeByAlias;
        return (pattern, codeByAlias);
    }

    public static IReadOnlyList<ScriptureRefMatch> Scan(string text, IReadOnlyList<BookTocEntry> toc)
    {
        if (string.IsNullOrEmpty(text) || toc.Count == 0)
        {
            return Array.Empty<ScriptureRefMatch>();
        }

        var (pattern, codeByAlias) = GetOrBuild(toc);

        var results = new List<ScriptureRefMatch>();
        foreach (Match m in pattern.Matches(text))
        {
            if (!codeByAlias.TryGetValue(m.Groups["book"].Value, out var code))
            {
                continue;
            }

            var chapter = m.Groups["chapter"].Value;
            var verse = m.Groups["verse"].Value; // a verse range collapses to its first verse only
            results.Add(new ScriptureRefMatch(m.Index, m.Length, $"{code}.{chapter}.{verse}"));
        }

        return results;
    }
}
