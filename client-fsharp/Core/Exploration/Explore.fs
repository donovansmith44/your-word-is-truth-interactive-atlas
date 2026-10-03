namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract
open FSharpPlus.Data

type Link = { Kind: EdgeKind; Target: PositionRef }

type Explorer = { Resolve: PositionRef list -> Async<Result<Explorable list, Failure>> }
type Explore<'a> = private Explore of ReaderT<Explorer, StateT<Trail, ResultT<Async<Result<'a * Trail, Failure>>>>>

module rec Explore =
    let here () : Explore<Explorable> = walk<Explorable> (fun _ trail -> async { return Ok(Trail.current trail, trail) })

    let links (kind: EdgeKind) (cursor: EdgePageCursor option) : Explore<Frontier<EdgePageCursor>> =
        BibleAtlas.FSharp.Domain.DomainSkeleton.pending "Explore.links"

    let follow (link: Link) : Explore<Explorable> =
        walk<Explorable> (fun explorer trail -> async {
            let! resolved = explorer.Resolve [link.Target]
            match resolved with
            | Error failure -> return Error failure
            | Ok [target] ->
                let followed = Trail.follow { Kind = link.Kind; Target = target } trail
                if Trail.onOneRoot followed then return Ok(target, followed)
                else return! run<Explorable> explorer followed (renew ())
            | Ok _ -> return Error(Contract "a single target did not resolve to one element")
        })

    let back () : Explore<Explorable> =
        walk<Explorable> (fun _ trail -> async {
            match Trail.breadcrumb trail |> List.rev with
            | last :: earlier ->
                let source = List.tryHead earlier |> Option.map _.Target |> Option.defaultValue trail.Start
                let back = Trail.follow { Kind = EdgeKinds.dual last.Kind; Target = source } trail
                return Ok(source, back)
            | [] -> return Ok(Trail.current trail, trail)
        })

    let renew () : Explore<Explorable> =
        walk<Explorable> (fun explorer trail -> async {
            let! resolved = explorer.Resolve(Trail.walked trail |> List.map Explorable.position)
            return resolved |> Result.bind (fun resolved -> Trail.resume resolved (trail.Steps |> List.map _.Kind)) |> Result.map (fun trail -> Trail.current trail, trail)
        })

    let resume explorer start (links: Link list) = async {
        let! resolved = explorer.Resolve(start :: List.map (fun (link: Link) -> link.Target) links)
        return resolved |> Result.bind (fun resolved -> Trail.resume resolved (List.map (fun (link: Link) -> link.Kind) links))
    }

    let map<'a, 'b> (project: 'a -> 'b) (action: Explore<'a>) : Explore<'b> = bind<'a, 'b> (project >> result<'b>) action

    let bind<'a, 'b> (next: 'a -> Explore<'b>) (Explore walk: Explore<'a>) : Explore<'b> =
        Explore(ReaderT.bind (fun value -> let (Explore next) = next value in next) walk)

    let result<'a> (value: 'a) : Explore<'a> = Explore(ReaderT(fun _ -> StateT(fun trail -> ResultT(async { return Ok(value, trail) }))))

    let run<'a> (explorer: Explorer) (trail: Trail) (Explore walk: Explore<'a>) : Async<Result<'a * Trail, Failure>> = ReaderT.run walk explorer |> fun state -> StateT.run state trail |> ResultT.run

    let internal walk<'a> (action: Explorer -> Trail -> Async<Result<'a * Trail, Failure>>) : Explore<'a> = Explore(ReaderT(fun explorer -> StateT(fun trail -> ResultT(action explorer trail))))

type ExploreBuilder() =
    member _.Return value = Explore.result value
    member _.ReturnFrom action = action
    member _.Bind(action, next) = Explore.bind next action
    member _.Delay body = Explore.walk (fun explorer trail -> async { return! Explore.run explorer trail (body ()) })

[<AutoOpen>]
module ExplorationExpression =
    let explore = ExploreBuilder()
