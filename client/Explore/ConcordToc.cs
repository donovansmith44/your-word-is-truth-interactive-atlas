namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch CORPREAD-1b (ticket C, owner order verbatim: "I should have a menu
/// through which I can navigate to different parts in the BoC"): the ten
/// traditional documents of the Book of Concord, in canonical part order --
/// "the ConcordRef part→article grammar as the data provides ... the
/// traditional documents: Creeds, AC, Apology, Smalcald, Treatise, SC, LC,
/// FC as they exist in the corpus."
///
/// DISCLOSED: this table is a plain client-side constant, not served by any
/// endpoint (Concord has no TOC/structure route -- CORP-1's own report
/// confirms the only Concord-serving endpoint is the generic reading spine,
/// `GET /api/text?corpus=concord`), so it is hand-authored here rather than
/// fetched -- per this batch's own zero-server-changes rule. It mirrors,
/// verbatim, `server/atlas-etl/src/concord.rs`'s own
/// <c>pub const DOCUMENTS: &amp;[ConcordDocSpec]</c> table (part number,
/// title) -- READ directly before writing this, not guessed -- so it names
/// exactly the same ten documents, in the same part order, the server
/// itself already parses from the real vendored corpus
/// (`data/raw/concord/*.html`). Server-only code was not touched; this is a
/// disclosed, independent client-side copy of already-stable, already-real
/// data (the same class of thing `AtlasClient.Books()`'s own curated TOC
/// already is, just not reachable via a fetch for this corpus).
///
/// Fix round (S-7, TRIVIA -- review): an earlier draft of this comment (and
/// Concord.razor's/CONTRACT.md's own mirroring text) named this table
/// "CONCORD_DOC_SPECS" -- that is the RECORD TYPE's own name
/// (<see cref="ConcordDocEntry"/> here, `ConcordDocSpec` server-side), not
/// the constant itself (`DOCUMENTS`). The CONTENT mirror was always exact
/// (all ten part/title pairs verified identical, unchanged by this fix) --
/// this was a citation slip only, corrected here.
/// </summary>
public sealed record ConcordDocEntry(int Part, string Title);

public static class ConcordToc
{
    public static readonly IReadOnlyList<ConcordDocEntry> Documents = new[]
    {
        new ConcordDocEntry(1, "Preface to the Book of Concord"),
        new ConcordDocEntry(2, "The Three Ecumenical Creeds"),
        new ConcordDocEntry(3, "The Augsburg Confession"),
        new ConcordDocEntry(4, "Apology of the Augsburg Confession"),
        new ConcordDocEntry(5, "The Smalcald Articles"),
        new ConcordDocEntry(6, "Treatise on the Power and Primacy of the Pope"),
        new ConcordDocEntry(7, "The Small Catechism"),
        new ConcordDocEntry(8, "The Large Catechism"),
        new ConcordDocEntry(9, "Formula of Concord: Epitome"),
        new ConcordDocEntry(10, "Formula of Concord: Solid Declaration"),
    };

    /// <summary>The ref this document's own reading flow begins at -- every
    /// document root starts its own article/paragraph numbering fresh at
    /// 1.1 (verified against concord.rs's own parser, which numbers each
    /// document independently), matching the SAME "part.1.1" fallback the
    /// pre-existing numeric picker already assumes for an unfilled
    /// article/paragraph.</summary>
    public static string StartRef(int part) => $"BoC {part}.1.1";

    public static string TitleOf(int part) => Documents.FirstOrDefault(d => d.Part == part)?.Title ?? $"Part {part}";
}
