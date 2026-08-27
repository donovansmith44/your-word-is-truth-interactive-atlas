using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Microsoft.JSInterop;
using BibleAtlas.Client;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.State;
using BibleAtlas.Client.Views;

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
//
// Fix round 1 (ruling 6.i -- controller, binding): the four JS-independent
// atom registrations (Locus/TimeWindow/ViewArrangement/FocusStack) moved
// into AppServices.AddStateAtoms -- this is the SAME code, called from here,
// not a parallel copy -- so client.Tests can exercise REAL DI resolution +
// singleton lifetime for each migrated AtomNames entry (see
// ConformanceTests.cs's own rewritten atom-registration test; a Program.cs
// TEXT scan, the original approach, could never prove registration or
// lifetime, only that a constructor call appears somewhere in the file).
AppServices.AddStateAtoms(builder.Services);

// Batch ST-3 (R1): the effect-ownership registry (client/Contracts/State.cs's
// own IStateEffect<T>/IEffectRegistry, controller addition) -- ONE singleton
// registered under its concrete type (so call sites needing EffectClaim's
// own ReconcileTask, e.g. World.razor's follow-scene re-seat, don't need a
// downcast) AND under the interface (so Contracts/-facing code can depend on
// the abstraction), resolving to the SAME instance either way.
builder.Services.AddSingleton<EffectRegistry>();
builder.Services.AddSingleton<IEffectRegistry>(sp => sp.GetRequiredService<EffectRegistry>());

// Batch ST-3 (R4): the multi-instance ownership registry for the FocusStack
// atom -- see OwnershipRegistry.cs's own header for why this is a separate
// primitive from EffectRegistry above, not a reuse of it.
builder.Services.AddSingleton<OwnershipRegistry>();

// Batch ST-3 (R2), fix round 1 (Q-2 -- IMPORTANT, review, "no load-bearing
// constructors"): the Selection atom -- see AppServices.AddSelectionAtom's
// own doc comment for the seed/persistence-write mechanism, and Selection.cs's
// own header for why the persistence write lives directly in this factory
// now instead of the retired SelectionTrayService.
AppServices.AddSelectionAtom(builder.Services);

// Batch G2 decision 4: saved explorations, a localStorage-backed singleton
// (LocalStore.cs's own header comment has the full reasoning) -- resolved
// lazily off the SAME IJSRuntime the framework already registers, cast once
// here to IJSInProcessRuntime (Blazor WebAssembly's own DI-registered
// instance always implements it -- there is no cross-process hop the way
// Blazor Server has). Genuinely load-bearing (unlike the retired
// SelectionTrayService above): MainLayout.razor injects it directly for the
// hamburger panel's own Available/Items/Save/Rename/Delete surface.
builder.Services.AddSingleton(sp => new SavedExplorationsService((IJSInProcessRuntime)sp.GetRequiredService<IJSRuntime>()));

// Batch VC-1 (R1): the view registry -- one singleton, built from the SAME
// DI-singleton services its own hatches close over (see
// ViewRegistrySetup.Build's own header for why no live component instance
// is needed).
builder.Services.AddSingleton(sp => ViewRegistrySetup.Build(
    sp.GetRequiredService<StateAtom<ViewArrangement>>(),
    sp.GetRequiredService<ViewStateService>(),
    sp.GetRequiredService<StateAtom<Locus>>(),
    sp.GetRequiredService<NavigationManager>()));

await builder.Build().RunAsync();
