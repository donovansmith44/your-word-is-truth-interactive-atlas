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
    ///
    /// <para>FIX ROUND 1 (review M-4): a NULL document is no longer the same
    /// answer as an unregistered id. They are different failures and the
    /// reader must be told which one happened. `null` means the REGISTRY
    /// could not be loaded -- an infrastructure fault about US, transient
    /// (<see cref="AsyncMemo{T}"/> resets on fault, so the next fetch
    /// self-heals) and no evidence whatsoever about the data. Rendering it
    /// as <c>Unrecognized source "kjv". Please report it.</c> accused
    /// perfectly well-registered data of having no source, on EVERY
    /// affordance on the popover, and asked the reader to report a bug that
    /// does not exist. It now resolves to
    /// <see cref="ResolvedProvenance.RegistryUnavailable"/>, which says so.
    /// An id the registry genuinely does not contain keeps the loud
    /// notice -- that one IS a piece of data with no source.</para>
    /// </summary>
    public static ResolvedProvenance Resolve(SourcesDocumentOut? doc, string id)
    {
        if (doc is null)
        {
            return ResolvedProvenance.RegistryUnavailable(id ?? "");
        }

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
        var row = doc.ProvenancesOrEmpty.FirstOrDefault(p => p.Id == kind);
        if (row is null)
        {
            return ResolvedProvenance.Unresolved(id);
        }

        var source = doc.Sources.FirstOrDefault(s => s.Id == row.Source);
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
    ///
    /// <para>FIX ROUND 1 (review H-1, HIGH): THE BLANK FILTER IS GONE, and
    /// its removal is the point of the fix. This method used to
    /// <c>.Where(id =&gt; !string.IsNullOrWhiteSpace(id))</c> before
    /// resolving, as did three other call sites; between them they made
    /// <see cref="Resolve"/>'s own blank-id branch DEAD CODE from the UI's
    /// perspective, while three source comments and the batch report
    /// asserted that a blank provenance renders as a LOUD notice. It did
    /// not. It rendered as NOTHING -- a curatorial claim with no
    /// attribution and no sign that attribution was missing, which is the
    /// silent blank the brief forbids and precisely how the ATTEST-1 leper
    /// row hid.</para>
    ///
    /// <para>THE DISTINCTION THAT SURVIVES, because it is a real one: an
    /// EMPTY LIST still renders nothing at all (conditional presence -- the
    /// surface has no provenance section, which is honest: a Passage's
    /// cross-refs before M-3, an event with no accounts). A list CONTAINING
    /// a blank id renders the loud notice, because something claimed to
    /// have an attribution and handed over nothing. "Nothing to say" and
    /// "something illegible to say" are not the same fact and no longer
    /// look the same.</para>
    ///
    /// <para>FIX ROUND 2 (review L-NEW-3): blanks are NORMALISED before
    /// <c>Distinct</c>, not merely null-coalesced. <c>""</c> and <c>"  "</c>
    /// are different strings but the same fact -- "we were handed nothing to
    /// attribute this with" -- and before this they deduped to two entries
    /// and rendered the identical "no source at all" notice TWICE. This is a
    /// normalisation, NOT a return of the filter H-1 removed: every blank
    /// still survives as an entry and still shouts, there is just one of it.
    /// Zero ids in, zero out; any number of blanks in, exactly one loud
    /// entry out.</para>
    /// </summary>
    public static IReadOnlyList<ResolvedProvenance> ResolveAll(SourcesDocumentOut? doc, IEnumerable<string>? ids) =>
        (ids ?? Enumerable.Empty<string>())
            .Select(NormalizeId)
            .Distinct()
            .Select(id => Resolve(doc, id))
            .ToList();

    /// <summary>
    /// One provenance id, in the form every surface de-duplicates on. Each
    /// flavour of "blank" (null, <c>""</c>, whitespace) becomes the SAME
    /// empty string, so a section handed several of them says "no source at
    /// all" once rather than once per flavour. Fix round 2, review L-NEW-3.
    /// Deliberately NOT a filter -- see <see cref="ResolveAll"/> on H-1.
    /// </summary>
    public static string NormalizeId(string? id) => string.IsNullOrWhiteSpace(id) ? "" : id;

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
/// <see cref="ProvenanceStatus.Unresolved"/> is the fail-loud case: every
/// display field is empty and <see cref="Id"/> holds the id that could not
/// be named, so the affordance says so out loud instead of rendering blank.
///
/// <para>FIX ROUND 1 (review M-4): there are now THREE states, not two,
/// because there were always three failures and only two renderings. See
/// <see cref="ProvenanceStatus"/>.</para>
/// </summary>
public sealed record ResolvedProvenance(
    string Id,
    string Title,
    string WhatItIs,
    string License,
    string? Link,
    string? CategoryLabel,
    string Confidence,
    string? Locator,
    ProvenanceStatus Status = ProvenanceStatus.Resolved)
{
    public bool IsResolved => Status == ProvenanceStatus.Resolved;

    /// <summary>A DATA fault: this id is not in the registry (or its row
    /// names a source that is not). The atlas is serving something it
    /// cannot attribute, which is worth shouting about.</summary>
    public static ResolvedProvenance Unresolved(string id) =>
        new(Id: id, Title: "", WhatItIs: "", License: "", Link: null, CategoryLabel: null, Confidence: "", Locator: null,
            Status: ProvenanceStatus.Unresolved);

    /// <summary>An INFRASTRUCTURE fault: the registry itself could not be
    /// loaded, so nothing can be resolved and nothing about the DATA has
    /// been learned. Transient (AsyncMemo resets on fault). Says so instead
    /// of accusing the corpus.</summary>
    public static ResolvedProvenance RegistryUnavailable(string id) =>
        new(Id: id, Title: "", WhatItIs: "", License: "", Link: null, CategoryLabel: null, Confidence: "", Locator: null,
            Status: ProvenanceStatus.RegistryUnavailable);
}

/// <summary>
/// Batch PROV-1 FIX ROUND 1 (review M-4): why an affordance could not name
/// a source. THREE outcomes, because a reader who is owed the truth is owed
/// a DIFFERENT truth in each case:
///
/// <list type="bullet">
/// <item><see cref="Resolved"/> -- it was named.</item>
/// <item><see cref="Unresolved"/> -- the registry loaded and does not
/// contain this id (or its row dangles). A piece of data with no source:
/// OUR data bug, and the loud red notice names the id so it can be
/// fixed.</item>
/// <item><see cref="RegistryUnavailable"/> -- the registry did not load at
/// all (<c>/api/sources</c> failed, or has not resolved yet). Nothing has
/// been learned about the data. Before this fix both non-Resolved cases
/// rendered the same accusation, so one dropped request made every "?" on
/// the popover tell the reader the atlas had unattributed data and ask them
/// to report a bug that did not exist.</item>
/// </list>
/// </summary>
public enum ProvenanceStatus
{
    Resolved,
    Unresolved,
    RegistryUnavailable,
}
