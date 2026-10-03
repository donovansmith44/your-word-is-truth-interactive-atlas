namespace BibleAtlas.FSharp.Paging

open BibleAtlas.FSharp.Domain

type ReadingKey<'nodeId> = private { Container: 'nodeId }

type ReadingPageKey<'nodeId, 'cursor> = private { Reading: ReadingKey<'nodeId>; Cursor: 'cursor option }

type ReadingBounds = private { PageSize: Positive; WindowSize: Positive }

type ReadingPage<'nodeId, 'cursor, 'unit> =
    private
        { Key: ReadingPageKey<'nodeId, 'cursor>
          Units: 'unit list
          Previous: 'cursor option
          Next: 'cursor option }

type ReadingWindow<'root, 'nodeId, 'cursor, 'unit> =
    private
        { Root: 'root
          Key: ReadingKey<'nodeId>
          Bounds: ReadingBounds
          Pages: Rooted<'root, ReadingPage<'nodeId, 'cursor, 'unit>> list }

[<RequireQualifiedAccess>]
type ReadingExtent<'root, 'nodeId, 'cursor, 'reference, 'partRole> =
    | Paged of ReadingWindow<'root, 'nodeId, 'cursor, TextUnit<'nodeId, 'reference, 'partRole>>
    | Whole of Rooted<'root, WholeChapter<'nodeId, 'reference, 'partRole>>

[<RequireQualifiedAccess>]
type PageAdmissionFailure = OversizedPage | EmptyContinuation

[<RequireQualifiedAccess>]
type WindowFailure<'root> =
    | WrongQuery
    | WrongCursor
    | OversizedPage
    | EmptyContinuation
    | ChangedRoot of RootMismatch<'root>

module ReadingPages =
    let admit (bounds: ReadingBounds) (key: ReadingPageKey<'nodeId, 'cursor>) (units: 'unit list) (previous: 'cursor option) (next: 'cursor option) : Result<ReadingPage<'nodeId, 'cursor, 'unit>, PageAdmissionFailure> =
        DomainSkeleton.pending "ReadingPages.admit"
    let key (page: ReadingPage<'nodeId, 'cursor, 'unit>) : ReadingKey<'nodeId> = page.Key.Reading
    let cacheKey (page: ReadingPage<'nodeId, 'cursor, 'unit>) : ReadingPageKey<'nodeId, 'cursor> = page.Key
    let units (page: ReadingPage<'nodeId, 'cursor, 'unit>) : 'unit list = page.Units
    let before (page: ReadingPage<'nodeId, 'cursor, 'unit>) : 'cursor option = page.Previous
    let after (page: ReadingPage<'nodeId, 'cursor, 'unit>) : 'cursor option = page.Next

module ReadingWindows =
    let defaultBounds () : ReadingBounds =
        DomainSkeleton.pending "ReadingWindows.defaultBounds"
    let empty (root: 'root) (key: ReadingKey<'nodeId>) (bounds: ReadingBounds) : ReadingWindow<'root, 'nodeId, 'cursor, 'unit> =
        { Root = root; Key = key; Bounds = bounds; Pages = [] }
    let openAt (bounds: ReadingBounds) (page: Rooted<'root, ReadingPage<'nodeId, 'cursor, 'unit>>) : ReadingWindow<'root, 'nodeId, 'cursor, 'unit> =
        DomainSkeleton.pending "ReadingWindows.openAt"
    let append (page: Rooted<'root, ReadingPage<'nodeId, 'cursor, 'unit>>) (window: ReadingWindow<'root, 'nodeId, 'cursor, 'unit>) : Result<ReadingWindow<'root, 'nodeId, 'cursor, 'unit>, WindowFailure<'root>> =
        DomainSkeleton.pending "ReadingWindows.append"
    let prepend (page: Rooted<'root, ReadingPage<'nodeId, 'cursor, 'unit>>) (window: ReadingWindow<'root, 'nodeId, 'cursor, 'unit>) : Result<ReadingWindow<'root, 'nodeId, 'cursor, 'unit>, WindowFailure<'root>> =
        DomainSkeleton.pending "ReadingWindows.prepend"
    let units (window: ReadingWindow<'root, 'nodeId, 'cursor, 'unit>) : 'unit list =
        DomainSkeleton.pending "ReadingWindows.units"
    let before (window: ReadingWindow<'root, 'nodeId, 'cursor, 'unit>) : 'cursor option =
        DomainSkeleton.pending "ReadingWindows.before"
    let after (window: ReadingWindow<'root, 'nodeId, 'cursor, 'unit>) : 'cursor option =
        DomainSkeleton.pending "ReadingWindows.after"
    let key (window: ReadingWindow<'root, 'nodeId, 'cursor, 'unit>) : ReadingKey<'nodeId> = window.Key
    let root (window: ReadingWindow<'root, 'nodeId, 'cursor, 'unit>) : 'root = window.Root
    let pageSize (bounds: ReadingBounds) : Positive = bounds.PageSize
    let windowSize (bounds: ReadingBounds) : Positive = bounds.WindowSize

module ReadingKeys =
    let container (id: 'nodeId) : ReadingKey<'nodeId> = { Container = id }
    let page (key: ReadingKey<'nodeId>) (cursor: 'cursor option) : ReadingPageKey<'nodeId, 'cursor> =
        { Reading = key; Cursor = cursor }
