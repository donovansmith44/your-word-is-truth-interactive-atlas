using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

// Batch PROV-1 (owner order 1, verbatim: "one thing we definitely need for
// EVERY PIECE OF DATA is the source from which it came. openbible, etc.";
// owner order 2: "add a ? button on our frontier interface that gives
// provenance (i.e., sourced from openbible.com) or whatever").
//
// THE CLIENT HALF of the resolution law. The SERVER half
// (server/atlas-graph/tests/provenance_registry_real_data.rs) proves that
// every distinct provenance id in the real compiled artifact HAS a registry
// row, and fails the build if one does not. This file proves the other
// thing: that when resolution fails anyway, it fails LOUDLY -- which is
// exactly the case no real document can produce while the server law holds,
// and therefore exactly the case only a hand-built registry can test.
//
// ProvenanceResolver is static and pure (it takes the document as a
// parameter rather than injecting a client), which is what lets every test
// below run with no HTTP and no component host at all.
public class ProvenanceResolverTests
{
    private static SourcesDocumentOut Registry() => new(
        Categories: new List<SourceCategoryDto>
        {
            new("scripture", "Scripture & Text"),
            new("our-work", "Our Own Curated Work"),
        },
        Sources: new List<SourceEntryDto>
        {
            new("kjv-text", "scripture", "The King James Version", "The canonical English text this app reads.", "Compiled into verses-kjv.json.", "Public domain", "https://example.invalid/kjv", "KJV text"),
            new("openbible-xrefs", "scripture", "OpenBible.info Cross-References", "A cross-reference dataset.", "Compiled into cross-refs.json.", "Free to use with credit", null, "OpenBible.info cross-references"),
            new("our-curated-work", "our-work", "Our Own Curated Work", "Everything this project authored directly.", "Hand-authored under data/curated/.", "CC0 1.0 Universal", null, "All curated data"),
        },
        Provenances: new List<ProvenanceEntryDto>
        {
            new("kjv", "kjv-text", "CanonicalText", "verses-kjv.json"),
            new("openbible.info-cross-references", "openbible-xrefs", "Imported", "cross_references.txt"),
            new("curated", "our-curated-work", "Curated"),
            new("chronology-derivation", "our-curated-work", "Derived"),
            new("kretzmann", "our-curated-work", "Imported"),
            // A row whose `source` names nothing -- `validate_structure`
            // rejects this on the server, so only a fixture can produce it.
            new("dangling", "no-such-source", "Imported"),
        });

    // ---- THE ID GRAMMAR ------------------------------------------------
    // Mirrors atlas_core::sources::split_provenance_id exactly; that
    // function's own #[cfg(test)] module asserts the same three cases on
    // the Rust side, and the two must never disagree.

    [Fact]
    public void ABareIdIsAllKindAndNoLocator()
    {
        Assert.Equal(("theographic", null), ProvenanceResolver.SplitId("theographic"));
    }

    [Fact]
    public void ADottedIdIsNotSplitBecauseOnlySlashesSeparate()
    {
        // The real id `openbible.info-cross-references` carries dots. If
        // dots split, the owner's own headline source would resolve to
        // nothing.
        Assert.Equal(("openbible.info-cross-references", null), ProvenanceResolver.SplitId("openbible.info-cross-references"));
    }

    [Fact]
    public void ASuffixedIdSplitsAtTheFirstSlashOnlySoAMultiSegmentLocatorSurvives()
    {
        // kretzmann_adapter's own real shape: one id PER commentary unit,
        // kind + a two-segment locator. Splitting at the LAST slash would
        // mis-read the kind as "kretzmann/jeremiah" and resolve nothing.
        Assert.Equal(("kretzmann", "jeremiah/1"), ProvenanceResolver.SplitId("kretzmann/jeremiah/1"));
    }

    // ---- RESOLUTION ----------------------------------------------------

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
        // The ROW's locator wins over the registry's general one: "jeremiah/1"
        // is what THIS row cites; a registry locator would be what the
        // source generally covers.
        var r = ProvenanceResolver.Resolve(Registry(), "kretzmann/jeremiah/1");

