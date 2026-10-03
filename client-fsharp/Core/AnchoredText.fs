namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract

type TextPiece = { Text: string; IsWordsOfChrist: bool; Anchor: Anchor option }

module AnchoredText =
    let runs (unit: UnitText) =
        let offsets = unit.Text.EnumerateRunes() |> Seq.scan (fun offset rune -> offset + rune.Utf16SequenceLength) 0 |> Seq.toArray
        let utf16 scalar = offsets[max 0 (min scalar (offsets.Length - 1))]
        let red = unit.WordsOfChrist |> List.map (fun span -> utf16 span.Start, utf16 span.End)
        let anchored = unit.Anchors |> List.map (fun anchor -> utf16 anchor.Start, utf16 anchor.End, anchor)
        let cuts =
            [0; unit.Text.Length]
            @ List.collect (fun (start, finish) -> [start; finish]) red
            @ List.collect (fun (start, finish, _) -> [start; finish]) anchored
            |> List.distinct |> List.sort
        let pieces =
            cuts |> List.pairwise |> List.map (fun (start, finish) ->
                { Text = unit.Text.Substring(start, finish - start)
                  IsWordsOfChrist = red |> List.exists (fun (left, right) -> left <= start && finish <= right)
                  Anchor = anchored |> List.tryPick (fun (left, right, anchor) -> if left <= start && finish <= right then Some anchor else None) })
        pieces |> List.fold (fun runs piece ->
            match runs with
            | (first :: earlier) :: rest when first.IsWordsOfChrist = piece.IsWordsOfChrist -> (piece :: first :: earlier) :: rest
            | _ -> [piece] :: runs) []
        |> List.rev |> List.map List.rev
