module BibleAtlas.FSharp.Tests.Domain.HttpUrlLaws

open FsCheck.Xunit
open BibleAtlas.FSharp.Admission

[<Property>]
let ``HTTP and HTTPS admission preserves a complete generated absolute URL`` (secure: bool) (identity: uint16) =
    let scheme = if secure then "https" else "http"
    let candidate = $"{scheme}://example.org/{identity}?page={identity}#source"
    (HttpUrls.admit candidate |> Result.map HttpUrls.text) = Ok candidate

[<Property>]
let ``non web schemes are refused before a visit link is admitted`` (identity: uint16) =
    let candidates = [$"javascript:alert({identity})"; $"mailto:user{identity}@example.org"; $"ftp://example.org/{identity}"; $"file:///tmp/{identity}"; $"data:text/plain,{identity}"]
    (candidates |> List.map HttpUrls.admit) = List.replicate candidates.Length (Error HttpUrlFailure.UnsupportedScheme)

[<Property>]
let ``malformed and relative URLs are refused rather than repaired`` (identity: uint16) =
    let candidates = [null; ""; "https://"; $"/source/{identity}"; $"../source/{identity}"]
    (candidates |> List.map HttpUrls.admit) = List.replicate candidates.Length (Error HttpUrlFailure.MalformedUrl)

