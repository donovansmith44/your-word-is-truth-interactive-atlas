module rec BibleAtlas.FSharp.Tests.Domain.JsonFailureLaws

open System
open System.Text
open System.Text.Json
open FsCheck.Xunit
open Xunit
open Microsoft.FSharp.Reflection
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Admission
open BibleAtlas.FSharp.Contract
open BibleAtlas.FSharp.Tests

[<Property>]
let ``malformed JSON retains its exact zero based line and byte position without prose`` (suffix: byte) =
    let lines = int suffix % lineVariations
    let body = String.replicate lines "\n" + "{"
    let expected = wire (WireFailure.NotJson { Path = None; LineNumber = Some (int64 lines); BytePositionInLine = Some finalOpeningBraceLength })
    Assert.Equal(expected, Json.decode<NodeRef> body)

[<Property>]
let ``JSON null cannot become a successful contract answer of any current answer shape`` (suffix: byte) =
    let body = String.replicate (int suffix % lineVariations) " " + "null"
    let expected = wire WireFailure.NullAnswer
    let actual = Json.decode<NodeRef> body |> Result.map box, Json.decode<TextRef> body |> Result.map box, Json.decode<ElementPage> body |> Result.map box, Json.decode<string> body |> Result.map box
    Assert.Equal((expected, expected, expected, expected), actual)

[<Property>]
let ``a missing required field retains the serializer location instead of interpreting its message`` (suffix: uint16) =
    let body = JsonSerializer.Serialize {| id = $"Person:{suffix}"; kind = "Person"; label = $"Label {suffix}" |}
    let expected = wire (WireFailure.UnreadableAnswer { Path = Some "$"; LineNumber = Some firstLine; BytePositionInLine = Some (int64 (Encoding.UTF8.GetByteCount body)) })
    Assert.Equal(expected, Json.decode<NodeRecord> body)

[<Property>]
let ``a wrong JSON primitive retains the whole serializer location`` (suffix: uint16) =
    let body = JsonSerializer.Serialize($"received-{suffix}")
    let expected = wire (WireFailure.UnreadableAnswer { Path = Some "$"; LineNumber = Some firstLine; BytePositionInLine = Some (int64 (Encoding.UTF8.GetByteCount body)) })
    Assert.Equal(expected, Json.decode<int> body)

[<Property>]
let ``primitive refusal positions retain the original whitespace envelope rather than a reparsed fragment`` (suffix: byte) =
    let lines = int suffix % lineVariations
    let value = JsonSerializer.Serialize($"received-{suffix}")
    let body = String.replicate lines "\n" + value
    let expected = wire (WireFailure.UnreadableAnswer { Path = Some "$"; LineNumber = Some (int64 lines); BytePositionInLine = Some (int64 (Encoding.UTF8.GetByteCount value)) })
    Assert.Equal(expected, Json.decode<int> body)

[<Property>]
let ``a nested converter failure retains its reported fragment location without inventing a field path`` (suffix: uint16) =
    let body = JsonSerializer.Serialize {| id = $"Person:{suffix}"; kind = "Person"; label = suffix |}
    let expected = wire (WireFailure.UnreadableAnswer { Path = Some "$"; LineNumber = Some firstLine; BytePositionInLine = Some (int64 (suffix.ToString(System.Globalization.CultureInfo.InvariantCulture).Length)) })
    let actual = Json.decode<NodeRef> body
    Assert.True((expected = actual), sprintf "Expected %A; actual %A" expected actual)

[<Property>]
let ``readable generated records retain their complete value through the shared JSON door`` (suffix: uint16) =
    let expected: NodeRef = { Id = WireFixtures.identity<NodeId> $"Person:{suffix}"; Kind = NodeKind.Person; Label = $"Label {suffix}" }
    Assert.Equal(Ok expected, Json.decode<NodeRef> (Json.encode expected))

[<Property(MaxTest = 1)>]
let ``wire failures expose only syntax null and unreadable answer observations and no invented semantic categories`` () =
    let actual = FSharpType.GetUnionCases(typeof<WireFailure>) |> Array.map (fun case -> case.Name, case.GetFields() |> Array.map _.PropertyType |> Array.toList) |> Array.toList
    let expected = ["NotJson", [typeof<JsonErrorLocation>]; "NullAnswer", []; "UnreadableAnswer", [typeof<JsonErrorLocation>]]
    Assert.Equal<(string * Type list) list>(expected, actual)

let private wire<'answer> (failure: WireFailure) : Result<'answer, Failure> = BibleAtlas.FSharp.Failure.Read(ReadFailure.Terminal(TerminalFailure.InvalidAnswer failure)) |> Error

let private lineVariations = 16
let private firstLine = 0L
let private finalOpeningBraceLength = 1L
