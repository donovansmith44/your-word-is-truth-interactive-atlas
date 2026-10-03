namespace BibleAtlas.FSharp.Client

open System
open System.Net.Http
open Microsoft.AspNetCore.Components.WebAssembly.Hosting
open Microsoft.Extensions.Configuration
open Microsoft.Extensions.DependencyInjection

module Program =
    [<EntryPoint>]
    let main args =
        let builder = WebAssemblyHostBuilder.CreateDefault(args)
        builder.RootComponents.Add<App>("#app")
        builder.Services.AddScoped<HttpClient>(fun _ ->
            new HttpClient(BaseAddress = Uri(builder.Configuration.GetValue<string>("ApiBase", builder.HostEnvironment.BaseAddress)))) |> ignore
        builder.Build().RunAsync() |> ignore
        0
