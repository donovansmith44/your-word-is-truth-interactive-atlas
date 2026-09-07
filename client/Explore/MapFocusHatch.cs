namespace BibleAtlas.Client.Explore;

/// <summary>
/// EVT-3 Ticket 3 (the map-focus-at-time hatch -- owner verbatim,
/// EVENT-TIMEPLACE-1: "exploration of places belonging to events is a
/// function of that location and the event's time, and it yields a side
/// effect of the map opening with the appropriate state"): THE ONE named
/// site that builds a map-focus-at-time query -- carrying {place, window}
/// -- so any surface bearing a (place id, time window) pair composes it
/// the SAME way (the owner's own composability bar, "make sure all the
/// new bits made for this are composable and reusable"), never a one-off
/// inline string built per call site.
///
/// TERMINOLOGY (fix round 1, Q-1, review finding -- corrected labeling,
/// not a design change): this is a "declared, one-named-site" pattern in
/// the SAME spirit as design spec §5's own escape-hatch law, but it is
/// NOT an instance of this codebase's own FORMAL <see cref="Contracts.IEscapeHatch"/>
/// machinery (<c>HatchKinds</c>/<c>ViewRegistrySetup.Build</c>/
/// <c>ViewRegistryConformanceTests</c> -- see <c>EnterSplitHatch.cs</c>/
/// <c>ToggleFollowHatch.cs</c>) and this doc comment no longer claims that
/// it is. THE GENUINE CONFLICT (why NOT formalized, not merely undone
/// because it was inconvenient): every real <c>IEscapeHatch</c> in this
/// app is constructed EXACTLY ONCE, at <c>ViewRegistrySetup.Build()</c>
/// (registry-build) time, closing ONLY over DI-singleton services --
/// <c>Invoke()</c> is PARAMETERLESS and does the identical thing on every
/// call for a given (view, kind) pair (<c>EnterSplitHatch</c>'s own doc
/// comment, verbatim: "every one of OpenSplit/OpenReadBesideMap/the new
/// Sources hatch turns out to need nothing instance-specific at all, so a
/// hatch can be constructed ONCE"). <c>MapFocusHatch.Query</c> is the
/// opposite shape BY NECESSITY -- its own {place, window} pair is EVENT
/// DATA, varying per popover-section invocation (Nazareth/AD 31 today,
/// any other located event tomorrow), never a fixed per-VIEW fact
/// resolvable at registry-build time. Forcing it into `IEscapeHatch`
/// would require either (a) a breaking, controller-routed signature
/// change to the compiled `IEscapeHatch.Invoke()` contract itself,
/// rippling across every existing hatch, or (b) smuggling the current
/// {place, window} through hidden AMBIENT MUTABLE STATE set just before
/// `Invoke()` -- exactly the kind of implicit, non-instance-specific
/// state the registry's own "buildable once, nothing instance-specific"
/// design principle exists to rule out. Neither is proportionate to a
/// terminology finding; the correct fix is the label, not the wiring.
///
/// Realized as an ADDITIVE extension of the EXISTING, already-wired
/// <see cref="ExplorationTarget.NavigateWorld"/> shape (the SAME
/// <c>from=/to=</c> window every other map-opening chip already sends,
/// e.g. <see cref="EventNode.ExploreAsync"/>'s own "Show on the map"
/// chip) rather than a new <see cref="ExplorationTarget"/> case -- no new
/// data is needed to realize this: <c>Pages/World.razor</c> already fetches
/// a full <c>Scene</c> for any given window (every ordinary time-mode
/// navigation does), and that scene ALREADY carries every lit place's own
/// real Lat/Lon (<c>ScenePlace</c>) -- so an additive <c>place={id}</c>
/// param, read AFTER that same scene loads (World.razor's own
/// <c>ApplyExternalQuery</c>/<c>SyncFromQuery</c>), is genuinely zero new
/// fetches, just one more field read off data already in hand.
///
/// DELIVERABILITY (NAV-2 law: "no located place ⇒ no Place: row link" --
/// the same §4e "an affordance that opens something empty is worse than no
/// affordance at all" ruling HATCH-DELIVERABLE-1 already applied to the
/// event-level map chip) is the CALLER's own job: <see cref="Explore.EventDateAndPlacesSection"/>
/// only ever invokes this hatch for a place already present in
/// <c>EventDetail.Places</c>, itself only ever populated server-side for a
/// witness located WITHIN the event's own window -- see
/// <see cref="EventNode"/>'s own HATCH-DELIVERABLE-1 comment for the fuller
/// "Places.Count > 0" deliverability story this hatch inherits for free,
/// no re-derivation needed.
/// </summary>
public static class MapFocusHatch
{
    /// <param name="placeId">The place to focus -- looked up by id in the
    /// window's own scene once it loads (World.razor's own DebouncedLoadScene).</param>
    /// <param name="window">The event's own time window -- the SAME
    /// TimeRangeDto every other map-opening chip already sends as
    /// from=/to=.</param>
    public static string Query(string placeId, TimeRangeDto window) =>
        $"from={window.FromYear}&to={window.ToYear}&place={Uri.EscapeDataString(placeId)}";
}
