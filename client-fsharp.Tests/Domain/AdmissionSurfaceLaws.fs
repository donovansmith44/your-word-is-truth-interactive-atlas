module BibleAtlas.FSharp.Tests.Domain.AdmissionSurfaceLaws

open System.IO
open FSharp.Compiler.CodeAnalysis
open FSharp.Compiler.Diagnostics
open FSharp.Compiler.Text
open FsCheck.Xunit
open BibleAtlas.FSharp.Domain

[<Property(MaxTest = 10)>]
let ``every new private representation refuses direct construction while its public door compiles`` (suffix: uint16) =
    let checker = FSharpChecker.Create()
    let core = typeof<Positive>.Assembly.Location
    let collections = typeof<FSharpPlus.Data.NonEmptyList<int>>.Assembly.Location
    let publicExpressions =
        [ "Positive.admit 1"
          "NonEmpty.admit [1]"
          "ArrayIndices.admit 0"
          "LineNumbers.admit 1L"
          "ByteColumns.admit 1L"
          "Latitudes.admit 0.0"
          "Longitudes.admit 0.0"
          "HttpUrls.admit \"https://example.org/\"" ]
    let privateExpressions =
        [ "Positive 0"
          "NonEmpty (FSharpPlus.Data.NonEmptyList.singleton 1)"
          "ArrayIndex -1"
          "LineNumber 0L"
          "ByteColumn 0L"
          "Latitude nan"
          "Longitude infinity"
          "HttpUrl (System.Uri \"file:///tmp/source\")" ]
    let actual =
        [ publicExpressions; privateExpressions ]
        |> List.mapi (fun index expressions ->
            let declarations = expressions |> List.mapi (fun item expression -> $"let admitted{item} = {expression}") |> String.concat "\n"
            let source = SourceText.ofString $"#r @\"{core}\"\n#r @\"{collections}\"\nopen BibleAtlas.FSharp.Domain\nopen BibleAtlas.FSharp.Admission\n{declarations}\n"
            let file = Path.Combine(Path.GetTempPath(), $"admission-{suffix}-{index}.fsx")
            let options, scriptDiagnostics = checker.GetProjectOptionsFromScript(file, source, assumeDotNetFramework = false) |> Async.RunSynchronously
            let _, result = checker.ParseAndCheckFileInProject(file, 0, source, options) |> Async.RunSynchronously
            match result with
            | FSharpCheckFileAnswer.Succeeded checkedFile ->
                let errors = checkedFile.Diagnostics |> Array.filter (fun diagnostic -> diagnostic.Severity = FSharpDiagnosticSeverity.Error) |> Array.map _.ErrorNumber |> Array.toList
                Some (scriptDiagnostics |> List.map _.ErrorNumber, errors)
            | FSharpCheckFileAnswer.Aborted -> None)
    let inaccessibleRepresentation = 1093
    let expected = [ Some ([], []); Some ([], List.replicate privateExpressions.Length inaccessibleRepresentation) ]
    actual = expected
