using BibleAtlas.Client.Contract;
using System.Text.Json;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public class ProvenanceRegistryConformanceTests
{
    private static SourcesDocument RealRegistry()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null && !Directory.Exists(Path.Combine(dir.FullName, "data", "compiled")))
        {
            dir = dir.Parent;
        }
        Assert.NotNull(dir);

        var path = Path.Combine(dir!.FullName, "data", "compiled", "sources.json");
        var doc = JsonSerializer.Deserialize<SourcesDocument>(File.ReadAllText(path));
        Assert.NotNull(doc);
        return doc!;
    }

    [Fact]
    public void TheCommittedRegistryCarriesAProvenanceTableAcrossThisAppsOwnJsonPolicy()
    {
        Assert.NotEmpty(RealRegistry().Provenances ?? []);
    }

    [Theory]
    [InlineData("openbible.info-cross-references", "OpenBible.info Cross-References")]
    [InlineData("kjv", "The King James Version")]
    [InlineData("theographic", "Theographic Bible Metadata")]
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
        var registry = RealRegistry();
        var unresolved = (registry.Provenances ?? [])
            .Select(p => ProvenanceResolver.Resolve(registry, p.Id))
            .Where(r => !r.IsResolved)
            .Select(r => r.Id)
            .ToList();

        Assert.True(unresolved.Count == 0, $"declared provenance rows that this app cannot resolve: {string.Join(", ", unresolved)}");
    }
}
