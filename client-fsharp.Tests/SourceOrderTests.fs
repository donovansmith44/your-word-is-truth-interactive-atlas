module rec BibleAtlas.FSharp.Tests.SourceOrderTests

open System
open System.IO
open FSharp.Compiler.CodeAnalysis
open FSharp.Compiler.Syntax
open FSharp.Compiler.Text
open Xunit
open FsCheck.Xunit

type private OrderBinding = { Name: string; Range: range; Scope: int * int; Owner: string option; IsHelper: bool }

[<Property(MaxTest = 1)>]
let ``helper functions members and shared test fixtures follow their first caller in every FSharp source`` () =
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

[<Property>]
let ``the order gate checks every exported helper regardless of its visibility`` (suffix: uint16) (internalVisibility: bool) =
    let helper = $"detail{suffix}"
    let visibility = if internalVisibility then "internal " else ""
    let ascending = $"module rec Example\nlet {visibility}{helper} value = value + 1\nlet intent value = {helper} value\n"
    let descending = $"module rec Example\nlet intent value = {helper} value\nlet {visibility}{helper} value = value + 1\n"
    let expected = ([$"example.fs: {helper} appears before intent"], [])
    Assert.Equal<string list * string list>(expected, (violations "example.fs" ascending, violations "example.fs" descending))

[<Property>]
let ``the order gate checks helpers reached through a qualified module reference`` (suffix: uint16) =
    let helper = $"detail{suffix}"
    let ascending = $"module rec Example\nlet private {helper} value = value + 1\nlet intent value = Example.{helper} value\n"
    let descending = $"module rec Example\nlet intent value = Example.{helper} value\nlet private {helper} value = value + 1\n"
    let expected = ([$"example.fs: {helper} appears before intent"], [])
    Assert.Equal<string list * string list>(expected, (violations "example.fs" ascending, violations "example.fs" descending))

[<Property>]
let ``the order gate finds module helpers called inside nested and destructured bindings`` (suffix: uint16) (destructured: bool) =
    let helper = $"detail{suffix}"
    let body = if destructured then $"    let answer, other = {helper} value, value\n    answer + other\n" else $"    let answer = {helper} value\n    answer\n"
    let detail = $"let private {helper} value = value + 1\n"
    let intent = "let intent value =\n" + body
    let expected = ([$"example.fs: {helper} appears before intent"], [])
    Assert.Equal<string list * string list>(expected, (violations "example.fs" ("module rec Example\n" + detail + intent), violations "example.fs" ("module rec Example\n" + intent + detail)))

[<Property>]
let ``the order gate checks type member helpers below their first member caller`` (suffix: uint16) =
    let helper = $"Detail{suffix}"
    let ascending = $"module Example\ntype Example private () =\n    static member private {helper} (value: int) = value + 1\n    static member Intent (value: int) = Example.{helper} value\n"
    let descending = $"module Example\ntype Example private () =\n    static member Intent (value: int) = Example.{helper} value\n    static member private {helper} (value: int) = value + 1\n"
    let expected = ([$"example.fs: {helper} appears before Intent"], [])
    Assert.Equal<string list * string list>(expected, (violations "example.fs" ascending, violations "example.fs" descending))

[<Property>]
let ``the order gate checks local helper chains within their binding scope`` (suffix: uint16) =
    let helper = $"detail{suffix}"
    let ascending = $"module Example\nlet intent value =\n    let {helper} argument = argument + 1\n    let answer argument = {helper} argument\n    answer value\n"
    let descending = $"module Example\nlet intent value =\n    let rec answer argument = {helper} argument\n    and {helper} argument = argument + 1\n    answer value\n"
    let expected = ([$"example.fs: {helper} appears before answer"], [])
    Assert.Equal<string list * string list>(expected, (violations "example.fs" ascending, violations "example.fs" descending))

[<Property>]
let ``the order gate distinguishes a shadowing parameter from the outer helper`` (suffix: uint16) =
    let helper = $"detail{suffix}"
    let source = $"module Example\nlet private {helper} value = value + 1\nlet intent {helper} = {helper} 2\n"
    Assert.Equal<string list>([], violations "example.fs" source)

