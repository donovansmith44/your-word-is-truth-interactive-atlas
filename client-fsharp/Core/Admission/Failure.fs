namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract

type Failure =
    | Transport of string
    | Contract of string
    | ArtifactMoved of resolved: ArtifactRoot * serving: ArtifactRoot
