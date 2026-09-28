using System.Reflection;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.State;
using BibleAtlas.Client.Views;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Tests.State;

public class ViewRegistryConformanceTests
{
    private sealed class FakeNavigationManager : NavigationManager
    {
        public FakeNavigationManager() => Initialize("https://example.test/", "https://example.test/");

        protected override void NavigateToCore(string uri, NavigationOptions options)
        {
        }
    }

    private static ViewRegistry BuildRegistry() => BuildRegistry(new StateAtom<ViewArrangement>(AtomNames.ViewArrangement, ViewArrangement.Default));

    private static ViewRegistry BuildRegistry(StateAtom<ViewArrangement> arrangement) => ViewRegistrySetup.Build(
        arrangement,
        new ViewStateService(),
        new StateAtom<Locus>(AtomNames.Locus, Locus.Default),
        new FakeNavigationManager());

    private static List<string> ViewNamesConstants() =>
        typeof(ViewNames).GetFields(BindingFlags.Public | BindingFlags.Static)
            .Where(f => f.FieldType == typeof(string))
            .Select(f => (string)f.GetValue(null)!)
            .ToList();

    private static string Capitalize(string name) => char.ToUpperInvariant(name[0]) + name[1..];

    [Fact]
    public void Registry_EveryViewNamesConstant_ResolvesInTheRegistry()
    {
        var registry = BuildRegistry();
        var names = ViewNamesConstants();

        Assert.NotEmpty(names);
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

        Assert.Equal(5, values.Count);
        Assert.Equal(values.Count, values.Distinct().Count());
    }

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

