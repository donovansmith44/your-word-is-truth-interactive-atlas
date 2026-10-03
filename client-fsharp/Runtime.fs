namespace BibleAtlas.FSharp.Client

open System.Net.Http
open System.Text.Json
open System.Threading
open Elmish
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

module Runtime =
    let rec command (http: HttpClient) effect =
        match effect with
        | ReadContents(corpus, request) ->
            let corpusName = JsonSerializer.Deserialize<string>(Json.encode corpus)
            Cmd.OfAsync.perform (Api.read http CancellationToken.None) (Reads.contents corpusName) (fun answer -> ContentsLoaded(corpus, request, answer))
        | ReadText(request, read) -> Cmd.OfAsync.perform (Api.read http CancellationToken.None) read (fun answer -> TextLoaded(request, answer))
        | ReadSources request -> Cmd.OfAsync.perform (Api.read http CancellationToken.None) (Reads.sources()) (fun answer -> SourcesLoaded(request, answer))
        | ReadOpening(request, position) ->
            Cmd.OfAsync.perform (Explore.beginAt (Graph.explorer http)) position (fun answer -> FocusLoaded(request, answer))
        | WalkFocus(request, trail, traversal) ->
            Cmd.OfAsync.perform (walk http trail) traversal (fun answer -> FocusLoaded(request, answer))

    and private walk http trail traversal = async {
        let action =
            match traversal with
            | Traversal.Follow link -> Explore.follow link
            | Traversal.Back -> Explore.back
            | Traversal.Renew -> Explore.renew
        let! answer = Explore.run (Graph.explorer http) trail action
        return answer |> Result.map snd
    }
