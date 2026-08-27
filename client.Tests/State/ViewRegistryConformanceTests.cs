using System.Reflection;
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
///
/// Fix round 1 (S-6, IMPORTANT -- review): the registry-resolution tests
/// below now REFLECT over <see cref="ViewNames"/> instead of iterating a
/// hand-written array literal -- a new <c>ViewNames</c> constant with no
/// matching registration now fails THIS test directly, not incidentally via
/// an unrelated count assertion whose natural repair ("bump 3 to 4") would
/// have registered nothing.
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

    private static List<string> ViewNamesConstants() =>
        typeof(ViewNames).GetFields(BindingFlags.Public | BindingFlags.Static)
            .Where(f => f.FieldType == typeof(string))
            .Select(f => (string)f.GetValue(null)!)
            .ToList();

    private static string RepoRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null && !Directory.Exists(Path.Combine(dir.FullName, "client")))
        {
            dir = dir.Parent;
        }

        Assert.NotNull(dir);
        return dir!.FullName;
    }

    private static string Capitalize(string name) => char.ToUpperInvariant(name[0]) + name[1..];

    // ------------------------------------------------------------------
    // R1: registry conformance -- every arrangement-reachable name
    // resolves; every registered name unique.
    // ------------------------------------------------------------------

    [Fact]
    public void Registry_EveryViewNamesConstant_ResolvesInTheRegistry()
    {
        var registry = BuildRegistry();
        var names = ViewNamesConstants();

        Assert.NotEmpty(names); // never vacuous
        foreach (var name in names)
        {
            Assert.True(registry.TryGet(name, out var view), $"'{name}' (a ViewNames constant, found via REFLECTION) did not resolve in the registry -- add its own RegisteredView in ViewRegistrySetup.Build.");
            Assert.Equal(name, view.Name);
        }
    }

    [Fact]
    public void Registry_EveryViewNamesConstantValue_IsUnique()
    {
        var values = ViewNamesConstants();

        // Batch CORP-1: reader, world, sources, kretzmann, concord -- grows
        // deliberately, not by accident (this test's own header comment).
        Assert.Equal(5, values.Count);
        Assert.Equal(values.Count, values.Distinct().Count());
    }

    // Fix round 1 (S-6): the constructor GENUINELY throws on a duplicate
    // name (ViewRegistry.cs's own `views.ToDictionary(v => v.Name)`) --
    // proven directly against a planted duplicate, rather than asserting
    // uniqueness on a state the registry can never actually reach.
    [Fact]
    public void Registry_ConstructorThrows_OnAPlantedDuplicateName()
    {
        RenderFragment Empty(ViewMountContext ctx) => builder => { };
        var first = new RegisteredView(ViewNames.Reader, ViewCapabilities.None, Empty, Array.Empty<IEscapeHatch>());
        var duplicate = new RegisteredView(ViewNames.Reader, ViewCapabilities.BearsWindow, Empty, Array.Empty<IEscapeHatch>());

        Assert.Throws<ArgumentException>(() => new ViewRegistry(new[] { first, duplicate }));
    }

    [Fact]
    public void Registry_UnknownName_ThrowsRatherThanSilentlyReturningNull()
    {
        var registry = BuildRegistry();

        Assert.Throws<InvalidOperationException>(() => registry.Get("not-a-real-view"));
        Assert.False(registry.TryGet("not-a-real-view", out _));
        Assert.Equal(ViewCapabilities.None, registry.CapabilitiesOf("not-a-real-view"));
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

        // Batch CORP-1 (R2/R3): Kretzmann PROJECTS the shared Locus atom
        // (BearsLocus, the SAME declaration Reader carries -- this is
        // exactly why the split-follow-by-construction proof holds); Concord
        // declares no capability (R3: navigates its own structure, not
        // scripture locus).
        Assert.Equal(ViewCapabilities.BearsLocus, registry.CapabilitiesOf(ViewNames.Kretzmann));
        Assert.Equal(ViewCapabilities.None, registry.CapabilitiesOf(ViewNames.Concord));
    }

    // ------------------------------------------------------------------
    // R4: hatch conformance -- every declared enter-split hatch resolves
    // BOTH its views in the registry (owner + partner). Fix round 1
    // (controller ruling 2): "no separate CanHost flag -- the declared
    // hatch IS the hosting declaration." HostView (distinct from OwnerView
    // -- see EnterSplitHatch.cs's own header) is what actually hosts when
    // the hatch fires; the tripwire below proves every HostView's own
    // component genuinely uses CompositionSplit, by name.
    // ------------------------------------------------------------------

    [Fact]
    public void HatchConformance_EveryEnterSplitHatch_ResolvesBothItsViewsInTheRegistry()
    {
        var registry = BuildRegistry();
        var hatchesFound = 0;

        foreach (var view in registry.All)
        {
            foreach (var hatch in view.EscapeHatches.OfType<EnterSplitHatch>())
            {
                hatchesFound++;

                Assert.True(registry.TryGet(hatch.OwnerView, out var owner), $"Hatch owner '{hatch.OwnerView}' does not resolve in the registry.");
                Assert.Equal(view.Name, owner.Name);
                Assert.True(registry.TryGet(hatch.PartnerView, out _), $"Hatch partner '{hatch.PartnerView}' does not resolve in the registry.");
                Assert.True(registry.TryGet(hatch.HostView, out _), $"Hatch HostView '{hatch.HostView}' does not resolve in the registry.");
                Assert.NotEqual(hatch.OwnerView, hatch.PartnerView); // a hatch never partners a view with itself
            }
        }

        // R4's own ship list, verbatim: reader<->world (two hatches, one per
        // side) PLUS Sources' own "read-beside" proof -- three at VC-1.
        // Batch CORP-1 adds two more, self-hosting like Sources: Kretzmann's
        // own "read-beside" and Concord's own "read-beside" -- five, never
        // silently zero (a registry with no hatches at all would pass every
        // loop above vacuously).
        Assert.Equal(5, hatchesFound);
    }

    [Fact]
    public void HatchConformance_ReaderAndSources_OwnHatchHostsThemselves()
    {
        var registry = BuildRegistry();

        var readerHatch = (EnterSplitHatch)registry.Get(ViewNames.Reader).EscapeHatches.Single(h => h.Kind == HatchKinds.EnterSplit);
        Assert.Equal(ViewNames.Reader, readerHatch.OwnerView);
        Assert.Equal(ViewNames.World, readerHatch.PartnerView);
        Assert.Equal(ViewNames.Reader, readerHatch.HostView);

        var sourcesHatch = (EnterSplitHatch)registry.Get(ViewNames.Sources).EscapeHatches.Single(h => h.Kind == HatchKinds.EnterSplit);
        Assert.Equal(ViewNames.Sources, sourcesHatch.OwnerView);
        Assert.Equal(ViewNames.Reader, sourcesHatch.PartnerView);
        Assert.Equal(ViewNames.Sources, sourcesHatch.HostView);
    }

    // Batch CORP-1 (R2/R3): Kretzmann and Concord each declare their OWN
    // "read-beside" hatch, self-hosting -- the identical shape Sources' own
    // hatch already proved generic immediately above.
    [Fact]
    public void HatchConformance_KretzmannAndConcord_OwnHatchHostsThemselves()
    {
        var registry = BuildRegistry();

        var kretzmannHatch = (EnterSplitHatch)registry.Get(ViewNames.Kretzmann).EscapeHatches.Single(h => h.Kind == HatchKinds.EnterSplit);
        Assert.Equal(ViewNames.Kretzmann, kretzmannHatch.OwnerView);
        Assert.Equal(ViewNames.Reader, kretzmannHatch.PartnerView);
        Assert.Equal(ViewNames.Kretzmann, kretzmannHatch.HostView);

        var concordHatch = (EnterSplitHatch)registry.Get(ViewNames.Concord).EscapeHatches.Single(h => h.Kind == HatchKinds.EnterSplit);
        Assert.Equal(ViewNames.Concord, concordHatch.OwnerView);
        Assert.Equal(ViewNames.Reader, concordHatch.PartnerView);
        Assert.Equal(ViewNames.Concord, concordHatch.HostView);
    }

    [Fact]
    public void HatchConformance_World_DeclaresItsOwnHatchButReaderIsTheHost()
    {
        // R7: byte-identical to pre-VC-1 -- "Read beside the map" (declared
        // BY World) still makes READER the host, not World. This is exactly
        // why HostView is a separate field from OwnerView (EnterSplitHatch.cs's
        // own header).
        var registry = BuildRegistry();

        var worldHatch = (EnterSplitHatch)registry.Get(ViewNames.World).EscapeHatches.Single(h => h.Kind == HatchKinds.EnterSplit);
        Assert.Equal(ViewNames.World, worldHatch.OwnerView);
        Assert.Equal(ViewNames.Reader, worldHatch.PartnerView);
        Assert.Equal(ViewNames.Reader, worldHatch.HostView);
    }

    // Fix round 1 (controller ruling 2, THE tripwire): "the conformance test
    // asserts every declared enter-split hatch's host renders through
    // CompositionSplit -- that is the tripwire that makes a pasted-wrapper
    // fourth host impossible." A source scan, disclosed (client.Tests has no
    // Razor-rendering harness) -- but a REAL one: it reads the actual
    // shipped .razor file for every distinct HostView across every declared
    // hatch and requires the literal `<CompositionSplit` usage, keyed to
    // THAT view's own `ViewNames` constant. A future host declared via a
    // hatch but wired with a pasted, ad-hoc wrapper (or no CompositionSplit
    // at all) fails this loudly.
    [Fact]
    public void HatchConformance_EveryHatchsHostView_RendersThroughCompositionSplit()
    {
        var registry = BuildRegistry();
        var repoRoot = RepoRoot();
        var checkedHostViews = new HashSet<string>();

        foreach (var view in registry.All)
        {
            foreach (var hatch in view.EscapeHatches.OfType<EnterSplitHatch>())
            {
                if (!checkedHostViews.Add(hatch.HostView))
                {
                    continue; // already checked this HostView via a different hatch (e.g. World's own hatch also names "reader")
                }

                var path = Path.Combine(repoRoot, "client", "Pages", Capitalize(hatch.HostView) + ".razor");
                Assert.True(File.Exists(path), $"Expected a page file at '{path}' for hatch HostView '{hatch.HostView}'.");

                var text = File.ReadAllText(path);
                Assert.Contains("<CompositionSplit", text);
                Assert.Contains($"HostName=\"@ViewNames.{Capitalize(hatch.HostView)}\"", text);
            }
        }

        Assert.NotEmpty(checkedHostViews); // never vacuous -- this batch's own ship list touches "reader" and "sources"
    }

    // ------------------------------------------------------------------
    // R6: arrangement-vocabulary -- every LayoutKind is a real, known value
    // (proven directly, structurally); unknown fails loud. CompositionSplit's
    // OWN "unknown kind" branch reads LayoutKinds.IsKnown -- see that
    // component's own header -- so this test proves the SAME predicate its
    // markup uses, not a source-text proxy for it. The actual RENDERING
    // proof (CompositionSplit genuinely handles single/split-h/an injected
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

    // ------------------------------------------------------------------
    // Fix round 1 (S-1, CRITICAL -- controller ruling 3): "the live
    // arrangement materializes through the compiled contract -- your
    // LayoutKind values implement ICompositionLayout, and the active
    // composition exposes an IViewComposition (Members = registry-backed
    // IView instances, Layout = the kind)." Proven directly against
    // ViewRegistry.ComposeFrom -- the SAME method CompositionSplit.razor
    // calls every render (not a parallel proof).
    // ------------------------------------------------------------------

    public static IEnumerable<object[]> RepresentativeArrangements()
    {
        yield return new object[] { ViewArrangement.Default };
        yield return new object[] { new ViewArrangement(new[] { ViewNames.World }, LayoutKinds.Single, null, false) };
        yield return new object[] { new ViewArrangement(new[] { ViewNames.Reader, ViewNames.World }, LayoutKinds.SplitH, 0.5, true) };
        yield return new object[] { new ViewArrangement(new[] { ViewNames.Sources, ViewNames.Reader }, LayoutKinds.SplitH, null, false) };
    }

    [Theory]
    [MemberData(nameof(RepresentativeArrangements))]
    public void ComposeFrom_ImplementsTheCompiledContract_MembersAndLayoutAgreeWithTheArrangementValue(ViewArrangement arrangement)
    {
        var registry = BuildRegistry();

        var composition = registry.ComposeFrom(arrangement);

        // The types themselves -- not just duck-typed shapes -- are the
        // compiled §4b contract (client/Contracts/Views.cs).
        Assert.IsAssignableFrom<IViewComposition>(composition);
        Assert.IsAssignableFrom<ICompositionLayout>(composition.Layout);

        // Agreement: the contract's own Members/Layout agree with the
        // ViewArrangement atom value this composition was built from.
        Assert.Equal(arrangement.LayoutKind, composition.Layout.Kind);
        Assert.Equal(arrangement.Members, composition.Members.Select(m => m.Name).ToList());
    }

    [Fact]
    public void ComposeFrom_MembersAreTheSameRegisteredViewInstancesTheRegistryHolds()
    {
        // Not fresh stand-ins per call -- the SAME IView object the
        // registry itself would hand back from Get(name), proving Members
        // is a real registry-backed projection, not a name echoed into a
        // lookalike shape.
        var registry = BuildRegistry();
        var arrangement = new ViewArrangement(new[] { ViewNames.Reader, ViewNames.World }, LayoutKinds.SplitH, null, true);

        var composition = registry.ComposeFrom(arrangement);

        Assert.Same(registry.Get(ViewNames.Reader), composition.Members[0]);
        Assert.Same(registry.Get(ViewNames.World), composition.Members[1]);
    }

    [Fact]
    public void ComposeFrom_TwoCallsAgainstTheSameArrangement_AgreeOnTheSameMemberInstances()
    {
        var registry = BuildRegistry();
        var arrangement = new ViewArrangement(new[] { ViewNames.Sources, ViewNames.Reader }, LayoutKinds.SplitH, null, false);

        var first = registry.ComposeFrom(arrangement);
        var second = registry.ComposeFrom(arrangement);

        Assert.Same(first.Members[0], second.Members[0]);
        Assert.Same(first.Members[1], second.Members[1]);
        Assert.Equal(first.Layout.Kind, second.Layout.Kind);
    }

    [Fact]
    public void ComposeFrom_EscapeHatches_IsTheUnionOfEveryMembersOwnHatches()
    {
        var registry = BuildRegistry();
        var arrangement = new ViewArrangement(new[] { ViewNames.Reader, ViewNames.World }, LayoutKinds.SplitH, null, true);

        var composition = registry.ComposeFrom(arrangement);

        Assert.Equal(
            registry.Get(ViewNames.Reader).EscapeHatches.Count + registry.Get(ViewNames.World).EscapeHatches.Count,
            composition.EscapeHatches.Count);
    }
}
