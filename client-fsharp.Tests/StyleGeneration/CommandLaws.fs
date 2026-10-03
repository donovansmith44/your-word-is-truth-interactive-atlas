module BibleAtlas.FSharp.Tests.StyleGeneration.CommandLaws

open System
open System.Threading.Tasks
open FsCheck.Xunit
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Client
open BibleAtlas.FSharp.Contract

[<Property>]
let ``a Sources command reads its generated descriptor once and returns the entire typed message`` (steps: byte) (cards: byte) (failed: bool) =
    let request = SourcesFixtures.ticket steps
    let document = (SourcesFixtures.document 0uy cards 3uy false).Document
    let answer = if failed then Error(ReadFailure.Unreachable "offline") else Ok document
    let requested = TaskCompletionSource<Request<SourcesDocument>>(TaskCreationOptions.RunContinuationsAsynchronously)
    let delivered = TaskCompletionSource<SourcesMessage>(TaskCreationOptions.RunContinuationsAsynchronously)
    let read descriptor = async { requested.SetResult descriptor; return answer }
    let command = SourcesRuntime.command read (SourcesEffect.Read request)
    Assert.Single(command) |> ignore
    command |> List.iter (fun execute -> execute delivered.SetResult)
    let timeout = TimeSpan.FromSeconds 5.0
    let actual = requested.Task.WaitAsync(timeout).GetAwaiter().GetResult(), delivered.Task.WaitAsync(timeout).GetAwaiter().GetResult()
    Assert.Equal((Reads.sources(), SourcesMessage.Loaded(request, answer)), actual)
