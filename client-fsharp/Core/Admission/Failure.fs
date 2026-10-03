namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract
open BibleAtlas.FSharp.Admission

type Failure =
    | Read of ReadFailure
    | ArtifactMoved of resolved: ArtifactRoot * serving: ArtifactRoot

module Failures =
    let wire (failure: WireFailure) : Failure =
        Read(ReadFailure.Terminal(TerminalFailure.InvalidAnswer failure))

    let graph (failure: GraphFailure) : Failure =
        Read(ReadFailure.Terminal(TerminalFailure.InvalidGraph failure))
