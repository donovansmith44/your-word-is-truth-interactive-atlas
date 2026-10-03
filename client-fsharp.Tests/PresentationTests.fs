module rec BibleAtlas.FSharp.Tests.PresentationTests

open FsCheck.Xunit
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract
open BibleAtlas.FSharp.Admission

[<Property>]
let ``a node card composes exactly its served label and provenance title`` (suffix: uint16) =
    let record = node suffix
    let expected = PopoverPresentation.Card { Title = record.Label; Fields = [{ Name = FieldName.Provenance; Value = record.Provenance.Title }] }
    Assert.Equal(Ok expected, present record.Version (Element.Node { Node = record }))

[<Property>]
let ``a polity card uses the served reign label without formatting its dates`` (suffix: uint16) =
    let record = node suffix
    let reign = window suffix
    let polity = { record with Kind = NodeKind.Polity; Polity = Some { Reign = reign } }
    let expected = PopoverPresentation.Card { Title = record.Label; Fields = [{ Name = FieldName.Reign; Value = reign.Label }; { Name = FieldName.Provenance; Value = record.Provenance.Title }] }
    Assert.Equal(Ok expected, present record.Version (Element.Node { Node = polity }))

[<Property>]
let ``a TextUnit presentation preserves its entire served text record and provenance title`` (suffix: uint16) =
    let record = node suffix
    let body = { Text = $"Served words {suffix}"; Locus = TextRef.Bible { Book = BookId.GEN; Chapter = 1; Verse = 1 }; Anchors = []; WordsOfChrist = [] }
    let text = { record with Kind = NodeKind.TextUnit; Text = Some body }
    let expected = PopoverPresentation.Text { Unit = body; Fields = [{ Name = FieldName.Provenance; Value = record.Provenance.Title }] }
    Assert.Equal(Ok expected, present record.Version (Element.Node { Node = text }))

[<Property>]
let ``a TextUnit without its served text is an explicit contract failure`` (suffix: uint16) =
    let record = { node suffix with Kind = NodeKind.TextUnit }
    Assert.Equal(Error(WireFixtures.graphFailure(GraphFailure.TextMissing record.Id)), present record.Version (Element.Node { Node = record }))

[<Property>]
let ``an edge card preserves its served label and optional provenance title`` (suffix: uint16) (hasProvenance: bool) =
    let record = node suffix
    let endpoint = PositionRef.Node { Node = { Id = record.Id; Kind = record.Kind; Label = record.Label } }
    let provenance = if hasProvenance then Some record.Provenance else None
    let edge = { Id = WireFixtures.identity<EdgeId> $"edge-{suffix}"; Kind = EdgeKind.Contains; Label = $"Served edge label {suffix}"; Provenance = provenance; EdgeSummary = []; Subject = endpoint; Object = endpoint; Narrative = None; Votes = None; Parentage = None }
    let fields = if hasProvenance then [{ Name = FieldName.Provenance; Value = record.Provenance.Title }] else []
    Assert.Equal(Ok(PopoverPresentation.Card { Title = edge.Label; Fields = fields }), present record.Version (Element.Edge { Edge = edge }))

[<Property>]
let ``a map or era card uses the whole served window label`` (suffix: uint16) (isMap: bool) =
    let record = node suffix
    let span = window suffix
    let element = if isMap then { record with Kind = NodeKind.Map; Map = Some { Window = span } } else { record with Kind = NodeKind.Era; Era = Some { Window = span } }
    let expected = PopoverPresentation.Card { Title = record.Label; Fields = [{ Name = FieldName.Window; Value = span.Label }; { Name = FieldName.Provenance; Value = record.Provenance.Title }] }
    Assert.Equal(Ok expected, present record.Version (Element.Node { Node = element }))

[<Property>]
let ``a place card composes its name and date claims without a blurb`` (suffix: uint16) =
    let record = node suffix
    let span = window suffix
    let canonical = $"Served canonical name {suffix}"
    let established = $"Served establishment {suffix}"
    let destroyed = $"Served destruction {suffix}"
    let place = { record with Kind = NodeKind.Place; Place = Some { Lat = 1.; Lon = 2.; DisplayName = $"Served display name {suffix}"; CanonicalName = Some canonical; Established = Some(claim span established); Destroyed = Some(claim span destroyed) } }
    let fields = [{ Name = FieldName.CanonicalName; Value = canonical }; { Name = FieldName.Established; Value = established }; { Name = FieldName.Destroyed; Value = destroyed }; { Name = FieldName.Provenance; Value = record.Provenance.Title }]
    Assert.Equal(Ok(PopoverPresentation.Card { Title = record.Label; Fields = fields }), present record.Version (Element.Node { Node = place }))

[<Property>]
let ``every presentation field has exactly its declared caption`` (reverse: bool) =
    let cases = Microsoft.FSharp.Reflection.FSharpType.GetUnionCases(typeof<FieldName>)
    let ordered = if reverse then Array.rev cases else cases
    let actual = ordered |> Array.map (fun case -> Microsoft.FSharp.Reflection.FSharpValue.MakeUnion(case, [||]) |> unbox<FieldName> |> Presenter.caption)
    let captions = [|"Window"; "Canonical name"; "Established"; "Destroyed"; "Reign"; "Provenance"|]
    Assert.Equal<string array>((if reverse then Array.rev captions else captions), actual)

let private present root element = Explorable.ofElement root element |> Result.map Focus.on |> Result.bind Presenter.popover

let resolved (record: NodeRecord) : Explorable =
    match Explorable.ofElement record.Version (Element.Node { Node = record }) with
    | Ok element -> element
    | Error failure -> failwithf "%A" failure

let node (suffix: uint16) : NodeRecord =
    { Id = WireFixtures.identity<NodeId> $"TextUnit:served-{suffix}"; Kind = NodeKind.Person; Label = $"Served label {suffix}"; Provenance = { Id = $"source-{suffix}"; Title = $"Served source title {suffix}" }; EdgeSummary = []; Version = WireFixtures.identity<ArtifactRoot> ((int suffix).ToString("x32")); Book = None; Catechism = None; Description = None; Era = None; Event = None; Map = None; Person = None; Place = None; Polity = None; Text = None }

let private window (suffix: uint16) =
    { From = { Value = -1406; Label = $"Served from {suffix}" }; To = { Value = -1200; Label = $"Served to {suffix}" }; Label = $"Served window label {suffix}" }

let private claim span label : DateClaim = { Label = label; When = span; Note = None; Verses = []; Event = None }
