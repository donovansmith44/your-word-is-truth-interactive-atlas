namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract

type Link = { Kind: EdgeKind; Target: PositionRef }
type Resolved =
    private
    | NodeResolved of root: string * node: NodeRecord
    | EdgeResolved of root: string * edge: EdgeRecord

module Resolved =
    let ofElement root element =
        match element with
        | Element.Node node -> Ok(NodeResolved(root, node.Node))
        | Element.Edge edge -> Ok(EdgeResolved(root, edge.Edge))
        | Element.Missing missing -> Error(Contract $"the element read names nothing for {missing.Id}")

    let root element =
        match element with
        | NodeResolved(root, _) | EdgeResolved(root, _) -> root

    let element resolved =
        match resolved with
        | NodeResolved(_, node) -> Element.Node { Node = node }
        | EdgeResolved(_, edge) -> Element.Edge { Edge = edge }

    let position element =
        match element with
        | NodeResolved(_, node) -> PositionRef.Node { Node = { Id = node.Id; Kind = node.Kind; Label = node.Label } }
        | EdgeResolved(_, edge) -> PositionRef.Edge { Edge = { Id = edge.Id; Kind = edge.Kind; Label = edge.Label } }

    let internal sameIdentity left right =
        match position left, position right with
        | PositionRef.Node left, PositionRef.Node right -> left.Node.Kind = right.Node.Kind && left.Node.Id = right.Node.Id
        | PositionRef.Edge left, PositionRef.Edge right -> left.Edge.Kind = right.Edge.Kind && left.Edge.Id = right.Edge.Id
        | PositionRef.Node _, PositionRef.Edge _ | PositionRef.Edge _, PositionRef.Node _ -> false

type Step = { Kind: EdgeKind; Target: Resolved }
type Trail = private { Start: Resolved; Steps: Step list }

module Trail =
    let beginAt start = { Start = start; Steps = [] }
    let follow (step: Step) trail = { trail with Steps = trail.Steps @ [step] }
    let current trail = List.tryLast trail.Steps |> Option.map _.Target |> Option.defaultValue trail.Start
    let walked trail = trail.Start :: List.map _.Target trail.Steps

    let breadcrumb trail =
        let collapse (crumbs: Step list) (step: Step) =
            match crumbs with
            | previous :: earlier when step.Kind = EdgeKinds.dual previous.Kind ->
                let source = List.tryHead earlier |> Option.map _.Target |> Option.defaultValue trail.Start
                if Resolved.sameIdentity step.Target source then earlier else step :: crumbs
            | _ -> step :: crumbs
        trail.Steps |> List.fold collapse [] |> List.rev

    let onOneRoot trail = walked trail |> List.forall (fun element -> Resolved.root element = Resolved.root (current trail))

    let resume walked kinds =
        match walked with
        | start :: targets when targets.Length = List.length kinds ->
            let trail = { Start = start; Steps = List.map2 (fun kind target -> { Kind = kind; Target = target }) kinds targets }
            if onOneRoot trail then Ok trail
            else Error(ArtifactMoved(Resolved.root start, Resolved.root (current trail)))
        | _ -> Error(Contract $"the resolved journey has {List.length walked} elements for {List.length kinds} steps")

type Explorer = { Resolve: PositionRef list -> Async<Result<Resolved list, Failure>> }
type Explore<'a> = private Explore of (Explorer -> Trail -> Async<Result<'a * Trail, Failure>>)

module Explore =
    let result value = Explore(fun _ trail -> async { return Ok(value, trail) })

    let bind next (Explore walk) =
        Explore(fun explorer trail -> async {
            let! walked = walk explorer trail
            match walked with
            | Error failure -> return Error failure
            | Ok(value, trail) ->
                let (Explore continueWith) = next value
                return! continueWith explorer trail
        })

    let map project action = bind (project >> result) action
    let run explorer trail (Explore walk) = walk explorer trail
    let here = Explore(fun _ trail -> async { return Ok(Trail.current trail, trail) })

    let renew =
        Explore(fun explorer trail -> async {
            let! resolved = explorer.Resolve(Trail.walked trail |> List.map Resolved.position)
            return resolved |> Result.bind (fun resolved -> Trail.resume resolved (trail.Steps |> List.map _.Kind)) |> Result.map (fun trail -> Trail.current trail, trail)
        })

    let follow (link: Link) =
        Explore(fun explorer trail -> async {
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
        Explore(fun _ trail -> async {
            match Trail.breadcrumb trail |> List.rev with
            | last :: earlier ->
                let source = List.tryHead earlier |> Option.map _.Target |> Option.defaultValue trail.Start
                let back = Trail.follow { Kind = EdgeKinds.dual last.Kind; Target = source } trail
                return Ok(source, back)
            | [] -> return Ok(Trail.current trail, trail)
        })

    let beginAt explorer start = async {
        let! resolved = explorer.Resolve [start]
        return resolved |> Result.bind (fun resolved -> Trail.resume resolved [])
    }

    let resume explorer start (links: Link list) = async {
        let! resolved = explorer.Resolve(start :: List.map (fun (link: Link) -> link.Target) links)
        return resolved |> Result.bind (fun resolved -> Trail.resume resolved (List.map (fun (link: Link) -> link.Kind) links))
    }

type ExploreBuilder() =
    member _.Return value = Explore.result value
    member _.ReturnFrom action = action
    member _.Bind(action, next) = Explore.bind next action
