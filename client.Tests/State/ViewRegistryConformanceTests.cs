using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.State;
using BibleAtlas.Client.Views;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// Batch VC-1 (R6): "registry conformance (R1); an arrangement-vocabulary
/// test (every LayoutKind rendered by the host; unknown name/kind fails
/// loud with the contract clause); ... hatch conformance (every declared
/// enter-split hatch resolves both its views in the registry)." Builds the
/// REAL <see cref="ViewRegistry"/> via <see cref="ViewRegistrySetup.Build"/>
/// -- the SAME factory Program.cs calls -- against real (non-JS-dependent)
/// singletons, exactly the DI-realism discipline
/// <c>ConformanceTests.AtomRegistrationConformance</c> already established
/// for atoms (S-10/ruling 6.i).
/// </summary>
public class ViewRegistryConformanceTests
{
    /// A minimal, real <see cref="NavigationManager"/> -- ViewRegistrySetup's
    /// own hatches close over it but this suite never actually INVOKES a
    /// hatch (that would navigate/dispatch for real, exercised instead by
    /// ViewArrangementTests.cs's own intent-level tests and
    /// tests/ux/composition.spec.ts at the browser level) -- constructing a
    /// real, working instance is enough to prove the registry itself builds
    /// and every registration is genuinely resolvable.
    private sealed class FakeNavigationManager : NavigationManager
    {
        public FakeNavigationManager() => Initialize("https://example.test/", "https://example.test/");

        protected override void NavigateToCore(string uri, NavigationOptions options)
        {
        }
    }

    private static ViewRegistry BuildRegistry() => ViewRegistrySetup.Build(
        new StateAtom<ViewArrangement>(AtomNames.ViewArrangement, ViewArrangement.Default),
        new ViewStateService(),
        new StateAtom<Locus>(AtomNames.Locus, Locus.Default),
        new FakeNavigationManager());

    // ------------------------------------------------------------------
    // R1: registry conformance -- every arrangement-reachable name
    // resolves; every registered name unique.
    // ------------------------------------------------------------------

    [Fact]
    public void Registry_EveryViewNamesConstant_ResolvesInTheRegistry()
    {
        var registry = BuildRegistry();

        foreach (var name in new[] { ViewNames.Reader, ViewNames.World, ViewNames.Sources })
        {
            Assert.True(registry.TryGet(name, out var view), $"'{name}' (ViewNames constant) did not resolve in the registry.");
            Assert.Equal(name, view.Name);
        }
    }

    [Fact]
    public void Registry_EveryRegisteredName_IsUnique()
    {
        var registry = BuildRegistry();
        var names = registry.All.Select(v => v.Name).ToList();

        Assert.Equal(names.Count, names.Distinct().Count());
    }

    [Fact]
    public void Registry_UnknownName_ThrowsRatherThanSilentlyReturningNull()
    {
        var registry = BuildRegistry();

        Assert.Throws<InvalidOperationException>(() => registry.Get("not-a-real-view"));
        Assert.False(registry.TryGet("not-a-real-view", out _));
        Assert.Equal(ViewCapabilities.None, registry.CapabilitiesOf("not-a-real-view"));
    }

    [Fact]
    public void Registry_EveryViewNamesConstantValue_IsUnique()
    {
        var values = typeof(ViewNames).GetFields(System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static)
            .Where(f => f.FieldType == typeof(string))
            .Select(f => (string)f.GetValue(null)!)
            .ToList();

        Assert.Equal(3, values.Count); // reader, world, sources -- this batch's own ship list; grows deliberately, not by accident
        Assert.Equal(values.Count, values.Distinct().Count());
    }

    // ------------------------------------------------------------------
    // R1: capability data -- declared, never inferred from name.
    // ------------------------------------------------------------------

    [Fact]
    public void Registry_CapabilityData_MatchesR1sOwnAssignment()
    {
        var registry = BuildRegistry();

        Assert.Equal(ViewCapabilities.BearsLocus, registry.CapabilitiesOf(ViewNames.Reader));
        Assert.Equal(ViewCapabilities.BearsWindow, registry.CapabilitiesOf(ViewNames.World));
        Assert.Equal(ViewCapabilities.None, registry.CapabilitiesOf(ViewNames.Sources));
    }

    // ------------------------------------------------------------------
    // R4: hatch conformance -- every declared enter-split hatch resolves
    // BOTH its views in the registry (owner + partner).
    // ------------------------------------------------------------------

