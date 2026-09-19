using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Views;

namespace BibleAtlas.Client.State;

/// <summary>
/// D2 (owner, 2026-09-15: "a clean way to window ANY two things side by
/// side: multiple maps, multiple readers ... the cartesian product ... with
/// a toggle for whether the two panes are following each other"): which
/// pane of a split a component renders in. Cascaded by
/// <c>CompositionSplit</c> to its guest mount; a component with no
/// cascading value is the host (or standalone), so nothing existing changes.
/// </summary>
public readonly record struct PaneId(int Index)
{
    public static readonly PaneId Host = new(0);
    public static readonly PaneId Guest = new(1);
}

/// <summary>The atoms a pane reads and writes.</summary>
public sealed record PaneAtoms(StateAtom<Locus> Locus, StateAtom<TimeWindow> TimeWindow);

/// <summary>
/// D2: per-pane atom resolution. RULING R-D2-1 (behaviour-preserving
/// generalization): a pane owns PRIVATE atoms only when it would otherwise
/// mirror its host -- i.e. in a SAME-VIEW split (reader‖reader, world‖world).
/// Every cross-view pair (reader‖world, kretzmann‖reader, sources‖reader,
/// concord‖reader) keeps sharing the DI singletons exactly as before, so
/// every existing follow mechanism (World's own FollowTextLink, the
/// Kretzmann/Reader shared locus) is untouched. In a same-view split the
/// host keeps the singletons and the guest gets a private Locus/TimeWindow,
/// seeded from the host's current values at mint time (two readers open on
/// the same chapter, then diverge); <see cref="Link"/> is the one-way
/// host -> guest mirror the pair's follow toggle turns on.
/// </summary>
public sealed class PaneScopes
{
    private readonly StateAtom<ViewArrangement> _arrangement;
    private readonly PaneAtoms _host;
    private PaneAtoms? _guest;

    public PaneScopes(StateAtom<ViewArrangement> arrangement, StateAtom<Locus> locus, StateAtom<TimeWindow> timeWindow)
    {
        _arrangement = arrangement;
        _host = new PaneAtoms(locus, timeWindow);
    }

    /// <summary>True when the arrangement is a split whose two members are the same view.</summary>
    public static bool IsSameViewSplit(ViewArrangement a) =>
        a.LayoutKind == LayoutKinds.SplitH && a.Members.Count == 2 && a.Members[0] == a.Members[1];

    /// <summary>Whether the guest pane currently resolves to its own private atoms.</summary>
    public bool GuestIsPrivate => IsSameViewSplit(_arrangement.Value);

    public PaneAtoms For(PaneId pane)
    {
        if (pane != PaneId.Guest || !GuestIsPrivate)
        {
            return _host;
        }

        return _guest ??= new PaneAtoms(
            new StateAtom<Locus>(AtomNames.Locus + "/guest", _host.Locus.Value),
            new StateAtom<TimeWindow>(AtomNames.TimeWindow + "/guest", _host.TimeWindow.Value));
    }

    /// <summary>Forgets the guest pane's private atoms (the split closed).</summary>
    public void ResetGuest() => _guest = null;

    /// <summary>
    /// The capability a same-view pair can follow through: a locus-bearing
    /// view mirrors its chapter, a window-bearing view its time window;
    /// anything else has nothing to share. Cross-view pairs return
    /// <see cref="ViewCapabilities.None"/> here on purpose -- they follow
    /// through their own existing mechanisms (R-D2-1).
    /// </summary>
    public static ViewCapabilities LinkableVia(string hostView, string guestView, ViewCapabilities hostCaps, ViewCapabilities guestCaps)
    {
        if (hostView != guestView)
        {
            return ViewCapabilities.None;
        }

        if ((hostCaps & guestCaps & ViewCapabilities.BearsLocus) != 0)
        {
            return ViewCapabilities.BearsLocus;
        }

        if ((hostCaps & guestCaps & ViewCapabilities.BearsWindow) != 0)
        {
            return ViewCapabilities.BearsWindow;
        }

        return ViewCapabilities.None;
    }

    /// <summary>
    /// A one-way mirror <paramref name="from"/> -> <paramref name="to"/> over
    /// the atom <paramref name="via"/> names, synced immediately and on every
    /// source change; disposing unlinks. Dispatches the atoms' own native
    /// intents (SetLocus / SetTimeWindow / SetScriptureWindow) rather than a
    /// StateLinkRunner's LinkDerivedIntent so a downstream link inside the
    /// target pane (World's own FollowTextLink) still sees an origin-less
    /// change and keeps working -- the runner's no-echo law would otherwise
    /// stop it one hop short. Never echoes back: the target is never a source.
    /// </summary>
    public IDisposable Link(PaneId from, PaneId to, ViewCapabilities via)
    {
        var source = For(from);
        var target = For(to);
        if (ReferenceEquals(source, target) || via == ViewCapabilities.None)
        {
            return new PaneLink(null);
        }

        if (via == ViewCapabilities.BearsLocus)
        {
            void Mirror() => target.Locus.Dispatch(new SetLocus(source.Locus.Value.Book, source.Locus.Value.Chapter));
            source.Locus.Changed += Mirror;
            Mirror();
            return new PaneLink(() => source.Locus.Changed -= Mirror);
        }

        void MirrorWindow()
        {
            switch (source.TimeWindow.Value)
            {
                case TimeMode t:
                    target.TimeWindow.Dispatch(new SetTimeWindow(t.From, t.To));
                    break;
                case ScriptureMode s:
                    target.TimeWindow.Dispatch(new SetScriptureWindow(s.Ref));
                    break;
            }
        }

        source.TimeWindow.Changed += MirrorWindow;
        MirrorWindow();
        return new PaneLink(() => source.TimeWindow.Changed -= MirrorWindow);
    }

    private sealed class PaneLink : IDisposable
    {
        private Action? _unlink;
        public PaneLink(Action? unlink) => _unlink = unlink;
        public void Dispose()
        {
            _unlink?.Invoke();
            _unlink = null;
        }
    }
}
