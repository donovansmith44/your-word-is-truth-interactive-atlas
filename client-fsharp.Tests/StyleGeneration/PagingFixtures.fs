module BibleAtlas.FSharp.Tests.StyleGeneration.PagingFixtures

open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

let window index size : TextWindow =
    { Units = [1 .. size] |> List.map (fun unit ->
        { Ref = $"opaque-{index}-{unit}"; Node = { Id = $"TextUnit:{index}:{unit}"; Kind = NodeKind.TextUnit; Label = $"Served {index} {unit}" }
          Heading = None; EdgeSummary = []
          Body = { Text = $"Body {index} {unit}"; Locus = TextRef.Concord { Part = 1; Article = 1; Paragraph = index }; Anchors = []; WordsOfChrist = [] } })
      Next = Some $"opaque-next-{index}"; Version = "root" }

let project (model: Model) =
    match model.Surface with
    | Surface.Concord page -> Ok(ConcordPage.reference page, ConcordPage.state page, model.Serial, model.Focus)
    | Surface.Reader _ | Surface.Sources _ | Surface.World | Surface.Kretzmann | Surface.NotFound -> Error model

let contents: Contents = { Corpus = Corpus.Concord; Roots = []; Version = "root" }
let pageLimit = 20
let futurePageCount = 10_000
