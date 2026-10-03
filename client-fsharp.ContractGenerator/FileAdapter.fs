namespace BibleAtlas.FSharp.ContractGenerator

open System.IO

type FileOperation = Read | Write

type GeneratorFileError =
    | FileUnavailable of FileOperation * path: string * diagnostic: string
    | InvalidContract of ContractError

module FileAdapter =
    let generate document output =
        let read path =
            try Ok(File.ReadAllText path |> ContractDocument.ofText)
            with
            | :? IOException as error -> Error(FileUnavailable(Read, path, error.Message))
            | :? System.UnauthorizedAccessException as error -> Error(FileUnavailable(Read, path, error.Message))
        let write path source =
            try
                Path.GetDirectoryName(Path.GetFullPath path) |> Directory.CreateDirectory |> ignore
                File.WriteAllText(path, GeneratedSource.text source)
                Ok ()
            with
            | :? IOException as error -> Error(FileUnavailable(Write, path, error.Message))
            | :? System.UnauthorizedAccessException as error -> Error(FileUnavailable(Write, path, error.Message))
        read document |> Result.bind (Generator.generate >> Result.mapError InvalidContract) |> Result.bind (write output)

    let render error =
        match error with
        | InvalidContract error -> ContractError.render error
        | FileUnavailable(operation, path, diagnostic) -> $"{operation} {path}: {diagnostic}"
