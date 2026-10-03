namespace BibleAtlas.FSharp.Tests.Domain

open FsCheck.Xunit
open BibleAtlas.FSharp.Domain
open BibleAtlas.FSharp.Paging

type PageCacheLaws() =
    [<Property>]
    member _.``a new cache retains its root and page budget with no entries`` (root: uint16) (budget: uint16) =
        let maximumTestPages = 32
        let pageBudget = 1 + int budget % maximumTestPages
        match Positive.admit pageBudget with
        | Ok capacity ->
            let cache = PageCache<uint16, int, int list>.Empty root capacity
            PageCacheLaws.snapshot cache = {| Root = root; Budget = pageBudget; Entries = [] |}
        | Error _ -> false

    [<Property>]
    member _.``put preserves a complete admitted page under its exact key`` (root: uint16) (key: int) (values: int list) =
        match Positive.admit 1, Rooted.admit root root values with
        | Ok capacity, Ok page ->
            let cache = PageCache<uint16, int, int list>.Empty root capacity
            (cache.Put key page |> Result.map PageCacheLaws.snapshot) = Ok {| Root = root; Budget = 1; Entries = [key, root, values] |}
        | Error _, _ | _, Error _ -> false

    [<Property>]
    member _.``a foreign page is refused without changing the previous cache`` (root: uint16) (key: int) (values: int list) =
        let foreignRoot = root + 1us
        match Positive.admit 1, Rooted.admit foreignRoot foreignRoot values with
        | Ok capacity, Ok page ->
            let cache = PageCache<uint16, int, int list>.Empty root capacity
            (cache.Put key page |> Result.mapError (fun mismatch -> {| Expected = mismatch.Expected; Observed = mismatch.Observed |})) = Error {| Expected = root; Observed = foreignRoot |}
            && PageCacheLaws.snapshot cache = {| Root = root; Budget = 1; Entries = [] |}
        | Error _, _ | _, Error _ -> false

    [<Property>]
    member _.``a missing cache lookup returns no page and the identical cache`` (root: uint16) (key: int) =
        match Positive.admit 1 with
        | Ok capacity ->
            let cache = PageCache<uint16, int, int list>.Empty root capacity
            let lookup = cache.Lookup key
            {| Page = lookup.Page; Cache = lookup.Cache |} = {| Page = None; Cache = cache |}
        | Error _ -> false

    [<Property>]
    member _.``a hit returns the whole page and refreshes recency before eviction`` (root: uint16) (first: int list) (second: int list) (third: int list) =
        match Positive.admit 2, Rooted.admit root root first, Rooted.admit root root second, Rooted.admit root root third with
        | Ok capacity, Ok a, Ok b, Ok c ->
            let empty = PageCache<uint16, int, int list>.Empty root capacity
            let beforeHit = empty.Put 1 a |> Result.bind (fun cache -> cache.Put 2 b)
            let actual =
                beforeHit |> Result.bind (fun cache ->
                    let hit = cache.Lookup 1
                    hit.Cache.Put 3 c |> Result.map (fun after ->
                        {| Page = hit.Page |> Option.map Rooted.value
                           Hit = PageCacheLaws.snapshot hit.Cache
                           After = PageCacheLaws.snapshot after
                           Original = PageCacheLaws.snapshot cache |}))
            actual = Ok {| Page = Some first
                           Hit = {| Root = root; Budget = 2; Entries = [1, root, first; 2, root, second] |}
                           After = {| Root = root; Budget = 2; Entries = [3, root, third; 1, root, first] |}
                           Original = {| Root = root; Budget = 2; Entries = [2, root, second; 1, root, first] |} |}
        | Error _, _, _, _ | _, Error _, _, _ | _, _, Error _, _ | _, _, _, Error _ -> false

    [<Property>]
    member _.``replacing a key installs its new page once and makes it most recent`` (root: uint16) (old: int list) (other: int list) (replacement: int list) =
        match Positive.admit 2, Rooted.admit root root old, Rooted.admit root root other, Rooted.admit root root replacement with
        | Ok capacity, Ok a, Ok b, Ok updated ->
            let cache = PageCache<uint16, int, int list>.Empty root capacity
            let actual = cache.Put 1 a |> Result.bind (fun current -> current.Put 2 b) |> Result.bind (fun current -> current.Put 1 updated) |> Result.map PageCacheLaws.snapshot
            actual = Ok {| Root = root; Budget = 2; Entries = [1, root, replacement; 2, root, other] |}
        | Error _, _, _, _ | _, Error _, _, _ | _, _, Error _, _ | _, _, _, Error _ -> false

    [<Property>]
    member _.``rebasing is identity at the same root and clears pages at another root`` (root: uint16) (values: int list) =
        let newRoot = root + 1us
        match Positive.admit 1, Rooted.admit root root values with
        | Ok capacity, Ok page ->
            let empty = PageCache<uint16, int, int list>.Empty root capacity
            let actual = empty.Put 1 page |> Result.map (fun cache ->
                let rebased = cache.Rebase newRoot
                {| Same = cache.Rebase root = cache
                   Different = PageCacheLaws.snapshot rebased
                   Idempotent = rebased.Rebase newRoot = rebased
                   Original = PageCacheLaws.snapshot cache |})
            actual = Ok {| Same = true
                           Different = {| Root = newRoot; Budget = 1; Entries = [] |}
                           Idempotent = true
                           Original = {| Root = root; Budget = 1; Entries = [1, root, values] |} |}
        | Error _, _ | _, Error _ -> false

    [<Property(MaxTest = 12)>]
    member _.``ten thousand distinct puts retain exactly the latest page budget and no prefix history`` (root: uint16) (budget: uint16) =
        let maximumTestPages = 32
        let turns = 10000
        let pageBudget = 1 + int budget % maximumTestPages
        match Positive.admit pageBudget with
        | Ok capacity ->
            let empty = PageCache<uint16, int, int>.Empty root capacity
            let actual =
                [1..turns]
                |> List.fold (fun current key ->
                    current |> Result.bind (fun (cache: PageCache<uint16, int, int>) ->
                        Rooted.admit root root key |> Result.bind (cache.Put key))) (Ok empty)
                |> Result.map PageCacheLaws.snapshot
            let latest = [turns - pageBudget + 1..turns] |> List.rev |> List.map (fun key -> key, root, key)
            actual = Ok {| Root = root; Budget = pageBudget; Entries = latest |}
        | Error _ -> false

    [<Property>]
    member _.``structured query and cursor keys retain independent complete pages`` (root: uint16) (query: string) (cursor: uint16) (first: int list) (second: int list) =
        let firstKey = {| Query = query; Cursor = cursor |}
        let secondKey = {| Query = query; Cursor = cursor + 1us |}
        match Positive.admit 2, Rooted.admit root root first, Rooted.admit root root second with
        | Ok capacity, Ok a, Ok b ->
            let empty = PageCache<_, _, _>.Empty root capacity
            let actual =
                empty.Put firstKey a
                |> Result.bind (fun cache -> cache.Put secondKey b)
                |> Result.map (fun cache ->
                    {| First = (cache.Lookup firstKey).Page |> Option.map Rooted.value
                       Second = (cache.Lookup secondKey).Page |> Option.map Rooted.value
                       Cache = PageCacheLaws.snapshot cache |})
            actual = Ok {| First = Some first
                           Second = Some second
                           Cache = {| Root = root; Budget = 2; Entries = [secondKey, root, second; firstKey, root, first] |} |}
        | Error _, _, _ | _, Error _, _ | _, _, Error _ -> false

    [<Property>]
    member _.``rebasing refuses an old root completion and retains the empty new root cache`` (root: uint16) (key: int) (values: int list) =
        let newRoot = root + 1us
        match Positive.admit 1, Rooted.admit root root values with
        | Ok capacity, Ok oldPage ->
            let before = PageCache<uint16, int, int list>.Empty root capacity
            let rebased = before.Rebase newRoot
            let refused = rebased.Put key oldPage |> Result.mapError (fun mismatch -> {| Expected = mismatch.Expected; Observed = mismatch.Observed |})
            {| Result = refused; Cache = PageCacheLaws.snapshot rebased |}
            = {| Result = Error {| Expected = newRoot; Observed = root |}
                 Cache = {| Root = newRoot; Budget = 1; Entries = [] |} |}
        | Error _, _ | _, Error _ -> false

    static member private snapshot<'root, 'key, 'page when 'root: equality and 'key: equality> (cache: PageCache<'root, 'key, 'page>) : {| Root: 'root; Budget: int; Entries: ('key * 'root * 'page) list |} =
        {| Root = cache.Root
           Budget = Positive.value cache.Capacity
           Entries = cache.Entries |> List.map (fun (key, page) -> key, Rooted.root page, Rooted.value page) |}
