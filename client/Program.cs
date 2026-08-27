using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Microsoft.JSInterop;
using BibleAtlas.Client;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.State;

var builder = WebAssemblyHostBuilder.CreateDefault(args);
builder.RootComponents.Add<App>("#app");
builder.RootComponents.Add<HeadOutlet>("head::after");

builder.Services.AddScoped(sp => new HttpClient { BaseAddress = new Uri(builder.HostEnvironment.BaseAddress) });

builder.Services.AddSingleton(_ =>
{
    var baseAddress = AtlasClient.ResolveBaseAddress(builder.Configuration, builder.HostEnvironment);
    return new AtlasClient(new HttpClient { BaseAddress = baseAddress });
});

// Batch M-D2 (P7 closure): the generic IExplorableClient contract, wired to
// its ONE concrete implementation here -- every consumer depends on the
// interface (constructor-injected), never GraphExplorableClient directly.
// A separate HttpClient instance (not AtlasClient's own) -- see
// GraphExplorableClient's own doc comment for why.
builder.Services.AddSingleton<IExplorableClient>(_ =>
{
    var baseAddress = AtlasClient.ResolveBaseAddress(builder.Configuration, builder.HostEnvironment);
    return new GraphExplorableClient(new HttpClient { BaseAddress = baseAddress });
});

// Batch H (split-view study): lightweight in-memory view-state service --
// see ViewStateService.cs's own header comment for why AddSingleton (not
// AddScoped) is the right, deliberate choice in Blazor WASM specifically.
// Batch ST-1 demotes this BENEATH the atoms below for the state it now
// shares with them (Locus persistence, the Map/Reader halves that overlap
// with Locus) -- ViewStateService itself is UNCHANGED code; only what reads/
// writes it moves. See ViewStateService.cs's own updated header comment.
builder.Services.AddSingleton<ViewStateService>();

// Batch ST-1: the state atom layer (spec §4d, client/Contracts/State.cs).
// Concrete StateAtom<T> instances, registered singleton -- same DI treatment
// as ViewStateService immediately above, same reasoning (Blazor WASM has one
// scope for the app's whole life). Components inject the CONCRETE StateAtom<T>
// (not just IStateAtom<T>) so they can construct their own Projection<T> for
// reads and Dispatch directly for writes; StateLinkRunner<A,B> (client/State/
// StateLinkRunner.cs) also needs the concrete type for its own no-echo
// bookkeeping (LastOrigin). AtomNames.Locus/AtomNames.TimeWindow (Contracts/
// State.cs) are the canonical names, threaded straight through as each
// atom's own Name.
builder.Services.AddSingleton(_ => new StateAtom<Locus>(AtomNames.Locus, Locus.Default));
builder.Services.AddSingleton(_ => new StateAtom<TimeWindow>(AtomNames.TimeWindow, TimeWindow.Default));

// Batch ST-2: the ViewArrangement atom (controller ruling R4) -- same
// singleton treatment as Locus/TimeWindow above.
builder.Services.AddSingleton(_ => new StateAtom<ViewArrangement>(AtomNames.ViewArrangement, ViewArrangement.Default));

// Batch ST-3 (R1): the effect-ownership registry (client/Contracts/State.cs's
// own IStateEffect<T>/IEffectRegistry, controller addition) -- ONE singleton
// registered under its concrete type (so call sites needing EffectClaim's
// own ReconcileTask, e.g. World.razor's follow-scene re-seat, don't need a
// downcast) AND under the interface (so Contracts/-facing code can depend on
// the abstraction), resolving to the SAME instance either way.
builder.Services.AddSingleton<EffectRegistry>();
builder.Services.AddSingleton<IEffectRegistry>(sp => sp.GetRequiredService<EffectRegistry>());

// Batch ST-3 (R4): the multi-instance ownership registry for the FocusStack
// atom below -- see OwnershipRegistry.cs's own header for why this is a
// separate primitive from EffectRegistry above, not a reuse of it.
builder.Services.AddSingleton<OwnershipRegistry>();

// Batch ST-3 (R3): the FocusStack atom -- one popover session's stack +
// trail, as one value (see FocusStack.cs's own header). Empty at every
// fresh app load (ExplorerPopover.razor's own OnInitializedAsync always
// claims ownership and seeds it before ever rendering from it -- there is
// no cross-reload persistence for this atom, matching the pre-atom
// component-local _stack/_trail fields' own "resets on a fresh open"
// behavior exactly).
builder.Services.AddSingleton(_ => new StateAtom<FocusStack>(AtomNames.FocusStack, FocusStack.Empty));

// Batch ST-3 (R2): the Selection atom -- seeded directly from whatever
// "selection-v1" doc localStorage already holds (the SAME LocalStore.Read/
// Probe idiom SelectionTrayService's own pre-atom constructor used to run
// itself), off the SAME already-registered IJSInProcessRuntime cast the
// saved-explorations/selection-tray registrations below already perform --
// no separate JS-interop bootstrapping step. SequenceEqualityComparer
// (client/State/SequenceEqualityComparer.cs) is REQUIRED here, not
// optional -- see that type's own header for why List<T>'s lack of
// structural equality would otherwise break laws 2/5 for a list-valued atom.
builder.Services.AddSingleton(sp =>
{
    var js = (IJSInProcessRuntime)sp.GetRequiredService<IJSRuntime>();
    var initial = LocalStore.Probe(js)
        ? (IReadOnlyList<ExplorationDescriptor>)LocalStore.Read(js, SelectionTrayService.StorageKey, new List<ExplorationDescriptor>())
        : Selection.Empty;
    return new StateAtom<IReadOnlyList<ExplorationDescriptor>>(AtomNames.Selection, initial, SequenceEqualityComparer<ExplorationDescriptor>.Instance);
});

// Batch G2 decisions 4/6: saved explorations + the selection tray's own
// PERSISTENCE half (Batch ST-3, R2 -- see SelectionTrayService.cs's own
// updated header), each a localStorage-backed singleton (LocalStore.cs's
// own header comment has the full reasoning) -- resolved lazily off the
// SAME IJSRuntime the framework already registers, cast once here to
// IJSInProcessRuntime (Blazor WebAssembly's own DI-registered instance
// always implements it -- there is no cross-process hop the way Blazor
// Server has).
builder.Services.AddSingleton(sp => new SavedExplorationsService((IJSInProcessRuntime)sp.GetRequiredService<IJSRuntime>()));
builder.Services.AddSingleton(sp => new SelectionTrayService(
    (IJSInProcessRuntime)sp.GetRequiredService<IJSRuntime>(),
    sp.GetRequiredService<StateAtom<IReadOnlyList<ExplorationDescriptor>>>()));

var host = builder.Build();

// Batch ST-3 (R2): a plain `AddSingleton` factory is LAZY -- it only ever
// runs the first time something actually resolves the service. Pre-ST-3,
// SelectionTrayService was always resolved anyway (every consumer injected
// it directly for Items/Toggle/Remove/Clear). Post-ST-3, NOTHING injects
// it any more (SelectionTray.razor/Reader.razor/World.razor all moved onto
// the atom directly, per R2) -- its own constructor (where the
// persistence-writing `Changed` subscriber gets wired) would otherwise
// never run at all, silently dropping "selection-v1" writes forever. Real,
// live-caught regression (Playwright's own pre-existing selection-tray.spec.ts
// reload test, SELECTION-1, went red): forcing resolution here, once, right
// after the host is built, is the fix -- the service has no OTHER job left
// (see its own updated header comment) but this one still needs a live
// instance to exist.
host.Services.GetRequiredService<SelectionTrayService>();

await host.RunAsync();
