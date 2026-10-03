namespace BibleAtlas.FSharp

open System
open System.Net.Http
open System.Threading
open BibleAtlas.FSharp.Contract
open BibleAtlas.FSharp.Admission

module rec Api =
    let read<'answer> (http: HttpClient) (cancellation: CancellationToken) (request: Request<'answer>) : Async<Result<'answer, Failure>> =
      task {
        try
            use! response = http.GetAsync(Request.uri request, cancellation)
            let! body = response.Content.ReadAsStringAsync(cancellation)
            if response.IsSuccessStatusCode then return Json.decode<'answer> body
            else return Error(BibleAtlas.FSharp.Failure.Read(refusal response body))
        with
        | :? OperationCanceledException when cancellation.IsCancellationRequested -> return Error(BibleAtlas.FSharp.Failure.Read ReadFailure.Cancelled)
        | :? HttpRequestException | :? OperationCanceledException -> return Error(BibleAtlas.FSharp.Failure.Read(ReadFailure.Transient TransientFailure.Unreachable))
      } |> Async.AwaitTask

    let private refusal (response: HttpResponseMessage) (body: string) : ReadFailure =
        let code = Json.decode<ErrorBody> body |> Result.toOption |> Option.map _.Error.Code
        match RefusalStatuses.client (int response.StatusCode), RefusalStatuses.server (int response.StatusCode) with
        | Ok status, _ -> ReadFailure.Terminal(TerminalFailure.ClientRefusal(status, code))
        | _, Ok status -> ReadFailure.Transient(TransientFailure.ServerRefusal(status, code))
        | Error _, Error _ when RefusalStatuses.invalid (int response.StatusCode) -> ReadFailure.Transient(TransientFailure.InvalidStatus(response.StatusCode, code))
        | Error _, Error _ -> ReadFailure.Terminal(TerminalFailure.UnexpectedStatus(response.StatusCode, code))
