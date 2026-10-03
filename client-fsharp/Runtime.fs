namespace BibleAtlas.FSharp.Client

open System.Net.Http
open System.Text.Json
open System.Threading
open Elmish
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

module rec Runtime =
    let command (http: HttpClient) (effect: Effect) : Cmd<Message> =
        match effect with
        | ReadContents(corpus, request) ->
            let corpusName = JsonSerializer.Deserialize<string>(Json.encode corpus)
            Cmd.OfAsync.perform (Api.read http CancellationToken.None) (Reads.contents corpusName) (fun answer -> readingMessage corpus (ReadingMessage.ContentsLoaded(request, answer)))
        | ReadText(corpus, request, read) -> Cmd.OfAsync.perform (Api.read http CancellationToken.None) read (fun answer -> readingMessage corpus (ReadingMessage.TextLoaded(request, answer)))
        | ReadSources request ->
            SourcesRuntime.command (Api.readContract http CancellationToken.None) (SourcesEffect.Read request)
            |> Cmd.map (SurfaceMessage.Sources >> Page)
        | ReadOpening(request, position) ->
            Cmd.OfAsync.perform (Explore.beginAt (Graph.explorer http)) position (fun answer -> FocusLoaded(request, answer))
        | WalkFocus(request, trail, traversal) ->
            Cmd.OfAsync.perform (walk http trail) traversal (fun answer -> FocusLoaded(request, answer))

    let private walk (http: HttpClient) (trail: Trail) (traversal: Traversal) : Async<Result<Trail, Failure>> = async {
        let action =
            match traversal with
            | Traversal.Follow link -> Explore.follow link
            | Traversal.Back -> Explore.back
            | Traversal.Renew -> Explore.renew
        let! answer = Explore.run (Graph.explorer http) trail action
        return answer |> Result.map snd
    }

    let private readingMessage (corpus: Corpus) (message: ReadingMessage) : Message =
        match corpus with
        | Corpus.Bible -> Page(SurfaceMessage.Reader message)
        | Corpus.Concord -> Page(SurfaceMessage.Concord(ConcordMessage.Reading message))
