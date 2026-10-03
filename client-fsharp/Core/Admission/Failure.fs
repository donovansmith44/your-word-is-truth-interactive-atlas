namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract
open BibleAtlas.FSharp.Admission

type Failure =
    | Read of ReadFailure
    | Contract of string
    | ArtifactMoved of resolved: ArtifactRoot * serving: ArtifactRoot

module Failures =
    let graph (failure: GraphFailure) : Failure =
        Read(ReadFailure.Terminal(TerminalFailure.InvalidGraph failure))
