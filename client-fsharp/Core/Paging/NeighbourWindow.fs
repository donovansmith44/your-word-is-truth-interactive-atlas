namespace BibleAtlas.FSharp.Paging

open BibleAtlas.FSharp.Domain

type NeighbourKey<'position> = private { At: 'position; Through: RelationDirection }

type NeighbourPageKey<'position, 'cursor> = private { Neighbours: NeighbourKey<'position>; Cursor: 'cursor option }

type NeighbourEntry<'edge, 'position> = { Edge: 'edge; Position: 'position }

type NeighbourPage<'position, 'cursor, 'edge> =
    private
        { Key: NeighbourPageKey<'position, 'cursor>
          Entries: NeighbourEntry<'edge, 'position> list
          Previous: 'cursor option
          Next: 'cursor option }

type NeighbourCapacity = private { PageSize: Positive; WindowSize: Positive }

type NeighbourWindow<'root, 'position, 'cursor, 'edge> =
    private
        { Root: 'root
          Key: NeighbourKey<'position>
          Capacity: NeighbourCapacity
          Pages: Rooted<'root, NeighbourPage<'position, 'cursor, 'edge>> list }

[<RequireQualifiedAccess>]
type NeighbourCapacityFailure = WindowSmallerThanPage

module NeighbourPages =
    let admit (capacity: NeighbourCapacity) (key: NeighbourPageKey<'position, 'cursor>) (entries: NeighbourEntry<'edge, 'position> list) (previous: 'cursor option) (next: 'cursor option) : Result<NeighbourPage<'position, 'cursor, 'edge>, PageAdmissionFailure> =
        DomainSkeleton.pending "NeighbourPages.admit"
    let key (page: NeighbourPage<'position, 'cursor, 'edge>) : NeighbourKey<'position> = page.Key.Neighbours
    let cacheKey (page: NeighbourPage<'position, 'cursor, 'edge>) : NeighbourPageKey<'position, 'cursor> = page.Key

module Neighbours =
    let capacity (pageSize: Positive) (windowSize: Positive) : Result<NeighbourCapacity, NeighbourCapacityFailure> =
        DomainSkeleton.pending "Neighbours.capacity"
    let empty (root: 'root) (capacity: NeighbourCapacity) (key: NeighbourKey<'position>) : NeighbourWindow<'root, 'position, 'cursor, 'edge> =
        { Root = root; Key = key; Capacity = capacity; Pages = [] }
    let openAt (capacity: NeighbourCapacity) (page: Rooted<'root, NeighbourPage<'position, 'cursor, 'edge>>) : NeighbourWindow<'root, 'position, 'cursor, 'edge> =
        DomainSkeleton.pending "Neighbours.openAt"
    let append (page: Rooted<'root, NeighbourPage<'position, 'cursor, 'edge>>) (window: NeighbourWindow<'root, 'position, 'cursor, 'edge>) : Result<NeighbourWindow<'root, 'position, 'cursor, 'edge>, WindowFailure<'root>> =
        DomainSkeleton.pending "Neighbours.append"
    let prepend (page: Rooted<'root, NeighbourPage<'position, 'cursor, 'edge>>) (window: NeighbourWindow<'root, 'position, 'cursor, 'edge>) : Result<NeighbourWindow<'root, 'position, 'cursor, 'edge>, WindowFailure<'root>> =
        DomainSkeleton.pending "Neighbours.prepend"
    let entries (window: NeighbourWindow<'root, 'position, 'cursor, 'edge>) : NeighbourEntry<Rooted<'root, 'edge>, 'position> list =
        DomainSkeleton.pending "Neighbours.entries"
    let before (window: NeighbourWindow<'root, 'position, 'cursor, 'edge>) : 'cursor option =
        DomainSkeleton.pending "Neighbours.before"
    let after (window: NeighbourWindow<'root, 'position, 'cursor, 'edge>) : 'cursor option =
        DomainSkeleton.pending "Neighbours.after"
    let key (window: NeighbourWindow<'root, 'position, 'cursor, 'edge>) : NeighbourKey<'position> = window.Key
    let root (window: NeighbourWindow<'root, 'position, 'cursor, 'edge>) : 'root = window.Root

module NeighbourKeys =
    let at (position: 'position) (through: RelationDirection) : NeighbourKey<'position> =
        { At = position; Through = through }
    let page (key: NeighbourKey<'position>) (cursor: 'cursor option) : NeighbourPageKey<'position, 'cursor> =
        { Neighbours = key; Cursor = cursor }
