namespace BibleAtlas.FSharp.Domain

module internal DomainSkeleton =
    let pending<'value> (operation: string) : 'value =
        System.NotImplementedException(operation) |> raise
