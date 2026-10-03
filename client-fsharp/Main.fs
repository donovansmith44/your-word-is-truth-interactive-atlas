namespace BibleAtlas.FSharp.Client

open System.Net.Http
open Microsoft.AspNetCore.Components
open Elmish
open Bolero
open BibleAtlas.FSharp

type App() =
    inherit ProgramComponent<Model, Message>()

    [<Inject>]
    member val Http = Unchecked.defaultof<HttpClient> with get, set

    override this.Program =
        let init _ =
            let model, effects = Model.init (Routes.parse (this.NavigationManager.ToAbsoluteUri this.NavigationManager.Uri))
            model, this.Commands effects
        let update message model =
            let model, effects = Model.update message model
            model, this.Commands effects
        let router =
            { new IRouter<Model, Message> with
                member _.GetRoute model = (Routes.url (Model.route model)).TrimStart('/')
                member _.SetRoute uri = Some(Navigate(Routes.parse (this.NavigationManager.ToAbsoluteUri uri)))
                member _.NotFound = Some(Navigate Route.NotFound) }
        Program.mkProgram init update View.app
        |> Program.withRouter router

    member private this.Commands (effects: Effect list) : Cmd<Message> =
        effects |> List.map (Runtime.command this.Http) |> Cmd.batch
