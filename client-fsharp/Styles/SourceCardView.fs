namespace BibleAtlas.FSharp.Client

open Bolero
open Bolero.Html
open BibleAtlas.FSharp

module SourceCardView =
    let render (card: SourceCardPresentation) : Node =
        let source = card.Entry
        article {
            attr.``class`` "source-card"
            "data-testid" => ("source-" + source.Id)
            h3 { attr.``class`` "source-title"; source.Title }
            p { attr.``class`` "source-what"; source.WhatItIs }
            p { attr.``class`` "source-built"; span { attr.``class`` "source-label"; "What we built:" }; " " + source.WhatWeBuilt }
            p { attr.``class`` "source-license"; span { attr.``class`` "source-label"; "License:" }; " " + source.License }
            cond card.Visit <| function
                | None -> Node.Empty()
                | Some link ->
                    a {
                        attr.``class`` "source-link"
                        "data-testid" => ("source-link-" + source.Id)
                        attr.href link
                        attr.target "_blank"
                        attr.rel "noopener noreferrer"
                        "Visit source"
                    }
        }
