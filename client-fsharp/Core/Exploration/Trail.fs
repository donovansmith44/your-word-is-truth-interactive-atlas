namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract

type Step = { Kind: EdgeKind; Target: Explorable }
type Trail = private { Start: Explorable; Steps: Step list }

module Trail =
    let retainedSteps = 40
    let beginAt start = { Start = start; Steps = [] }
    let follow (step: Step) trail =
        let steps = trail.Steps @ [step]
        let forgotten, retained = List.splitAt (max 0 (steps.Length - retainedSteps)) steps
        { Start = forgotten |> List.tryLast |> Option.map _.Target |> Option.defaultValue trail.Start
          Steps = retained }
    let current trail = List.tryLast trail.Steps |> Option.map _.Target |> Option.defaultValue trail.Start
    let walked trail = trail.Start :: List.map _.Target trail.Steps

    let breadcrumb trail =
        let collapse (crumbs: Step list) (step: Step) =
            match crumbs with
            | previous :: earlier when step.Kind = EdgeKinds.dual previous.Kind ->
                let source = List.tryHead earlier |> Option.map _.Target |> Option.defaultValue trail.Start
                if Explorable.sameIdentity step.Target source then earlier else step :: crumbs
            | _ -> step :: crumbs
        trail.Steps |> List.fold collapse [] |> List.rev

    let onOneRoot trail = walked trail |> List.forall (fun element -> Explorable.root element = Explorable.root (current trail))

    let resume walked kinds =
        match walked with
        | start :: targets when targets.Length = List.length kinds ->
            let trail = { Start = start; Steps = List.map2 (fun kind target -> { Kind = kind; Target = target }) kinds targets }
            if onOneRoot trail then Ok (trail.Steps |> List.fold (fun retained step -> follow step retained) (beginAt start))
            else Error(ArtifactMoved(Explorable.root start, Explorable.root (current trail)))
        | _ -> Error(Contract $"the resolved journey has {List.length walked} elements for {List.length kinds} steps")
