namespace BibleAtlas.FSharp.Client

open Elmish
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

module SourcesRuntime =
    let command (read: Request<SourcesDocument> -> Async<Result<SourcesDocument, ReadFailure>>) (effect: SourcesEffect) : Cmd<SourcesMessage> =
        match effect with
        | SourcesEffect.Read request -> Cmd.OfAsync.perform read (Reads.sources()) (fun answer -> SourcesMessage.Loaded(request, answer))
