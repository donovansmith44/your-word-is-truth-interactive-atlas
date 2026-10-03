namespace BibleAtlas.FSharp.Client

open System.Net.Http
open System.Text.Json
open System.Threading
open Elmish
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

module Runtime =
    let command (http: HttpClient) effect =
        match effect with
        | ReadContents(corpus, request) ->
            let corpusName = JsonSerializer.Deserialize<string>(Json.encode corpus)
            Cmd.OfAsync.perform (Api.read http CancellationToken.None) (Reads.contents corpusName) (fun answer -> ContentsLoaded(corpus, request, answer))
        | ReadText(request, read) -> Cmd.OfAsync.perform (Api.read http CancellationToken.None) read (fun answer -> TextLoaded(request, answer))
        | ReadSources request -> Cmd.OfAsync.perform (Api.read http CancellationToken.None) (Reads.sources()) (fun answer -> SourcesLoaded(request, answer))
