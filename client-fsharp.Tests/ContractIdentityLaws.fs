module rec BibleAtlas.FSharp.Tests.ContractIdentityLaws

open System
open System.IO
open System.Globalization
open System.Reflection
open System.Text.Json
open Microsoft.FSharp.Reflection
open FSharp.Compiler.CodeAnalysis
open FSharp.Compiler.Diagnostics
open FSharp.Compiler.Text
open FsCheck.Xunit
open Xunit
open NSwag
open NJsonSchema
open BibleAtlas.Client.ContractGenerator
open BibleAtlas.FSharp.Contract

[<Property(MaxTest = 1)>]
let ``every published identity has its own private FSharp representation and exactly the declared widenings`` () =
    let document: OpenApiDocument = published ()
    let expectedShapes = IdentityTypes.Of(document) |> Seq.map (fun entry ->
        let primitive = if entry.Key.Type.HasFlag(JsonObjectType.Integer) then typeof<int>.FullName else typeof<string>.FullName
        entry.Value, primitive) |> Seq.toList
    let expectedShapes = (typeof<ReadingReference>.Name, typeof<string>.FullName) :: expectedShapes |> List.sort
    let shapes = identityTypes () |> List.map (fun identity ->
        let fields = (FSharpType.GetUnionCases(identity, bindingFlags ())).[0].GetFields()
        identity.Name, fields[0].PropertyType.FullName) |> List.sort
    let expectedWidenings = IdentityTypes.Widenings(document) |> Seq.map (fun (struct (from, into)) -> from, into) |> Seq.toList
    let expectedWidenings =
        expectedWidenings @
            [typeof<ChapterReference>.Name, typeof<ReadingReference>.Name
             typeof<VerseReference>.Name, typeof<ReadingReference>.Name
             typeof<ReadingReference>.Name, typeof<BibleReference>.Name
             typeof<ReadingReference>.Name, typeof<TextWindowReference>.Name] |> List.sort
    let actualWidenings = wideningMethods () |> List.map (fun method -> method.GetParameters().[0].ParameterType.Name, method.ReturnType.Name) |> List.sort
    Assert.True(((expectedShapes, expectedWidenings) = (shapes, actualWidenings)), sprintf "Expected %A\nActual %A" (expectedShapes, expectedWidenings) (shapes, actualWidenings))

[<Property>]
let ``every generated widening preserves the whole serialized identity for each accepted member`` (suffix: uint16) =
    let document: OpenApiDocument = published ()
    let expected, actual = wideningMethods () |> List.collect (fun method ->
        let source = method.GetParameters().[0].ParameterType
        let schema = document.Definitions[source.Name]
        let wires =
            if schema.IsEnumeration then schema.Enumeration |> Seq.map (fun wire -> JsonSerializer.Serialize wire) |> Seq.toList
            elif schema.Type.HasFlag(JsonObjectType.Integer) then [JsonSerializer.Serialize(int suffix)]
            else [JsonSerializer.Serialize($"identity-{suffix}-\u03bb")]
        wires |> List.map (fun wire ->
            let memberValue = JsonSerializer.Deserialize(wire, source)
            let widened = method.Invoke(null, [|memberValue|])
            let observed = JsonSerializer.Serialize(widened, method.ReturnType)
            (source.Name, method.ReturnType.Name, wire), (source.Name, method.ReturnType.Name, observed))) |> List.unzip
    Assert.True((expected = actual), sprintf "Expected %A\nActual %A" expected actual)

[<Property>]
let ``every identity round trips its entire primitive and refuses a mismatched primitive`` (suffix: uint16) =
    let document: OpenApiDocument = published ()
    let expected, actual = identityTypes () |> List.map (fun identity ->
        let integer = document.Definitions[identity.Name].Type.HasFlag(JsonObjectType.Integer)
        let wire = if integer then JsonSerializer.Serialize(int suffix) else JsonSerializer.Serialize($"wire-{suffix}-\u03bb")
        let mismatched = if integer then JsonSerializer.Serialize(string suffix) else JsonSerializer.Serialize(int suffix)
        let value = JsonSerializer.Deserialize(wire, identity)
        let whole = JsonSerializer.Serialize(value, identity)
        let refusal = Record.Exception(fun () -> JsonSerializer.Deserialize(mismatched, identity) |> ignore)
        let refusalName = if isNull refusal then None else Some (refusal.GetType().FullName)
        (identity.Name, wire, Some typeof<JsonException>.FullName), (identity.Name, whole, refusalName)) |> List.unzip
    Assert.Equal<(string * string * string option) list>(expected, actual)

[<Property>]
let ``every generated identity displays its exact primitive without exposing its constructor`` (suffix: uint16) =
    let document: OpenApiDocument = published ()
    let expected, actual = identityTypes () |> List.map (fun identity ->
        let integer = document.Definitions[identity.Name].Type.HasFlag(JsonObjectType.Integer)
        let text = if integer then (int suffix).ToString(CultureInfo.InvariantCulture) else $"wire-{suffix}-\u03bb"
        let wire = if integer then text else JsonSerializer.Serialize text
        let value = JsonSerializer.Deserialize(wire, identity)
        (identity.Name, text), (identity.Name, value.ToString())) |> List.unzip
    Assert.Equal<(string * string) list>(expected, actual)