    [Fact]
    public void Registry_CapabilityData_MatchesR1sOwnAssignment()
    {
        var registry = BuildRegistry();

        Assert.Equal(ViewCapabilities.BearsLocus, registry.CapabilitiesOf(ViewNames.Reader));
        Assert.Equal(ViewCapabilities.BearsWindow, registry.CapabilitiesOf(ViewNames.World));
        Assert.Equal(ViewCapabilities.None, registry.CapabilitiesOf(ViewNames.Sources));

        Assert.Equal(ViewCapabilities.BearsLocus, registry.CapabilitiesOf(ViewNames.Kretzmann));
        Assert.Equal(ViewCapabilities.None, registry.CapabilitiesOf(ViewNames.Concord));
    }

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
                foreach (var partner in hatch.PartnerViews)
                {
                    Assert.True(registry.TryGet(partner, out _), $"Hatch partner '{partner}' does not resolve in the registry.");
                }
                Assert.True(registry.TryGet(hatch.HostView, out _), $"Hatch HostView '{hatch.HostView}' does not resolve in the registry.");
                Assert.NotEqual(hatch.OwnerView, hatch.PartnerView);
            }
        }

        Assert.Equal(5, hatchesFound);
    }

    [Fact]
    public void HatchConformance_D2_GuestMenus_ReaderAndWorldOfferEachOtherThenThemselves_OthersOfferReaderOnly()
    {
        var registry = BuildRegistry();
        EnterSplitHatch Hatch(string view) => registry.Get(view).EscapeHatches.OfType<EnterSplitHatch>().Single();
        Assert.Equal(new[] { ViewNames.World, ViewNames.Reader }, Hatch(ViewNames.Reader).PartnerViews);
        Assert.Equal(new[] { ViewNames.Reader, ViewNames.World }, Hatch(ViewNames.World).PartnerViews);
        foreach (var other in new[] { ViewNames.Sources, ViewNames.Kretzmann, ViewNames.Concord })
        {
            Assert.Equal(new[] { ViewNames.Reader }, Hatch(other).PartnerViews);
        }
        Assert.Throws<ArgumentException>(() => { Hatch(ViewNames.Concord).InvokeWith(ViewNames.World).GetAwaiter().GetResult(); });
    }

    [Fact]
    public void HatchConformance_D2_SameViewSplit_EntersWithMembersHostHost_NotFollowing()
    {
        var arrangement = new StateAtom<ViewArrangement>(BibleAtlas.Client.Contracts.AtomNames.ViewArrangement, ViewArrangement.Default);
        var registry = BuildRegistry(arrangement);
        registry.Get(ViewNames.Reader).EscapeHatches.OfType<EnterSplitHatch>().Single().InvokeWith(ViewNames.Reader).GetAwaiter().GetResult();
        Assert.Equal(new[] { ViewNames.Reader, ViewNames.Reader }, arrangement.Value.Members);
        Assert.Equal(LayoutKinds.SplitH, arrangement.Value.LayoutKind);
        Assert.False(arrangement.Value.Follow);
        registry.Get(ViewNames.World).EscapeHatches.OfType<EnterSplitHatch>().Single().InvokeWith(ViewNames.World).GetAwaiter().GetResult();
        Assert.Equal(new[] { ViewNames.World, ViewNames.World }, arrangement.Value.Members);
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
        var registry = BuildRegistry();

        var worldHatch = (EnterSplitHatch)registry.Get(ViewNames.World).EscapeHatches.Single(h => h.Kind == HatchKinds.EnterSplit);
        Assert.Equal(ViewNames.World, worldHatch.OwnerView);
        Assert.Equal(ViewNames.Reader, worldHatch.PartnerView);
        Assert.Equal(ViewNames.Reader, worldHatch.HostView);
    }

    [Fact]
    public void HatchConformance_EveryHatchsHostView_RendersThroughCompositionSplit()
    {
        var registry = BuildRegistry();
        var repoRoot = ConformanceTests.RepoRoot();
        var checkedHostViews = new HashSet<string>();

        foreach (var view in registry.All)
        {
            foreach (var hatch in view.EscapeHatches.OfType<EnterSplitHatch>())
            {
                if (!checkedHostViews.Add(hatch.HostView))
                {
                    continue;
                }

                var path = Path.Combine(repoRoot, "client", "Pages", Capitalize(hatch.HostView) + ".razor");
                Assert.True(File.Exists(path), $"Expected a page file at '{path}' for hatch HostView '{hatch.HostView}'.");

                var text = File.ReadAllText(path);
                Assert.Contains("<CompositionSplit", text);
                Assert.Contains($"HostName=\"@ViewNames.{Capitalize(hatch.HostView)}\"", text);
            }
        }

        Assert.NotEmpty(checkedHostViews);
    }

    private static readonly string[] FollowReleaseExemptViews = { ViewNames.Reader };

    private static bool ViolatesFollowReleaseLaw(RegisteredView view) =>
        view.Capabilities.HasFlag(ViewCapabilities.BearsLocus)
        && !FollowReleaseExemptViews.Contains(view.Name)
        && !view.EscapeHatches.Any(h => h.Kind == HatchKinds.ToggleFollow);

    [Fact]
    public void HatchConformance_EveryBearsLocusView_DeclaresAToggleFollowHatch()
    {
        var registry = BuildRegistry();
        var bearsLocusViews = registry.All
            .Where(v => v.Capabilities.HasFlag(ViewCapabilities.BearsLocus))
            .Where(v => !FollowReleaseExemptViews.Contains(v.Name))
            .ToList();

        Assert.Single(bearsLocusViews);

        foreach (var view in bearsLocusViews)
        {
            Assert.False(ViolatesFollowReleaseLaw(view), $"'{view.Name}' declares BearsLocus without a toggle-follow hatch and is not on the exemption list.");
        }
    }

    [Fact]
    public void HatchConformance_TheFollowReleaseExemptionList_IsExactlyTheCanonicalLocusWriter()
    {
        Assert.Equal(new[] { ViewNames.Reader }, FollowReleaseExemptViews);
    }

    [Fact]
    public void HatchConformance_PlantedBearsLocusViewWithNoToggleFollowHatch_FailsTheLawsOwnCheck()
    {
        RenderFragment Empty(ViewMountContext ctx) => builder => { };
        var offender = new RegisteredView("planted-locus-view", ViewCapabilities.BearsLocus, Empty,
            new IEscapeHatch[] { new EnterSplitHatch("planted-locus-view", ViewNames.Reader, "planted-locus-view", () => Task.CompletedTask) });

        Assert.True(ViolatesFollowReleaseLaw(offender), "The planted view declares BearsLocus with no toggle-follow hatch -- the law's own check must catch this, or the real-registry proof above is vacuous.");
    }

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
    [InlineData("SPLIT-H")]
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
        var arrangement = new StateAtom<ViewArrangement>(AtomNames.ViewArrangement, ViewArrangement.Default);
        var registry = ViewRegistrySetup.Build(arrangement, new ViewStateService(), new StateAtom<Locus>(AtomNames.Locus, Locus.Default), new FakeNavigationManager());

        var hatch = registry.Get(ViewNames.Sources).EscapeHatches.Single(h => h.Kind == HatchKinds.EnterSplit);
        await hatch.Invoke();

        Assert.Equal(LayoutKinds.SplitH, arrangement.Value.LayoutKind);
        Assert.Equal(new[] { ViewNames.Sources, ViewNames.Reader }, arrangement.Value.Members);
    }

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

        Assert.IsAssignableFrom<IViewComposition>(composition);
        Assert.IsAssignableFrom<ICompositionLayout>(composition.Layout);

        Assert.Equal(arrangement.LayoutKind, composition.Layout.Kind);
        Assert.Equal(arrangement.Members, composition.Members.Select(m => m.Name).ToList());
    }

    [Fact]
    public void ComposeFrom_MembersAreTheSameRegisteredViewInstancesTheRegistryHolds()
    {
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
