using System.Text.RegularExpressions;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch CORPREAD-1b (ticket K/C, "verse ref detection in arbitrary prose"):
/// the scripture-reference-in-running-prose scanner Kretzmann's commentary
/// and the Book of Concord's confession text both need ("references inside
/// commentary prose ... are clickable explorables ... ONE-RULE for any
/// embedded verse text").
///
/// DISCLOSED, PER THE BRIEF'S OWN INSTRUCTION ("reuse/extend the existing
/// reference-parsing machinery where it exists ... if genuinely none exists
/// client-side ... implement the minimal disclosed one"): investigated
/// directly before writing this. Two candidate mechanisms already exist
/// client-side, and NEITHER is a scripture-ref-in-arbitrary-prose scanner:
/// <see cref="PlaceMentions"/> matches CURATED place/person NAMES already
/// attested to a specific verse (a closed candidate list per verse, no
/// chapter/verse NUMBER grammar at all); <see cref="CanonRef"/> only
/// parses/formats a ref string ALREADY KNOWN to be one (e.g. an xref
/// target, or the ScripturePicker's own composed sref) -- it has no
/// scanning/detection half. Server-side, `graph_wire`'s locus grammars
/// (`ConcordRef`/dot-ref encode/decode) are STRUCTURED-id round-trip
/// converters, not free-text citation scanners, and are server-only besides
/// (zero server changes permitted this batch). So this file is the minimal,
/// disclosed, newly-written one the brief anticipates -- not a hand-rolled
/// duplicate of something reachable.
///
/// GRAMMAR (verified against the real vendored prose,
/// `data/raw/kretzmann/genesis/1.html` and others, before writing this --
/// not guessed): "&lt;Book&gt; &lt;chapter&gt;[,:] &lt;verse&gt;[-&lt;verse&gt;]",
/// e.g. "John 1, 1" / "John 3:16" / "Cor. 4, 6" / "1 Cor. 15, 1-4". Book
/// tokens are the app's OWN canonical KJV names (from the already-fetched
/// `BookTocEntry` list, `AtlasClient.Books()` -- never re-guessed) PLUS a
/// small, disclosed table of the short forms this corpus's prose actually
/// uses (verified against real samples: "Cor.", "Ps.", "Pet.", "Gen.", ...),
/// mirroring `atlas-etl/src/kjv.rs`'s own `canon::resolve_alias` table in
/// SPIRIT (a citation alias is not a canonical name) without touching that
/// server-only code.
///
/// ACCEPTED, DISCLOSED LIMITATIONS (the same class <see cref="PlaceMentions"/>'s
/// own header already establishes for its domain, not a new standard):
/// (1) a citation naming a VERSE RANGE resolves to its FIRST verse only
/// (mirrors <see cref="CanonRef.TargetSpan"/>'s own precedent for
/// same-chapter xref targets -- this app's own "passage" concept is always
/// single-chapter); (2) an ambiguous bare abbreviation this batch's own
/// table does not carry (e.g. a citation style not observed in the real
/// vendored samples) is simply not matched -- a miss, never a
/// misattribution; (3) case-sensitive, whole-word matching only (no attempt
/// to resolve a reference split across a line/sentence boundary).
/// </summary>
public readonly record struct ScriptureRefMatch(int Start, int Length, string Sref);

