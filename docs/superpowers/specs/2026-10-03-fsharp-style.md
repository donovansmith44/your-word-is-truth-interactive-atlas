# F# reference style

Status: exemplar in progress; no approval or parity claim.

The generator door is `ContractDocument -> Result<GeneratedSource, ContractError>`.
`ContractDocument` names the input document, `ContractModel` contains schemas and
GET operations, and `GeneratedSource` names emitted F# source. Failure is a closed
union with a `DocumentPath`; YAML diagnostics occur only at the YAML adapter.
Names, primitive types, optionality, sequence/map shapes, discriminator cases,
and parameter locations are values. The emitter consumes this model and never
reads YAML. The facade is `read >> Result.map emit`.

The first reference files are `ContractModel.fs`, `YamlAdapter.fs`,
`ContractReader.fs`, `ContractEmitter.fs`, and `Generator.fs` in
`client-fsharp.ContractGenerator/`. Types precede functions. Public doors appear
first within their module; local pure helpers remain within that door. The reader
uses `module rec` for the actual `schema ↔ shape` recursion; the emitter uses it
for recursive type rendering. Those helpers follow their callers. Unrelated
modules do not use `module rec` to simulate forward declarations.

Library adapters alone may use library exceptions and mutable library objects.
Domain functions compose `Result`, immutable lists and exhaustive matches. No
domain function raises, catches, mutates, downcasts, or indexes an unproven list.
Schema-reader failures carry exact paths, and tests compare whole values.

Generated laws will vary schema shapes, required/nullable fields, primitive
identities and malformed documents. Each closed reader error must have a
reachable input. Current-contract generation and compiled wire round trips are
separate checks. Red evidence will be retained in the task report.

The remaining requested references are one typed JSON record decoder, one
Sources model/message/update/Cmd/view over presentation, and bounded paging
laws. They are required before this exemplar task enters review. No broad
rewrite or new feature work proceeds before Claude reviews this set and Donovan
signs off its style.

## Current interfaces

These are the current typed generator values; the opaque wrappers expose only
`ofText`/`text` or checked name construction. The implementation files named
above are the executable reference, rather than a second implementation here.

```fsharp
type ContractDocument = private ContractDocument of string

type GeneratedSource = private GeneratedSource of string

type SchemaName = private SchemaName of string

type PathSegment = Member of string | Item of int

type DocumentPath = private DocumentPath of PathSegment list

type YamlDiagnostic = { Line: int; Column: int; Message: string }

type ContractError =
    | InvalidYaml of YamlDiagnostic
    | NoSchemas of DocumentPath
    | ExpectedScalar of DocumentPath
    | ExpectedSequence of DocumentPath
    | ExpectedMapping of DocumentPath
    | MissingField of DocumentPath * string
    | UnsupportedType of DocumentPath * string list
    | UnsupportedUnion of DocumentPath
    | InvalidReference of DocumentPath * string
    | InvalidName of DocumentPath * string
    | EmptyVocabulary of DocumentPath
    | UnsupportedParameterLocation of DocumentPath * string

type YamlValue = Scalar of string | Sequence of YamlValue list | Mapping of (YamlValue * YamlValue) list

type Primitive = Text | Int32 | Int64 | Number | Boolean

type TypeShape = Primitive of Primitive | Named of SchemaName | Optional of TypeShape | Many of TypeShape | StringMap of TypeShape

type Field = { WireName: string; Name: SchemaName; Shape: TypeShape }

type EnumCase = { WireName: string; Name: SchemaName }

type TaggedCase = { Tag: EnumCase; Payload: SchemaName }

type SchemaDefinition =
    | Identity of Primitive
    | Alias of TypeShape
    | Record of Field list
    | Enumeration of EnumCase list
    | Tagged of discriminator: string * cases: TaggedCase list

type Schema = { Name: SchemaName; Definition: SchemaDefinition }

type ParameterLocation = Path | Query

type Parameter = { WireName: string; Shape: TypeShape; Required: bool; Location: ParameterLocation }

type Operation = { Name: SchemaName; Path: string; Response: TypeShape; Parameters: Parameter list }

type ContractModel = { Schemas: Schema list; Operations: Operation list }


Generator.generate : ContractDocument -> Result<GeneratedSource, ContractError>
ContractReader.read : ContractDocument -> Result<ContractModel, ContractError>
ContractEmitter.emit : ContractModel -> GeneratedSource
SchemaName.create : DocumentPath -> string -> Result<SchemaName, ContractError>
ContractError.render : ContractError -> string
```

The record boundary shares a single System.Text.Json adapter with the existing
compatibility door. It constructs the documented FSharp converter directly,
registering `IdentityConverters.all` ahead of it. There is no positional cast.
Generated nominal identities are reference DUs with private constructors;
their thin JSON converters explicitly handle null. The old generic JSON API
still renders typed failures into the old `Failure.Contract` text payload;
whole-client failure migration is not claimed by this small reference.

```fsharp
type JsonPath = private JsonPath of string

type JsonDiagnostic = { Path: JsonPath; Line: int64 option; Byte: int64 option; Message: string }

type WireFailure = InvalidSyntax of JsonDiagnostic | InvalidValue of JsonDiagnostic | NullPayload


WireDecoder.sourceEntry : string -> Result<SourceEntry, WireFailure>
WireFailure.render : WireFailure -> string
```

Paths in JSON diagnostics are exactly those supplied by the library. Its F#
record converter currently reports a bad field value at `$`; this prototype
neither infers a finer path from exception prose nor claims one.

Remaining before style review: the Sources end-to-end reference, exhaustive
transition and paging laws, and comprehensive newspaper/source closure. The
reader and emitter now place their public entry first, then private helpers
below their first caller; their recursion is actual schema/type recursion.
Local/member ordering elsewhere remains to be demonstrated. The current vocabulary generator and other
legacy F# modules have not been repainted.
