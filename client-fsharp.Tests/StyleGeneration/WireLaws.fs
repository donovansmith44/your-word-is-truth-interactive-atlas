module BibleAtlas.FSharp.Tests.StyleGeneration.WireLaws

open FsCheck
open FsCheck.Xunit
open global.Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

[<Property>]
let ``the typed source decoder round trips every field`` (NonNull text: NonNull<string>) link =
    let expected: SourceEntry =
        { Id = text; Category = text + ":category"; Title = text + ":title"
          License = text + ":license"; LicensesRowKey = text + ":license-row"
          WhatItIs = text + ":description"; WhatWeBuilt = text + ":use"
          Link = if link then Some(text + ":link") else None }
    Assert.Equal(Ok expected, WireDecoder.sourceEntry (Json.encode expected))

[<Property>]
let ``malformed JSON preserves its whole syntax diagnostic`` (indent: byte) =
    let indent = int indent % 12
    let malformed = String.replicate indent " " + "{"
    let expected =
        Error(InvalidSyntax
            { Path = JsonPath.ofLibraryPath "$"; Line = Some 0L; Byte = Some(int64 indent + 1L)
              Message = $"Expected depth to be zero at the end of the JSON payload. There is an open JSON object or array that should be closed. LineNumber: 0 | BytePositionInLine: {indent + 1}." })
    Assert.Equal(expected, WireDecoder.sourceEntry malformed)

[<Property>]
let ``a null record answer is refused at the typed door`` (indent: byte) =
    Assert.Equal(Error NullPayload, WireDecoder.sourceEntry (String.replicate (int indent % 12) " " + "null"))

[<Property>]
let ``valid JSON with the wrong field type is a typed value failure`` (number: int) =
    let body = $"{{\"id\":\"s\",\"category\":\"c\",\"title\":{number},\"license\":\"PD\",\"licenses_row_key\":\"s\",\"what_it_is\":\"text\",\"what_we_built\":\"reader\"}}"
    let tokenLength = (Json.encode number).Length
    let expected =
        Error(InvalidValue
            { Path = JsonPath.ofLibraryPath "$"; Line = Some 0L; Byte = Some(int64 tokenLength)
              Message = $"The JSON value could not be converted to System.String. Path: $ | LineNumber: 0 | BytePositionInLine: {tokenLength}." })
    Assert.Equal(expected, WireDecoder.sourceEntry body)
