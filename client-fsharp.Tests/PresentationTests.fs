module rec BibleAtlas.FSharp.Tests.PresentationTests

open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

[<Fact>]
let ``a node card composes exactly its served label and provenance`` () =
    Assert.Equal(Ok(PopoverPresentation.Card { Title = "Served label"; Fields = [{ Name = FieldName.Provenance; Value = "served-source" }] }), Presenter.popover (resolved node))

[<Fact>]
let ``a polity card uses the served reign label without formatting its dates`` () =
    let reign = { From = { Value = -1406; Label = "Served from" }; To = { Value = -1200; Label = "Served to" }; Label = "Served reign label" }
    let polity = { node with Kind = NodeKind.Polity; Polity = Some { Reign = reign } }
    let expected = PopoverPresentation.Card { Title = "Served label"; Fields = [{ Name = FieldName.Reign; Value = "Served reign label" }; { Name = FieldName.Provenance; Value = "served-source" }] }
    Assert.Equal(Ok expected, Presenter.popover (resolved polity))

[<Fact>]
let ``a TextUnit presentation preserves its entire served text record`` () =
    let body = { Text = "Served words"; Locus = TextRef.Bible { Book = BookId.GEN; Chapter = 1; Verse = 1 }; Anchors = []; WordsOfChrist = [] }
    let text = { node with Kind = NodeKind.TextUnit; Text = Some body }
    Assert.Equal(Ok(PopoverPresentation.Text { Unit = body; Fields = [{ Name = FieldName.Provenance; Value = "served-source" }] }), Presenter.popover (resolved text))

[<Fact>]
let ``a TextUnit without its served text is an explicit contract failure`` () =
    let text = { node with Kind = NodeKind.TextUnit }
    Assert.Equal(Error(Contract "TextUnit:served is served without its UnitText"), Presenter.popover (resolved text))

[<Fact>]
let ``an edge card has only its served title when provenance is absent`` () =
    let endpoint = Resolved.position (resolved node)
    let edge = { Id = "edge"; Kind = EdgeKind.Contains; Label = "Served edge label"; Provenance = None; EdgeSummary = []; Subject = endpoint; Object = endpoint; Narrative = None; Votes = None; Parentage = None }
    let element = Resolved.ofElement "root" (Element.Edge { Edge = edge }) |> Result.toOption |> Option.get
    Assert.Equal(Ok(PopoverPresentation.Card { Title = "Served edge label"; Fields = [] }), Presenter.popover element)

[<Theory>]
[<InlineData(true)>]
[<InlineData(false)>]
let ``a map or era card uses the whole served window label`` isMap =
    let window = { From = { Value = -1406; Label = "Served from" }; To = { Value = -1200; Label = "Served to" }; Label = "Served window label" }
    let record = if isMap then { node with Kind = NodeKind.Map; Map = Some { Window = window } } else { node with Kind = NodeKind.Era; Era = Some { Window = window } }
    Assert.Equal(Ok(PopoverPresentation.Card { Title = "Served label"; Fields = [{ Name = FieldName.Window; Value = "Served window label" }; { Name = FieldName.Provenance; Value = "served-source" }] }), Presenter.popover (resolved record))

[<Fact>]
let ``a place card composes its name and date claims without a blurb`` () =
    let window = { From = { Value = -1406; Label = "Served from" }; To = { Value = -1200; Label = "Served to" }; Label = "Served window label" }
    let claim label = { Label = label; When = window; Note = None; Verses = []; Event = None }
    let place = { node with Kind = NodeKind.Place; Place = Some { Lat = 1.; Lon = 2.; DisplayName = "Served display name"; CanonicalName = Some "Served canonical name"; Established = Some(claim "Served establishment"); Destroyed = Some(claim "Served destruction") } }
    let fields = [{ Name = FieldName.CanonicalName; Value = "Served canonical name" }; { Name = FieldName.Established; Value = "Served establishment" }; { Name = FieldName.Destroyed; Value = "Served destruction" }; { Name = FieldName.Provenance; Value = "served-source" }]
    Assert.Equal(Ok(PopoverPresentation.Card { Title = "Served label"; Fields = fields }), Presenter.popover (resolved place))

[<Fact>]
let ``every presentation field has exactly its declared caption`` () =
    let actual = Microsoft.FSharp.Reflection.FSharpType.GetUnionCases(typeof<FieldName>) |> Array.map (fun case -> Microsoft.FSharp.Reflection.FSharpValue.MakeUnion(case, [||]) |> unbox<FieldName> |> Presenter.caption)
    Assert.Equal<string array>([|"Window"; "Canonical name"; "Established"; "Destroyed"; "Reign"; "Provenance"|], actual)
let resolved (record: NodeRecord) : Resolved = Resolved.ofElement "root" (Element.Node { Node = record }) |> Result.toOption |> Option.get

let node: NodeRecord = { Id = "TextUnit:served"; Kind = NodeKind.Person; Label = "Served label"; Provenance = "served-source"; EdgeSummary = []; Version = "root"; Book = None; Catechism = None; Description = None; Era = None; Event = None; Map = None; Person = None; Place = None; Polity = None; Text = None }