public static class ScriptureRefScan
{
    // Citation ALIASES this corpus's own prose actually uses beyond a
    // book's full curated name (which the caller's own BookTocEntry list
    // already supplies) -- verified against real vendored HTML samples
    // (data/raw/kretzmann/**/*.html) before writing, not guessed. Ordered
    // book-by-book (canon order) for readability; matching itself is
    // length-ordered at scan time (see BuildAliasTable), not by this
    // declaration order.
    private static readonly (string Alias, string Code)[] CitationAliases =
    {
        ("Gen", "GEN"), ("Exod", "EXO"), ("Exo", "EXO"), ("Lev", "LEV"), ("Num", "NUM"),
        ("Deut", "DEU"), ("Deu", "DEU"), ("Josh", "JOS"), ("Judg", "JDG"),
        ("1 Sam", "1SA"), ("2 Sam", "2SA"), ("1 Kings", "1KI"), ("1 Kgs", "1KI"),
        ("2 Kings", "2KI"), ("2 Kgs", "2KI"), ("1 Chron", "1CH"), ("1 Chr", "1CH"),
        ("2 Chron", "2CH"), ("2 Chr", "2CH"), ("Neh", "NEH"), ("Esth", "EST"),
        ("Ps", "PSA"), ("Pss", "PSA"), ("Psalm", "PSA"), ("Prov", "PRO"),
        ("Eccl", "ECC"), ("Isa", "ISA"), ("Jer", "JER"), ("Lam", "LAM"),
        ("Ezek", "EZE"), ("Eze", "EZE"), ("Dan", "DAN"), ("Hos", "HOS"),
        ("Obad", "OBA"), ("Mic", "MIC"), ("Nah", "NAH"), ("Hab", "HAB"),
        ("Zeph", "ZEP"), ("Hag", "HAG"), ("Zech", "ZEC"), ("Mal", "MAL"),
        ("Matt", "MAT"), ("Mat", "MAT"), ("Mk", "MAR"), ("Lk", "LUK"),
        ("Rom", "ROM"), ("1 Cor", "1CO"), ("Cor", "1CO"), ("2 Cor", "2CO"),
        ("Gal", "GAL"), ("Eph", "EPH"), ("Phil", "PHP"), ("Php", "PHP"),
        ("Col", "COL"), ("1 Thess", "1TH"), ("1 Thes", "1TH"), ("2 Thess", "2TH"),
        ("2 Thes", "2TH"), ("1 Tim", "1TI"), ("2 Tim", "2TI"), ("Tit", "TIT"),
        ("Philem", "PHM"), ("Heb", "HEB"), ("Jas", "JAM"),
        ("1 Pet", "1PE"), ("Pet", "1PE"), ("2 Pet", "2PE"),
        ("1 John", "1JO"), ("2 John", "2JO"), ("3 John", "3JO"),
        ("Jude", "JDE"), ("Rev", "REV"),
    };

    /// <summary>
    /// Book-token alternation built fresh from the caller's own already-fetched
    /// TOC (its curated full names, e.g. "Song of Solomon", "I Corinthians")
    /// plus <see cref="CitationAliases"/> -- never a hardcoded name list, so
    /// this scanner always agrees with whatever canon THIS app's own
    /// `/api/books` actually serves. Longest-alias-first (an alternation
    /// tries left-to-right; "1 Corinthians" must be offered before "1 Cor"
    /// or the shorter alias would win and strand " inthians" as plain text).
    /// </summary>
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

        // "<book>[.]? <chapter>[,:] <verse>[-<verse>]" -- an optional
        // trailing period on the book token (the abbreviated forms this
        // corpus's prose actually carries: "Gen.", "Cor.") and a single
        // required space before the chapter number; comma-or-colon before
        // the verse (both forms verified in real samples, see this file's
        // own header). Word-boundary anchors on the book token itself
        // (`\b`) so "John" inside a longer word never matches (mirrors
        // PlaceMentions.FindAll's own word-boundary guard).
        return new Regex(
            @"\b(?<book>" + bookAlternation + @")\.?\s+(?<chapter>\d{1,3})[,:]\s*(?<verse>\d{1,3})(?:-(?<verse2>\d{1,3}))?",
            RegexOptions.Compiled);
    }

    /// <summary>
    /// Scans <paramref name="text"/> for scripture citations, resolving each
    /// against <paramref name="toc"/>'s own canonical codes. Returns matches
    /// in left-to-right order, non-overlapping (a citation regex match never
    /// overlaps another by construction -- unlike <see cref="PlaceMentions"/>'s
    /// candidate-pool arbitration, there is only ever one alternation match
    /// per position here).
    /// </summary>
    public static IReadOnlyList<ScriptureRefMatch> Scan(string text, IReadOnlyList<BookTocEntry> toc)
    {
        if (string.IsNullOrEmpty(text) || toc.Count == 0)
        {
            return Array.Empty<ScriptureRefMatch>();
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

        var results = new List<ScriptureRefMatch>();
        foreach (Match m in pattern.Matches(text))
        {
            if (!codeByAlias.TryGetValue(m.Groups["book"].Value, out var code))
            {
                continue; // unreachable in practice (the alternation IS this table's own keys), defensive only
            }

            var chapter = m.Groups["chapter"].Value;
            var verse = m.Groups["verse"].Value; // range collapses to its FIRST verse -- see this file's own header, limitation (1)
            results.Add(new ScriptureRefMatch(m.Index, m.Length, $"{code}.{chapter}.{verse}"));
        }

        return results;
    }
}
