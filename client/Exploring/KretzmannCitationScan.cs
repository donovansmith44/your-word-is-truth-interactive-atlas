using BibleAtlas.Client.Contract;
using System.Text.RegularExpressions;

namespace BibleAtlas.Client.Exploring;

public static class KretzmannCitationScan
{
    public static IReadOnlyList<Anchor> Anchors(string prose, IReadOnlyList<CanonBook> toc)
    {
        var (pattern, codeByAlias) = GetOrBuild(toc);
        return pattern.Matches(prose)
            .Select(m => CitationOf(prose, m, $"{codeByAlias[m.Groups["book"].Value]}.{m.Groups["chapter"].Value}.{m.Groups["verse"].Value}"))
            .ToList();
    }

    private static Anchor CitationOf(string prose, Match citation, string verse) =>
        new(
            end: AnchoredText.ScalarOffsetOf(prose, citation.Index + citation.Length),
            kind: EdgeKind.Cites,
            node: new NodeRef(id: NodeIds.Of(NodeKind.TextUnit, verse), kind: NodeKind.TextUnit, label: verse),
            start: AnchoredText.ScalarOffsetOf(prose, citation.Index));

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

    private static Regex BuildPattern(IReadOnlyList<CanonBook> toc)
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

    private static IReadOnlyList<CanonBook>? _cachedToc;
    private static Regex? _cachedPattern;
    private static Dictionary<string, string>? _cachedCodeByAlias;

    private static (Regex Pattern, Dictionary<string, string> CodeByAlias) GetOrBuild(IReadOnlyList<CanonBook> toc)
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
}
