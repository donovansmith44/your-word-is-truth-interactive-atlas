#r "nuget: FSharp.Compiler.Service, 43.12.401"

open System
open System.IO
open System.Text.Json
open FSharp.Compiler.CodeAnalysis
open FSharp.Compiler.Diagnostics
open FSharp.Compiler.Symbols

let manifest = JsonDocument.Parse(File.ReadAllText fsi.CommandLineArgs[1])
let checker = FSharpChecker.Create(keepAssemblyContents = true)
let projects =
    manifest.RootElement.EnumerateArray()
    |> Seq.map (fun project ->
        let file = project.GetProperty("project").GetString()
        let directory = Path.GetDirectoryName file
        let arguments = project.GetProperty("arguments").EnumerateArray() |> Seq.map _.GetString() |> Seq.toArray
        let sources, flags = arguments |> Array.partition (fun argument -> argument.EndsWith(".fs") && not (argument.StartsWith("-")))
        let options = checker.GetProjectOptionsFromCommandLineArgs(file, flags)
        { options with SourceFiles = sources |> Array.map (fun source -> Path.GetFullPath(source, directory)) })
    |> Seq.toList
let options =
    projects |> List.fold (fun prior project ->
        let references =
            prior |> List.choose (fun dependency ->
                let name = Path.GetFileNameWithoutExtension dependency.ProjectFileName + ".dll"
                project.OtherOptions
                |> Array.tryFind (fun option -> option.StartsWith("-r:") && Path.GetFileName(option[3..]) = name)
                |> Option.map (fun reference -> FSharpReferencedProject.FSharpReference(reference[3..], dependency)))
        prior @ [{ project with ReferencedProjects = List.toArray references }]) []
let results = options |> List.map (fun project -> checker.ParseAndCheckProject project |> Async.RunSynchronously)
let errors = results |> List.collect (fun result -> result.Diagnostics |> Array.filter (fun diagnostic -> diagnostic.Severity = FSharpDiagnosticSeverity.Error) |> Array.map string |> Array.toList)
if not errors.IsEmpty then
    printfn "%s" (JsonSerializer.Serialize {| errors = errors |})
    exit 2
let sourceFiles = options |> List.collect (fun project -> Array.toList project.SourceFiles) |> Set.ofList
let authored (file: string) =
    Set.contains file sourceFiles && not (file.Contains("/obj/") || file.Contains("/bin/") || file.Contains(".Tests/"))
let uses = results |> List.collect (fun result -> result.GetAllUsesOfAllSymbols() |> Array.toList)
let key (symbol: FSharpSymbol) =
    symbol.DeclarationLocation |> Option.map (fun range -> range.FileName, range.StartLine, range.StartColumn, symbol.DisplayName)
let typeSymbols (shape: FSharpType) : FSharpSymbol list =
    if shape.HasTypeDefinition then [shape.TypeDefinition :> FSharpSymbol] else []
let rec expressionSymbols (expression: FSharpExpr) : FSharpSymbol list =
    let direct =
        match expression with
        | FSharpExprPatterns.Call(_, memberSymbol, ownerTypes, argumentTypes, _) -> (memberSymbol :> FSharpSymbol) :: ((ownerTypes @ argumentTypes) |> List.collect typeSymbols)
        | FSharpExprPatterns.Value value -> [value :> FSharpSymbol]
        | FSharpExprPatterns.NewObject(constructor, _, _) -> [constructor :> FSharpSymbol]
        | FSharpExprPatterns.NewUnionCase(_, unionCase, _) -> [unionCase :> FSharpSymbol]
        | FSharpExprPatterns.UnionCaseGet(_, _, unionCase, field) -> [unionCase :> FSharpSymbol; field :> FSharpSymbol]
        | FSharpExprPatterns.FSharpFieldGet(_, _, field) -> [field :> FSharpSymbol]
        | _ -> []
    typeSymbols expression.Type @ direct @ (expression.ImmediateSubExpressions |> Seq.toList |> List.collect expressionSymbols)
let rec bodies (declaration: FSharpImplementationFileDeclaration) =
    match declaration with
    | FSharpImplementationFileDeclaration.Entity(_, declarations) -> declarations |> List.collect bodies
    | FSharpImplementationFileDeclaration.MemberOrFunctionOrValue(symbol, _, body) -> if symbol.IsCompilerGenerated then [] else [(symbol, body)]
    | FSharpImplementationFileDeclaration.InitAction body -> []
