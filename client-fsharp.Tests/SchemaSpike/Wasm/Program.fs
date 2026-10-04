module rec BibleAtlas.FSharp.Tests.SchemaSpike.Wasm.Program

open System
open System.Diagnostics
open System.Net.Http
open System.Reflection
open Microsoft.AspNetCore.Components
open Microsoft.AspNetCore.Components.WebAssembly.Hosting
open Microsoft.Extensions.DependencyInjection
open Bolero
open Bolero.Html
open Elmish
open FsCheck
open FsCheck.Xunit
open Json.Schema
open BibleAtlas.FSharp.Tests.SchemaSpike
open BibleAtlas.FSharp.Tests.SchemaSpike.SurveyData

[<EntryPoint>]
let main arguments =
    let builder = WebAssemblyHostBuilder.CreateDefault arguments
    builder.RootComponents.Add<Host>("#app")
    builder.Services.AddScoped<HttpClient>(fun _ -> new HttpClient(BaseAddress = Uri builder.HostEnvironment.BaseAddress)) |> ignore
    builder.Build().RunAsync() |> ignore
    0

type private Host() =
    inherit ProgramComponent<State, Message>()

    [<Inject>]
    member val Http = Unchecked.defaultof<HttpClient> with get, set

    override this.Program =
        Elmish.Program.mkProgram (initialize this.Http) update view

let private initialize (http: HttpClient) _ =
    Loading, Cmd.OfAsync.either (run http) () Passed (fun error -> Failed(error.ToString()))

let private update message _ =
    match message with
    | Passed report -> Complete report, Cmd.none
    | Failed error -> Refused error, Cmd.none

let private view state _ =
    div {
        match state with
        | Loading -> p { attr.id "status"; "loading" }
        | Complete report ->
            p { attr.id "status"; "passed" }
            pre { attr.id "report"; text (BibleAtlas.FSharp.Json.encode report) }
        | Refused error ->
            p { attr.id "status"; "failed" }
            pre { attr.id "report"; text error }
    }

let private run (http: HttpClient) () = async {
    let! body = http.GetStringAsync("survey.json") |> Async.AwaitTask
    let inputs =
        match BibleAtlas.FSharp.Json.decode<Inputs> body with
        | Ok inputs -> inputs
        | Error failure -> failwithf "Survey input refused: %A" failure
    return check inputs
}

let private check (inputs: Inputs) : Report =
    let elapsed = Stopwatch.StartNew()
    SchemaSurvey.initialize inputs
    let properties =
        typeof<Inputs>.Assembly.GetType("BibleAtlas.FSharp.Tests.SchemaSpike.SchemaLaws", true)
            .GetMethods(BindingFlags.Public ||| BindingFlags.Static ||| BindingFlags.DeclaredOnly)
        |> Array.filter (fun method -> method.IsDefined(typeof<PropertyAttribute>, false))
        |> Array.sortBy (fun method -> method.MetadataToken)
    for property in properties do
        let attribute = property.GetCustomAttribute<PropertyAttribute>()
        let cases = if attribute.MaxTest > 0 then attribute.MaxTest else generatedCases
        Check.Method(configuration cases, property, None)
    { Properties = properties |> Array.map (fun property -> property.Name) |> Array.toList
      Fixtures = inputs.Examples.Length
      MaxGeneratedCases = generatedCases
      ElapsedMilliseconds = elapsed.ElapsedMilliseconds
      ValidatorAssembly = typeof<JsonSchema>.Assembly.GetName().FullName }

let private configuration cases : Config = Config.QuickThrowOnFailure.WithMaxTest cases
let private generatedCases = 100

type private Report =
    { Properties: string list
      Fixtures: int
      MaxGeneratedCases: int
      ElapsedMilliseconds: int64
      ValidatorAssembly: string }

type private State = Loading | Complete of Report | Refused of string
type private Message = Passed of Report | Failed of string
