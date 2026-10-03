namespace BibleAtlas.Admission

open BibleAtlas.Domain

type ArrayIndex
type LineNumber
type ByteColumn
type ScalarOffset
type Utf16Offset
type ScalarSpan
type Utf16Span
type HttpUrl
type Latitude
type Longitude
type ColorToken
type WireField
type DocumentField
type Vocabulary
type Discriminator
type Bound
type Keyword
type ClientStatus
type ServerStatus
type RefusalCode

type TextPosition = { Line: LineNumber; Column: ByteColumn }
[<RequireQualifiedAccess>]
type Path<'step> = private Path of 'step list
[<RequireQualifiedAccess>]
type JsonStep = Property of WireField | Index of ArrayIndex
[<RequireQualifiedAccess>]
type DocumentStep = Property of DocumentField | Index of ArrayIndex
type JsonPath = Path<JsonStep>
type DocumentPath = Path<DocumentStep>

[<RequireQualifiedAccess>]
type JsonKind = Object | Array | String | Number | Boolean | Null
[<RequireQualifiedAccess>]
type IdentityFailure = BlankIdentity | WrongShape | WrongKind | UnknownIdentity

[<RequireQualifiedAccess>]
type WireFailure =
    | NotJson of TextPosition
    | NullAnswer
    | MissingField of JsonPath * WireField
    | WrongKind of JsonPath * expected: JsonKind * actual: JsonKind
    | UnknownCase of JsonPath * Vocabulary
    | UnknownTag of JsonPath * Discriminator
    | OutOfBounds of JsonPath * Bound
    | InvalidIdentity of JsonPath * IdentityFailure

[<RequireQualifiedAccess>]
type ContractError =
    | UnreadableDocument
    | InvalidDocument of NonEmpty<DocumentPath>
    | UnsupportedKeyword of DocumentPath * Keyword
    | DuplicateName of first: DocumentPath * second: DocumentPath
    | UnsupportedOperation of DocumentPath
    | UnsupportedSchema of DocumentPath

[<RequireQualifiedAccess>]
type TransientFailure = Unreachable | Cancelled | ServerRefusal of ServerStatus * RefusalCode
[<RequireQualifiedAccess>]
type TerminalFailure = ClientRefusal of ClientStatus * RefusalCode | InvalidAnswer of WireFailure
[<RequireQualifiedAccess>]
type ReadFailure = Transient of TransientFailure | Terminal of TerminalFailure

[<RequireQualifiedAccess>]
type BoundaryFailure =
    | NegativeIndex
    | NonPositivePosition
    | NegativeOffset
    | ReversedOffsetSpan
    | SpanOutsideText
    | InvalidUrl
    | InvalidLatitude
    | InvalidLongitude
    | UnlistedColor
    | InvalidRefusalStatus
    | Identity of IdentityFailure

module Paths =
    val root<'step>: Path<'step>
    val append: 'step -> Path<'step> -> Path<'step>
    val steps: Path<'step> -> 'step list

module ArrayIndices =
    val create: int -> Result<ArrayIndex, BoundaryFailure>
    val value: ArrayIndex -> int

module Positions =
    val line: int64 -> Result<LineNumber, BoundaryFailure>
    val column: int64 -> Result<ByteColumn, BoundaryFailure>

module TextSpans =
    val scalarOffset: int -> Result<ScalarOffset, BoundaryFailure>
    val utf16Offset: int -> Result<Utf16Offset, BoundaryFailure>
    val scalarSpan: Endpoints<ScalarOffset> -> Result<ScalarSpan, BoundaryFailure>
    val utf16Span: Endpoints<Utf16Offset> -> Result<Utf16Span, BoundaryFailure>
    val inText: TextBody -> ScalarSpan -> Result<Utf16Span, BoundaryFailure>

module HttpUrls =
    val create: string -> Result<HttpUrl, BoundaryFailure>

module Coordinates =
    val latitude: float -> Result<Latitude, BoundaryFailure>
    val longitude: float -> Result<Longitude, BoundaryFailure>

module RefusalStatuses =
    val client: int -> Result<ClientStatus, BoundaryFailure>
    val server: int -> Result<ServerStatus, BoundaryFailure>

module Colors =
    val admit: string -> Result<ColorToken, BoundaryFailure>
