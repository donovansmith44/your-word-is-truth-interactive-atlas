using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Views;

namespace BibleAtlas.Client.State;

public readonly record struct PaneId(int Index)
{
    public static readonly PaneId Host = new(0);
    public static readonly PaneId Guest = new(1);
}

public sealed record PaneAtoms(StateAtom<Locus> Locus, StateAtom<TimeWindow> TimeWindow);

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

    public static bool IsSameViewSplit(ViewArrangement a) =>
        a.LayoutKind == LayoutKinds.SplitH && a.Members.Count == 2 && a.Members[0] == a.Members[1];

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

    public void ResetGuest() => _guest = null;

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

    // Dispatches the atoms' native intents (not a LinkDerivedIntent) so a downstream link inside
    // the target pane (e.g. World's FollowTextLink) still sees an origin-less change; a
    // LinkDerivedIntent's no-echo law would stop it one hop short. Never echoes back.
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
