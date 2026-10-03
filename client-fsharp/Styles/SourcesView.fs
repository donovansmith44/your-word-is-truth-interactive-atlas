namespace BibleAtlas.FSharp.Client

open Bolero
open Bolero.Html
open BibleAtlas.FSharp

module SourcesView =
    let render (state: SourcesState) (dispatch: SourcesMessage -> unit) : Node =
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
                    | SourcesState.Loading _ -> p { attr.``class`` "sources-loading"; "Loading sources…" }
                    | SourcesState.Available presentation -> SourceSectionsView.render (SourcesPresentation.view presentation)
                    | SourcesState.RetryableFailure _ ->
                        concat {
                            p { attr.``class`` "popover-meta"; "data-testid" => "could-not-load"; "Couldn't load the sources — please try again." }
                            button {
                                attr.``type`` "button"
                                attr.``class`` "popover-reveal-link explorable-quiet"
                                "data-testid" => "could-not-load-retry"
                                on.click (fun _ -> dispatch SourcesMessage.Retry)
                                "Try again"
                            }
                        }
                    | SourcesState.ReadRejected _ ->
                        p { attr.``class`` "popover-meta"; "data-testid" => "could-not-display"; "The sources response cannot be displayed." }
                    | SourcesState.PresentationRejected _ ->
                        p { attr.``class`` "popover-meta"; "data-testid" => "could-not-display"; "The sources list contains inconsistent categories." }
            }
        }
