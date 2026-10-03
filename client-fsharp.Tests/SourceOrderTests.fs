module rec BibleAtlas.FSharp.Tests.SourceOrderTests

open System
open System.IO
open Microsoft.FSharp.Reflection
open FSharp.Compiler.CodeAnalysis
open FSharp.Compiler.Syntax
open FSharp.Compiler.Text
open Xunit
open FsCheck.Xunit

[<Property(MaxTest = 1)>]
let ``private helpers and test fixtures follow their first caller in every FSharp source`` () =
    let root = Path.GetFullPath(Path.Combine(__SOURCE_DIRECTORY__, ".."))
    let files =
        ["client-fsharp"; "client-fsharp.Tests"; "client-fsharp.ContractGenerator"]
        |> List.collect (fun directory -> Directory.GetFiles(Path.Combine(root, directory), "*.fs", SearchOption.AllDirectories) |> Array.toList)
        |> List.filter (fun path -> not (path.Contains("/bin/") || path.Contains("/obj/")))
    let generated = ["client-fsharp/Core/obj/Contract/Wire.g.fs"; "client-fsharp/Core/obj/Contract/Vocabulary.g.fs"] |> List.map (fun file -> Path.Combine(root, file))
    let actual = files @ generated |> List.collect (fun file -> violations file (File.ReadAllText file)) |> List.sort
    Assert.True(actual.IsEmpty, String.concat "\n" actual)

[<Property>]
let ``the order gate finds a fixture used through a served record field`` (suffix: uint16) =
    let fixture = $"served{suffix}"
    let ascending = $"module rec Example\nlet {fixture} = {{ Label = \"served\" }}\nlet intent () = {fixture}.Label\n"
    let descending = $"module rec Example\nlet intent () = {fixture}.Label\nlet {fixture} = {{ Label = \"served\" }}\n"
    Assert.Equal<string list>([$"/Example.Tests/record.fs: {fixture} appears before intent"], violations "/Example.Tests/record.fs" ascending)
    Assert.Equal<string list>([], violations "/Example.Tests/record.fs" descending)

[<Property>]
let ``the order gate refuses a helper above its caller and accepts descending intent`` (suffix: uint16) =
    let helper = $"detail{suffix}"
    let ascending = "module rec Example\nlet private detail value = value + 1\nlet intent value = detail value\n"
    let descending = "module rec Example\nlet intent value = detail value\nlet private detail value = value + 1\n"
    let ascending = ascending.Replace("detail", helper)
    let descending = descending.Replace("detail", helper)
    Assert.Equal<string list>([$"example.fs: {helper} appears before intent"], violations "example.fs" ascending)
    Assert.Equal<string list>([], violations "example.fs" descending)

let private violations (file: string) (source: string) : string list =
    let parsed = checker.ParseFile(file, SourceText.ofString source, { FSharpParsingOptions.Default with SourceFiles = [|file|] }) |> Async.RunSynchronously
    Assert.Empty(parsed.Diagnostics)
    match parsed.ParseTree with
    | ParsedInput.ImplFile(ParsedImplFileInput(contents = modules)) ->
        modules |> List.collect (fun (SynModuleOrNamespace(decls = declarations)) -> inspect file declarations)
    | ParsedInput.SigFile _ -> failwith "the source-order gate expects an implementation"

let rec private inspect (file: string) (declarations: SynModuleDecl list) : string list =
    let bindings = declarations |> List.collect (function SynModuleDecl.Let(bindings = bindings) -> bindings | _ -> [])
    let named = bindings |> List.choose (fun binding -> bindingName binding |> Option.map (fun name -> name, binding))
    let local =
        named |> List.choose (fun (name, (SynBinding(accessibility = access; attributes = attributes; headPat = pattern) as binding)) ->
            let isTest = attributes |> List.collect _.Attributes |> List.exists (fun attribute -> attribute.TypeName.LongIdent |> List.exists (fun name -> name.idText = "Fact" || name.idText = "Theory" || name.idText = "Property"))
            let patternAccess =
                match pattern with
                | SynPat.LongIdent(accessibility = access) | SynPat.Named(accessibility = access) -> access
                | _ -> None
            let guarded = (match access, patternAccess with Some(SynAccess.Private _), _ | _, Some(SynAccess.Private _) -> true | _ -> false) || (file.Contains(".Tests/") && not isTest)
            if not guarded then None
            else
                named |> List.tryFind (fun (caller, SynBinding(expr = body)) -> caller <> name && identifiers (box body) |> Set.contains name)
                |> Option.bind (fun (caller, other) ->
                    let line (SynBinding(range = range)) = range.StartLine
                    if line binding < line other then Some $"{file}: {name} appears before {caller}"
                    else None))
    local @ (declarations |> List.collect (function SynModuleDecl.NestedModule(decls = nested) -> inspect file nested | _ -> []))

let private bindingName (SynBinding(headPat = pattern)) =
    match pattern with
    | SynPat.LongIdent(longDotId = SynLongIdent(id = names)) -> names |> List.tryLast |> Option.map _.idText
    | SynPat.Named(ident = SynIdent(ident, _)) -> Some ident.idText
    | _ -> None

let rec private identifiers (value: obj) =
    match value with
    | null -> Set.empty
    | :? SynExpr as expression ->
        match expression with
        | SynExpr.Ident name -> Set.singleton name.idText
        | SynExpr.LongIdent(longDotId = SynLongIdent(id = names)) -> names |> List.tryHead |> Option.map (fun name -> Set.singleton name.idText) |> Option.defaultValue Set.empty
        | _ -> children value
    | _ -> children value

let private children (value: obj) =
    let shape = value.GetType()
    if FSharpType.IsUnion shape then FSharpValue.GetUnionFields(value, shape) |> snd |> Array.map identifiers |> Set.unionMany
    elif FSharpType.IsRecord shape then FSharpValue.GetRecordFields value |> Array.map identifiers |> Set.unionMany
    elif FSharpType.IsTuple shape then FSharpValue.GetTupleFields value |> Array.map identifiers |> Set.unionMany
    elif shape.IsArray then (value :?> Array) |> Seq.cast<obj> |> Seq.map identifiers |> Set.unionMany
    else Set.empty

let private checker: FSharpChecker = FSharpChecker.Create()
