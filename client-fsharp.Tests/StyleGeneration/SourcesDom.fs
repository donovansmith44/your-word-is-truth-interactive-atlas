module BibleAtlas.FSharp.Tests.StyleGeneration.SourcesDom

open Microsoft.AspNetCore.Components
open Bolero
open Bunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Client

type Component() =
    inherit ElmishComponent<SourcesState, SourcesMessage>()
    override _.View state dispatch = SourcesView.render state dispatch

type RenderAdapter =
    static member Sources context state dispatch : IRenderedComponent<Component> =
        RenderAdapter.Component<Component, SourcesState, SourcesMessage> context state dispatch
    static member App context model dispatch : IRenderedComponent<AppView> =
        RenderAdapter.Component<AppView, Model, Message> context model dispatch
    static member private Component<'view, 'model, 'message when 'view :> IComponent>
        (context: BunitContext) (model: 'model) (dispatch: 'message -> unit) : IRenderedComponent<'view> =
        context.Render<'view>(RenderFragment(fun builder ->
            builder.OpenComponent<'view>(0)
            builder.AddAttribute(1, nameof(Unchecked.defaultof<Component>.Model), model)
            builder.AddAttribute(2, nameof(Unchecked.defaultof<Component>.Dispatch), dispatch)
            builder.CloseComponent()))

let render context state dispatch = RenderAdapter.Sources context state dispatch
let renderApp context model dispatch = RenderAdapter.App context model dispatch
