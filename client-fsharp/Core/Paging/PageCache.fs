namespace BibleAtlas.FSharp.Paging

open BibleAtlas.FSharp.Domain

[<RequireQualifiedAccess>]
type PageCacheKey<'nodeId, 'position, 'edgeCursor> =
    | Reading of ReadingPageKey<'nodeId, 'edgeCursor>
    | Neighbours of NeighbourPageKey<'position, 'edgeCursor>

[<RequireQualifiedAccess>]
type CachedPage<'nodeId, 'position, 'edgeCursor, 'unit, 'edge> =
    | Reading of ReadingPage<'nodeId, 'edgeCursor, 'unit>
    | Neighbours of NeighbourPage<'position, 'edgeCursor, 'edge>

type PageCache<'root, 'key, 'page when 'root: equality and 'key: equality> =
    private
        { Version: 'root
          PageBudget: Positive
          MostRecent: ('key * Rooted<'root, 'page>) list }

    static member Empty (root: 'root) (capacity: Positive) : PageCache<'root, 'key, 'page> =
        { Version = root; PageBudget = capacity; MostRecent = [] }

    member cache.Put (key: 'key) (page: Rooted<'root, 'page>) : Result<PageCache<'root, 'key, 'page>, RootMismatch<'root>> =
        Rooted.admit cache.Version (Rooted.root page) page
        |> Result.map (fun _ -> cache.Remember key page)

    member cache.Lookup (key: 'key) : CacheLookup<'root, 'key, 'page> =
        match cache.MostRecent |> List.tryFind (fun (storedKey, _) -> storedKey = key) with
        | Some (_, page) -> CacheLookup (Some page, cache.Remember key page)
        | None -> CacheLookup (None, cache)

    member cache.Rebase (root: 'root) : PageCache<'root, 'key, 'page> =
        if root = cache.Version then cache
        else PageCache<'root, 'key, 'page>.Empty root cache.PageBudget

    member cache.Root : 'root = cache.Version
    member cache.Capacity : Positive = cache.PageBudget
    member cache.Entries : ('key * Rooted<'root, 'page>) list = cache.MostRecent

    member private cache.Remember (key: 'key) (page: Rooted<'root, 'page>) : PageCache<'root, 'key, 'page> =
        { cache with
            MostRecent =
                (key, page) :: cache.MostRecent
                |> List.distinctBy fst
                |> List.truncate (Positive.value cache.PageBudget) }

and CacheLookup<'root, 'key, 'page when 'root: equality and 'key: equality> =
    private CacheLookup of page: Rooted<'root, 'page> option * cache: PageCache<'root, 'key, 'page>
    with

    member lookup.Page : Rooted<'root, 'page> option = match lookup with CacheLookup (page, _) -> page
    member lookup.Cache : PageCache<'root, 'key, 'page> = match lookup with CacheLookup (_, cache) -> cache