[<Property>]
let ``the order gate distinguishes a shadowing lambda argument from the outer helper`` (suffix: uint16) (typed: bool) =
    let helper = $"detail{suffix}"
    let argument = if typed then $"({helper}: int -> int)" else helper
    let source = $"module Example\nlet private {helper} value = value + 1\nlet intent functions = functions |> List.map (fun {argument} -> {helper} 2)\n"
    Assert.Equal<string list>([], violations "example.fs" source)

[<Property>]
let ``the order gate preserves private helper checking inside nested modules`` (suffix: uint16) =
    let helper = $"detail{suffix}"
    let ascending = $"module rec Example\nmodule rec Details =\n    let private {helper} value = value + 1\n    let intent value = {helper} value\n"
    let descending = $"module rec Example\nmodule rec Details =\n    let intent value = {helper} value\n    let private {helper} value = value + 1\n"
    let expected = ([$"example.fs: {helper} appears before intent"], [])
    Assert.Equal<string list * string list>(expected, (violations "example.fs" ascending, violations "example.fs" descending))

[<Property>]
let ``the order gate permits local inputs before computations that consume them`` (suffix: uint16) =
    let input = $"input{suffix}"
    let source = $"module Example\nlet intent value =\n    let {input} = value + 1\n    let answer = {input} + 1\n    answer\n"
    Assert.Equal<string list>([], violations "example.fs" source)

[<Property>]
let ``the order gate permits module data before an operation that reads it`` (suffix: uint16) =
    let input = $"input{suffix}"
    let source = $"module Example\nlet {input} = {int suffix}\nlet intent () = {input}\n"
    Assert.Equal<string list>([], violations "example.fs" source)

let private violations (file: string) (source: string) : string list =
    let parsed = checker.ParseFile(file, SourceText.ofString source, { FSharpParsingOptions.Default with SourceFiles = [|file|] }) |> Async.RunSynchronously
    Assert.Empty(parsed.Diagnostics)
    let bindings, uses, parameterNames = inventory file parsed.ParseTree
    bindings
    |> List.filter _.IsHelper
    |> List.choose (fun callee ->
        uses
        |> List.choose (fun (path, names) ->
            path |> List.tryPick (function
                | SyntaxNode.SynBinding binding ->
                    bindings |> List.tryFind (fun caller -> caller.Range = binding.RangeOfBindingWithRhs && caller.Scope = callee.Scope)
                    |> Option.bind (fun caller ->
                        if caller.Range = callee.Range then None
                        elif references parameterNames callee.Name caller.Owner binding path names then Some caller else None)
                | _ -> None))
        |> List.sortBy (fun caller -> caller.Range.StartLine, caller.Range.StartColumn)
        |> List.tryHead
        |> Option.bind (fun caller ->
            if (callee.Range.StartLine, callee.Range.StartColumn) < (caller.Range.StartLine, caller.Range.StartColumn) then
                Some $"{file}: {callee.Name} appears before {caller.Name}"
            else None))
    |> List.distinct
    |> List.sort

let private inventory (file: string) (tree: ParsedInput) : OrderBinding list * (SyntaxVisitorPath * string list) list * Map<int * int, Set<string>> =
    ParsedInput.fold (fun (bindings, uses, parameterNames) path node ->
        match node with
        | SyntaxNode.SynBinding binding ->
            match bindingName binding with
            | Some name ->
                let scope, owner, local, memberScope = enclosingScope path
                let helper = memberScope || isFunction binding || (file.Contains(".Tests/") && not local)
                ({ Name = name; Range = binding.RangeOfBindingWithRhs; Scope = scope; Owner = owner; IsHelper = helper } :: bindings, uses, parameterNames)
            | None -> bindings, uses, parameterNames
        | SyntaxNode.SynExpr (SynExpr.Ident identifier) -> bindings, (path, [identifier.idText]) :: uses, parameterNames
        | SyntaxNode.SynExpr (SynExpr.LongIdent(longDotId = SynLongIdent(id = names))) ->
            bindings, (path, List.map (fun (identifier: Ident) -> identifier.idText) names) :: uses, parameterNames
        | SyntaxNode.SynPat (SynPat.Named(ident = SynIdent(identifier, _))) ->
            let enclosing = path |> List.tryPick (function SyntaxNode.SynBinding binding -> Some binding | _ -> None)
            let parameters =
                enclosing |> Option.bind (fun (SynBinding(headPat = pattern) as binding) ->
                    match pattern with
                    | SynPat.LongIdent _ when Range.rangeContainsRange pattern.Range node.Range ->
                        let identity = bindingKey binding
                        let names = parameterNames |> Map.tryFind identity |> Option.defaultValue Set.empty
                        Some (Map.add identity (Set.add identifier.idText names) parameterNames)
                    | _ -> None)
            bindings, uses, Option.defaultValue parameterNames parameters
        | _ -> bindings, uses, parameterNames) ([], [], Map.empty) tree

