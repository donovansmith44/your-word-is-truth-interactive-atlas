namespace BibleAtlas.FSharp.Admission

type Path<'step> = private Path of 'step list

[<RequireQualifiedAccess>]
type JsonStep = Property of string | Index of ArrayIndex

type JsonPath = Path<JsonStep>

module Paths =
    let root<'step> : Path<'step> = Path []
    let append (step: 'step) (Path steps) : Path<'step> = Path (steps @ [step])
    let steps (Path steps) : 'step list = steps
