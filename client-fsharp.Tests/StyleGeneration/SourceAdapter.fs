module rec BibleAtlas.FSharp.Tests.StyleGeneration.SourceAdapter

open System
open Microsoft.FSharp.Reflection
open FSharp.Compiler.CodeAnalysis
open FSharp.Compiler.Syntax
open FSharp.Compiler.Text

type Fault = ExceptionFlow | Mutation | MutableContainer | PartialAccess | PositionalCast | Newspaper

type Diagnostic = { Fault: Fault; Line: int; Name: string }

let inspect (filename: string) (source: string) : Diagnostic list =
    let parsed = checker.ParseFile(filename, SourceText.ofString source, { FSharpParsingOptions.Default with SourceFiles = [|filename|] }) |> Async.RunSynchronously
    let nodes = descendants (box parsed.ParseTree)
    let syntax = nodes |> List.collect syntaxFaults
    let groups = nodes |> List.choose (function
        | :? SynModuleOrNamespace as scope ->
            let (SynModuleOrNamespace(decls = declarations)) = scope
            Some(declarations |> List.collect (function SynModuleDecl.Let(bindings = bindings) -> bindings | _ -> []))
        | :? SynModuleDecl as scope ->
            match scope with
            | SynModuleDecl.NestedModule(decls = declarations) -> Some(declarations |> List.collect (function SynModuleDecl.Let(bindings = bindings) -> bindings | _ -> []))
            | _ -> None
        | :? SynTypeDefn as scope ->
            let (SynTypeDefn(typeRepr = representation; members = members)) = scope
            let members =
                match representation with
                | SynTypeDefnRepr.ObjectModel(members = inside) -> inside @ members
                | SynTypeDefnRepr.Simple _ | SynTypeDefnRepr.Exception _ -> members
            Some(members |> List.choose (function SynMemberDefn.Member(memberDefn = binding) -> Some binding | _ -> None))
        | _ -> None)
    let order = groups |> List.collect newspaper
    List.distinct (syntax @ order) |> List.sortBy (fun diagnostic -> diagnostic.Line, diagnostic.Fault, diagnostic.Name)

let private newspaper (bindings: SynBinding list) =
    let named = bindings |> List.choose (fun binding ->
        bindingName binding |> Option.map (fun name ->
            let (SynBinding(headPat = pattern; expr = body)) = binding
            let bound = descendants (box pattern) |> List.choose (function
                | :? SynPat as pattern ->
                    match pattern with
                    | SynPat.Named(ident = SynIdent(ident, _)) -> Some ident.idText
                    | _ -> None
                | _ -> None) |> Set.ofList
            name, binding, Set.difference (references (box body)) bound))
    named |> List.choose (fun (name, SynBinding(range = range), _) ->
        named |> List.tryFind (fun (caller, _, calls) -> caller <> name && Set.contains name calls)
        |> Option.bind (fun (caller, SynBinding(range = callerRange), _) ->
            if range.StartLine < callerRange.StartLine then Some { Fault = Newspaper; Line = range.StartLine; Name = name + " before " + caller }
            else None))

let private syntaxFaults (node: obj) =
    match node with
    | :? SynBinding as binding ->
        match binding with
        | SynBinding(isMutable = true; range = range) -> [{ Fault = Mutation; Line = range.StartLine; Name = "mutable" }]
        | SynBinding _ -> []
    | :? SynExpr as expression ->
        let found fault name = [{ Fault = fault; Line = expression.Range.StartLine; Name = name }]
        match expression with
        | SynExpr.TryWith _ | SynExpr.TryFinally _ -> found ExceptionFlow "try"
        | SynExpr.For _ | SynExpr.ForEach _ | SynExpr.While _ | SynExpr.Set _ | SynExpr.LongIdentSet _ -> found Mutation "loop or assignment"
        | SynExpr.Downcast _ | SynExpr.InferredDowncast _ -> found PositionalCast "downcast"
        | SynExpr.Ident name -> identifierFault expression.Range.StartLine [name.idText]
        | SynExpr.LongIdent(longDotId = SynLongIdent(id = names)) -> identifierFault expression.Range.StartLine (names |> List.map _.idText)
        | _ -> []
    | :? SynType as shape ->
        match shape with
        | SynType.LongIdent(SynLongIdent(id = names)) -> identifierFault shape.Range.StartLine (names |> List.map _.idText)
        | _ -> []
    | _ -> []

let private identifierFault line names =
    match List.tryLast names with
    | Some("raise" | "failwith" | "invalidOp" as name) -> [{ Fault = ExceptionFlow; Line = line; Name = name }]
    | Some("ResizeArray" | "Dictionary" as name) -> [{ Fault = MutableContainer; Line = line; Name = name }]
    | Some("head" | "last" | "Head" | "Last" | "Value" as name) when List.length names > 1 -> [{ Fault = PartialAccess; Line = line; Name = name }]
    | Some _ | None -> []

let private bindingName (SynBinding(headPat = pattern)) =
    match pattern with
    | SynPat.LongIdent(longDotId = SynLongIdent(id = names)) -> names |> List.tryLast |> Option.map _.idText
    | SynPat.Named(ident = SynIdent(ident, _)) -> Some ident.idText
    | _ -> None

let private references value =
    descendants value |> List.collect (function
        | :? SynExpr as expression ->
            match expression with
            | SynExpr.Ident name -> [name.idText]
            | SynExpr.LongIdent(longDotId = SynLongIdent(id = names)) -> names |> List.map _.idText
            | _ -> []
        | _ -> []) |> Set.ofList

let private descendants (value: obj) : obj list =
    match value with
    | null -> []
    | value -> value :: (children value |> List.collect descendants)

let private children (value: obj) =
    let shape = value.GetType()
    if FSharpType.IsUnion shape then FSharpValue.GetUnionFields(value, shape) |> snd |> Array.toList
    elif FSharpType.IsRecord shape then FSharpValue.GetRecordFields value |> Array.toList
    elif FSharpType.IsTuple shape then FSharpValue.GetTupleFields value |> Array.toList
    elif shape.IsArray then (value :?> Array) |> Seq.cast<obj> |> Seq.toList
    else []

let private checker: FSharpChecker = FSharpChecker.Create()
