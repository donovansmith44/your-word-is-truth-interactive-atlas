module rec BibleAtlas.FSharp.Tests.Domain.DeclarationLaws

open System.IO
open FSharp.Compiler.CodeAnalysis
open FSharp.Compiler.Syntax
open FSharp.Compiler.Text
open FsCheck.Xunit
open Xunit

[<Property(MaxTest = 1)>]
let ``every client concept has one declaration home`` () =
    let root = Path.GetFullPath(Path.Combine(__SOURCE_DIRECTORY__, "../../client-fsharp"))
    let declarations =
        Directory.GetFiles(root, "*.fs", SearchOption.AllDirectories)
        |> Array.filter (fun path -> not (path.Contains("/obj/") || path.Contains("/bin/")))
        |> Array.toList
        |> List.collect (fun file -> names file (File.ReadAllText file) |> List.map (fun name -> name, file))
    Assert.Equal<string list>([], duplicates declarations)

[<Property>]
let ``duplicate declaration detection permits companions in one home and refuses names in two homes`` (suffix: uint16) =
    let name = $"Concept{suffix}"
    let first = names "first.fs" $"namespace First\ntype {name} = {{ Value: int }}\nmodule {name} =\n    let value x = x.Value\n"
    let second = names "second.fs" $"namespace Second\ntype {name} = One | Two\n"
    Assert.Equal<string list>([], duplicates (first |> List.map (fun name -> name, "first.fs")))
    Assert.Equal<string list>([$"{name}: first.fs, second.fs"], duplicates ((first |> List.map (fun name -> name, "first.fs")) @ (second |> List.map (fun name -> name, "second.fs"))))

let private duplicates (declarations: (string * string) list) : string list =
    declarations
    |> List.groupBy fst
    |> List.choose (fun (name, declarations) ->
        let homes = declarations |> List.map snd |> List.distinct |> List.sort
        if homes.Length > 1 then Some (name + ": " + String.concat ", " homes) else None)
    |> List.sort

let private names (file: string) (source: string) : string list =
    let parsed = checker.ParseFile(file, SourceText.ofString source, { FSharpParsingOptions.Default with SourceFiles = [|file|] }) |> Async.RunSynchronously
    Assert.Empty(parsed.Diagnostics)
    match parsed.ParseTree with
    | ParsedInput.ImplFile(ParsedImplFileInput(contents = modules)) ->
        modules |> List.collect (fun (SynModuleOrNamespace(longId = id; kind = kind; decls = declarations)) ->
            let outer = match kind with SynModuleOrNamespaceKind.NamedModule -> id |> List.tryLast |> Option.map _.idText |> Option.toList | _ -> []
            outer @ (declarations |> List.collect declarationNames))
    | ParsedInput.SigFile _ -> []

let private declarationNames (declaration: SynModuleDecl) : string list =
    match declaration with
    | SynModuleDecl.Types(typeDefns = definitions) ->
        definitions |> List.collect (fun (SynTypeDefn(typeInfo = SynComponentInfo(longId = id))) -> id |> List.tryLast |> Option.map _.idText |> Option.toList)
    | SynModuleDecl.NestedModule(moduleInfo = SynComponentInfo(longId = id)) -> id |> List.tryLast |> Option.map _.idText |> Option.toList
    | _ -> []

let private checker: FSharpChecker = FSharpChecker.Create()
