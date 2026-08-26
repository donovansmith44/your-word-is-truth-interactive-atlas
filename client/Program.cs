using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Microsoft.JSInterop;
using BibleAtlas.Client;
using BibleAtlas.Client.Contracts;
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

// Batch G2 decisions 4/6: saved explorations + the selection tray, each a
// localStorage-backed singleton (LocalStore.cs's own header comment has the
// full reasoning) -- resolved lazily off the SAME IJSRuntime the framework
// already registers, cast once here to IJSInProcessRuntime (Blazor
// WebAssembly's own DI-registered instance always implements it -- there is
// no cross-process hop the way Blazor Server has).
builder.Services.AddSingleton(sp => new SavedExplorationsService((IJSInProcessRuntime)sp.GetRequiredService<IJSRuntime>()));
builder.Services.AddSingleton(sp => new SelectionTrayService((IJSInProcessRuntime)sp.GetRequiredService<IJSRuntime>()));

await builder.Build().RunAsync();
