module BibleAtlas.FSharp.Tests.StyleGeneration.SourceLaws

open System.IO
open FsCheck.Xunit
open Xunit

[<Property>]
let ``the source gate catches exception mutation partial access and mutable containers as syntax`` (suffix: uint16) =
    let code = $"module Example\nlet value{suffix} () =\n    let mutable count = 0\n    let entries = ResizeArray<int>()\n    try\n        count <- count + 1\n        List.head []\n    with _ -> failwith \"bad\"\n"
    let actual = SourceAdapter.inspect "example.fs" code
    let expected: SourceAdapter.Diagnostic list =
        [ { Fault = SourceAdapter.Mutation; Line = 3; Name = "mutable" }
          { Fault = SourceAdapter.MutableContainer; Line = 4; Name = "ResizeArray" }
          { Fault = SourceAdapter.ExceptionFlow; Line = 5; Name = "try" }
          { Fault = SourceAdapter.Mutation; Line = 6; Name = "loop or assignment" }
          { Fault = SourceAdapter.PartialAccess; Line = 7; Name = "head" }
          { Fault = SourceAdapter.ExceptionFlow; Line = 8; Name = "failwith" } ]
    Assert.Equal<SourceAdapter.Diagnostic list>(expected, actual)

[<Property>]
let ``the source gate ignores exception words in string literals and follows member and local intent`` (suffix: uint16) =
    let source = $"module Example\ntype Example =\n    static member Intent value = Example.Detail value\n    static member private Detail value =\n        let name{suffix} = \"raise mutable ResizeArray List.head\"\n        value + name{suffix}\n"
    Assert.Equal<SourceAdapter.Diagnostic list>([], SourceAdapter.inspect "example.fs" source)

[<Property>]
let ``the source gate refuses helpers placed above their first module or member caller`` (suffix: uint16) =
    let helper = $"Detail{suffix}"
    let source = $"module rec Example\nlet private {helper} value = value\nlet Intent value = {helper} value\n"
    let expected: SourceAdapter.Diagnostic list = [{ Fault = SourceAdapter.Newspaper; Line = 2; Name = helper + " before Intent" }]
    Assert.Equal<SourceAdapter.Diagnostic list>(expected, SourceAdapter.inspect "example.fs" source)
    let source = $"module Example\ntype Example =\n    static member private {helper} value = value\n    static member Intent value = Example.{helper} value\n"
    let expected: SourceAdapter.Diagnostic list = [{ Fault = SourceAdapter.Newspaper; Line = 3; Name = helper + " before Intent" }]
    Assert.Equal<SourceAdapter.Diagnostic list>(expected, SourceAdapter.inspect "example.fs" source)

[<Property(MaxTest = 1)>]
let ``the style exemplars keep domain code total and helpers below their callers`` () =
    let root = Path.GetFullPath(Path.Combine(__SOURCE_DIRECTORY__, "../.."))
    let files =
        ["client-fsharp.ContractGenerator/ContractModel.fs"; "client-fsharp.ContractGenerator/ContractReader.fs"; "client-fsharp.ContractGenerator/ContractEmitter.fs"; "client-fsharp.ContractGenerator/Generator.fs"; "client-fsharp.ContractGenerator/Results.fs"
         "client-fsharp/Core/WireFailure.fs"; "client-fsharp/Core/WireDecoder.fs"; "client-fsharp/Core/ReadFailure.fs"; "client-fsharp/Core/NonEmpty.fs"; "client-fsharp/Core/SourcesPresentation.fs"; "client-fsharp/Core/Sources.fs"
         "client-fsharp/Styles/SourcesRuntime.fs"; "client-fsharp/Styles/SourceCardView.fs"; "client-fsharp/Styles/SourceSectionsView.fs"; "client-fsharp/Styles/SourcesView.fs"]
    let actual = files |> List.collect (fun file -> SourceAdapter.inspect file (File.ReadAllText(Path.Combine(root, file))) |> List.map (fun diagnostic -> file, diagnostic))
    Assert.Equal<(string * SourceAdapter.Diagnostic) list>([], actual)
