namespace BibleAtlas.FSharp.Admission

open System

[<RequireQualifiedAccess>]
type HttpUrlFailure = MalformedUrl | UnsupportedScheme

type HttpUrl = private HttpUrl of Uri

module HttpUrls =
    let admit (candidate: string) : Result<HttpUrl, HttpUrlFailure> =
        match Uri.TryCreate(candidate, UriKind.RelativeOrAbsolute) with
        | true, uri when not uri.IsAbsoluteUri -> Error HttpUrlFailure.MalformedUrl
        | true, uri when uri.Scheme = Uri.UriSchemeHttp || uri.Scheme = Uri.UriSchemeHttps -> Ok (HttpUrl uri)
        | true, _ -> Error HttpUrlFailure.UnsupportedScheme
        | false, _ -> Error HttpUrlFailure.MalformedUrl
    let text (HttpUrl uri) : string = uri.OriginalString
