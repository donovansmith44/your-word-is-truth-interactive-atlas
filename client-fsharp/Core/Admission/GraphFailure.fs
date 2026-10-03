namespace BibleAtlas.FSharp.Admission

open BibleAtlas.FSharp.Contract

type ElementCounts = { Requested: uint64; Received: uint64 }
type PositionAnswer = { Requested: PositionRef; Received: PositionRef }
type CorpusAnswer = { Requested: Corpus; Received: Corpus }

[<RequireQualifiedAccess>]
type GraphFailure =
    | MissingElement of ElementId
    | ElementCountMismatch of ElementCounts
    | UnexpectedElement of PositionAnswer
    | EmptyContinuation of ElementPageCursor
    | ExcessContinuation of ElementPageCursor
    | TextMissing of NodeId
    | ContentsCorpusMismatch of CorpusAnswer
    | TextCorpusMismatch of CorpusAnswer
    | MissingOpening of Corpus
