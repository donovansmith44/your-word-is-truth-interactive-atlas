namespace BibleAtlas.FSharp.Client

open Bolero
open Bolero.Html
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

module View =
    let rec app (model: Model) (dispatch: Message -> unit) : Node =
        concat {
            headerView ()
            main {
                attr.``class`` "app-main"
                cond model.Route <| function
                    | Route.Sources -> sources model.Sources dispatch
                    | Route.Reader | Route.Read _ | Route.World | Route.Kretzmann | Route.Concord _ | Route.NotFound -> Node.Empty()
            }
        }

    and private headerView () : Node =
        header {
            attr.``class`` "app-header header-parchment"
            a {
                attr.``class`` "wordmark"
                attr.href "/"
                "Bible Explorer "
                span { attr.``class`` "wordmark-glyph"; "aria-hidden" => "true"; "∴" }
            }
            nav {
                attr.``class`` "app-nav"
                "aria-label" => "Primary"
                a { attr.``class`` "nav-link"; attr.href "/"; "data-testid" => "nav-reader"; "Reader" }
                a { attr.``class`` "nav-link"; attr.href "/world"; "data-testid" => "nav-world"; "World" }
                a { attr.``class`` "nav-link"; attr.href "/kretzmann"; "data-testid" => "nav-kretzmann"; "Kretzmann" }
                a { attr.``class`` "nav-link"; attr.href "/concord"; "data-testid" => "nav-concord"; "Concord" }
            }
            div {
                attr.``class`` "header-tools"
                label { attr.``class`` "translation-label"; attr.``for`` "translation-select"; "Translation" }
                select {
                    attr.id "translation-select"
                    attr.``class`` "translation-select"
                    "data-testid" => "translation-select"
                    option { attr.value "kjv"; "KJV" }
                }
                a { attr.``class`` "attribution"; "data-testid" => "attribution"; attr.href "/sources"; "Credits" }
            }
        }

    and private sources (state: LoadState<SourcesDocument>) dispatch : Node =
        div {
            attr.``class`` "sources-page"
            "data-testid" => "sources-page"
            div {
                attr.``class`` "sources-column"
                h1 { attr.``class`` "sources-title"; "Sources" }
                p {
                    attr.``class`` "sources-intro"
                    "Bible Explorer stands on the work of translators, researchers, and scholars across many centuries. Every source below is honestly named: what it is, what we built from it, and its license."
                }
                cond state <| function
                    | Empty | Loading _ -> p { attr.``class`` "sources-loading"; "Loading sources…" }
                    | Failed _ -> failed dispatch
                    | Ready document ->
                        let categories = document.Sources |> List.groupBy _.Category |> Map.ofList
                        forEach document.Categories <| fun category ->
                            section {
                                attr.``class`` "sources-category"
                                "data-testid" => ("sources-category-" + category.Id)
                                h2 { attr.``class`` "sources-category-title"; category.Label }
                                div {
                                    attr.``class`` "sources-grid"
                                    forEach (Map.tryFind category.Id categories |> Option.defaultValue []) sourceCard
                                }
                            }
            }
        }

    and private failed dispatch : Node =
        concat {
            p { attr.``class`` "popover-meta"; "data-testid" => "could-not-load"; "Couldn't load this — check your connection and try again." }
            button {
                attr.``type`` "button"
                attr.``class`` "popover-reveal-link explorable-quiet"
                "data-testid" => "could-not-load-retry"
                on.click (fun _ -> dispatch Retry)
                "Try again"
            }
        }

    and private sourceCard (source: SourceEntry) : Node =
        article {
            attr.``class`` "source-card"
            "data-testid" => ("source-" + source.Id)
            h3 { attr.``class`` "source-title"; source.Title }
            p { attr.``class`` "source-what"; source.WhatItIs }
            p { attr.``class`` "source-built"; span { attr.``class`` "source-label"; "What we built:" }; " " + source.WhatWeBuilt }
            p { attr.``class`` "source-license"; span { attr.``class`` "source-label"; "License:" }; " " + source.License }
            cond source.Link <| function
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

type AppView() =
    inherit ElmishComponent<Model, Message>()
    override _.View model dispatch = View.app model dispatch
