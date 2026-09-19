using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.State;
using BibleAtlas.Client.Views;
using Xunit;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// D2 (owner, 2026-09-15): pane-scoped state. RULING R-D2-1: a guest pane owns
/// private atoms only in a SAME-VIEW split; every cross-view pair keeps the
/// singletons (behaviour-preserving for reader‖world, kretzmann‖reader, ...).
/// </summary>
public class PaneScopeTests
{
    private static (PaneScopes Scopes, StateAtom<ViewArrangement> Arrangement, StateAtom<Locus> Locus, StateAtom<TimeWindow> Window) Build(ViewArrangement? arrangement = null)
    {
        var a = new StateAtom<ViewArrangement>(AtomNames.ViewArrangement, arrangement ?? ViewArrangement.Default);
        var l = new StateAtom<Locus>(AtomNames.Locus, Locus.Default);
        var w = new StateAtom<TimeWindow>(AtomNames.TimeWindow, TimeWindow.Default);
        return (new PaneScopes(a, l, w), a, l, w);
    }

    private static ViewArrangement Split(string host, string guest, bool follow = false) =>
        new(new[] { host, guest }, LayoutKinds.SplitH, null, follow);

    [Fact]
    public void Host_pane_atoms_are_the_DI_singletons_always()
    {
        var (scopes, _, locus, window) = Build(Split(ViewNames.Reader, ViewNames.Reader));
        Assert.Same(locus, scopes.For(PaneId.Host).Locus);
        Assert.Same(window, scopes.For(PaneId.Host).TimeWindow);
    }

    [Fact]
    public void Cross_view_guest_keeps_the_singletons_R_D2_1()
    {
        var (scopes, _, locus, window) = Build(Split(ViewNames.Reader, ViewNames.World, follow: true));
        Assert.False(scopes.GuestIsPrivate);
        Assert.Same(locus, scopes.For(PaneId.Guest).Locus);
        Assert.Same(window, scopes.For(PaneId.Guest).TimeWindow);
    }

    [Fact]
    public void Same_view_guest_gets_private_stable_atoms_seeded_from_the_host()
    {
        var (scopes, _, locus, _) = Build(Split(ViewNames.Reader, ViewNames.Reader));
        locus.Dispatch(new SetLocus("EXO", 20));
        var g1 = scopes.For(PaneId.Guest);
        var g2 = scopes.For(PaneId.Guest);
        Assert.True(scopes.GuestIsPrivate);
        Assert.NotSame(locus, g1.Locus);
        Assert.Same(g1.Locus, g2.Locus);
        Assert.Equal("EXO.20", g1.Locus.Value.Ref);
    }

    [Fact]
    public void Two_readers_hold_two_different_chapters_without_follow()
    {
        var (scopes, _, _, _) = Build(Split(ViewNames.Reader, ViewNames.Reader));
        scopes.For(PaneId.Host).Locus.Dispatch(new SetLocus("GEN", 1));
        scopes.For(PaneId.Guest).Locus.Dispatch(new SetLocus("EXO", 20));
        Assert.Equal("GEN.1", scopes.For(PaneId.Host).Locus.Value.Ref);
        Assert.Equal("EXO.20", scopes.For(PaneId.Guest).Locus.Value.Ref);
    }

    [Fact]
    public void Locus_link_mirrors_host_into_guest_one_way_and_unlinks_on_dispose()
    {
        var (scopes, _, _, _) = Build(Split(ViewNames.Reader, ViewNames.Reader, follow: true));
        scopes.For(PaneId.Guest).Locus.Dispatch(new SetLocus("EXO", 20));
        using (scopes.Link(PaneId.Host, PaneId.Guest, ViewCapabilities.BearsLocus))
        {
            Assert.Equal("GEN.1", scopes.For(PaneId.Guest).Locus.Value.Ref); // synced on link
            scopes.For(PaneId.Host).Locus.Dispatch(new SetLocus("JHN", 3));
            Assert.Equal("JHN.3", scopes.For(PaneId.Guest).Locus.Value.Ref);
            scopes.For(PaneId.Guest).Locus.Dispatch(new SetLocus("ROM", 8));
            Assert.Equal("JHN.3", scopes.For(PaneId.Host).Locus.Value.Ref); // guest never writes back
        }

        scopes.For(PaneId.Host).Locus.Dispatch(new SetLocus("PSA", 23));
        Assert.Equal("ROM.8", scopes.For(PaneId.Guest).Locus.Value.Ref); // unlinked
    }

    [Fact]
    public void Window_link_mirrors_both_time_and_scripture_modes()
    {
        var (scopes, _, _, _) = Build(Split(ViewNames.World, ViewNames.World, follow: true));
        using var _ = scopes.Link(PaneId.Host, PaneId.Guest, ViewCapabilities.BearsWindow);
        scopes.For(PaneId.Host).TimeWindow.Dispatch(new SetTimeWindow(-1000, -900));
        Assert.Equal(new TimeMode(-1000, -900), scopes.For(PaneId.Guest).TimeWindow.Value);
        scopes.For(PaneId.Host).TimeWindow.Dispatch(new SetScriptureWindow("GEN.12"));
        Assert.Equal(new ScriptureMode("GEN.12"), scopes.For(PaneId.Guest).TimeWindow.Value);
    }

    [Fact]
    public void ResetGuest_forgets_the_private_atoms()
    {
        var (scopes, _, _, _) = Build(Split(ViewNames.Reader, ViewNames.Reader));
        var g = scopes.For(PaneId.Guest);
        scopes.ResetGuest();
        Assert.NotSame(g.Locus, scopes.For(PaneId.Guest).Locus);
    }

    [Theory]
    [InlineData(ViewNames.Reader, ViewNames.Reader, ViewCapabilities.BearsLocus)]
    [InlineData(ViewNames.World, ViewNames.World, ViewCapabilities.BearsWindow)]
    [InlineData(ViewNames.Reader, ViewNames.World, ViewCapabilities.None)]
    [InlineData(ViewNames.Concord, ViewNames.Concord, ViewCapabilities.None)]
    public void LinkableVia_is_same_view_only_by_capability(string host, string guest, ViewCapabilities expected)
    {
        static ViewCapabilities Caps(string v) => v switch
        {
            ViewNames.Reader => ViewCapabilities.BearsLocus,
            ViewNames.World => ViewCapabilities.BearsWindow,
            _ => ViewCapabilities.None,
        };
        Assert.Equal(expected, PaneScopes.LinkableVia(host, guest, Caps(host), Caps(guest)));
    }
}
