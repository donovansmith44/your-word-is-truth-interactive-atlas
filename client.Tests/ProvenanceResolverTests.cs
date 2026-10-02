using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public class ProvenanceResolverTests
{
    private static SourcesDocument Registry() => new(
        categories: new List<SourceCategory>
        {
            new(id: "scripture", label: "Scripture & Text"),
            new(id: "our-work", label: "Our Own Curated Work"),
        },
        sources: new List<SourceEntry>
        {
            new(id: "kjv-text", category: "scripture", title: "The King James Version", whatItIs: "The canonical English text this app reads.", whatWeBuilt: "Compiled into verses-kjv.json.", license: "Public domain", link: "https://example.invalid/kjv", licensesRowKey: "KJV text"),
            new(id: "openbible-xrefs", category: "scripture", title: "OpenBible.info Cross-References", whatItIs: "A cross-reference dataset.", whatWeBuilt: "Compiled into cross-refs.json.", license: "Free to use with credit", link: null, licensesRowKey: "OpenBible.info cross-references"),
            new(id: "our-curated-work", category: "our-work", title: "Our Own Curated Work", whatItIs: "Everything this project authored directly.", whatWeBuilt: "Hand-authored under data/curated/.", license: "CC0 1.0 Universal", link: null, licensesRowKey: "All curated data"),
        },
        provenances: new List<ProvenanceEntry>
        {
            new(id: "kjv", source: "kjv-text", confidence: Confidence.CanonicalText, locator: "verses-kjv.json"),
            new(id: "openbible.info-cross-references", source: "openbible-xrefs", confidence: Confidence.Imported, locator: "cross_references.txt"),
            new(id: "curated", source: "our-curated-work", confidence: Confidence.Curated, locator: null),
            new(id: "chronology-derivation", source: "our-curated-work", confidence: Confidence.Derived, locator: null),
            new(id: "kretzmann", source: "our-curated-work", confidence: Confidence.Imported, locator: null),
            new(id: "dangling", source: "no-such-source", confidence: Confidence.Imported, locator: null),
        });

    [Fact]
    public void ABareIdIsAllKindAndNoLocator()
    {
        Assert.Equal(("theographic", null), ProvenanceResolver.SplitId("theographic"));
    }

    [Fact]
    public void ADottedIdIsNotSplitBecauseOnlySlashesSeparate()
    {
        Assert.Equal(("openbible.info-cross-references", null), ProvenanceResolver.SplitId("openbible.info-cross-references"));
    }

    [Fact]
    public void ASuffixedIdSplitsAtTheFirstSlashOnlySoAMultiSegmentLocatorSurvives()
    {
        Assert.Equal(("kretzmann", "jeremiah/1"), ProvenanceResolver.SplitId("kretzmann/jeremiah/1"));
    }

    [Fact]
    public void AKnownIdResolvesToItsSourcesOwnTitleLicenseAndCategory()
    {
        var r = ProvenanceResolver.Resolve(Registry(), "openbible.info-cross-references");

        Assert.True(r.IsResolved);
        Assert.Equal("OpenBible.info Cross-References", r.Title);
        Assert.Equal("Free to use with credit", r.License);
        Assert.Equal("Scripture & Text", r.CategoryLabel);
        Assert.Equal("cross_references.txt", r.Locator);
    }

    [Fact]
    public void ASuffixedIdResolvesThroughItsKindAndKeepsTheRowsOwnLocator()
    {
        var r = ProvenanceResolver.Resolve(Registry(), "kretzmann/jeremiah/1");

        Assert.True(r.IsResolved);
        Assert.Equal("Our Own Curated Work", r.Title);
        Assert.Equal("jeremiah/1", r.Locator);
    }

    [Fact]
    public void AnUnknownIdIsLoudlyUnresolvedAndCarriesTheOffendingIdBack()
    {
        var r = ProvenanceResolver.Resolve(Registry(), "no-such-provenance");

        Assert.False(r.IsResolved);
        Assert.Equal("no-such-provenance", r.Id);
        Assert.Equal("", r.Title);
    }

    [Fact]
    public void ARowWhoseSourceDoesNotExistIsUnresolvedRatherThanACrashOrAHalfLabel()
    {
        var r = ProvenanceResolver.Resolve(Registry(), "dangling");

        Assert.False(r.IsResolved);
        Assert.Equal("dangling", r.Id);
    }

    [Fact]
    public void AnEmptyIdIsUnresolvedRatherThanMatchingSomeArbitraryRow()
    {
        var r = ProvenanceResolver.Resolve(Registry(), "");
        Assert.False(r.IsResolved);
        Assert.Equal(ProvenanceStatus.Unresolved, r.Status);
    }

    [Fact]
    public void ResolveAllKeepsABlankIdAsALoudUnresolvedEntryInsteadOfFilteringItIntoSilence()
    {
        var all = ProvenanceResolver.ResolveAll(Registry(), new[] { "" });

        Assert.Single(all);
        Assert.False(all[0].IsResolved);
        Assert.Equal(ProvenanceStatus.Unresolved, all[0].Status);
    }

    [Fact]
    public void AnEmptyListStillSaysNothingBecauseNoAttributionSectionIsNotTheSameFactAsAnIllegibleOne()
    {
        Assert.Empty(ProvenanceResolver.ResolveAll(Registry(), Array.Empty<string>()));
        Assert.Single(ProvenanceResolver.ResolveAll(Registry(), new[] { "   " }));
    }

    [Fact]
    public void ANullRegistryReportsThatTheRegistryIsUnavailableRatherThanAccusingTheData()
    {
        var r = ProvenanceResolver.Resolve(null, "kjv");

        Assert.False(r.IsResolved);
        Assert.Equal(ProvenanceStatus.RegistryUnavailable, r.Status);
        Assert.Equal("kjv", r.Id);
    }

    [Fact]
    public void TheTwoFailuresAreDistinguishableBecauseTheyAreDifferentFailures()
    {
        var infrastructure = ProvenanceResolver.Resolve(null, "kjv");
        var data = ProvenanceResolver.Resolve(Registry(), "no-such-provenance");

        Assert.NotEqual(infrastructure.Status, data.Status);
        Assert.False(infrastructure.IsResolved);
        Assert.False(data.IsResolved);
    }

    [Fact]
    public void AResolvedEntryReportsResolvedStatusSoTheThreeStatesStayTotal()
    {
        Assert.Equal(ProvenanceStatus.Resolved, ProvenanceResolver.Resolve(Registry(), "kjv").Status);
    }

    [Fact]
    public void ASectionDrawingOnTwoSourcesResolvesBothRatherThanCollapsingToOne()
    {
        var all = ProvenanceResolver.ResolveAll(Registry(), new[] { "kjv", "curated" });

        Assert.Equal(2, all.Count);
        Assert.Equal("The King James Version", all[0].Title);
        Assert.Equal("Our Own Curated Work", all[1].Title);
    }

    [Fact]
    public void ResolveAllDedupesByIdAndPreservesOrderAndNoLongerDropsBlanks()
    {
        var all = ProvenanceResolver.ResolveAll(Registry(), new[] { "curated", "", "kjv", "curated", "  " });

        Assert.Equal(3, all.Count);
        Assert.Equal("Our Own Curated Work", all[0].Title);
        Assert.Equal(ProvenanceStatus.Unresolved, all[1].Status);
        Assert.Equal("The King James Version", all[2].Title);
    }

    [Fact]
    public void EveryFlavourOfBlankIsTheSameFactAndSaysSoExactlyOnce()
    {
        var all = ProvenanceResolver.ResolveAll(Registry(), new string[] { null!, "", "   ", "\t" });

        Assert.Single(all);
        Assert.Equal(ProvenanceStatus.Unresolved, all[0].Status);
        Assert.Equal("", all[0].Id);
        Assert.False(all[0].IsResolved);
    }

    [Fact]
    public void ResolveAllOfNothingIsEmptySoTheAffordanceRendersNothingAtAll()
    {
        Assert.Empty(ProvenanceResolver.ResolveAll(Registry(), null));
        Assert.Empty(ProvenanceResolver.ResolveAll(Registry(), Array.Empty<string>()));
    }

    [Fact]
    public void CanonicalTextAndDerivedDoNotLookAlike()
    {
        var canonical = ProvenanceResolver.ConfidenceNote(Confidence.CanonicalText);
        var derived = ProvenanceResolver.ConfidenceNote(Confidence.Derived);

        Assert.NotNull(canonical);
        Assert.NotNull(derived);
        Assert.NotEqual(canonical, derived);
    }

    [Fact]
    public void CuratedSaysSoOutLoudBecauseTotalCaptureHonestyDependsOnIt()
    {
        Assert.Equal("Our own curated work", ProvenanceResolver.ConfidenceNote(Confidence.Curated));
    }

    [Fact]
    public void ImportedIsTheSilentDefaultBecauseSayingItEverywhereWouldTrainTheEyeToSkipIt()
    {
        Assert.Null(ProvenanceResolver.ConfidenceNote(Confidence.Imported));
    }

    [Fact]
    public void AnAbsentConfidenceRendersNoLineRatherThanAnEmptyOne()
    {
        Assert.Null(ProvenanceResolver.ConfidenceNote(null));
    }
}
