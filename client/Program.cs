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

await builder.Build().RunAsync();
