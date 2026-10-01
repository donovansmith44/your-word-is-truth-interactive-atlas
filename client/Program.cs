using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Microsoft.JSInterop;
using BibleAtlas.Client;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.Geography;
using BibleAtlas.Client.State;
using BibleAtlas.Client.Views;

namespace BibleAtlas.Client;

// An explicit Main, not a top-level-statement entry point: Stryker's mutation rebuild of this
// project does not honour the executable output kind top-level statements require (CS8805) even
// when Program.cs carries no mutation of its own, so the whole gate cannot run against this
// project while it uses one.
public static class Program
{
    public static async Task Main(string[] args)
    {
        var builder = WebAssemblyHostBuilder.CreateDefault(args);
        builder.RootComponents.Add<App>("#app");
        builder.RootComponents.Add<HeadOutlet>("head::after");

        builder.Services.AddScoped(sp => new HttpClient { BaseAddress = new Uri(builder.HostEnvironment.BaseAddress) });

        builder.Services.AddSingleton(_ =>
        {
            var baseAddress = AtlasClient.ResolveBaseAddress(builder.Configuration, builder.HostEnvironment);
            return new AtlasClient(new HttpClient { BaseAddress = baseAddress });
        });

        builder.Services.AddSingleton<IMapSource>(sp => new AtlasMapSource(sp.GetRequiredService<AtlasClient>()));

        builder.Services.AddSingleton<IExplorableClient>(_ =>
        {
            var baseAddress = AtlasClient.ResolveBaseAddress(builder.Configuration, builder.HostEnvironment);
            return new GraphExplorableClient(new HttpClient { BaseAddress = baseAddress });
        });

        builder.Services.AddSingleton<IExplorer>(sp => new GraphExplorer(sp.GetRequiredService<IExplorableClient>()));

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
    }
}
