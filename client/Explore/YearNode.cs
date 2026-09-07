using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch E: a curated established/destroyed date claim as its own
/// explorable node -- user direction (2026-08-19): "timestamps/timeranges
/// are THEMSELVES explorable: hover a year -&gt; an 'Explore' button -&gt;
/// supporting verses first... this whole thing is a giant DAG where every
/// node is explorable." Constructed by <c>PlaceCard</c>, which already has
/// the place name and the fetched <see cref="DateClaimOut"/> in hand -- no
/// AtlasClient fetch needed here, same pattern as <see cref="TimeAndPlaceNode"/>.
/// Title mirrors the card's own instrument-face text exactly (both go
/// through <see cref="YearText.FormatClaim"/>), e.g. "Established c. 1003 BC".
/// </summary>
public sealed class YearNode : IExplorable
{
    private readonly TimeRangeDto _when;
    private readonly List<string> _verses;

    // EVT-3 Ticket 3 (owner verbatim, EVENT-TIMEPLACE-1: "If i click on AD
    // 31 i should see, laid out, chronologically, the events that occurred
    // in that year"): true for a YearNode reached via an event's own
    // explorable Time: row (EventDateAndPlacesSection) -- BodyAsync below
    // branches on this to show that YEAR's own events instead of the
    // place-date-claim's own supporting verses. Rides the EXISTING YearNode
    // kind (Kind stays "Year" either way -- a client-synthesized view, not
    // a matrix-governed capability, per Contracts/Frontier.cs's own
    // body-only-kinds comment) rather than a new node kind, per the brief's
    // own "ride the existing YearNode + chronology data" instruction.
    private readonly bool _isEventTime;

    // Batch G2 decision 3: PlaceId + the raw Label (below) let
    // ExplorationDescriptor.Capture build a stable "{placeId}|{label}"
    // reconstruction key -- PlaceCard (this node's only PRE-EVT-3
    // constructor) already has Place.Id on hand for both its
    // "Established"/"Destroyed" call sites, so this costs it nothing new.
    // Reconstruction re-fetches AtlasClient.PlaceHistory(placeId) and reads
    // back whichever of History.Established/.Destroyed matches Label -- see
    // ExplorationDescriptor.cs's own doc comment. UNCHANGED by EVT-3 -- the
    // new event-time constructor below is purely additive.
    public YearNode(string placeId, string label, TimeRangeDto when, List<string> verses, string? note)
    {
        PlaceId = placeId;
        Label = label;
        Title = $"{label} {YearText.FormatClaim(when.FromYear, when.ToYear, note)}";
        _when = when;
        _verses = verses;
        _isEventTime = false;
    }

    /// <summary>
    /// EVT-3 Ticket 3: the event-time constructor -- reached from an
    /// event's own explorable Time: row (EventDateAndPlacesSection), which
    /// already has the event's own <see cref="TimeRangeDto"/> in hand (no
    /// fetch needed just to render the header, the same discipline every
    /// other <c>IExplorable</c> node in this file follows). Title is the
    /// SAME <see cref="YearText.FormatRange"/> text the Time: row's own
    /// value chip already shows (e.g. "AD 31") -- decisive identity, no
    /// second date format invented for this one path.
    /// </summary>
    public YearNode(TimeRangeDto when)
    {
        PlaceId = "";
        Label = "";
        Title = YearText.FormatRange(when.FromYear, when.ToYear);
        _when = when;
        _verses = new List<string>();
        _isEventTime = true;
    }

    public string PlaceId { get; }
    public string Label { get; }
    public string Title { get; }
    public string Kind => "Year";

