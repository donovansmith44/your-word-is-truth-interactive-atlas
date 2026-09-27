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

builder.Services.AddSingleton<IExplorableClient>(_ =>
{
    var baseAddress = AtlasClient.ResolveBaseAddress(builder.Configuration, builder.HostEnvironment);
    return new GraphExplorableClient(new HttpClient { BaseAddress = baseAddress });
});

builder.Services.AddSingleton<ViewStateService>();

AppServices.AddStateAtoms(builder.Services);

builder.Services.AddSingleton<EffectRegistry>();
builder.Services.AddSingleton<IEffectRegistry>(sp => sp.GetRequiredService<EffectRegistry>());

builder.Services.AddSingleton<OwnershipRegistry>();

AppServices.AddSelectionAtom(builder.Services);

builder.Services.AddSingleton(sp => new SavedExplorationsService((IJSInProcessRuntime)sp.GetRequiredService<IJSRuntime>()));

builder.Services.AddSingleton(sp => ViewRegistrySetup.Build(
    sp.GetRequiredService<StateAtom<ViewArrangement>>(),
    sp.GetRequiredService<ViewStateService>(),
    sp.GetRequiredService<StateAtom<Locus>>(),
    sp.GetRequiredService<NavigationManager>()));

await builder.Build().RunAsync();
