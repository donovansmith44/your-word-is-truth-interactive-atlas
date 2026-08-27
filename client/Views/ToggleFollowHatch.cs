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
/// <see cref="ViewCapabilities.BearsLocus"/> (and is not on that file's own
/// pinned <c>FollowReleaseExemptViews</c> list) without one of these in its
/// own <see cref="Contracts.IView.EscapeHatches"/> list. World itself
/// declares <see cref="ViewCapabilities.BearsWindow"/>, not BearsLocus, so
/// it is not COMPELLED by that tripwire -- it still gets one declared here
/// (deliverable 0a, verbatim: "the world map's follow chip becomes the
/// declared instance of the same hatch kind") as the precedent instance for
/// its own window, for vocabulary completeness, not because the tripwire
/// demands it.
///
/// ADJUDICATION G (fix round, review): Reader used to declare one too
/// (BearsLocus literally compelled it) with NO rendered chip anywhere in the
/// app -- verified exhaustively (every <c>EscapeHatches</c> consumer in
/// shipped code selects by <c>Kind == EnterSplit</c> or by a specific view's
/// OWN name; nothing renders a generic "every hatch" button) to be genuinely
/// UNREACHABLE, not merely undemonstrated. Reader is the CANONICAL LOCUS
/// WRITER -- its own route already IS the shared value's projection
/// (CORPREAD-1a's URL-projection contract), so there is no external value
/// for it to stop following, and a "released Reader" would render a chapter
/// its own URL contradicts. A declared-but-unreachable hatch is contract
/// theater, the opposite of this codebase's "the vocabulary IS what's real"
/// discipline -- deleted; Reader is now exempted BY NAME (with this reason)
/// in <c>ViewRegistryConformanceTests.cs</c>'s own
/// <c>FollowReleaseExemptViews</c>, a closed/pinned list a future batch
/// cannot silently widen (that file's own companion test guards it). If
/// Reader ever gains a genuine guest-mode independent-browse capability, its
/// own release semantics and chip get designed and declared together, then.
///
/// S-4 (IMPORTANT, review, one-line justified rather than rewired): World's
/// own declared instance (<c>worldFollowHatch</c>, <c>ViewRegistrySetup.cs</c>)
/// is DECLARATION-ONLY -- its semantics remain owned by World's own chip
/// (<c>OnToggleFollowClick</c>, World.razor), which does strictly MORE than
/// this hatch's bare <c>Invoke()</c> (it also calls <c>SyncViewState()</c>
/// and awaits an <c>EnterTimeMode</c> reconcile). The two are NOT
/// interchangeable: invoking this hatch's own <c>Invoke()</c> for World would
/// flip Follow without that scene sync. Left un-rewired deliberately -- World's
/// own richer sequence is real, load-bearing, already-shipped behavior
/// (deliverable 0a's own "zero visual change to world" instruction), and
/// rewiring it through this generic closure is a real behavior-risk change
/// this batch's fix round did not need to take to satisfy the vocabulary
/// requirement (a registered `"toggle-follow"`-Kind hatch exists for World,
/// satisfying inventory/conformance completeness). Kretzmann's own instance
/// IS wired directly through <see cref="Invoke"/> (new code this batch,
/// no legacy risk) -- the two owners are deliberately NOT symmetric, and
/// this is why.
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
