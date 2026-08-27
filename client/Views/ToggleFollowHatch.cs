using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.Views;

/// <summary>
/// Batch CORPREAD-1b, DELIVERABLE 0a -- THE FOLLOW-RELEASE LAW (design spec
/// §5, owner ruling 2026-08-27, verbatim: "on every thing sharing state with
/// the reader state, we need an escape hatch to stop following the reader as
/// well"). The ONE concrete <see cref="IEscapeHatch"/> declared, once per
/// owner, by every locus-bearing view -- <see cref="EnterSplitHatch"/>'s own
/// sibling: same "declared once, at registry-build time, over DI-singleton
/// services only" shape (<see cref="ViewRegistrySetup.Build"/>'s own
/// construction site), because <c>ToggleFollow</c> (client/State/
/// ViewArrangement.cs) is ALREADY a pure write against the ONE shared
/// <c>ViewArrangement</c> atom -- exactly what a hatch closure needs to be
/// buildable once and invoked from any live button that happens to render
/// it, the same reasoning <see cref="EnterSplitHatch"/>'s own header already
/// gives.
///
/// <see cref="OwnerView"/> is the ONLY per-hatch data this record carries
/// (unlike <see cref="EnterSplitHatch"/>, there is no partner/host to
/// declare -- toggling Follow never changes WHO is in the split, only
/// whether the current locus-bearing member's own display tracks the shared
/// Locus atom or a view-local, released position). <see cref="Invoke"/> is
/// shared across every declaring owner (Reader/World/Kretzmann all pass the
/// SAME closure, per <see cref="ViewRegistrySetup.Build"/>'s own
/// construction) since Follow is genuinely one flag on the one shared
/// arrangement, not a per-owner copy -- flipping it always means the same
/// thing regardless of which view's own chip fired it.
///
/// CONFORMANCE (the follow-release law's own §5 tripwire, this batch's
/// deliverable 0a): <c>ViewRegistryConformanceTests.cs</c>'s
/// <c>HatchConformance_EveryBearsLocusView_DeclaresAToggleFollowHatch</c>
/// fails loud if a registered view declares
/// <see cref="ViewCapabilities.BearsLocus"/> without one of these in its own
/// <see cref="Contracts.IView.EscapeHatches"/> list. World itself declares
/// <see cref="ViewCapabilities.BearsWindow"/>, not BearsLocus, so it is not
/// COMPELLED by that tripwire -- it still gets one declared here (deliverable
/// 0a, verbatim: "the world map's follow chip becomes the declared instance
/// of the same hatch kind") as the precedent instance for its own window,
/// for vocabulary completeness, not because the tripwire demands it.
/// </summary>
public sealed class ToggleFollowHatch : IEscapeHatch
{
    private readonly Func<Task> _invoke;

    public ToggleFollowHatch(string ownerView, Func<Task> invoke)
    {
        OwnerView = ownerView;
        _invoke = invoke;
    }

    public string Kind => HatchKinds.ToggleFollow;

    public string OwnerView { get; }

    public Task Invoke() => _invoke();
}
