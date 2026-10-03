namespace BibleAtlas.FSharp.Exploring

open BibleAtlas.FSharp.Domain

type TrailCapacity = private TrailCapacity of Positive

type Transition<'root, 'position, 'value, 'edge> =
    private
        { From: Resolved<'root, 'position, 'value>
          Through: Rooted<'root, 'edge>
          To: Resolved<'root, 'position, 'value> }

type Trail<'root, 'position, 'value, 'edge> =
    private
        { Capacity: TrailCapacity
          Start: Resolved<'root, 'position, 'value>
          Steps: Transition<'root, 'position, 'value, 'edge> list }

type Renewal<'root, 'position, 'value, 'edge> =
    private { Previous: Trail<'root, 'position, 'value, 'edge>; ObservedRoot: 'root }

type RenewalEvidence<'root, 'position, 'value, 'edge> =
    private
        { Start: Resolved<'root, 'position, 'value>
          Steps: Transition<'root, 'position, 'value, 'edge> list }

[<RequireQualifiedAccess>]
type StepOutcome<'root, 'position, 'value, 'edge> =
    | Continued of Trail<'root, 'position, 'value, 'edge>
    | NeedsRenewal of Renewal<'root, 'position, 'value, 'edge>

[<RequireQualifiedAccess>]
type BackOutcome<'root, 'position, 'value, 'edge> =
    | Backed of Trail<'root, 'position, 'value, 'edge>
    | AtStart

[<RequireQualifiedAccess>]
type TransitionFailure<'root> = ChangedRoot of RootMismatch<'root> | UnrelatedPositions

[<RequireQualifiedAccess>]
type StepFailure = WrongStart

[<RequireQualifiedAccess>]
type RenewalFailure<'root> =
    | ChangedRoot of RootMismatch<'root>
    | MissingPosition
    | DifferentIdentity
    | BrokenChain

module Trails =
    let capacity (steps: Positive) : TrailCapacity = TrailCapacity steps
    let beginAt (capacity: TrailCapacity) (start: Resolved<'root, 'position, 'value>) : Trail<'root, 'position, 'value, 'edge> =
        { Capacity = capacity; Start = start; Steps = [] }
    let transition (connects: 'edge -> 'position -> 'position -> bool) (edge: Rooted<'root, 'edge>) (fromPosition: Resolved<'root, 'position, 'value>) (toPosition: Resolved<'root, 'position, 'value>) : Result<Transition<'root, 'position, 'value, 'edge>, TransitionFailure<'root>> =
        DomainSkeleton.pending "Trails.transition"
    let step (transition: Transition<'root, 'position, 'value, 'edge>) (trail: Trail<'root, 'position, 'value, 'edge>) : Result<StepOutcome<'root, 'position, 'value, 'edge>, StepFailure> =
        DomainSkeleton.pending "Trails.step"
    let back (trail: Trail<'root, 'position, 'value, 'edge>) : BackOutcome<'root, 'position, 'value, 'edge> =
        DomainSkeleton.pending "Trails.back"
    let renewal (observedRoot: 'root) (trail: Trail<'root, 'position, 'value, 'edge>) : Renewal<'root, 'position, 'value, 'edge> =
        { Previous = trail; ObservedRoot = observedRoot }
    let admitRenewal (start: Resolved<'root, 'position, 'value>) (steps: Transition<'root, 'position, 'value, 'edge> list) : Result<RenewalEvidence<'root, 'position, 'value, 'edge>, RenewalFailure<'root>> =
        DomainSkeleton.pending "Trails.admitRenewal"
    let renew (evidence: RenewalEvidence<'root, 'position, 'value, 'edge>) (renewal: Renewal<'root, 'position, 'value, 'edge>) : Result<Trail<'root, 'position, 'value, 'edge>, RenewalFailure<'root>> =
        DomainSkeleton.pending "Trails.renew"
    let current (trail: Trail<'root, 'position, 'value, 'edge>) : Resolved<'root, 'position, 'value> =
        DomainSkeleton.pending "Trails.current"
    let requestedPositions (renewal: Renewal<'root, 'position, 'value, 'edge>) : NonEmpty<'position> =
        DomainSkeleton.pending "Trails.requestedPositions"
    let steps (trail: Trail<'root, 'position, 'value, 'edge>) : Transition<'root, 'position, 'value, 'edge> list = trail.Steps
    let fromPosition (transition: Transition<'root, 'position, 'value, 'edge>) : Resolved<'root, 'position, 'value> = transition.From
    let toPosition (transition: Transition<'root, 'position, 'value, 'edge>) : Resolved<'root, 'position, 'value> = transition.To
    let edge (transition: Transition<'root, 'position, 'value, 'edge>) : Rooted<'root, 'edge> = transition.Through
