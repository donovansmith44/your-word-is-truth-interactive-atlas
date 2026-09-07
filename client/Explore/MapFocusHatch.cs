namespace BibleAtlas.Client.Explore;

/// <summary>
/// EVT-3 Ticket 3 (the §5-declared map-focus-at-time hatch -- owner
/// verbatim, EVENT-TIMEPLACE-1: "exploration of places belonging to events
/// is a function of that location and the event's time, and it yields a
/// side effect of the map opening with the appropriate state"): THE ONE
/// named site that builds a map-focus-at-time query -- carrying
/// {place, window} -- so any surface bearing a (place id, time window)
/// pair composes it the SAME way (the owner's own composability bar,
/// "make sure all the new bits made for this are composable and
/// reusable"), never a one-off inline string built per call site.
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
