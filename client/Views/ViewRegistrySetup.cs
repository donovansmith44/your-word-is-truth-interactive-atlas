using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Pages;
using BibleAtlas.Client.State;
using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Rendering;

namespace BibleAtlas.Client.Views;

public static class ViewRegistrySetup
{
    public static ViewRegistry Build(StateAtom<ViewArrangement> arrangement, ViewStateService viewState, StateAtom<Locus> locus, NavigationManager nav)
    {
        Task EnterSplitReaderHostsWorld()
        {
            arrangement.Dispatch(new EnterSplit(ViewNames.Reader, ViewNames.World, viewState.Map.Follow, viewState.Map.DividerFraction));
            return Task.CompletedTask;
        }

        Task EnterSplitWorldRequestsReader()
        {
            // World never hosts, so there is no live CompositionSplit here to resync the URL through --
            // this is the one hatch that still builds its own Nav.NavigateTo.
            var follow = viewState.Map.Follow ? $"&{SplitUrlContract.FollowParam}={SplitUrlContract.FollowTrueValue}" : "";
            nav.NavigateTo($"/read/{locus.Value.Book}/{locus.Value.Chapter}?{SplitUrlContract.SplitParam}={ViewNames.World}{follow}");
            return Task.CompletedTask;
        }

        Task EnterSplitSourcesHostsReader()
        {
            arrangement.Dispatch(new EnterSplit(ViewNames.Sources, ViewNames.Reader, DefaultFollow: false, DefaultDividerFraction: null));
            return Task.CompletedTask;
        }

        Task EnterSplitKretzmannHostsReader()
        {
            arrangement.Dispatch(new EnterSplit(ViewNames.Kretzmann, ViewNames.Reader, DefaultFollow: true, DefaultDividerFraction: null));
            return Task.CompletedTask;
        }

        Task EnterSplitConcordHostsReader()
        {
            arrangement.Dispatch(new EnterSplit(ViewNames.Concord, ViewNames.Reader, DefaultFollow: false, DefaultDividerFraction: null));
            return Task.CompletedTask;
        }

        Task ToggleFollowGlobal()
        {
            arrangement.Dispatch(new ToggleFollow(!arrangement.Value.Follow));
            return Task.CompletedTask;
        }

        // worldFollowHatch is declaration-only: World's own chip (OnToggleFollowClick) does strictly more
        // than Invoke() here (it also syncs scene state), so the two are not interchangeable.
        var worldFollowHatch = new ToggleFollowHatch(ViewNames.World, ToggleFollowGlobal);
        var kretzmannFollowHatch = new ToggleFollowHatch(ViewNames.Kretzmann, ToggleFollowGlobal);

        Task EnterSplitAny(string host, string guest)
        {
            arrangement.Dispatch(new EnterSplit(host, guest, DefaultFollow: false, DefaultDividerFraction: null));
            return Task.CompletedTask;
        }

        var readerHatch = new EnterSplitHatch(ViewNames.Reader, new[] { ViewNames.World, ViewNames.Reader }, hostView: ViewNames.Reader,
            guest => guest == ViewNames.World ? EnterSplitReaderHostsWorld() : EnterSplitAny(ViewNames.Reader, guest),
            guestLabel: guest => guest == ViewNames.World ? "The map" : "Another reader");
        var worldHatch = new EnterSplitHatch(ViewNames.World, new[] { ViewNames.Reader, ViewNames.World }, hostView: ViewNames.Reader,
            guest => guest == ViewNames.Reader ? EnterSplitWorldRequestsReader() : EnterSplitAny(ViewNames.World, guest),
            guestLabel: guest => guest == ViewNames.Reader ? "The text" : "Another map");
        var sourcesHatch = new EnterSplitHatch(ViewNames.Sources, ViewNames.Reader, hostView: ViewNames.Sources, EnterSplitSourcesHostsReader);
        var kretzmannHatch = new EnterSplitHatch(ViewNames.Kretzmann, ViewNames.Reader, hostView: ViewNames.Kretzmann, EnterSplitKretzmannHostsReader);
        var concordHatch = new EnterSplitHatch(ViewNames.Concord, ViewNames.Reader, hostView: ViewNames.Concord, EnterSplitConcordHostsReader);

        var views = new List<RegisteredView>
        {
            new(ViewNames.Reader, ViewCapabilities.BearsLocus, ctx => builder =>
            {
                builder.OpenComponent<Reader>(0);
                builder.AddAttribute(1, nameof(Reader.SplitMode), (bool?)ctx.SplitMode);
                builder.CloseComponent();
            }, new IEscapeHatch[] { readerHatch }),

            new(ViewNames.World, ViewCapabilities.BearsWindow, ctx => builder =>
            {
                builder.OpenComponent<World>(0);
                builder.AddAttribute(1, nameof(World.SplitMode), ctx.SplitMode);
                builder.AddAttribute(2, nameof(World.OnRequestClose), ctx.OnRequestClose);
                builder.AddAttribute(3, nameof(World.RegisterQueryHandler), ctx.RegisterQueryHandler);
                builder.CloseComponent();
            }, new IEscapeHatch[] { worldHatch, worldFollowHatch }),

            new(ViewNames.Sources, ViewCapabilities.None, ctx => builder =>
            {
                builder.OpenComponent<Sources>(0);
                builder.AddAttribute(1, nameof(Sources.SplitMode), (bool?)ctx.SplitMode);
                builder.CloseComponent();
            }, new IEscapeHatch[] { sourcesHatch }),

            new(ViewNames.Kretzmann, ViewCapabilities.BearsLocus, ctx => builder =>
            {
                builder.OpenComponent<Kretzmann>(0);
                builder.AddAttribute(1, nameof(Kretzmann.SplitMode), (bool?)ctx.SplitMode);
                builder.CloseComponent();
            }, new IEscapeHatch[] { kretzmannHatch, kretzmannFollowHatch }),

            new(ViewNames.Concord, ViewCapabilities.None, ctx => builder =>
            {
                builder.OpenComponent<Concord>(0);
                builder.AddAttribute(1, nameof(Concord.SplitMode), (bool?)ctx.SplitMode);
                builder.CloseComponent();
            }, new IEscapeHatch[] { concordHatch }),
        };

        return new ViewRegistry(views);
    }
}
