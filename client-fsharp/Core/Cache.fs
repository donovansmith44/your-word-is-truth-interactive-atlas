namespace BibleAtlas.FSharp

type Cache<'key, 'value when 'key: comparison> = private Cache of capacity: int * entries: Map<'key, 'value> * recent: 'key list

module Cache =
    let empty capacity = Cache(max 0 capacity, Map.empty, [])

    let find key (Cache(capacity, entries, recent)) =
        match Map.tryFind key entries with
        | None -> None, Cache(capacity, entries, recent)
        | Some value -> Some value, Cache(capacity, entries, key :: List.filter ((<>) key) recent)

    let put key value (Cache(capacity, entries, recent)) =
        if capacity = 0 then Cache(capacity, Map.empty, [])
        else
            let touched = key :: List.filter ((<>) key) recent
            let kept = List.truncate capacity touched
            let entries = Map.add key value entries
            let entries = List.skip kept.Length touched |> List.fold (fun entries key -> Map.remove key entries) entries
            Cache(capacity, entries, kept)

    let count (Cache(_, entries, _)) = Map.count entries
