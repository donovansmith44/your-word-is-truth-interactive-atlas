namespace BibleAtlas.FSharp.Admission

open System.Net

open BibleAtlas.FSharp.Domain

type ColorToken = private ColorToken of string

type ClientStatus = private ClientStatus of int

type ServerStatus = private ServerStatus of int

[<RequireQualifiedAccess>]
type ColorFailure = UnlistedColor

[<RequireQualifiedAccess>]
type ClientStatusFailure = NotClientRefusal

[<RequireQualifiedAccess>]
type ServerStatusFailure = NotServerRefusal

module Colors =
    let admit (allowed: Set<string>) (color: string) : Result<ColorToken, ColorFailure> =
        DomainSkeleton.pending "Colors.admit"
    let text (ColorToken color) : string = color

module RefusalStatuses =
    let private informationalMinimum = int HttpStatusCode.Continue
    let private clientRefusalMinimum = int HttpStatusCode.BadRequest
    let private serverRefusalMinimum = int HttpStatusCode.InternalServerError
    let private afterServerRefusals = 600

    let client (status: int) : Result<ClientStatus, ClientStatusFailure> =
        if status >= clientRefusalMinimum && status < serverRefusalMinimum then Ok(ClientStatus status)
        else Error ClientStatusFailure.NotClientRefusal
    let server (status: int) : Result<ServerStatus, ServerStatusFailure> =
        if status >= serverRefusalMinimum && status < afterServerRefusals then Ok(ServerStatus status)
        else Error ServerStatusFailure.NotServerRefusal
    let invalid (status: int) : bool = status < informationalMinimum || status >= afterServerRefusals
    let clientValue (ClientStatus status) : int = status
    let serverValue (ServerStatus status) : int = status
