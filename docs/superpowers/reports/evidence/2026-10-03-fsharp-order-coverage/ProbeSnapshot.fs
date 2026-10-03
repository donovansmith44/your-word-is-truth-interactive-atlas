module ProbeSnapshot

open System
open System.IO
open System.Reflection
open System.Text.Json
open FsCheck.Xunit

[<Property(MaxTest = 1)>]
let ``the expanded gate records the complete source inventory and finds the real public dependency`` () =
    let assembly = Assembly.GetExecutingAssembly()
    let gateType = assembly.GetType("BibleAtlas.FSharp.Tests.SourceOrderTests", true)
    let gate = gateType.GetMethod("violations", BindingFlags.NonPublic ||| BindingFlags.Static)
    let ownRoot = Path.GetFullPath(Path.Combine(__SOURCE_DIRECTORY__, "../../../../.."))
    let roots =
        match Environment.GetEnvironmentVariable "ATLAS_ORDER_PARENT_SOURCE" with
        | null | "" -> [ownRoot]
        | parent -> [ownRoot; Path.GetFullPath parent]
    let paths = roots |> List.collect (fun root ->
        ["client-fsharp"; "client-fsharp.Tests"; "client-fsharp.ContractGenerator"]
        |> List.collect (fun directory -> Directory.GetFiles(Path.Combine(root, directory), "*.fs", SearchOption.AllDirectories) |> Array.toList)
        |> List.filter (fun file -> not (file.Contains("/bin/") || file.Contains("/obj/"))))

    let results = paths |> List.map (fun file ->
        let source = File.ReadAllText file
        let findings = gate.Invoke(null, [|box file; box source|]) :?> string list
        {| File = file; Sha256 = System.Security.Cryptography.SHA256.HashData(System.Text.Encoding.UTF8.GetBytes source) |> System.Convert.ToHexString; Findings = findings |})
    let output = Environment.GetEnvironmentVariable "ATLAS_ORDER_REPORT"
    File.WriteAllText(output, JsonSerializer.Serialize(results, JsonSerializerOptions(WriteIndented = true)))
    let actual = results |> List.filter (fun result -> result.File.EndsWith("/Domain/Rooted.fs")) |> List.map (fun result -> result.File, result.Findings) |> List.sort
    let expected = roots |> List.map (fun root ->
        let file = Path.Combine(root, "client-fsharp/Core/Domain/Rooted.fs")
        file, [file + ": admit appears before map2"]) |> List.sort
    actual = expected
