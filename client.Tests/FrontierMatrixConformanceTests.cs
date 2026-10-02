using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Legacy;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Tests;

/// <summary>
/// EVT-3 Ticket 1 (the smart-frontier restructure), §4e conformance,
/// DIRECTION (a) -- "no provider renders for a kind outside its matrix
/// set." Every one of the nine capability-family providers
/// (Explore/PopoverSectionProviders.cs) now derives its own AppliesTo from
/// <see cref="FrontierMatrix"/> through the ONE chokepoint
/// (<see cref="FocusKinds.Parse"/>); this test enumerates providers x
/// every <see cref="FocusKind"/> and proves the two agree EXACTLY, both
/// directions at once (a provider must say yes for every kind the matrix
/// allows AND no for every kind it doesn't -- a single equality check
/// covers both halves of §4e's own conformance corollary for this
/// provider).
///
/// Same discipline as <c>PopoverChromeConformanceTests.cs</c>'s own
/// CHROME-UNIFORMITY-1 precedent (itself citing
/// <c>ViewRegistryConformanceTests.cs</c>'s Q-5 finding): a shared
/// predicate (<see cref="AssertMatchesMatrix"/>) is called by BOTH the
/// real-provider sweep below AND a deliberately-planted violation
/// (<see cref="BrokenCrossRefsSection"/>) -- proof the check has real
/// teeth, not merely a vacuous pass against every real provider by
/// coincidence (a self-referential re-implementation of the same
/// comparison inline would prove nothing about whether THIS check would
/// ever catch a genuine defect).
///
/// Direction (b) -- "the implementing-without-content direction... a
/// live-data test that the section CAN produce content for at least one
/// real node of that kind" -- needs REAL compiled data (a verse that
/// genuinely has cross-references, an event that genuinely has a
/// chronology position, ...), which this project's own established house
/// rule (<c>GraphExplorableClientTests.cs</c>'s own doc comment: "no
/// dedicated unit tests for AtlasClient itself... live server behavior is
/// exercised end to end by the Playwright suite") puts in Playwright, not
/// here -- see <c>tests/ux/frontier-matrix.spec.ts</c>.
/// </summary>
public class FrontierMatrixConformanceTests
{
    private sealed record FakeNode(string Kind) : IExplorable
    {
        public string Title => Kind;
        public NodeRef Identity => throw new NotSupportedException("AppliesTo must never call this -- it's the cheap, synchronous half of the provider contract.");
        public Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api) => throw new NotSupportedException("AppliesTo must never call this -- it's the cheap, synchronous half of the provider contract.");
        public Task<RenderFragment> BodyAsync(AtlasClient api) => throw new NotSupportedException("AppliesTo must never call this -- it's the cheap, synchronous half of the provider contract.");
    }

    private static readonly FocusKind[] AllKinds = Enum.GetValues<FocusKind>();

    /// The ONE shared check -- every kind in `matrixSet` must make
    /// `provider.AppliesTo` true, and every kind NOT in it must make it
    /// false. Throws (via Assert.True) on the FIRST disagreement found,
    /// naming the provider and kind -- both this file's own real-provider
    /// sweep and the planted-violation proof call this exact method.
    private static void AssertMatchesMatrix(IPopoverSectionProvider provider, IReadOnlySet<FocusKind> matrixSet)
    {
        foreach (var kind in AllKinds)
        {
            var applies = provider.AppliesTo(new FakeNode(kind.ToString()));
            var allowed = matrixSet.Contains(kind);
            Assert.True(applies == allowed,
                $"{provider.GetType().Name}.AppliesTo({kind}) returned {applies}, but FrontierMatrix says {allowed} for that cell.");
        }
    }

    // Every matrix-governed capability family, checked directly against
    // its OWN provider instance (independent of registration/order --
    // VersePersonsSection is unregistered per O4 but still makes a real,
    // still-true capability claim; this test proves it just as
    // rigorously as a registered one).
    [Fact] public void CrossRefsSection_MatchesMatrix() => AssertMatchesMatrix(new CrossRefsSection(), FrontierMatrix.CrossReferences);
    [Fact] public void VerseParallelsSection_MatchesMatrix() => AssertMatchesMatrix(new VerseParallelsSection(), FrontierMatrix.Parallels);
    [Fact] public void VerseEventMembershipSection_MatchesMatrix() => AssertMatchesMatrix(new VerseEventMembershipSection(), FrontierMatrix.EventMembership);
    [Fact] public void VersePassageMembershipSection_MatchesMatrix() => AssertMatchesMatrix(new VersePassageMembershipSection(), FrontierMatrix.PassageMembership);
    [Fact] public void VersePersonsSection_MatchesMatrix() => AssertMatchesMatrix(new VersePersonsSection(), FrontierMatrix.Persons);
    [Fact] public void CatechismSeamSection_MatchesMatrix() => AssertMatchesMatrix(new CatechismSeamSection(), FrontierMatrix.CatechismSupport);
    [Fact] public void EventChronologySection_MatchesMatrix() => AssertMatchesMatrix(new EventChronologySection(), FrontierMatrix.Chronology);
    [Fact] public void EventDateAndPlacesSection_MatchesMatrix() => AssertMatchesMatrix(new EventDateAndPlacesSection(), FrontierMatrix.TimeAndPlace);
    [Fact] public void EventWitnessesSection_MatchesMatrix() => AssertMatchesMatrix(new EventWitnessesSection(), FrontierMatrix.Accounts);

    /// The owner's own calibration row, pinned directly against the REAL
    /// registry (not a copy): an Event frontier never renders cross
    /// references (Contracts/Frontier.cs's own "we are not yet at the
    /// point of being able ot provide lots of cross references for
    /// events" comment) -- proven by running CrossRefsSection's own
    /// AppliesTo against a real Event node, not merely reading the matrix
    /// set back at itself.
    [Fact]
    public void EventNeverGetsCrossReferences()
    {
        Assert.False(new CrossRefsSection().AppliesTo(new FakeNode("Event")));
    }

    /// Full registry sweep -- every provider PopoverSectionRegistry
    /// actually wires in, cross-checked against the matrix wherever a
    /// matrix cell governs it (a core/body-only provider, e.g.
    /// ChapterCardSection, is out of THIS test's own scope -- §4e's own
    /// "body-only sections... are core focus-presentation, not matrix
    /// entries" clause; every core provider's own single-kind chokepoint
    /// retirement is proven instead by PopoverSectionRegistryTests.cs's
    /// existing ordering/uniqueness checks and by this same file's
    /// per-family tests above for every family that DOES have a cell).
    [Fact]
    public void NoRegisteredProviderRendersOutsideItsOwnMatrixSet()
    {
        var matrixByProviderType = new Dictionary<Type, IReadOnlySet<FocusKind>>
        {
            [typeof(CrossRefsSection)] = FrontierMatrix.CrossReferences,
            [typeof(VerseParallelsSection)] = FrontierMatrix.Parallels,
            [typeof(VerseEventMembershipSection)] = FrontierMatrix.EventMembership,
            [typeof(VersePassageMembershipSection)] = FrontierMatrix.PassageMembership,
            [typeof(CatechismSeamSection)] = FrontierMatrix.CatechismSupport,
            [typeof(EventChronologySection)] = FrontierMatrix.Chronology,
            [typeof(EventDateAndPlacesSection)] = FrontierMatrix.TimeAndPlace,
            [typeof(EventWitnessesSection)] = FrontierMatrix.Accounts,
        };

        var checkedCount = 0;
        foreach (var provider in PopoverSectionRegistry.Providers)
        {
            if (!matrixByProviderType.TryGetValue(provider.GetType(), out var matrixSet))
            {
                continue; // core/body-only provider -- not a matrix cell, out of scope
            }
            AssertMatchesMatrix(provider, matrixSet);
            checkedCount++;
        }

        // Never-vacuous guard (the same discipline every conformance test
        // in this project carries): if a future rename/retirement silently
        // dropped every matrix-governed provider from the registry, this
        // whole test would otherwise pass by doing nothing.
        Assert.Equal(matrixByProviderType.Count, checkedCount);
    }

    /// THE PLANTED-VIOLATION PROOF. A synthetic provider that declares
    /// <see cref="IHasCrossReferences"/> (the real marker interface) but
    /// wrongly answers AppliesTo true for Event too -- exactly the
    /// owner's own calibration violation, reintroduced on purpose.
    /// AssertMatchesMatrix above is the SAME method the real sweep calls;
    /// this proves it actually fails on a genuine defect rather than
    /// vacuously agreeing with every real provider by construction.
    private sealed class BrokenCrossRefsSection : IPopoverSectionProvider, IHasCrossReferences
    {
        public bool AppliesTo(IExplorable node) =>
            FrontierMatrix.CrossReferences.Contains(FocusKinds.Parse(node.Kind)) || FocusKinds.Parse(node.Kind) == FocusKind.Event;

        public Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx) =>
            Task.FromResult<PopoverSection?>(null);
    }

    [Fact]
    public void PlantedViolationProof_TheSharedCheckActuallyCatchesAMatrixViolation()
    {
        var ex = Record.Exception(() => AssertMatchesMatrix(new BrokenCrossRefsSection(), FrontierMatrix.CrossReferences));
        Assert.NotNull(ex);
        Assert.Contains("BrokenCrossRefsSection", ex!.Message);
        Assert.Contains("Event", ex.Message);
    }
}