[<Property(MaxTest = 1)>]
let ``each published cursor default serializes as the exact first page for that cursor`` () =
    let document: OpenApiDocument = published ()
    let expected = IdentityTypes.Of(document) |> Seq.choose (fun entry ->
        if isNull entry.Key.Default then None else Some (entry.Value, JsonSerializer.Serialize(Convert.ToInt32 entry.Key.Default))) |> Seq.sort |> Seq.toList
    let actual =
        [typeof<EdgePageCursor>.Name, JsonSerializer.Serialize EdgePageCursor.first
         typeof<ElementPageCursor>.Name, JsonSerializer.Serialize ElementPageCursor.first] |> List.sort
    Assert.Equal<(string * string) list>(expected, actual)

[<Property(MaxTest = 2)>]
let ``every generated identity refuses an external constructor and incompatible identities fail compilation`` (suffix: uint16) =
    let checker = FSharpChecker.Create()
    let assembly = typeof<ArtifactRoot>.Assembly.Location
    let cases =
        identityTypes () |> List.map (fun identity ->
            let primitive = if (FSharpType.GetUnionCases(identity, bindingFlags ())).[0].GetFields().[0].PropertyType = typeof<int> then "0" else "\"wire\""
            identity.Name, $"let forged = {identity.Name} {primitive}", [1093])
    let wrongConversions =
        ["ArtifactRoot", "NodeId"
         "ElementPageCursor", "EdgePageCursor"
         "VerseReference", "ChapterReference"]
        |> List.map (fun (from, into) -> $"{from} -> {into}", $"let wrong (value: {from}) : {into} = value", [1])
    let accepted =
        "let typed (node: NodeId) (edge: EdgeId) (contents: ContentsReference) =\n    ElementId.ofNodeId node, ElementId.ofEdgeId edge, TextWindowReference.ofContentsReference contents, EdgePageCursor.first, ElementPageCursor.first"
    let cases = ("declared doors", accepted, []) :: cases @ wrongConversions
    let expected = cases |> List.map (fun (name, _, errors) -> name, Some ([], errors))
    let actual = cases |> List.mapi (fun caseNumber (name, expression, _) ->
        let source = SourceText.ofString $"#r @\"{assembly}\"\nopen BibleAtlas.FSharp.Contract\n{expression}\n"
        let file = Path.Combine(Path.GetTempPath(), $"wire-identity-{suffix}-{caseNumber}.fsx")
        let options, scriptDiagnostics = checker.GetProjectOptionsFromScript(file, source, assumeDotNetFramework = false) |> Async.RunSynchronously
        let _, answer = checker.ParseAndCheckFileInProject(file, 0, source, options) |> Async.RunSynchronously
        let errors =
            match answer with
            | FSharpCheckFileAnswer.Aborted -> None
            | FSharpCheckFileAnswer.Succeeded checkedFile ->
                let errors = checkedFile.Diagnostics |> Array.filter (fun diagnostic -> diagnostic.Severity = FSharpDiagnosticSeverity.Error) |> Array.map _.ErrorNumber |> Array.toList
                Some (scriptDiagnostics |> List.map _.ErrorNumber, errors)
        name, errors)
    Assert.True((expected = actual), sprintf "Expected %A\nActual %A" expected actual)

let private published () : OpenApiDocument =
    OpenApiYamlDocument.FromFileAsync(Path.Combine(__SOURCE_DIRECTORY__, "../contracts/openapi.yaml")).GetAwaiter().GetResult()

let private wideningMethods () : MethodInfo list =
    typeof<ArtifactRoot>.Assembly.GetTypes()
    |> Array.collect (fun owner -> owner.GetMethods(BindingFlags.Public ||| BindingFlags.Static ||| BindingFlags.DeclaredOnly))
    |> Array.filter (fun method -> method.Name.StartsWith("of", StringComparison.Ordinal) && method.GetParameters().Length = 1 && List.contains method.ReturnType (identityTypes ()))
    |> Array.sortBy (fun method -> method.GetParameters().[0].ParameterType.Name, method.ReturnType.Name)
    |> Array.toList

let private identityTypes () : Type list =
    typeof<ArtifactRoot>.Assembly.GetTypes()
    |> Array.filter (fun candidate ->
        if candidate.Namespace <> typeof<ArtifactRoot>.Namespace || not (FSharpType.IsUnion(candidate, bindingFlags ())) then false
        else
            let cases = FSharpType.GetUnionCases(candidate, bindingFlags ())
            cases.Length = 1 && cases[0].GetFields().Length = 1 && candidate.GetCustomAttribute<System.Text.Json.Serialization.JsonConverterAttribute>() <> null)
    |> Array.sortBy _.Name
    |> Array.toList

let private bindingFlags () = BindingFlags.Public ||| BindingFlags.NonPublic
