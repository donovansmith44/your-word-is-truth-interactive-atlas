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
                    | Route.Reader | Route.Read _ -> reader model dispatch
                    | Route.Concord _ -> concord model dispatch
                    | Route.World | Route.Kretzmann | Route.NotFound -> Node.Empty()
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

    and private reader model dispatch : Node =
        div {
            attr.``class`` "reader-frame"
            "data-testid" => "reader-frame"
            div {
                attr.``class`` "reader-page"
                "data-testid" => "reader-root"
                cond (readingState Corpus.Bible model) <| function
                    | Empty | Loading _ -> p { attr.``class`` "reader-loading"; "Loading…" }
                    | Failed _ -> failed dispatch
                    | Ready window ->
                        article {
                            attr.``class`` "reader-column"
                            readerTitle model window
                            forEach window.Units <| fun unit ->
                                concat {
                                    cond unit.Heading <| function
                                        | None -> Node.Empty()
                                        | Some heading -> unitHeading heading dispatch
                                    cond unit.Body.Locus <| function
                                        | TextRef.Bible locus -> verse unit locus.Verse dispatch
                                        | TextRef.Concord _ -> Node.Empty()
                                }
                        }
            }
        }

    and private readingState corpus model =
        match model.Reading, Map.tryFind corpus model.Contents with
        | Empty, Some(Failed(request, failure, _)) -> Failed(request, failure, None)
        | state, _ -> state

    and private readerTitle model (window: TextWindow) : Node =
        match window.Units |> List.tryHead with
        | Some unit ->
            match unit.Body.Locus with
            | TextRef.Bible locus ->
                let title =
                    match Map.tryFind Corpus.Bible model.Contents with
                    | Some(Ready contents) ->
                        contents.Roots |> List.tryPick (fun root ->
                            match root.Locus with
                            | TextRef.Bible first when first.Book = locus.Book -> Some root.Title
                            | TextRef.Bible _ | TextRef.Concord _ -> None)
                    | None | Some Empty | Some(Loading _) | Some(Failed _) -> None
                h1 {
                    attr.``class`` "chapter-head"
                    span { attr.``class`` "chapter-head-book"; title |> Option.defaultValue unit.Node.Label }
                    span { attr.``class`` "chapter-head-num"; string locus.Chapter }
                }
            | TextRef.Concord _ -> Node.Empty()
        | None -> Node.Empty()

    and private verse (unit: TextUnit) number dispatch : Node =
        div {
            attr.``class`` "verse-line explorable"
            "data-testid" => $"verse-line-{number}"
            "data-focal" => "false"
            attr.id $"v{number}"
            attr.tabindex 0
            "role" => "button"
            "aria-label" => ("Explore " + unit.Node.Label)
            on.click (fun _ -> openNode unit.Node dispatch)
            on.keydown (fun event -> activate event unit.Node dispatch)
            button {
                attr.``type`` "button"
                attr.``class`` "verse-num"
                "data-testid" => $"verse-num-{number}"
                on.click (fun _ -> openNode unit.Node dispatch)
                on.stopPropagation "click" true
                on.stopPropagation "keydown" true
                string number
            }
            span {
                attr.``class`` "verse-text"
                anchoredText unit.Body (fun piece ->
                    match piece.Anchor with
                    | None -> text piece.Text
                    | Some anchor ->
                        let className = if anchor.Node.Kind = NodeKind.Person then "verse-mention verse-mention-person" else "verse-mention"
                        textPiece className $"verse-mention-{number}-{anchor.Node.Id}" piece anchor dispatch)
            }
        }

    and private concord model dispatch : Node =
        div {
            attr.``class`` "concord-page"
            "data-testid" => "concord-page"
            div {
                attr.``class`` "concord-reading"
                article {
                    attr.``class`` "concord-column reader-column"
                    h1 { attr.``class`` "concord-title"; "The Book of Concord" }
                    p { attr.``class`` "concord-intro"; "The Lutheran confessions of 1580 — the church's own confession, subordinate to the Scripture it confesses." }
                    cond (readingState Corpus.Concord model) <| function
                        | Empty | Loading _ -> p { attr.``class`` "concord-loading"; "Loading…" }
                        | Failed _ -> failed dispatch
                        | Ready window -> forEach window.Units <| fun unit ->
                            div {
                                let explorable = not unit.EdgeSummary.IsEmpty
                                let interactive =
                                    if explorable then
                                        attrs {
                                            attr.tabindex 0
                                            "role" => "button"
                                            "aria-label" => ("Explore " + unit.Node.Label)
                                            on.click (fun _ -> openNode unit.Node dispatch)
                                            on.keydown (fun event -> activate event unit.Node dispatch)
                                        }
                                    else Attr.Empty()
                                attr.``class`` (if explorable then "concord-unit explorable" else "concord-unit")
                                "data-testid" => ("concord-unit-" + unit.Node.Id)
                                interactive
                                span { attr.``class`` "concord-unit-ref"; unit.Ref }
                                anchoredText unit.Body (fun piece ->
                                    match piece.Anchor with
                                    | None -> text piece.Text
                                    | Some anchor -> textPiece "concord-ref" $"concord-ref-{unit.Node.Id}-{anchor.Start}" piece anchor dispatch)
                            }
                }
            }
        }

    and private anchoredText body decorate : Node =
        concat {
            forEach (AnchoredText.runs body) <| function
                | [] -> Node.Empty()
                | first :: remaining ->
                    let pieces = concat { forEach (first :: remaining) decorate }
                    cond first.IsWordsOfChrist <| function
                        | true -> span { attr.``class`` "words-of-christ"; pieces }
                        | false -> pieces
        }

    and private textPiece className testId piece (anchor: Anchor) dispatch : Node =
        span {
            attr.``class`` className
            "data-testid" => testId
            attr.tabindex 0
            "role" => "button"
            "aria-label" => ("Explore " + (if anchor.Node.Kind = NodeKind.Person then anchor.Node.Label else piece.Text))
            on.click (fun _ -> openNode anchor.Node dispatch)
            on.stopPropagation "click" true
            on.keydown (fun event -> activate event anchor.Node dispatch)
            on.stopPropagation "keydown" true
            piece.Text
        }

    and private unitHeading (heading: UnitHeading) dispatch : Node =
        h2 {
            attr.``class`` ("pericope-heading" + (if heading.IsContinuation then " pericope-heading-continuation" else "") + (if heading.Kind = EventKind.General then " explorable-quiet" else " explorable"))
            "data-testid" => ("pericope-heading-" + heading.Event.Id)
            "data-continuation" => (if heading.IsContinuation then "true" else "false")
            attr.tabindex 0
            "role" => "button"
            "aria-label" => ("Explore " + heading.Event.Label + (if heading.IsContinuation then " (continued)" else ""))
            on.click (fun _ -> openNode heading.Event dispatch)
            on.keydown (fun event -> activate event heading.Event dispatch)
            if heading.IsContinuation then
                span {
                    attr.``class`` "pericope-heading-continuation-marker"
                    "data-testid" => ("pericope-heading-continuation-marker-" + heading.Event.Id)
                    "continued"
                }
            heading.Event.Label
        }

    and private openNode node dispatch = dispatch (OpenPosition(PositionRef.Node { Node = node }))

    and private activate (event: Microsoft.AspNetCore.Components.Web.KeyboardEventArgs) node dispatch =
        match event.Key with
        | "Enter" | " " -> openNode node dispatch
        | _ -> ()

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