let implementations = results |> List.collect (fun result -> result.AssemblyContents.ImplementationFiles |> Seq.toList |> List.collect (fun file -> if authored file.FileName then file.Declarations |> List.collect bodies else []))
let pendingImplementations =
    implementations |> List.filter (fun (_, body) ->
        expressionSymbols body |> List.exists (fun called -> called.FullName = "BibleAtlas.FSharp.Domain.DomainSkeleton.pending"))
let pending = pendingImplementations |> List.choose (fun (symbol, _) -> key symbol) |> Set.ofList
let pendingParameter file line =
    pendingImplementations |> List.exists (fun (symbol, body) ->
        let start = symbol.DeclarationLocation
        start.FileName = file && start.StartLine <= line && line <= body.Range.EndLine)
let rec withOwner (symbol: FSharpSymbol) : FSharpSymbol list =
    if symbol.DeclarationLocation |> Option.exists (fun location -> authored location.FileName) then
        let owner =
            match symbol with
            | :? FSharpMemberOrFunctionOrValue as value -> value.DeclaringEntity
            | :? FSharpEntity as entity -> entity.DeclaringEntity
            | :? FSharpField as field -> field.DeclaringEntity
            | :? FSharpUnionCase as unionCase -> Some unionCase.DeclaringEntity
            | _ -> None
        symbol :: (owner |> Option.map (fun entity -> withOwner entity) |> Option.defaultValue [])
    else []
let entryPoints = implementations |> List.map fst |> List.filter (fun symbol -> symbol.Attributes |> Seq.exists (fun attribute -> attribute.AttributeType.FullName = "Microsoft.FSharp.Core.EntryPointAttribute")) |> List.map (fun symbol -> symbol :> FSharpSymbol)
let referenced =
    entryPoints @ (uses |> List.filter (fun usage -> not usage.IsFromDefinition && authored usage.FileName) |> List.map _.Symbol)
    @ (implementations |> List.collect (fun (_, body) -> expressionSymbols body))
    |> List.collect withOwner |> List.choose key |> Set.ofList
let declarations =
    uses |> List.filter (fun usage -> usage.IsFromDefinition && authored usage.FileName)
    |> List.choose (fun usage -> key usage.Symbol |> Option.map (fun identity -> identity, usage.Symbol))
    |> List.distinctBy fst
let rec typeParameters (shape: FSharpType) =
    if shape.IsGenericParameter then Set.singleton shape.GenericParameter.Name
    elif shape.HasTypeDefinition || shape.IsFunctionType || shape.IsTupleType || shape.IsAnonRecordType then
        shape.GenericArguments |> Seq.map typeParameters |> Set.unionMany
    else Set.empty
let unusedGenerics =
    declarations |> List.collect (fun (_, symbol) ->
        match symbol with
        | :? FSharpMemberOrFunctionOrValue as value when not value.IsCompilerGenerated ->
            let used = typeParameters value.FullType
            value.GenericParameters |> Seq.toList |> List.choose (fun parameter ->
                let location = parameter.DeclarationLocation
                if not parameter.IsCompilerGenerated && not (Set.contains parameter.Name used) && not (pendingParameter location.FileName location.StartLine) then
                    Some {| file = location.FileName; line = location.StartLine; column = location.StartColumn; name = parameter.Name |}
                else None)
        | _ -> [])
let unused =
    (declarations |> List.choose (fun ((file, line, column, name) as identity, symbol) ->
        let checkedSymbol =
            match symbol with
            | :? FSharpEntity as entity -> not entity.IsNamespace
            | :? FSharpMemberOrFunctionOrValue as value ->
                not value.IsCompilerGenerated && not value.IsMemberThisValue && not value.IsConstructorThisValue && name <> "_" && not value.IsConstructor && not value.IsOverrideOrExplicitInterfaceImplementation &&
                not (value.Attributes |> Seq.exists (fun attribute -> attribute.AttributeType.FullName = "Microsoft.FSharp.Core.EntryPointAttribute"))
            | :? FSharpGenericParameter as parameter -> not parameter.IsCompilerGenerated
            | :? FSharpUnionCase -> true
            | :? FSharpField as field -> not field.IsCompilerGenerated && not field.IsUnionCaseField
            | _ -> false
        if checkedSymbol && not (Set.contains identity referenced) && not (Set.contains identity pending) && not (pendingParameter file line) then Some {| file = file; line = line; column = column; name = name |} else None)
    ) @ unusedGenerics
    |> List.distinct
    |> List.sortBy (fun item -> item.file, item.line, item.column)
printfn "%s" (JsonSerializer.Serialize({| errors = errors; pending_bodies = Set.count pending; unused = unused |}, JsonSerializerOptions(WriteIndented = true)))
exit (if unused.IsEmpty then 0 else 1)
