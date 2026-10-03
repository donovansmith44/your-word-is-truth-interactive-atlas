namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract
open FSharpPlus.Data

type Link = { Kind: EdgeKind; Target: PositionRef }

type Explorer = { Resolve: PositionRef list -> Async<Result<Explorable list, Failure>> }
type Explore<'a> = private Explore of ReaderT<Explorer, StateT<Trail, ResultT<Async<Result<'a * Trail, Failure>>>>>

module Explore =
    let result (value: 'a) : Explore<'a> = Explore(ReaderT(fun _ -> StateT(fun trail -> ResultT(async { return Ok(value, trail) }))))

    let bind (next: 'a -> Explore<'b>) (Explore walk: Explore<'a>) : Explore<'b> =
        Explore(ReaderT.bind (fun value -> let (Explore next) = next value in next) walk)

    let map project action = bind (project >> result) action
    let run explorer trail (Explore walk) = ReaderT.run walk explorer |> fun state -> StateT.run state trail |> ResultT.run

    let internal walk action = Explore(ReaderT(fun explorer -> StateT(fun trail -> ResultT(action explorer trail))))
    let here = walk (fun _ trail -> async { return Ok(Trail.current trail, trail) })

    let links (kind: EdgeKind) (cursor: EdgePageCursor option) : Explore<Frontier<EdgePageCursor>> =
        BibleAtlas.FSharp.Domain.DomainSkeleton.pending "Explore.links"

    let renew =
        walk (fun explorer trail -> async {
            let! resolved = explorer.Resolve(Trail.walked trail |> List.map Explorable.position)
            return resolved |> Result.bind (fun resolved -> Trail.resume resolved (trail.Steps |> List.map _.Kind)) |> Result.map (fun trail -> Trail.current trail, trail)
        })

    let follow (link: Link) =
        walk (fun explorer trail -> async {
            let! resolved = explorer.Resolve [link.Target]
            match resolved with
            | Error failure -> return Error failure
            | Ok [target] ->
                let followed = Trail.follow { Kind = link.Kind; Target = target } trail
                if Trail.onOneRoot followed then return Ok(target, followed)
                else return! run explorer followed renew
            | Ok _ -> return Error(Contract "a single target did not resolve to one element")
        })

    let back =
        walk (fun _ trail -> async {
            match Trail.breadcrumb trail |> List.rev with
            | last :: earlier ->
                let source = List.tryHead earlier |> Option.map _.Target |> Option.defaultValue trail.Start
                let back = Trail.follow { Kind = EdgeKinds.dual last.Kind; Target = source } trail
                return Ok(source, back)
            | [] -> return Ok(Trail.current trail, trail)
        })

    let resume explorer start (links: Link list) = async {
        let! resolved = explorer.Resolve(start :: List.map (fun (link: Link) -> link.Target) links)
        return resolved |> Result.bind (fun resolved -> Trail.resume resolved (List.map (fun (link: Link) -> link.Kind) links))
    }

type ExploreBuilder() =
    member _.Return value = Explore.result value
    member _.ReturnFrom action = action
    member _.Bind(action, next) = Explore.bind next action
    member _.Delay body = Explore.walk (fun explorer trail -> async { return! Explore.run explorer trail (body ()) })

[<AutoOpen>]
module ExplorationExpression =
    let explore = ExploreBuilder()