    [Fact]
    public void HatchConformance_EveryEnterSplitHatch_ResolvesBothItsViewsInTheRegistry()
    {
        var registry = BuildRegistry();
        var hatchesFound = 0;

        foreach (var view in registry.All)
        {
            foreach (var hatch in view.EscapeHatches.Where(h => h.Kind == HatchKinds.EnterSplit))
            {
                hatchesFound++;
                Assert.IsType<EnterSplitHatch>(hatch);
                var enterSplit = (EnterSplitHatch)hatch;

                Assert.True(registry.TryGet(enterSplit.OwnerView, out var owner), $"Hatch owner '{enterSplit.OwnerView}' does not resolve in the registry.");
                Assert.Equal(view.Name, owner.Name);
                Assert.True(registry.TryGet(enterSplit.PartnerView, out _), $"Hatch partner '{enterSplit.PartnerView}' does not resolve in the registry.");
                Assert.NotEqual(enterSplit.OwnerView, enterSplit.PartnerView); // a hatch never partners a view with itself
            }
        }

        // R4's own ship list, verbatim: reader<->world (two hatches, one per
        // side) PLUS Sources' own "read-beside" proof -- exactly three,
        // never silently zero (a registry with no hatches at all would pass
        // every loop above vacuously).
        Assert.Equal(3, hatchesFound);
    }

    [Fact]
    public void HatchConformance_ReaderAndWorld_EachDeclareTheirOwnEnterSplitHatchToTheOther()
    {
        var registry = BuildRegistry();

        var readerHatch = (EnterSplitHatch)registry.Get(ViewNames.Reader).EscapeHatches.Single(h => h.Kind == HatchKinds.EnterSplit);
        Assert.Equal(ViewNames.Reader, readerHatch.OwnerView);
        Assert.Equal(ViewNames.World, readerHatch.PartnerView);

        var worldHatch = (EnterSplitHatch)registry.Get(ViewNames.World).EscapeHatches.Single(h => h.Kind == HatchKinds.EnterSplit);
        Assert.Equal(ViewNames.World, worldHatch.OwnerView);
        Assert.Equal(ViewNames.Reader, worldHatch.PartnerView);
    }

    [Fact]
    public void HatchConformance_Sources_DeclaresTheR4ProofHatchToReader()
    {
        var registry = BuildRegistry();

        var sourcesHatch = (EnterSplitHatch)registry.Get(ViewNames.Sources).EscapeHatches.Single(h => h.Kind == HatchKinds.EnterSplit);
        Assert.Equal(ViewNames.Sources, sourcesHatch.OwnerView);
        Assert.Equal(ViewNames.Reader, sourcesHatch.PartnerView);
    }

    // ------------------------------------------------------------------
    // R6: arrangement-vocabulary -- every LayoutKind is a real, known value
    // (proven directly, structurally); unknown fails loud. CompositionHost's
    // OWN "unknown kind" branch reads LayoutKinds.IsKnown -- see that
    // component's own header -- so this test proves the SAME predicate its
    // markup uses, not a source-text proxy for it. The actual RENDERING
    // proof (CompositionHost genuinely handles single/split-h/an injected
    // unknown kind in a live browser) lives at tests/ux/composition.spec.ts
    // -- client.Tests has no Razor-component-rendering harness (no bUnit
    // referenced), disclosed rather than faked with a source scan.
    // ------------------------------------------------------------------

    [Theory]
    [InlineData(LayoutKinds.Single)]
    [InlineData(LayoutKinds.SplitH)]
    public void LayoutKinds_EveryVocabularyConstant_IsKnown(string kind)
    {
        Assert.True(LayoutKinds.IsKnown(kind));
    }

    [Theory]
    [InlineData("")]
    [InlineData("overlay")]
    [InlineData("SPLIT-H")] // case-sensitive -- not a known kind by accident of casing
    public void LayoutKinds_UnrecognizedValue_IsNotKnown(string kind)
    {
        Assert.False(LayoutKinds.IsKnown(kind));
    }

    [Fact]
    public void LayoutKinds_All_HasNoDuplicates()
    {
        Assert.Equal(LayoutKinds.All.Count, LayoutKinds.All.Distinct().Count());
    }

    [Fact]
    public async Task Hatch_Invoke_DispatchesTheExpectedEnterSplitArrangement()
    {
        // The one behavioral (not just structural) proof at this level --
        // invoking Sources' own declared hatch genuinely dispatches
        // EnterSplit(sources, reader) onto the SAME atom instance the
        // registry was built against (composition.spec.ts proves the
        // browser-level, end-to-end version of this same fact).
        var arrangement = new StateAtom<ViewArrangement>(AtomNames.ViewArrangement, ViewArrangement.Default);
        var registry = ViewRegistrySetup.Build(arrangement, new ViewStateService(), new StateAtom<Locus>(AtomNames.Locus, Locus.Default), new FakeNavigationManager());

        var hatch = registry.Get(ViewNames.Sources).EscapeHatches.Single(h => h.Kind == HatchKinds.EnterSplit);
        await hatch.Invoke();

        Assert.Equal(LayoutKinds.SplitH, arrangement.Value.LayoutKind);
        Assert.Equal(new[] { ViewNames.Sources, ViewNames.Reader }, arrangement.Value.Members);
    }
}
