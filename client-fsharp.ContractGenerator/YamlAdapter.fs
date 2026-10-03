namespace BibleAtlas.FSharp.ContractGenerator

open System.IO
open YamlDotNet.Core
open YamlDotNet.RepresentationModel

module YamlAdapter =
    let read (document: ContractDocument) : Result<YamlValue, ContractError> =
        let rec value path (node: YamlNode) =
            match node with
            | :? YamlScalarNode as scalar -> Ok(Scalar(if isNull scalar.Value then "" else scalar.Value))
            | :? YamlSequenceNode as sequence ->
                sequence.Children
                |> Seq.mapi (fun index item -> value (DocumentPath.itemAt index path) item)
                |> Seq.fold (fun result item -> Result.bind (fun items -> Result.map (fun item -> item :: items) item) result) (Ok [])
                |> Result.map (List.rev >> Sequence)
            | :? YamlMappingNode as mapping ->
                mapping.Children
                |> Seq.fold (fun result pair ->
                    Result.bind (fun pairs ->
                        value path pair.Key
                        |> Result.bind (fun key -> value path pair.Value |> Result.map (fun item -> (key, item) :: pairs))) result) (Ok [])
                |> Result.map (List.rev >> Mapping)
            | _ -> Error(ExpectedMapping path)
        try
            let stream = YamlStream()
            use reader = new StringReader(ContractDocument.text document)
            stream.Load(reader)
            match Seq.toList stream.Documents with
            | [document] -> value DocumentPath.root document.RootNode
            | [] -> Error(NoSchemas(DocumentPath.memberAt "components" DocumentPath.root |> DocumentPath.memberAt "schemas"))
            | _ -> Error(InvalidYaml { Line = 1; Column = 1; Message = "expected one document" })
        with :? YamlException as error ->
            Error(InvalidYaml { Line = int error.Start.Line; Column = int error.Start.Column; Message = error.Message })
