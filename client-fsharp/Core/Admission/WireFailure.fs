namespace BibleAtlas.FSharp.Admission

type JsonErrorLocation =
    { Path: string option
      LineNumber: int64 option
      BytePositionInLine: int64 option }

[<RequireQualifiedAccess>]
type WireFailure =
    | NotJson of JsonErrorLocation
    | NullAnswer
    | UnreadableAnswer of JsonErrorLocation
