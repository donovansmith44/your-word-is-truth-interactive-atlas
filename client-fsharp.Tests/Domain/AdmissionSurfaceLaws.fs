module BibleAtlas.FSharp.Tests.Domain.AdmissionSurfaceLaws

open System.IO
open FSharp.Compiler.CodeAnalysis
open FSharp.Compiler.Diagnostics
open FSharp.Compiler.Text
open FsCheck.Xunit
open BibleAtlas.FSharp.Domain

[<Property(MaxTest = 3)>]
let ``every private representation refuses direct construction while its own public door compiles`` (suffix: uint16) =
    let checker = FSharpChecker.Create()
    let core = typeof<Positive>.Assembly.Location
    let collections = typeof<FSharpPlus.Data.NonEmptyList<int>>.Assembly.Location
    let inaccessibleRepresentation = 1093
    let missingMember = 39
    let cases =
        [ {| Public = "Positive.admit 1"; Forbidden = "Positive 0"; Errors = [inaccessibleRepresentation] |}
          {| Public = "NonEmpty.admit [1]"; Forbidden = "NonEmpty (FSharpPlus.Data.NonEmptyList.singleton 1)"; Errors = [inaccessibleRepresentation] |}
          {| Public = "Rooted.admit 0us 0us [1]"; Forbidden = "Rooted (0us, [1])"; Errors = [inaccessibleRepresentation] |}
          {| Public = "Rooted.admit 0us 1us [1] |> Result.mapError id"; Forbidden = "RootMismatch (0us, 0us)"; Errors = [inaccessibleRepresentation] |}
          {| Public = "Positive.admit 1 |> Result.map (BibleAtlas.FSharp.Paging.PageCache<uint16, int, int>.Empty 0us)"
             Forbidden = "Positive.admit 1 |> Result.map (fun capacity -> ({ Version = 0us; PageBudget = capacity; MostRecent = [] }: BibleAtlas.FSharp.Paging.PageCache<uint16, int, int>))"
             Errors = List.replicate 6 inaccessibleRepresentation |}
          {| Public = "Positive.admit 1 |> Result.map (fun capacity -> (BibleAtlas.FSharp.Paging.PageCache<uint16, int, int>.Empty 0us capacity).Lookup 0)"
             Forbidden = "Positive.admit 1 |> Result.map (fun capacity -> BibleAtlas.FSharp.Paging.CacheLookup (None, BibleAtlas.FSharp.Paging.PageCache<uint16, int, int>.Empty 0us capacity))"
             Errors = [inaccessibleRepresentation] |}
          {| Public = "ArrayIndices.admit 0"; Forbidden = "ArrayIndex -1"; Errors = [inaccessibleRepresentation] |}
          {| Public = "LineNumbers.admit 1L"; Forbidden = "LineNumber 0L"; Errors = [inaccessibleRepresentation] |}
          {| Public = "ByteColumns.admit 1L"; Forbidden = "ByteColumn 0L"; Errors = [inaccessibleRepresentation] |}
          {| Public = "Latitudes.admit 0.0"; Forbidden = "Latitude nan"; Errors = [inaccessibleRepresentation] |}
          {| Public = "Longitudes.admit 0.0"; Forbidden = "Longitude infinity"; Errors = [inaccessibleRepresentation] |}
          {| Public = "RefusalStatuses.client 400 |> Result.map (fun status -> BibleAtlas.FSharp.Failure.Read(ReadFailure.Terminal(TerminalFailure.ClientRefusal(status, None))))"; Forbidden = "BibleAtlas.FSharp.Failure.Transport \"raw reason\""; Errors = [missingMember] |}
          {| Public = "ReadFailure.Terminal(TerminalFailure.InvalidAnswer WireFailure.NullAnswer) |> BibleAtlas.FSharp.Failure.Read"; Forbidden = "BibleAtlas.FSharp.Failure.Contract \"raw reason\""; Errors = [missingMember] |}
          {| Public = "RefusalStatuses.client 400"; Forbidden = "ClientStatus 400"; Errors = [inaccessibleRepresentation] |}
          {| Public = "RefusalStatuses.server 500"; Forbidden = "ServerStatus 500"; Errors = [inaccessibleRepresentation] |}
          {| Public = "HttpUrls.admit \"https://example.org/\""; Forbidden = "HttpUrl (System.Uri \"file:///tmp/source\")"; Errors = [inaccessibleRepresentation] |} ]
    let actual = cases |> List.mapi (fun caseNumber case ->
        let outcomes =
            [ case.Public; case.Forbidden ]
            |> List.mapi (fun variant expression ->
                let source = SourceText.ofString $"#r @\"{core}\"\n#r @\"{collections}\"\nopen BibleAtlas.FSharp.Domain\nopen BibleAtlas.FSharp.Admission\nlet admitted = {expression}\n"
                let file = Path.Combine(Path.GetTempPath(), $"admission-{suffix}-{caseNumber}-{variant}.fsx")
                let options, scriptDiagnostics = checker.GetProjectOptionsFromScript(file, source, assumeDotNetFramework = false) |> Async.RunSynchronously
                let _, result = checker.ParseAndCheckFileInProject(file, 0, source, options) |> Async.RunSynchronously
                match result with
                | FSharpCheckFileAnswer.Succeeded checkedFile ->
                    let errors = checkedFile.Diagnostics |> Array.filter (fun diagnostic -> diagnostic.Severity = FSharpDiagnosticSeverity.Error) |> Array.map _.ErrorNumber |> Array.toList
                    Some (scriptDiagnostics |> List.map _.ErrorNumber, errors)
                | FSharpCheckFileAnswer.Aborted -> None)
        {| Door = case.Public; Outcomes = outcomes |})
    let expected = cases |> List.map (fun case -> {| Door = case.Public; Outcomes = [Some ([], []); Some ([], case.Errors)] |})
    Xunit.Assert.True((actual = expected), sprintf "Expected %A\nActual %A" expected actual)
