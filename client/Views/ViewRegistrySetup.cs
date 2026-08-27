using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Pages;
using BibleAtlas.Client.State;
using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Rendering;

namespace BibleAtlas.Client.Views;

/// <summary>
/// R1/R4: builds the ONE <see cref="ViewRegistry"/> instance (registered
/// singleton, Program.cs) -- three registrations (Reader/World/Sources),
/// each a mount factory (a <see cref="RenderTreeBuilder"/> closure -- the
/// same low-level mechanism <see cref="RenderFragment"/> itself is defined
/// in terms of, letting <see cref="Components.CompositionSplit"/> mount ANY
/// registered view by NAME with zero per-view branching of its own -- fix
/// round 2, N-6 doc rot: this used to name the retired
/// <c>Components.CompositionHost</c>) plus a
/// declared enter-split hatch where one exists today.
///
/// Every hatch closure below is expressed purely against DI-singleton
/// services (<paramref name="arrangement"/>/<paramref name="viewState"/>/
/// <paramref name="locus"/>/<paramref name="nav"/>, all already
/// app-lifetime singletons per Program.cs) -- see
/// <see cref="EnterSplitHatch"/>'s own header for why this needs no live
/// component instance at all, and can therefore be built exactly ONCE, here,
/// at DI-registration time.
/// </summary>
public static class ViewRegistrySetup
{
    public static ViewRegistry Build(StateAtom<ViewArrangement> arrangement, ViewStateService viewState, StateAtom<Locus> locus, NavigationManager nav)
    {
        Task EnterSplitReaderHostsWorld()
        {
            // Mirrors Reader.razor's own pre-VC-1 OpenSplit exactly (byte-
            // identical dispatch + query-string shape) -- see that method's
            // retirement note.
            arrangement.Dispatch(new EnterSplit(ViewNames.Reader, ViewNames.World, viewState.Map.Follow, viewState.Map.DividerFraction));
            var uri = nav.GetUriWithQueryParameters(new Dictionary<string, object?> { ["split"] = "1" });
            nav.NavigateTo(uri, replace: true);
            return Task.CompletedTask;
        }

        Task EnterSplitWorldRequestsReader()
        {
            // Mirrors World.razor's own pre-VC-1 OpenReadBesideMap exactly:
            // READER still ends up hosting (R7 -- byte-identical user-visible
            // behavior for the two pre-existing entry points), reached via
            // the SAME navigation, not a locally-hosted "world hosts reader"
            // arrangement -- the owner ruling retires the PRIVILEGE (any view
            // COULD host), not this pairing's own established shape.
            nav.NavigateTo($"/read/{locus.Value.Book}/{locus.Value.Chapter}?split=1");
            return Task.CompletedTask;
        }

        Task EnterSplitSourcesHostsReader()
        {
            // R4's own generality proof: Sources stays on its own URL (no
            // navigation -- unlike the reader/world pair, Sources has no
            // per-route split query to keep in sync, disclosed limitation:
            // a hard refresh while this split is open does not restore it,
            // matching this atom's own baseline -- ViewArrangement was never
            // localStorage-persisted either, see that atom's own header) and
            // becomes host; Reader becomes guest.
            arrangement.Dispatch(new EnterSplit(ViewNames.Sources, ViewNames.Reader, DefaultFollow: false, DefaultDividerFraction: null));
            return Task.CompletedTask;
        }

        // Batch CORP-1 (R2/R3): Kretzmann and Concord each declare their OWN
        // "read-beside" hatch, self-hosting -- the SAME shape Sources' own
        // EnterSplitSourcesHostsReader establishes immediately above (owner
        // becomes host, Reader becomes guest, no per-route split query to
        // keep in sync). Kretzmann PROJECTS the shared Locus atom directly
        // (R2) -- it never navigates on entering split; the already-shared
        // atom is why the guest Reader pane tracks it "by construction," not
        // a link. Concord (R3) is identical in shape, just with a different
        // owner name; it bears no locus/window capability, so nothing about
        // this hatch differs from Sources' own proof that any two views may
        // pair this way.
        Task EnterSplitKretzmannHostsReader()
        {
            arrangement.Dispatch(new EnterSplit(ViewNames.Kretzmann, ViewNames.Reader, DefaultFollow: false, DefaultDividerFraction: null));
            return Task.CompletedTask;
        }

        Task EnterSplitConcordHostsReader()
        {
            arrangement.Dispatch(new EnterSplit(ViewNames.Concord, ViewNames.Reader, DefaultFollow: false, DefaultDividerFraction: null));
            return Task.CompletedTask;
        }

        // Fix round 1 (controller ruling 2): HostView is the hosting
        // declaration -- Reader's and Sources' own hatches host themselves;
        // World's own hatch (declared by World, "Read beside the map")
        // still makes READER the host, byte-identical to pre-VC-1 (R7) --
        // see EnterSplitHatch.cs's own header for the OwnerView-vs-HostView
        // distinction this encodes.
        var readerHatch = new EnterSplitHatch(ViewNames.Reader, ViewNames.World, hostView: ViewNames.Reader, EnterSplitReaderHostsWorld);
        var worldHatch = new EnterSplitHatch(ViewNames.World, ViewNames.Reader, hostView: ViewNames.Reader, EnterSplitWorldRequestsReader);
        var sourcesHatch = new EnterSplitHatch(ViewNames.Sources, ViewNames.Reader, hostView: ViewNames.Sources, EnterSplitSourcesHostsReader);
        var kretzmannHatch = new EnterSplitHatch(ViewNames.Kretzmann, ViewNames.Reader, hostView: ViewNames.Kretzmann, EnterSplitKretzmannHostsReader);
        var concordHatch = new EnterSplitHatch(ViewNames.Concord, ViewNames.Reader, hostView: ViewNames.Concord, EnterSplitConcordHostsReader);

        // Fix round 1 (Adjudication F): Reader/Sources no longer take
        // IsHost/OnRequestClose parameters at all -- both are host-CAPABLE
        // views that embed their own <CompositionSplit HostName="..."/>
        // (see that component's own header), which computes host-vs-guest
        // internally from SplitMode alone (null = self-routed = host-or-
        // single; explicit true = registry-mounted = guest, always) and
        // dispatches CloseGuest itself when guest, with no externally
        // threaded callback. World is UNCHANGED -- it never hosts, so it
        // still needs an externally-supplied OnRequestClose the same way it
        // always has.
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
            }, new IEscapeHatch[] { worldHatch }),

            // Fix round 2 (Q-4, trivia -- re-review, PARTIAL, one-line
            // justified): this closure is not invoked by any shipped
            // pairing today (nothing hosts Sources as ITS guest) -- kept for
            // the SAME reason Sources.SplitMode itself is kept (see that
            // parameter's own comment): every registered view gets the
            // identical mount-closure shape, by construction (R1), not a
            // per-view special case for "currently unreachable."
            new(ViewNames.Sources, ViewCapabilities.None, ctx => builder =>
            {
                builder.OpenComponent<Sources>(0);
                builder.AddAttribute(1, nameof(Sources.SplitMode), (bool?)ctx.SplitMode);
                builder.CloseComponent();
            }, new IEscapeHatch[] { sourcesHatch }),

            // Batch CORP-1 (R2): the Kretzmann commentary browser -- PROJECTS
            // the shared Locus atom (BearsLocus), same capability declaration
            // Reader carries, which is exactly why the split-follow-by-
            // construction proof holds (both members read the SAME atom;
            // there is no link to wire).
            new(ViewNames.Kretzmann, ViewCapabilities.BearsLocus, ctx => builder =>
            {
                builder.OpenComponent<Kretzmann>(0);
                builder.AddAttribute(1, nameof(Kretzmann.SplitMode), (bool?)ctx.SplitMode);
                builder.CloseComponent();
            }, new IEscapeHatch[] { kretzmannHatch }),

            // Batch CORP-1 (R3): the Book of Concord structure browser --
            // declares NO capability (navigates its own part/article/paragraph
            // shape, not scripture locus); its own browsing position is
            // view-local component state, not a shared atom (see
            // Concord.razor's own header).
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
