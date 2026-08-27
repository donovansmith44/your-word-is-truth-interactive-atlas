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
        // Batch CORPREAD-1a (SPLIT-PERSIST-1, RULING 5 -- CompositionSplit.razor's
        // own header): every hatch below that dispatches EnterSplit for a
        // host with a LIVE CompositionSplit already mounted on its own
        // current page (Reader/Sources/Kretzmann/Concord -- every hatch
        // except EnterSplitWorldRequestsReader) no longer builds its own
        // Nav.NavigateTo -- CompositionSplit's own subscription resyncs the
        // URL automatically, as a side effect of the SAME dispatch, for
        // every host generically. This also RETIRES the "disclosed
        // limitation" the Sources/Kretzmann/Concord hatches used to carry
        // ("a hard refresh while this split is open does not restore it") --
        // every pairing now gets the identical refresh-survives-split
        // guarantee Reader always had.
        Task EnterSplitReaderHostsWorld()
        {
            arrangement.Dispatch(new EnterSplit(ViewNames.Reader, ViewNames.World, viewState.Map.Follow, viewState.Map.DividerFraction));
            return Task.CompletedTask;
        }

        Task EnterSplitWorldRequestsReader()
        {
            // World navigates AWAY to Reader's own route -- there is no live
            // CompositionSplit instance on THIS page to resync a URL
            // through (World never hosts), so this is the ONE hatch that
            // still builds its own Nav.NavigateTo, from the ONE declared
            // split-URL vocabulary (SplitUrlContract), not a hand-rolled
            // literal. Reader still ends up hosting (R7 -- byte-identical
            // user-visible behavior for the two pre-existing entry points)
            // -- its own CompositionSplit restores EnterSplit(reader, world,
            // ...) from this URL on arrival (RULING 5's restore-once-on-load
            // half).
            var follow = viewState.Map.Follow ? $"&{SplitUrlContract.FollowParam}={SplitUrlContract.FollowTrueValue}" : "";
            nav.NavigateTo($"/read/{locus.Value.Book}/{locus.Value.Chapter}?{SplitUrlContract.SplitParam}={ViewNames.World}{follow}");
            return Task.CompletedTask;
        }

        Task EnterSplitSourcesHostsReader()
        {
            // R4's own generality proof: Sources becomes host; Reader
            // becomes guest.
            arrangement.Dispatch(new EnterSplit(ViewNames.Sources, ViewNames.Reader, DefaultFollow: false, DefaultDividerFraction: null));
            return Task.CompletedTask;
        }

        // Batch CORP-1 (R2/R3): Kretzmann and Concord each declare their OWN
        // "read-beside" hatch, self-hosting -- the SAME shape Sources' own
        // EnterSplitSourcesHostsReader establishes immediately above (owner
        // becomes host, Reader becomes guest). Kretzmann PROJECTS the shared
        // Locus atom directly (R2) -- the already-shared atom is why the
        // guest Reader pane tracks it "by construction," not a link. Concord
        // (R3) is identical in shape, just with a different owner name; it
        // bears no locus/window capability, so nothing about this hatch
        // differs from Sources' own proof that any two views may pair this
        // way.
        // Batch CORPREAD-1b (ticket K, "following by default"): DefaultFollow
        // flips true here -- unlike R2's own original registration (Follow
        // was meaningless for this pairing until this batch gave it a real
        // reader, ToggleFollowHatch's own Invoke below), a freshly opened
        // kretzmann+reader split now starts FOLLOWING the shared Locus atom,
        // per the follow-release law's own default ("following = render
        // from the shared atom" is the STARTING state; release is the
        // escape hatch away from it, never the other way around).
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

        // Batch CORPREAD-1b, DELIVERABLE 0a (THE FOLLOW-RELEASE LAW, design
        // spec §5): ONE shared closure -- `ToggleFollow` (client/State/
        // ViewArrangement.cs) is a pure write against the ONE shared
        // ViewArrangement atom ("flip Follow"), the SAME meaning regardless
        // of which locus-bearing owner's own chip fired it (see
        // ToggleFollowHatch.cs's own header for the full reasoning).
        // Kretzmann declares BearsLocus (registrations below) and is
        // COMPELLED by the standing conformance tripwire
        // (ViewRegistryConformanceTests.cs's own
        // HatchConformance_EveryBearsLocusView_DeclaresAToggleFollowHatch)
        // to carry one of these; World declares BearsWindow, not BearsLocus,
        // and gets one too per deliverable 0a's own explicit instruction
        // ("the world map's follow chip becomes the declared instance of the
        // same hatch kind") -- not because the tripwire demands it.
        //
        // ADJUDICATION G (fix round, review): Reader no longer gets one.
        // Reader declares BearsLocus and the tripwire's own LITERAL
        // predicate used to compel a hatch here too -- but Reader is the
        // CANONICAL LOCUS WRITER (its own route IS the shared value's
        // projection; there is no external value for it to "stop
        // following"), and no UI anywhere in the app ever invoked it (every
        // EscapeHatches consumer selects by Kind==EnterSplit or by a
        // specific view's OWN name -- verified exhaustively by the review).
        // A declared-but-unreachable hatch is contract theater, not vocabulary
        // -- deleted, and Reader is now exempted BY NAME, with its own
        // reason, in ViewRegistryConformanceTests.cs's own
        // FollowReleaseExemptViews (see that file's own header for the full
        // reasoning; ToggleFollowHatch.cs's own header carries the same
        // note). If Reader ever gains a genuine guest-mode independent-
        // browse capability, its own release semantics and chip should be
        // designed and declared together, then -- not kept as an inert
        // placeholder now.
        Task ToggleFollowGlobal()
        {
            arrangement.Dispatch(new ToggleFollow(!arrangement.Value.Follow));
            return Task.CompletedTask;
        }

        var worldFollowHatch = new ToggleFollowHatch(ViewNames.World, ToggleFollowGlobal);
        var kretzmannFollowHatch = new ToggleFollowHatch(ViewNames.Kretzmann, ToggleFollowGlobal);

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
            }, new IEscapeHatch[] { worldHatch, worldFollowHatch }), // S-4 (IMPORTANT, one-line justified below, ToggleFollowHatch.cs's own header)

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
            }, new IEscapeHatch[] { kretzmannHatch, kretzmannFollowHatch }),

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
