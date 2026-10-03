namespace BibleAtlas.FSharp.Exploring

open BibleAtlas.FSharp.Domain
open BibleAtlas.FSharp.Paging

type FrontierCapacity = private FrontierCapacity of Positive

type Frontier<'root, 'position, 'cursor, 'edge> =
    private
        { Capacity: FrontierCapacity
          Windows: Map<RelationDirection, NeighbourWindow<'root, 'position, 'cursor, 'edge>> }

type Focus<'root, 'position, 'value, 'cursor, 'edge> =
    private
        { Current: Resolved<'root, 'position, 'value>
          Frontier: Frontier<'root, 'position, 'cursor, 'edge> }

[<RequireQualifiedAccess>]
type FocusFailure<'root> = WrongPosition | ChangedRoot of RootMismatch<'root> | OversizedFrontier

module Focuses =
    let capacity (entries: Positive) : FrontierCapacity = FrontierCapacity entries
    let beginAt (capacity: FrontierCapacity) (resolved: Resolved<'root, 'position, 'value>) : Focus<'root, 'position, 'value, 'cursor, 'edge> =
        { Current = resolved; Frontier = { Capacity = capacity; Windows = Map.empty } }
    let change (resolved: Resolved<'root, 'position, 'value>) (focus: Focus<'root, 'position, 'value, 'cursor, 'edge>) : Focus<'root, 'position, 'value, 'cursor, 'edge> =
        DomainSkeleton.pending "Focuses.change"
    let showNeighbours (window: NeighbourWindow<'root, 'position, 'cursor, 'edge>) (focus: Focus<'root, 'position, 'value, 'cursor, 'edge>) : Result<Focus<'root, 'position, 'value, 'cursor, 'edge>, FocusFailure<'root>> =
        DomainSkeleton.pending "Focuses.showNeighbours"
    let current (focus: Focus<'root, 'position, 'value, 'cursor, 'edge>) : Resolved<'root, 'position, 'value> = focus.Current
    let frontier (focus: Focus<'root, 'position, 'value, 'cursor, 'edge>) : Frontier<'root, 'position, 'cursor, 'edge> = focus.Frontier
    let windows (frontier: Frontier<'root, 'position, 'cursor, 'edge>) : NeighbourWindow<'root, 'position, 'cursor, 'edge> list =
        frontier.Windows |> Map.toList |> List.map snd
