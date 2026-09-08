using System.Text.Json;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

// Batch PROV-1: the CLIENT-SIDE half of the resolution law, run against the
// REAL committed registry -- data/compiled/sources.json, deserialized
// through this app's own wire DTOs and its own Wire.Options policy, exactly
// as the browser receives it from GET /api/sources.
//
// Why this exists alongside the Rust law: those are two different claims.
// server/atlas-graph/tests/provenance_registry_real_data.rs proves the
// registry COVERS the artifact (no piece of data lacks a source). This file
// proves the client can actually READ that registry -- that the JSON policy
// carries the new table across the wire at all, and that the ids the owner
// named by hand resolve to the sources he named them for. A snake_case
// mapping bug would leave the Rust law perfectly green while every "?" in
// the browser reported "Unrecognized source" (the exact class of bug
// CatechismProofVerseDto.Vref's own doc comment records this project
// already shipping once).
public class ProvenanceRegistryConformanceTests
{
    private static SourcesDocumentOut RealRegistry()
    {
        // client.Tests runs from its own bin directory; the repo root is
        // four levels up (bin/{Config}/{Tfm}/ under client.Tests/). Resolved
        // by walking UP to the directory that actually contains data/, so
        // this keeps working if the TFM or configuration path changes.
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null && !Directory.Exists(Path.Combine(dir.FullName, "data", "compiled")))
        {
            dir = dir.Parent;
        }
        Assert.NotNull(dir);

        var path = Path.Combine(dir!.FullName, "data", "compiled", "sources.json");
        var doc = JsonSerializer.Deserialize<SourcesDocumentOut>(File.ReadAllText(path), Wire.Options);
        Assert.NotNull(doc);
        return doc!;
    }

    [Fact]
    public void TheCommittedRegistryCarriesAProvenanceTableAcrossThisAppsOwnJsonPolicy()
    {
        // The whole-table smoke test: a snake_case/casing mismatch on the
        // new field deserializes to null, silently, and every affordance in
        // the app goes unresolved at once.
        Assert.NotEmpty(RealRegistry().ProvenancesOrEmpty);
    }

    [Theory]
    // The owner's own headline case, verbatim: "sourced from openbible.com".
    [InlineData("openbible.info-cross-references", "OpenBible.info Cross-References")]
    // The KJV verse a reader is actually looking at.
    [InlineData("kjv", "The King James Version")]
    // An imported event.
    [InlineData("theographic", "Theographic Bible Metadata")]
    // TOTAL-CAPTURE HONESTY: a hand-authored event, and the ATTEST-1
    // hand-repaired attestation rows the leper case turned on. Both must
    // land in "Our Own Curated Work" and nowhere else.
    [InlineData("curated", "Our Own Curated Work")]
    [InlineData("attestation-corrections", "Our Own Curated Work")]
    public void TheIdsTheOwnerNamedResolveToTheSourcesHeNamedThemFor(string provenanceId, string expectedTitle)
    {
        var resolved = ProvenanceResolver.Resolve(RealRegistry(), provenanceId);

        Assert.True(resolved.IsResolved, $"'{provenanceId}' did not resolve against the committed registry");
        Assert.Equal(expectedTitle, resolved.Title);
        Assert.NotEqual("", resolved.License);
    }

    [Fact]
    public void ACuratedRowAnnouncesItselfSoItCannotWearAnImportedSourcesClothes()
    {
        // The leper lesson, end to end through the real registry: the
        // repaired attestation rows must not merely resolve -- they must
        // resolve to something a reader can TELL APART from Theographic.
        var registry = RealRegistry();
        var curated = ProvenanceResolver.Resolve(registry, "attestation-corrections");
        var imported = ProvenanceResolver.Resolve(registry, "theographic");

        Assert.Equal("Our own curated work", ProvenanceResolver.ConfidenceNote(curated.Confidence));
        Assert.Null(ProvenanceResolver.ConfidenceNote(imported.Confidence));
        Assert.NotEqual(curated.Title, imported.Title);
    }

    [Fact]
    public void EveryProvenanceRowInTheCommittedRegistryResolvesThroughThisAppsOwnResolver()
    {
        // The client-side mirror of the Rust law's "no dangling source"
        // clause: if the resolver cannot follow a declared row all the way
        // to a title, that row is dead weight at the browser regardless of
        // what the server thinks.
        var registry = RealRegistry();
        var unresolved = registry.ProvenancesOrEmpty
            .Select(p => ProvenanceResolver.Resolve(registry, p.Id))
            .Where(r => !r.IsResolved)
            .Select(r => r.Id)
            .ToList();

        Assert.True(unresolved.Count == 0, $"declared provenance rows that this app cannot resolve: {string.Join(", ", unresolved)}");
    }

    [Fact]
    public void EveryConfidenceInTheCommittedRegistryIsOneThisAppKnowsHowToDisplay()
    {
        // "Display" includes deliberately displaying NOTHING (Imported).
        // What must never happen is a value outside the vocabulary, which
        // would silently fall through ConfidenceNote's own default arm and
        // be indistinguishable from Imported at the reader -- so this
        // asserts membership, not renderability.
        var vocabulary = new[] { "CanonicalText", "Curated", "Imported", "Derived" };
        var strays = RealRegistry().ProvenancesOrEmpty
            .Where(p => !vocabulary.Contains(p.Confidence))
            .Select(p => $"{p.Id}={p.Confidence}")
            .ToList();

        Assert.True(strays.Count == 0, $"off-vocabulary confidence values: {string.Join(", ", strays)}");
    }
}