let private references parameterNames name owner binding path names =
    let shadowed binding = parameterNames |> Map.tryFind (bindingKey binding) |> Option.exists (Set.contains name)
    let lambdaShadowed = path |> List.exists (function
        | SyntaxNode.SynExpr (SynExpr.Lambda(args = SynSimplePats.SimplePats(pats = patterns))) -> patterns |> List.exists (lambdaParameter name)
        | _ -> false)
    match names with
    | [identifier] when identifier = name ->
        path
        |> List.choose (function SyntaxNode.SynBinding binding -> Some binding | _ -> None)
        |> List.exists shadowed
        |> fun shadowed -> not (shadowed || lambdaShadowed)
    | first :: _ when first = name -> not (shadowed binding || lambdaShadowed)
    | _ ->
        match List.rev names with
        | identifier :: qualifier :: _ -> identifier = name && (Some qualifier = owner || receiver binding = Some qualifier)
        | _ -> false

let rec private lambdaParameter name pattern =
    match pattern with
    | SynSimplePat.Id(ident = identifier) -> identifier.idText = name
    | SynSimplePat.Typed(pat = pattern) | SynSimplePat.Attrib(pat = pattern) -> lambdaParameter name pattern

let private enclosingScope path =
    path |> List.tryPick (fun node ->
        let identify owner local memberScope = Some ((node.Range.StartLine, node.Range.StartColumn), owner, local, memberScope)
        match node with
        | SyntaxNode.SynBinding binding -> identify (bindingName binding) true false
        | SyntaxNode.SynTypeDefn (SynTypeDefn(typeInfo = SynComponentInfo(longId = names))) -> identify (names |> List.tryLast |> Option.map _.idText) false true
        | SyntaxNode.SynModule (SynModuleDecl.NestedModule(moduleInfo = SynComponentInfo(longId = names))) -> identify (names |> List.tryLast |> Option.map _.idText) false false
        | SyntaxNode.SynModuleOrNamespace (SynModuleOrNamespace(longId = names)) -> identify (names |> List.tryLast |> Option.map _.idText) false false
        | _ -> None)
    |> Option.defaultValue ((0, 0), None, false, false)

let private bindingName (SynBinding(headPat = pattern)) =
    match pattern with
    | SynPat.LongIdent(longDotId = SynLongIdent(id = names)) -> names |> List.tryLast |> Option.map _.idText
    | SynPat.Named(ident = SynIdent(ident, _)) -> Some ident.idText
    | _ -> None

let private isFunction (SynBinding(headPat = pattern; expr = body)) =
    match pattern, body with
    | SynPat.LongIdent(argPats = SynArgPats.Pats (_ :: _)), _ -> true
    | _, (SynExpr.Lambda _ | SynExpr.MatchLambda _) -> true
    | _ -> false

let private bindingKey (binding: SynBinding) =
    let range = binding.RangeOfBindingWithRhs
    range.StartLine, range.StartColumn

let private receiver (SynBinding(headPat = pattern)) =
    match pattern with
    | SynPat.LongIdent(longDotId = SynLongIdent(id = receiver :: _ :: _)) -> Some receiver.idText
    | _ -> None

let private checker: FSharpChecker = FSharpChecker.Create()
