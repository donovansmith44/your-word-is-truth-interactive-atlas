namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract
open BibleAtlas.FSharp.Domain
open BibleAtlas.FSharp.Paging

type Focus = private Focus of Explorable

type FrontierCapacity = private FrontierCapacity of Positive

type Frontier<'cursor> =
    private
        { At: Focus
          Capacity: FrontierCapacity
          Windows: Map<RelationDirection, NeighbourWindow<string, PositionRef, 'cursor, EdgeRecord>> }

[<RequireQualifiedAccess>]
type FrontierFailure = WrongPosition | ChangedRoot of RootMismatch<string> | OversizedFrontier

module Focus =
    let on (explorable: Explorable) : Focus = Focus explorable
    let explorable (Focus explorable) : Explorable = explorable

module Frontiers =
    let capacity (entries: Positive) : FrontierCapacity = FrontierCapacity entries
    let empty (capacity: FrontierCapacity) (focus: Focus) : Frontier<'cursor> =
        { At = focus; Capacity = capacity; Windows = Map.empty }
    let showNeighbours (window: NeighbourWindow<string, PositionRef, 'cursor, EdgeRecord>) (frontier: Frontier<'cursor>) : Result<Frontier<'cursor>, FrontierFailure> =
        DomainSkeleton.pending "Frontiers.showNeighbours"
    let focus (frontier: Frontier<'cursor>) : Focus = frontier.At
    let windows (frontier: Frontier<'cursor>) : NeighbourWindow<string, PositionRef, 'cursor, EdgeRecord> list =
        frontier.Windows |> Map.toList |> List.map snd
