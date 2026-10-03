namespace BibleAtlas.FSharp

type Failure =
    | Transport of string
    | Contract of string
    | ArtifactMoved of resolved: string * serving: string
