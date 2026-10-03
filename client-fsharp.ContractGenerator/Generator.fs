namespace BibleAtlas.FSharp.ContractGenerator

module Generator =
    let generate : ContractDocument -> Result<GeneratedSource, ContractError> =
        ContractReader.read >> Result.map ContractEmitter.emit
