namespace BibleAtlas.FSharp.Admission

[<RequireQualifiedAccess>]
type JsonKind = Object | Array | String | Number | Boolean | Null

[<RequireQualifiedAccess>]
type IdentityFailure = BlankIdentity | WrongShape | WrongKind | UnknownIdentity

[<RequireQualifiedAccess>]
type WireFailure =
    | NotJson of TextPosition
    | NullAnswer
    | MissingField of JsonPath
    | WrongKind of path: JsonPath * expected: JsonKind * actual: JsonKind
    | UnknownCase of path: JsonPath * received: string
    | UnknownTag of path: JsonPath * received: string
    | OutOfBounds of path: JsonPath
    | InvalidIdentity of JsonPath * IdentityFailure
