using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Microsoft.JSInterop;
using BibleAtlas.Client;

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
builder.Services.AddSingleton<ViewStateService>();

// Batch G2 decisions 4/6: saved explorations + the selection tray, each a
// localStorage-backed singleton (LocalStore.cs's own header comment has the
// full reasoning) -- resolved lazily off the SAME IJSRuntime the framework
// already registers, cast once here to IJSInProcessRuntime (Blazor
// WebAssembly's own DI-registered instance always implements it -- there is
// no cross-process hop the way Blazor Server has).
builder.Services.AddSingleton(sp => new SavedExplorationsService((IJSInProcessRuntime)sp.GetRequiredService<IJSRuntime>()));
builder.Services.AddSingleton(sp => new SelectionTrayService((IJSInProcessRuntime)sp.GetRequiredService<IJSRuntime>()));

await builder.Build().RunAsync();
