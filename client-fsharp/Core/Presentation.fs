namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract

[<RequireQualifiedAccess>]
type FieldName = Window | CanonicalName | Established | Destroyed | Reign | Provenance

type PresentationField = { Name: FieldName; Value: string }
type CardPresentation = { Title: string; Fields: PresentationField list }
type TextPresentation = { Unit: UnitText; Fields: PresentationField list }

[<RequireQualifiedAccess>]
type PopoverPresentation = Card of CardPresentation | Text of TextPresentation

module rec Presenter =
    let popover (element: Resolved) : Result<PopoverPresentation, Failure> = Resolved.fold node edge element

    let caption (name: FieldName) : string =
        match name with
        | FieldName.Window -> "Window"
        | FieldName.CanonicalName -> "Canonical name"
        | FieldName.Established -> "Established"
        | FieldName.Destroyed -> "Destroyed"
        | FieldName.Reign -> "Reign"
        | FieldName.Provenance -> "Provenance"

    let private node (record: NodeRecord) : Result<PopoverPresentation, Failure> =
        let provenance = [{ Name = FieldName.Provenance; Value = record.Provenance }]
        match record.Kind with
        | NodeKind.TextUnit ->
            match record.Text with
            | Some unit -> Ok(PopoverPresentation.Text { Unit = unit; Fields = provenance })
            | None -> Error(Contract $"{record.Id} is served without its UnitText")
        | NodeKind.Container | NodeKind.Event | NodeKind.Narrative | NodeKind.Place | NodeKind.Person
        | NodeKind.Anchor | NodeKind.Era | NodeKind.Polity | NodeKind.CatechismItem | NodeKind.Source
        | NodeKind.Translation | NodeKind.PeopleGroup | NodeKind.CommentaryItem | NodeKind.LexiconEntry | NodeKind.Map ->
            let fields =
                [ field FieldName.Window (record.Map |> Option.map _.Window.Label)
                  field FieldName.Window (record.Era |> Option.map _.Window.Label)
                  field FieldName.CanonicalName (record.Place |> Option.bind _.CanonicalName)
                  field FieldName.Established (record.Place |> Option.bind _.Established |> Option.map _.Label)
                  field FieldName.Destroyed (record.Place |> Option.bind _.Destroyed |> Option.map _.Label)
                  field FieldName.Reign (record.Polity |> Option.map _.Reign.Label) ]
                |> List.choose id
            Ok(PopoverPresentation.Card { Title = record.Label; Fields = fields @ provenance })

    let private edge (record: EdgeRecord) : Result<PopoverPresentation, Failure> =
        let fields = field FieldName.Provenance record.Provenance |> Option.toList
        Ok(PopoverPresentation.Card { Title = record.Label; Fields = fields })

    let private field (name: FieldName) (value: string option) : PresentationField option = value |> Option.map (fun value -> { Name = name; Value = value })
