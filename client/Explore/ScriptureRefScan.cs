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
/// single-chapter); (2) an unlisted abbreviation (a citation style not
/// observed in the real vendored samples) is simply not matched -- a miss,
/// never a misattribution -- see <see cref="CitationAliases"/>'s own header
/// for the fix-round history of making this literally true; (3)
/// case-sensitive, whole-word matching only (no attempt to resolve a
/// reference split across a line/sentence boundary).
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
    // CODES verified live against THIS app's own real GET /api/books
    // response (not re-guessed from an ETL source file a second time) --
    // a real, live-caught bug (KRETZMANN-10's own first Playwright run):
    // an earlier draft of this table copied several codes from
    // `data/raw/brain-fuel-bible/data/books.json` verbatim, which turned
    // out to disagree with this app's OWN canonical codes for several
    // books (e.g. John is "JHN" here, not that source's "JOH"; Ezekiel is
    // "EZK", not "EZE"; Mark is "MRK", not "MAR"; James is "JAS", not
    // "JAM"; Jude is "JUD", not "JDE"; Nahum is "NAM", not "NAH"; the
    // Johannine epistles are "1JN"/"2JN"/"3JN", not "1JO"/"2JO"/"3JO") --
    // corrected here. This particular mismatch was HARMLESS for a book
    // matched by its own full curated NAME (the live-fetched TOC always
    // supplies the correct code for that path, regardless of this table),
    // but would have been a real, silent misattribution for a book ONLY
    // ever matched via one of these short ALIASES.
    //
    // Fix round (S-5, IMPORTANT -- review): three earlier entries here
    // ("Cor"->1CO, "Pet"->1PE, "Phil"->PHP) resolved a genuinely AMBIGUOUS
    // bare abbreviation BY GUESS (a bare "Cor." is at least as often an
    // elided 2 Corinthians as 1; "Phil" is ambiguous between Philippians and
    // Philemon) -- contradicting this file's own disclosed-limitations
    // claim ("an unlisted abbreviation is a miss, never a misattribution").
    // The review verified the exposure empirically against BOTH vendored
    // corpora: zero bare "Cor."/"Pet." occurrences in data/raw/kretzmann/
    // (every real occurrence there is already preceded by "1 "/"2 ", which
    // the longer, unambiguous aliases below correctly route), and every
    // "Cor."/"Pet." occurrence in data/raw/concord/*.html is likewise
    // preceded by "1 "/"2 " -- so the three entries were latent, not live,
    // and unused by real prose. Deleted rather than kept "for future
    // corpora" -- an unlisted abbreviation is a miss (silent, safe) and a
    // future corpus that genuinely needs a bare "Cor."/"Pet."/"Phil." can
    // add it back with its OWN verified default, not inherit this batch's
    // unverified guess.
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

    // Fix round (Q-1, CRITICAL -- review): BuildPattern/the alias dictionary
    // used to be rebuilt (sort ~130 tokens, Regex.Escape each, compile a
    // fresh RegexOptions.Compiled pattern) on EVERY Scan() call -- and
    // Scan() runs once per ScriptureRefText instance, on every one of that
    // component's renders (Kretzmann.LoadCommentaryAsync alone calls
    // StateHasChanged three times per chapter navigation, plus every
    // popover open/close). Genesis 1 renders ~31 instances; the review
    // measured this as "on the order of 90+ compiled 130-branch regexes"
    // per chapter turn -- inside Blazor WASM, on the app's primary reading
    // surface. Memoized here: the caller's own `toc` reference is the SAME
    // `List<BookTocEntry>` instance every render (AtlasClient.Books() is a
    // singleton-cached `??=` -- see that method's own header -- so every
    // caller across the whole app shares ONE instance for the app's entire
    // lifetime), so a plain "last built against THIS reference" cache is
    // correct and never stale: the TOC is fetched once and never mutated,
    // and CitationAliases is a `static readonly` compile-time constant --
    // there is no code path that could change either input out from under
    // a cached build. Not a ConditionalWeakTable (unnecessary complexity for
    // a single long-lived key) -- the review's own sanctioned simpler
    // alternative.
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

        var (pattern, codeByAlias) = GetOrBuild(toc);

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
