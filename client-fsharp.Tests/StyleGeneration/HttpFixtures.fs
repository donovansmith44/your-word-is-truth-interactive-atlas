module BibleAtlas.FSharp.Tests.StyleGeneration.HttpFixtures

open System.Net.Http
open System.Threading.Tasks

type ResponseHandler(response: HttpResponseMessage) =
    inherit HttpMessageHandler()
    override _.SendAsync(_, _) = Task.FromResult response

type FailureHandler(failure: exn) =
    inherit HttpMessageHandler()
    override _.SendAsync(_, _) = Task.FromException<HttpResponseMessage> failure
