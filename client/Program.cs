using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
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

// Batch H (split-view study): lightweight in-memory view-state service --
// see ViewStateService.cs's own header comment for why AddSingleton (not
// AddScoped) is the right, deliberate choice in Blazor WASM specifically.
builder.Services.AddSingleton<ViewStateService>();

await builder.Build().RunAsync();