        Assert.True(r.IsResolved);
        Assert.Equal("Our Own Curated Work", r.Title);
        Assert.Equal("jeremiah/1", r.Locator);
    }

    // ---- THE FAIL-LOUD LAW ---------------------------------------------

    [Fact]
    public void AnUnknownIdIsLoudlyUnresolvedAndCarriesTheOffendingIdBack()
    {
        var r = ProvenanceResolver.Resolve(Registry(), "no-such-provenance");

        Assert.False(r.IsResolved);
        // The id itself survives, so ProvenanceAffordance can NAME what it
        // could not resolve -- never a silent blank, never a guess.
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
        // FIX ROUND 1 (review H-1): and it is a DATA fault, not an
        // infrastructure one -- the registry loaded fine; what arrived was
        // nothing to look up.
        Assert.Equal(ProvenanceStatus.Unresolved, r.Status);
    }

    // ---- H-1: THE LOUD PATH IS LIVE, NOT DEAD CODE ----------------------
    // The batch asserted, in three source comments and in its report, that
    // an empty provenance renders as a LOUD unresolved notice. It did not:
    // ProvenanceAffordance, ResolveAll, FrontierProvenance.Distinct and
    // EventProvenanceSection each filtered whitespace ids out BEFORE
    // resolution, so Resolve's blank branch was unreachable from the UI and
    // a missing provenance rendered as no "?" at all -- a curatorial claim
    // with no attribution and no sign that attribution was missing.
    //
    // These two tests are what would have caught it: they assert on
    // ResolveAll (the collection entry point every call site goes through),
    // not on Resolve (the branch that was already green while dead).

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
        // The distinction the fix preserves. Zero ids = this surface has no
        // attribution to make (honest absence; the component renders
        // nothing). One blank id = something claimed an attribution and
        // handed over nothing (loud). Collapsing the two is what made the
        // silent blank possible.
        Assert.Empty(ProvenanceResolver.ResolveAll(Registry(), Array.Empty<string>()));
        Assert.Single(ProvenanceResolver.ResolveAll(Registry(), new[] { "   " }));
    }

    // ---- M-4: AN INFRASTRUCTURE FAULT IS NOT A DATA FAULT ---------------

    [Fact]
    public void ANullRegistryReportsThatTheRegistryIsUnavailableRatherThanAccusingTheData()
    {
        // Before this fix, `/api/sources` failing rendered
        // `Unrecognized source "kjv". Please report it.` -- in the loudest
        // register in the panel, on EVERY affordance on the popover -- for
        // data whose provenance is perfectly well-formed and perfectly well
        // registered. The reader was told the atlas had unattributed data
        // and asked to report a bug that did not exist.
        var r = ProvenanceResolver.Resolve(null, "kjv");

        Assert.False(r.IsResolved);
        Assert.Equal(ProvenanceStatus.RegistryUnavailable, r.Status);
        // The id survives, because it is the one true thing we can still say.
        Assert.Equal("kjv", r.Id);
    }

    [Fact]
    public void TheTwoFailuresAreDistinguishableBecauseTheyAreDifferentFailures()
    {
        var infrastructure = ProvenanceResolver.Resolve(null, "kjv");
        var data = ProvenanceResolver.Resolve(Registry(), "no-such-provenance");

        Assert.NotEqual(infrastructure.Status, data.Status);
        // ...and both are still NOT resolved, so neither can render a
        // fabricated label.
        Assert.False(infrastructure.IsResolved);
        Assert.False(data.IsResolved);
    }

    [Fact]
    public void AResolvedEntryReportsResolvedStatusSoTheThreeStatesStayTotal()
    {
        Assert.Equal(ProvenanceStatus.Resolved, ProvenanceResolver.Resolve(Registry(), "kjv").Status);
    }

    // ---- SECTION-LEVEL RESOLUTION (the leper lesson) --------------------

    [Fact]
    public void ASectionDrawingOnTwoSourcesResolvesBothRatherThanCollapsingToOne()
    {
        // THE LEPER LESSON, client-side: a hand-authored row under the same
        // heading as an imported one must stay visible AS hand-authored.
        var all = ProvenanceResolver.ResolveAll(Registry(), new[] { "kjv", "curated" });

        Assert.Equal(2, all.Count);
        Assert.Equal("The King James Version", all[0].Title);
        Assert.Equal("Our Own Curated Work", all[1].Title);
    }

    [Fact]
    public void ResolveAllDedupesByIdAndPreservesOrderAndNoLongerDropsBlanks()
    {
        // FIX ROUND 1 (review H-1): this test used to be named
        // "...AndDropsBlanks" and asserted 2 entries for this input --
        // it PINNED the silent blank as correct behavior. Dedup and order
        // are unchanged; the two blanks now dedupe to ONE loud entry
        // instead of vanishing.
        var all = ProvenanceResolver.ResolveAll(Registry(), new[] { "curated", "", "kjv", "curated", "  " });

        Assert.Equal(4, all.Count);
        Assert.Equal("Our Own Curated Work", all[0].Title);
        Assert.Equal(ProvenanceStatus.Unresolved, all[1].Status);
        Assert.Equal("The King James Version", all[2].Title);
        Assert.Equal(ProvenanceStatus.Unresolved, all[3].Status);
    }

    [Fact]
    public void ResolveAllOfNothingIsEmptySoTheAffordanceRendersNothingAtAll()
    {
        Assert.Empty(ProvenanceResolver.ResolveAll(Registry(), null));
        Assert.Empty(ProvenanceResolver.ResolveAll(Registry(), Array.Empty<string>()));
    }

    // ---- THE CONFIDENCE-DISPLAY RULE -----------------------------------
    // Brief: "Confidence is shown when it is not the obvious default -- a
    // CanonicalText claim and a Derived claim must not look alike."

    [Fact]
    public void CanonicalTextAndDerivedDoNotLookAlike()
    {
        var canonical = ProvenanceResolver.ConfidenceNote("CanonicalText");
        var derived = ProvenanceResolver.ConfidenceNote("Derived");

        Assert.NotNull(canonical);
        Assert.NotNull(derived);
        Assert.NotEqual(canonical, derived);
    }

    [Fact]
    public void CuratedSaysSoOutLoudBecauseTotalCaptureHonestyDependsOnIt()
    {
        // The ATTEST-1 leper lesson as a display rule: our own work must
        // announce itself, or it can pass for an imported source.
        Assert.Equal("Our own curated work", ProvenanceResolver.ConfidenceNote("Curated"));
    }

    [Fact]
    public void ImportedIsTheSilentDefaultBecauseSayingItEverywhereWouldTrainTheEyeToSkipIt()
    {
        Assert.Null(ProvenanceResolver.ConfidenceNote("Imported"));
    }

    [Fact]
    public void AnUnknownConfidenceRendersNoLineRatherThanAnEmptyOne()
    {
        Assert.Null(ProvenanceResolver.ConfidenceNote("Speculative"));
        Assert.Null(ProvenanceResolver.ConfidenceNote(null));
    }
}
