module BibleAtlas.FSharp.Tests.SchemaSpike.SurveyData

open System.Text.Json
open System.Text.Json.Nodes

type QueryExample = { SchemaName: string; FileName: string; Body: JsonElement }
type Inputs = { Document: JsonNode; Examples: QueryExample list }
