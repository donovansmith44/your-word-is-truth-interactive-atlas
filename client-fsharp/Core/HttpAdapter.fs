namespace BibleAtlas.FSharp

open System
open System.Net.Http
open System.Threading
open BibleAtlas.FSharp.Contract

module internal HttpAdapter =
    let read (http: HttpClient) (cancellation: CancellationToken) (request: Request<'a>) : Async<Result<'a, ReadFailure>> =
        task {
            try
                use! response = http.GetAsync(Request.uri request, cancellation)
                let! body = response.Content.ReadAsStringAsync cancellation
                if response.IsSuccessStatusCode then
                    return JsonAdapter.Decode<'a> body |> Result.mapError ReadFailure.InvalidAnswer
                else
                    return Error(ReadFailure.HttpRejected
                        { Status = response.StatusCode; Reason = Option.ofObj response.ReasonPhrase
                          Body = JsonAdapter.Decode<ErrorBody> body })
            with
            | :? HttpRequestException as error -> return Error(ReadFailure.Unreachable error.Message)
            | :? OperationCanceledException as error -> return Error(ReadFailure.Cancelled error.Message)
        } |> Async.AwaitTask
