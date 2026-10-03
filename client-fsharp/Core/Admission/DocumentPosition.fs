namespace BibleAtlas.FSharp.Admission

[<RequireQualifiedAccess>]
type LineNumberFailure = NonPositiveLine

type LineNumber = private LineNumber of int64

[<RequireQualifiedAccess>]
type ByteColumnFailure = NonPositiveColumn

type ByteColumn = private ByteColumn of int64

type TextPosition = { Line: LineNumber; Column: ByteColumn }

module LineNumbers =
    let admit (line: int64) : Result<LineNumber, LineNumberFailure> =
        if line > 0L then Ok (LineNumber line)
        else Error LineNumberFailure.NonPositiveLine
    let value (LineNumber line) : int64 = line

module ByteColumns =
    let admit (column: int64) : Result<ByteColumn, ByteColumnFailure> =
        if column > 0L then Ok (ByteColumn column)
        else Error ByteColumnFailure.NonPositiveColumn
    let value (ByteColumn column) : int64 = column
