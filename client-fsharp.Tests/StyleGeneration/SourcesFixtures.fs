module BibleAtlas.FSharp.Tests.StyleGeneration.SourcesFixtures

open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

let document (categorySeed: byte) (cardSeed: byte) (linkSeed: byte) reversed : SourceSections =
    let categoryCount = 1 + int categorySeed % 3
    let cardCount = 1 + int cardSeed % 5
    let sections =
        [1 .. categoryCount] |> List.map (fun category ->
            let category = { Id = $"category-{category}"; Label = $"Category {category}" }
            let cards =
                [1 .. cardCount] |> List.map (fun card ->
                    let identity = $"source-{category.Id}-{card}"
                    let link, visit =
                        match int linkSeed % 4 with
                        | 0 -> None, None
                        | 1 -> Some "", None
                        | 2 -> Some " \t ", None
                        | _ -> Some($"https://example.test/{identity}"), Some($"https://example.test/{identity}")
                    { Entry = { Id = identity; Category = category.Id; Title = $"Title {card}"
                                WhatItIs = $"Description {card}"; WhatWeBuilt = $"Use {card}"
                                License = $"License {card}"; LicensesRowKey = identity + "-row"; Link = link }
                      Visit = visit })
            { Category = category; Cards = cards })
    let sources = sections |> List.collect (fun section -> section.Cards |> List.map _.Entry)
    let ordered = if reversed then List.rev sections else sections
    { Document = { Categories = ordered |> List.map _.Category; Sources = sources
                   Provenances = Some [{ Id = "provenance"; Source = "source-category-1-1"; Confidence = Confidence.Curated; Locator = Some "locator" }] }
      Sections = ordered }

let ticket (steps: byte) = [1 .. int steps % 20] |> List.fold (fun ticket _ -> RequestId.next ticket) RequestId.initial
