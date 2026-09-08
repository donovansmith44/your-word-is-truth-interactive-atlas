using System.Collections.Generic;
using System.Linq;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch PROV-1 (owner order 2, verbatim: "add a ? button on our frontier
/// interface that gives provenance (i.e., sourced from openbible.com) or
/// whatever"): THE ONE place a provenance id becomes something a reader can
/// read.
///
/// <para>THE RESOLUTION LAW, and it is fail-loud. A provenance id that does
/// not resolve to a registry source is a LOUD, named error --
/// <see cref="ResolvedProvenance.Unresolved"/>, which
/// <c>ProvenanceAffordance.razor</c> renders as a visible notice carrying
/// the offending id -- never a silent blank and never a fabricated label.
/// The server side of the same law is
/// <c>atlas-graph/tests/provenance_registry_real_data.rs</c>, which fails
/// the BUILD if any distinct provenance id in the compiled artifact has no
/// registry row; this class is what happens if that law is ever
/// circumvented at runtime anyway.</para>
///
/// <para>NO NEW FETCH, EVER. Resolution reads the
/// <see cref="SourcesDocumentOut"/> the app has already fetched --
/// <c>AtlasClient.Sources()</c> is <see cref="AsyncMemo{T}"/>-backed, so the
/// first popover to ask triggers one request and every popover afterward
/// (and the Sources page itself) shares that exact task. The sub-100ms
/// frontier law is why: a per-popover fetch would put a round trip inside
/// the frontier's own budget. <c>tests/ux/provenance.spec.ts</c> asserts the
/// single fetch by counting requests, not by reading this comment.</para>
///
/// <para>THE ID GRAMMAR, mirrored EXACTLY from
/// <c>atlas_core::sources::split_provenance_id</c> (server): an id is
/// <c>kind</c> or <c>kind/locator</c>. The KIND is the registry key; the
/// remainder, when present, is THIS ROW's own locator -- which is precisely
/// the {source, locator} shape <c>atlas_graph_types::ingest::Provenance</c>
/// has declared since the ingest vocabulary was written. The suffix half is
/// not a convenience: <c>kretzmann_adapter</c> mints one provenance id per
/// commentary unit, so an exact-match-only registry would need tens of
/// thousands of curated rows to state one true fact. Split at the FIRST
/// slash, so a multi-segment locator ("jeremiah/1") stays intact.</para>
///
/// <para>Static and pure by construction -- it takes the document as a
/// parameter rather than injecting a client. That is what lets
/// <c>client.Tests</c> assert the law over hand-built registries (including
/// the unresolved case, which no real document can produce while the
/// server-side law holds) with no HTTP anywhere.</para>
/// </summary>
public static class ProvenanceResolver
{
    /// <summary>
    /// The id grammar, in one method. Mirrors
    /// <c>atlas_core::sources::split_provenance_id</c> exactly -- see this
    /// class's own doc comment.
    /// </summary>
    public static (string Kind, string? Locator) SplitId(string id)
    {
        var slash = id.IndexOf('/');
        return slash < 0 ? (id, null) : (id[..slash], id[(slash + 1)..]);
    }

    /// <summary>
    /// Resolves one provenance id against the already-fetched registry.
    /// Never throws, never returns null: an id with no registry row comes
    /// back as <see cref="ResolvedProvenance.Unresolved"/> carrying the id
    /// itself, so the affordance can say exactly what it could not name.
    /// </summary>
    public static ResolvedProvenance Resolve(SourcesDocumentOut? doc, string id)
    {
        if (string.IsNullOrWhiteSpace(id))
        {
            return ResolvedProvenance.Unresolved(id ?? "");
        }

        var (kind, locator) = SplitId(id);
        // Registry rows are unique by id (server: `validate_structure`
        // rejects a duplicate), so First is the whole answer -- but this
        // asks for the row and the source SEPARATELY rather than assuming
        // the join holds, because a dangling `source` must surface as
        // unresolved here rather than as a NullReferenceException.
        var row = doc?.ProvenancesOrEmpty.FirstOrDefault(p => p.Id == kind);
        if (row is null)
        {
            return ResolvedProvenance.Unresolved(id);
        }

        var source = doc!.Sources.FirstOrDefault(s => s.Id == row.Source);
        if (source is null)
        {
            return ResolvedProvenance.Unresolved(id);
        }

        var category = doc.Categories.FirstOrDefault(c => c.Id == source.Category)?.Label;
        return new ResolvedProvenance(
            Id: id,
            Title: source.Title,
            WhatItIs: source.WhatItIs,
            License: source.License,
            Link: source.Link,
            CategoryLabel: category,
            Confidence: row.Confidence,
            // The ROW's own locator (the id's suffix) wins over the
            // registry's general one: "jeremiah/1" is what THIS row cites;
            // the registry's is what the SOURCE generally covers. Falling
            // back keeps a bare id useful without inventing anything.
            Locator: locator ?? row.Locator);
    }

    /// <summary>
    /// Resolves a whole section's worth of ids, in the order given,
    /// de-duplicated by id. A section legitimately drawing on two sources
    /// shows BOTH -- THE SMALL CATECHISM really does
    /// (<c>curated-catechism</c> + <c>concord-sc-overlap</c>, measured and
    /// pinned server-side) -- because collapsing them to one is exactly the
    /// leper failure mode this affordance exists to make visible.
    /// </summary>
    public static IReadOnlyList<ResolvedProvenance> ResolveAll(SourcesDocumentOut? doc, IEnumerable<string>? ids) =>
        (ids ?? Enumerable.Empty<string>())
            .Where(id => !string.IsNullOrWhiteSpace(id))
            .Distinct()
            .Select(id => Resolve(doc, id))
            .ToList();

    /// <summary>
    /// Batch PROV-1, the CONFIDENCE-DISPLAY rule (brief: "Confidence is
    /// shown when it is not the obvious default -- a CanonicalText claim
    /// and a Derived claim must not look alike").
    ///
    /// <para>"Imported" is the default register of this atlas -- most rows
    /// come from a named outside corpus, and saying so on every popover
    /// would be noise that trains the eye to skip the line where it
    /// matters. The other three each carry a claim a reader should see:
    /// CanonicalText says "this is Scripture's own text," Curated says "we
    /// wrote this ourselves," Derived says "no one asserted this directly;
    /// the pipeline computed it." Returning null means "render no
    /// confidence line at all," never "render an empty one."</para>
    /// </summary>
    public static string? ConfidenceNote(string? confidence) => confidence switch
    {
        "CanonicalText" => "Canonical text",
        "Curated" => "Our own curated work",
        "Derived" => "Derived by this project's pipeline",
        _ => null,
    };
}

/// <summary>
/// Batch PROV-1: one provenance id, resolved into the four things the
/// affordance renders -- the source title as the owner phrased it ("Sourced
/// from OpenBible" register), what it is in one line, the license, and the
/// locator when one exists -- plus the confidence and the category label
/// ("Our Own Curated Work") that carry the total-capture honesty claim.
///
/// <see cref="IsResolved"/> false is the fail-loud case: every display
/// field is empty and <see cref="Id"/> holds the id that could not be
/// named, so the affordance says so out loud instead of rendering blank.
/// </summary>
public sealed record ResolvedProvenance(
    string Id,
    string Title,
    string WhatItIs,
    string License,
    string? Link,
    string? CategoryLabel,
    string Confidence,
    string? Locator)
{
    public bool IsResolved => Title.Length > 0;

    public static ResolvedProvenance Unresolved(string id) =>
        new(Id: id, Title: "", WhatItIs: "", License: "", Link: null, CategoryLabel: null, Confidence: "", Locator: null);
}
