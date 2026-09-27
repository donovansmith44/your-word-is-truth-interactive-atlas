namespace BibleAtlas.Client.Explore;

public enum SectionStyle
{
    Standard,

    // >=7:1 contrast floor (not body text's >=10:1); eligible to serve as a
    // superscript entry point into the popover.
    Quiet,
}

public enum SectionOrder
{
    // The server already delivers entries votes-ranked; never re-sort here.
    VotesRanked,

    // Order comes from the server (canon-sorted before the graph adapter sees
    // it), or is vacuously true for a single-locus list; never re-sorted client-side.
    Canonical,
}

/// One edge kind's own display policy -- style/initial-clamp/order.
/// <see cref="Renderer"/> is deliberately not modeled: every kind this
/// registry governs today renders through the same shared entry-list renderer.
public sealed record EdgeSectionSpec(EdgeKindId EdgeKind, SectionStyle Style, int InitialClamp, SectionOrder Order);

public static class EdgeSectionRegistry
{
    // 3 serves both the general xrefs-only cap and the superscript entry-point
    // cap -- keep these in sync if either changes.
    public static readonly EdgeSectionSpec Cites = new(new EdgeKindId("cites"), SectionStyle.Quiet, InitialClamp: 3, SectionOrder.VotesRanked);

    // Assumes a verse's total mentions (places+persons) never exceeds this;
    // no pagination if it ever does.
    public static readonly EdgeSectionSpec Mentions = new(new EdgeKindId("mentions"), SectionStyle.Standard, InitialClamp: 50, SectionOrder.Canonical);

    // First page only -- a busy person (e.g. ~900 mentions) needs real
    // server-side pagination; PersonMentionsList.razor fetches a second page on reveal.
    public static readonly EdgeSectionSpec MentionedIn = new(new EdgeKindId("mentioned-in"), SectionStyle.Standard, InitialClamp: 12, SectionOrder.Canonical);

    public static readonly EdgeSectionSpec CommentedOnBy = new(new EdgeKindId("commented-on-by"), SectionStyle.Standard, InitialClamp: 20, SectionOrder.Canonical);

    public static readonly IReadOnlyDictionary<EdgeKindId, EdgeSectionSpec> ByKind =
        new Dictionary<EdgeKindId, EdgeSectionSpec>
        {
            [Cites.EdgeKind] = Cites,
            [Mentions.EdgeKind] = Mentions,
            [MentionedIn.EdgeKind] = MentionedIn,
            [CommentedOnBy.EdgeKind] = CommentedOnBy,
        };
}
