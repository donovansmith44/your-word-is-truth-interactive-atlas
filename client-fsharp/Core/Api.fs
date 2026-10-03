namespace BibleAtlas.FSharp

module Api =
    let readContract http cancellation request = HttpAdapter.read http cancellation request

    let read http cancellation request =
        async {
            let! answer = HttpAdapter.read http cancellation request
            return answer |> Result.mapError (fun failure ->
                match failure with
                | ReadFailure.InvalidAnswer failure -> Contract(WireFailure.render failure)
                | ReadFailure.Unreachable _ | ReadFailure.Cancelled _ | ReadFailure.HttpRejected _ -> Transport(ReadFailure.describe failure))
        }
