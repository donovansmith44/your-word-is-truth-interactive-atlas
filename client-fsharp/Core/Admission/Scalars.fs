namespace BibleAtlas.FSharp.Admission

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
    let client (status: int) : Result<ClientStatus, ClientStatusFailure> =
        DomainSkeleton.pending "RefusalStatuses.client"
    let server (status: int) : Result<ServerStatus, ServerStatusFailure> =
        DomainSkeleton.pending "RefusalStatuses.server"
    let clientValue (ClientStatus status) : int = status
    let serverValue (ServerStatus status) : int = status
