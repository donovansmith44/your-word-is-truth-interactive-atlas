using BibleAtlas.Client.Contract;
using System.Collections.Generic;
using System.Linq;

namespace BibleAtlas.Client.Explore;

public static class ProvenanceResolver
{
    // Mirrors atlas_core::sources::split_provenance_id exactly: an id is `kind` or
    // `kind/locator`. Split at the FIRST slash only, so a multi-segment locator
    // (e.g. "jeremiah/1") stays intact.
    public static (string Kind, string? Locator) SplitId(string id)
    {
        var slash = id.IndexOf('/');
        return slash < 0 ? (id, null) : (id[..slash], id[(slash + 1)..]);
    }

    public static ResolvedProvenance Resolve(SourcesDocument? doc, string id)
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
        var row = (doc.Provenances ?? []).FirstOrDefault(p => p.Id == kind);
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
            // The row's own locator (the id's suffix) wins over the registry's general one:
            // "jeremiah/1" is what THIS row cites; the registry's is what the source generally covers.
            Locator: locator ?? row.Locator);
    }

    // Deliberately does not filter out blank ids: an empty list still renders nothing (no
    // provenance section at all), but a list CONTAINING a blank id resolves and renders a
    // loud notice, because something claimed an attribution and handed over nothing. Blanks
    // are normalized (via NormalizeId) before Distinct so several blank flavors collapse to
    // one notice, not one per flavor.
    public static IReadOnlyList<ResolvedProvenance> ResolveAll(SourcesDocument? doc, IEnumerable<string>? ids) =>
        (ids ?? Enumerable.Empty<string>())
            .Select(NormalizeId)
            .Distinct()
            .Select(id => Resolve(doc, id))
            .ToList();

    public static string NormalizeId(string? id) => string.IsNullOrWhiteSpace(id) ? "" : id;

    // "Imported" (the default register of this atlas) deliberately has no note here --
    // most rows come from a named outside corpus, and saying so on every popover would be
    // noise. Returning null means "render no confidence line at all," never an empty one.
    public static string? ConfidenceNote(Confidence? confidence) => confidence switch
    {
        Confidence.CanonicalText => "Canonical text",
        Confidence.Curated => "Our own curated work",
        Confidence.Derived => "Derived by this project's pipeline",
        _ => null,
    };
}

public sealed record ResolvedProvenance(
    string Id,
    string Title,
    string WhatItIs,
    string License,
    string? Link,
    string? CategoryLabel,
    Confidence? Confidence,
    string? Locator,
    ProvenanceStatus Status = ProvenanceStatus.Resolved)
{
    public bool IsResolved => Status == ProvenanceStatus.Resolved;

    public static ResolvedProvenance Unresolved(string id) =>
        new(Id: id, Title: "", WhatItIs: "", License: "", Link: null, CategoryLabel: null, Confidence: null, Locator: null,
            Status: ProvenanceStatus.Unresolved);

    public static ResolvedProvenance RegistryUnavailable(string id) =>
        new(Id: id, Title: "", WhatItIs: "", License: "", Link: null, CategoryLabel: null, Confidence: null, Locator: null,
            Status: ProvenanceStatus.RegistryUnavailable);
}

// Unresolved: the registry loaded but has no row for this id (or the row dangles) -- a data
// bug worth a loud, named notice. RegistryUnavailable: the registry itself failed to load,
// so nothing has been learned about the data -- must not be shown as the same accusation.
public enum ProvenanceStatus
{
    Resolved,
    Unresolved,
    RegistryUnavailable,
}
