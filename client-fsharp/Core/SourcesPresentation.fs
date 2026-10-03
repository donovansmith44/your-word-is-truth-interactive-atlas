namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract

type SourceCardPresentation = { Entry: SourceEntry; Visit: string option }
type SourceSectionPresentation = { Category: SourceCategory; Cards: SourceCardPresentation list }
type SourceSections = { Document: SourcesDocument; Sections: SourceSectionPresentation list }
type SourcesPresentation = private SourcesPresentation of SourceSections

[<RequireQualifiedAccess>]
type SourcePresentationFailure =
    | UnlistedCategories of NonEmpty<SourceEntry>
    | RepeatedCategories of NonEmpty<SourceCategory>

module SourcesPresenter =
    let present (document: SourcesDocument) : Result<SourcesPresentation, SourcePresentationFailure> =
        let repeatedIds =
            document.Categories |> List.groupBy _.Id
            |> List.choose (fun (identity, categories) ->
                match categories with
                | [] | [_] -> None
                | _ :: _ :: _ -> Some identity)
            |> Set.ofList
        let repeated = document.Categories |> List.filter (fun category -> Set.contains category.Id repeatedIds)
        match repeated with
        | first :: rest -> Error(SourcePresentationFailure.RepeatedCategories(NonEmpty.create first rest))
        | [] ->
            let known = document.Categories |> List.map _.Id |> Set.ofList
            let unlisted = document.Sources |> List.filter (fun source -> Set.contains source.Category known |> not)
            match unlisted with
            | first :: rest -> Error(SourcePresentationFailure.UnlistedCategories(NonEmpty.create first rest))
            | [] ->
                let grouped = document.Sources |> List.groupBy _.Category |> Map.ofList
                let sections = document.Categories |> List.map (fun category ->
                    let cards =
                        Map.tryFind category.Id grouped |> Option.defaultValue []
                        |> List.map (fun entry -> { Entry = entry; Visit = entry.Link |> Option.filter (System.String.IsNullOrWhiteSpace >> not) })
                    { Category = category; Cards = cards })
                Ok(SourcesPresentation { Document = document; Sections = sections })

module SourcesPresentation =
    let view (SourcesPresentation presentation) = presentation