    /// <summary>
    /// Leads with the curated SUPPORTING VERSES (one <see cref="VerseNode"/>
    /// push per verse, in curated order -- the order they best build the
    /// claim in, e.g. Jerusalem's established claim cites 2SA.5.6 then
    /// 2SA.5.7 then 2SA.5.9), then "Show this time on the map" last (DATE-1:
    /// "the card's date affordance opens the popover listing the curated
    /// supporting verses first").
    /// </summary>
    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        var list = new List<Exploration>();
        foreach (var vref in _verses)
        {
            var v = vref; // local copy -- captured per-chip by the Push target
            list.Add(new Exploration(v, $"popover-chip-verse-{v}", new ExplorationTarget.Push(new VerseNode(v))));
        }

        list.Add(new Exploration("Show this time on the map", "popover-chip-map",
            new ExplorationTarget.NavigateWorld($"from={_when.FromYear}&to={_when.ToYear}")));

        return Task.FromResult<IReadOnlyList<Exploration>>(list);
    }

    // EVT-3 Ticket 3: never actually invoked once YearFrontierSection
    // (Explore/PopoverSectionProviders.cs, Kind=="Year") is registered --
    // ExplorerPopover's own LoadCurrent only ever falls back to BodyAsync
    // when ZERO registered providers claim the current node's Kind
    // (PopoverSectionRegistry's own doc comment); a genuine explorable
    // frontier (the chronological event list below, and the pre-EVT-3
    // supporting-verses text) needs a real IPopoverSectionContext to push
    // through, which BodyAsync's own signature never carried -- see
    // ResolveFrontierAsync below, this node's own registry-facing
    // counterpart. Kept intact rather than deleted, same "defensive,
    // never-called-in-practice fallback" shape EventNode/PlaceNode/
    // TimeAndPlaceNode/PolityDeltaNode already document for their own
    // registry-covered kinds.
    public Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        var count = _verses.Count;
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-meta");
            builder.AddContent(2, count == 1 ? "1 supporting verse." : $"{count} supporting verses.");
            builder.CloseElement();
        };
        return Task.FromResult(fragment);
    }

    /// <summary>
    /// EVT-3 Ticket 3: this node's own REGISTRY-FACING body (see
    /// YearFrontierSection, Explore/PopoverSectionProviders.cs, the thin
    /// Kind=="Year" provider that calls straight through to this method) --
    /// the mode-branch lives here, alongside the state that decides it,
    /// rather than duplicated into the provider. `_isEventTime` picks
    /// between the two: the NEW chronological-year layout (below), or the
    /// pre-EVT-3 place-date-claim body, carried over byte-for-byte from the
    /// retired BodyAsync path above (still exactly "N supporting verses" --
    /// its own real explorable verse pushes stay chip-driven, via
    /// ExploreAsync above, UNCHANGED).
    /// </summary>
    public Task<PopoverSection?> ResolveFrontierAsync(AtlasClient api, IPopoverSectionContext ctx)
    {
        if (_isEventTime)
        {
            return ResolveChronologyAsync(api, ctx);
        }

        var count = _verses.Count;
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-meta");
            builder.AddContent(2, count == 1 ? "1 supporting verse." : $"{count} supporting verses.");
            builder.CloseElement();
        };
        return Task.FromResult<PopoverSection?>(new PopoverSection("year-body", fragment));
    }

    /// <summary>
    /// EVT-3 Ticket 3 (owner verbatim, EVENT-TIMEPLACE-1: "If i click on AD
    /// 31 i should see, laid out, chronologically, the events that occurred
    /// in that year"): fetches the SAME <c>Scene</c> every ordinary
    /// time-windowed /world visit already fetches (<c>AtlasClient.SceneTime</c>,
    /// window = THIS year -- one fetch, no waterfall) and lists every
    /// distinct dated event across every lit place in it, each row a real
    /// explorable push (<see cref="EventNode"/>, "event"-kind -- a
    /// time-windowed scene never surfaces a general-kind/dateless passage
    /// at all, since <c>SceneEvent</c> carries no key for one -- structural
    /// fact, not a filter this method applies itself). An event attested at
    /// more than one place in this same window (rare, but real) is
    /// deduped by id (first occurrence wins, server order) -- one row per
    /// event, never one per place.
    ///
    /// ORDERING (fix round 1, Q-2, review finding): sorted by each event's
    /// own <c>When.FromYear</c>, then <c>When.ToYear</c>, THEN label
    /// (ordinal) as the final, stable tiebreak -- genuinely chronological
    /// wherever the wire CAN distinguish two events (an event dated "AD
    /// 30-31" sorts before one dated "AD 31" alone, even though both
    /// legitimately appear in this SAME year-31 window) -- the prior
    /// revision sorted by label alone, which this fix round's own review
    /// correctly flagged as reading stronger than "chronologically"
    /// actually promised. HONEST LIMIT, disclosed rather than hidden: two
    /// events sharing the IDENTICAL `When` range (the common case for a
    /// single-year window, e.g. two events both dated exactly "AD 31")
    /// carry no wire signal finer than the year itself to order them BY --
    /// `SceneEvent` has no day/month/sequence field at all -- so those
    /// still fall back to the alphabetical tiebreak, same as before. This
    /// is a REAL improvement over pure-alphabetical, not a full fix to
    /// true intra-year sequencing (that would need a server-side ordering
    /// field, forbidden by the zero-server-changes mandate). Conditional presence:
    /// absent (no section at all) when this window's own scene has zero
    /// located, dated events -- an honestly rare case (the very event whose
    /// own Time: row was clicked is itself very likely IN this list,
    /// whenever it has a location at all).
    /// </summary>
    private async Task<PopoverSection?> ResolveChronologyAsync(AtlasClient api, IPopoverSectionContext ctx)
    {
        Scene scene;
        try
        {
            scene = await api.SceneTime(_when.FromYear, _when.ToYear);
        }
        catch (Exception)
        {
            return null; // fail soft -- same graceful-degradation policy every other lazy fetch in this app follows
        }

        var events = DedupeAndOrder(scene.Places.SelectMany(p => p.Events));

        if (events.Count == 0)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "div");
            builder.AddAttribute(seq++, "class", "popover-year-chronology");
            builder.AddAttribute(seq++, "data-testid", "year-chronology");
            foreach (var e in events)
            {
                var id = e.Id; // local copies -- captured per-row by the onclick closure below
                var label = e.Label;
                builder.OpenElement(seq++, "button");
                builder.AddAttribute(seq++, "type", "button");
                builder.AddAttribute(seq++, "class", "popover-event-row popover-event-row-button explorable");
                builder.AddAttribute(seq++, "data-testid", $"year-chronology-event-{id}");
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new EventNode(id, label, "event"))));
                builder.AddContent(seq++, label);
                builder.CloseElement();
            }
            builder.CloseElement();
        };
        return new PopoverSection("year-chronology", body);
    }

    /// <summary>
    /// Pure: dedupes an event-time window's own events by id (first
    /// occurrence wins, server order -- an event attested at more than one
    /// place in the SAME window is one row, never one per place) and
    /// orders them by <c>When.FromYear</c>, then <c>When.ToYear</c>, then
    /// label (ordinal) as the final tiebreak. See
    /// <see cref="ResolveChronologyAsync"/>'s own doc comment for the full
    /// "why this ordering, and its honest limit" story (fix round 1, Q-2).
    /// Extracted as pure logic (no fetch) so it is directly, cheaply unit
    /// testable -- client.Tests/YearNodeEventTimeTests.cs.
    /// </summary>
    public static List<SceneEvent> DedupeAndOrder(IEnumerable<SceneEvent> events) =>
        events
            .GroupBy(e => e.Id)
            .Select(g => g.First())
            .OrderBy(e => e.When.FromYear)
            .ThenBy(e => e.When.ToYear)
            .ThenBy(e => e.Label, StringComparer.Ordinal)
            .ToList();
}
