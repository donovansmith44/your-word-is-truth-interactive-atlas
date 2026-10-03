namespace BibleAtlas.FSharp

open System
open System.Net.Http
open System.Threading
open BibleAtlas.FSharp.Contract

module Api =
    let read (http: HttpClient) (cancellation: CancellationToken) (request: Request<'a>) =
      task {
        try
            use! response = http.GetAsync(Request.uri request, cancellation)
            let! body = response.Content.ReadAsStringAsync(cancellation)
            if response.IsSuccessStatusCode then return Json.decode<'a> body
            else
                let reason =
                    match Json.decode<ErrorBody> body with
                    | Ok refusal -> refusal.Error.Message
                    | Error _ -> response.ReasonPhrase
                return Error(Transport $"{int response.StatusCode}: {reason}")
        with
        | :? HttpRequestException as error -> return Error(Transport error.Message)
        | :? OperationCanceledException as error -> return Error(Transport error.Message)
      } |> Async.AwaitTask
