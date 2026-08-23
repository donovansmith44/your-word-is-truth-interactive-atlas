namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch M-D2 (P7 closure, "the popover section registry keys sections by
/// EDGE KIND with per-kind display rules" -- the controller brief's own
/// words): a small, REAL C# realization of the design spec's own
/// <c>SectionSpec</c> shape (§7: <c>{ kind, renderer, style, initial,
/// order }</c>) -- per-edge-kind display POLICY, keyed by the wire's own
/// edge-kind label string (<c>"cites"</c>, ...), consulted by
/// <see cref="CrossRefsSection"/> for the ONE number ("3") both its general
/// xrefs-only cap (F2, unchanged) and its NEW entry-point cap (this batch)
/// must agree on -- one source of truth for "how many xref entries show
/// initially," read from a registry, not duplicated as two separate
/// integer literals in two separate branches of one ternary.
///
/// STRANGLER SCOPE, disclosed (batch brief controller decision 1, "surfaces
/// not touched this batch may stay bespoke"): this registry governs DISPLAY
/// POLICY (cap/order/style) for the ONE edge kind the superscript work
/// actually touches (<c>cites</c>). It does NOT (yet) drive
/// <see cref="CrossRefsSection"/>'s own DATA FETCH -- that stays on the
/// existing bespoke path (<c>VerseDetail.CrossRefs</c> /
/// <c>PassageNode.XrefsAsync</c>), which resolves each target's own FULL
/// member-verse text for a same-chapter range (F2's own enrichment, e.g.
/// "COL.1.16-19" renders all four verses' real text). The generic
/// <c>cites</c> edge (design doc §4: "verse-level today, loci by design")
/// stores only a target's FIRST verse -- `to_last`/`target_display` live on
/// the AUTHORED `CrossRef` row, never lowered into the generic edge index's
/// `EdgeMeta` (only `Votes` is). Migrating the FETCH to the generic contract
/// this batch would silently truncate ~25% of real cross-reference previews
/// (F2's own measured figure) from their full range down to one verse --
/// widening `EdgeMeta` to carry range data is a real relation-shape decision
/// ("reviewed like any relation change," graph-types' own law), correctly
/// bigger than this batch's own scope, not attempted here. What DOES move to
/// the generic contract this batch: the superscript's own COUNT
/// (<c>VerseOut.XrefCount</c>, server-side, via <c>GraphQuery::edge_summary</c>
/// -- a genuinely NEW capability with no bespoke predecessor) and this
/// display-policy registry itself. See CONTRACT.md's own M-D2 strangler
/// inventory for the full, itemized list.
/// </summary>
public enum SectionStyle
{
    Standard,

    /// A superscript-marker-capable section: quiet accent styling
    /// (>=7:1 floor, not body-text's >=10:1), and eligible to serve as an
    /// ENTRY POINT (owner decree: the superscript is an entry point into
    /// THE one composable popover, never a parallel interface).
    Quiet,
}

public enum SectionOrder
{
    /// The edge's own ranking metadata already determines page order
    /// server-side (design types doc §3: <c>EdgeMeta::Votes</c>) -- entries
    /// arrive PRE-ORDERED; no client-side re-sort is ever needed or
    /// performed (server_atlas_server/tests/graph_api.rs's own
    /// `generic_cites_edges_are_already_votes_descending_...` test pins
    /// this as a tested fact, not an assumed one).
    VotesRanked,

    /// Batch P: the design types doc's own §7 vocabulary
    /// (<c>votes-ranked | chain | canonical | resolved-date</c>) --
    /// "canonical" realized for the first time. `mentioned-in` (a Person's
    /// own frontier of every verse mentioning them) is canon-ordered BY
    /// CONSTRUCTION, not by client-side sorting: `atlas_etl::people::
    /// parse_people` explicitly canon-sorts each person's own resolved
    /// verse_links before the graph adapter ever sees them, and the
    /// generic port's own BiIndex preserves row-insertion order end to
    /// end -- no re-sort exists anywhere on this path, server or client
    /// (server_atlas_graph's own person_adapter tests pin the row order as
    /// a tested fact). `mentions` (a verse's own forward frontier of
    /// places+persons) shares this label too: a single locus has no
    /// canon-ORDER distinction to violate among its own entries (there is
    /// only one locus), so "canonical" is vacuously true there, not a
    /// second, different ranking scheme.
    Canonical,
}

/// One edge kind's own display policy -- style/initial-clamp/order, per the
/// design spec's <c>SectionSpec</c> shape. <see cref="Renderer"/> is
/// deliberately NOT modeled here: every kind this registry governs today
/// renders through the SAME shared <see cref="Components.PassageList"/>
/// entry-list renderer (design doc §7's own "entry-list | text-flow |
/// map-pins | timeline-rows" vocabulary has exactly one member in live use
/// client-side so far) -- adding a field with only ever one value would be
/// ceremony, not a real seam; a second renderer kind, when one is actually
/// needed, is the moment to add it.
public sealed record EdgeSectionSpec(string EdgeKind, SectionStyle Style, int InitialClamp, SectionOrder Order);

/// The registry itself: a flat map, edge-kind label -> its own display
/// policy. Registering a new kind here is the whole of teaching a NEW
/// edge-kind-driven section its own display rules (P5: "a new node/edge
/// kind is a record + compiler rule + display rule," realized client-side).
public static class EdgeSectionRegistry
{
    /// The <c>cites</c> relation's own policy: quiet-accent styling
    /// (superscript-eligible), 3 initial entries (the owner's own words,
    /// "shows 3 explorable verses to start" -- also F2's pre-existing
    /// xrefs-only general-popover cap, so this ONE number correctly serves
    /// BOTH the entry-point case and the unchanged general case), votes-
    /// ranked order (already the wire's own order -- see
    /// <see cref="SectionOrder.VotesRanked"/>'s own doc comment).
    public static readonly EdgeSectionSpec Cites = new("cites", SectionStyle.Quiet, InitialClamp: 3, SectionOrder.VotesRanked);

    /// Batch P: the verse/passage popover's own PERSONS section
    /// (<c>VersePersonsSection</c>) reads this for its ONE fetch's `limit`
    /// -- a real verse's own total mentions (places+persons combined) is
    /// always small (spot-checked against the real compiled data: no
    /// verse comes remotely close to this), so a single generous page
    /// safely captures the complete set for virtually every real verse;
    /// see that provider's own doc comment for the honest, disclosed
    /// fallback on the rare chance it somehow doesn't.
    public static readonly EdgeSectionSpec Mentions = new("mentions", SectionStyle.Standard, InitialClamp: 50, SectionOrder.Canonical);

    /// Batch P: a Person's own "mentioned-in" frontier
    /// (<c>PersonCardAndMentionsSection</c>/<c>PersonMentionsList</c>) --
    /// FIRST page size only (a busy person, e.g. real committed data's own
    /// "David" at 896 mentions, needs genuine server-side pagination, not
    /// a client-side reveal over an already-fully-fetched list -- see
    /// <c>PersonMentionsList.razor</c>'s own header comment for why this
    /// is the one section in this app that fetches a SECOND real page on
    /// reveal instead of just un-hiding already-held rows).
    public static readonly EdgeSectionSpec MentionedIn = new("mentioned-in", SectionStyle.Standard, InitialClamp: 12, SectionOrder.Canonical);

    public static readonly IReadOnlyDictionary<string, EdgeSectionSpec> ByKind =
        new Dictionary<string, EdgeSectionSpec> { [Cites.EdgeKind] = Cites, [Mentions.EdgeKind] = Mentions, [MentionedIn.EdgeKind] = MentionedIn };
}
