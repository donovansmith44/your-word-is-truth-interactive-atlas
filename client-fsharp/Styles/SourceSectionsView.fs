namespace BibleAtlas.FSharp.Client

open Bolero
open Bolero.Html
open BibleAtlas.FSharp

module SourceSectionsView =
    let render (presentation: SourceSections) : Node =
        forEach presentation.Sections <| fun section ->
            Bolero.Html.section {
                attr.``class`` "sources-category"
                "data-testid" => ("sources-category-" + section.Category.Id)
                h2 { attr.``class`` "sources-category-title"; section.Category.Label }
                div { attr.``class`` "sources-grid"; forEach section.Cards SourceCardView.render }
            }
