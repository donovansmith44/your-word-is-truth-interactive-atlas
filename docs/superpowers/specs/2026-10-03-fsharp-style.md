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

## Sources reference interfaces

The next exemplar owns one page, including the response presentation. A pending
request carries its ticket; a successful page holds a presentation prepared
once when that ticket completes. Retry is an event, not a renderer decision.
The private model can be created only by `init` and advanced by `update`.
The public state is its complete read-only projection for rendering and laws.
No empty page or previous-response slot is kept: this page has no refresh path
from success, so those states cannot be reached by its events.

```fsharp
type HttpRejection =
    { Status: System.Net.HttpStatusCode
      Reason: string option
      Body: Result<ErrorBody, WireFailure> }

type ReadFailure =
    | Unreachable of diagnostic: string
    | Cancelled of diagnostic: string
    | HttpRejected of HttpRejection
    | InvalidAnswer of WireFailure

type NonEmpty<'a> = private NonEmpty of head: 'a * tail: 'a list

type SourceCardPresentation = { Entry: SourceEntry; Visit: string option }
type SourceSectionPresentation =
    { Category: SourceCategory; Cards: SourceCardPresentation list }
type SourceSections =
    { Document: SourcesDocument; Sections: SourceSectionPresentation list }
type SourcesPresentation = private SourcesPresentation of SourceSections

type SourcePresentationFailure =
    | UnlistedCategories of NonEmpty<SourceEntry>
    | RepeatedCategories of NonEmpty<SourceCategory>

type SourcesState =
    | Loading of RequestId
    | Available of SourcesPresentation
    | RetryableFailure of ReadFailure
    | ReadRejected of ReadFailure
    | PresentationRejected of SourcePresentationFailure

type SourcesModel = private SourcesModel of SourcesState

type SourcesMessage =
    | Retry
    | Loaded of RequestId * Result<SourcesDocument, ReadFailure>

type SourcesEffect = Read of RequestId

SourcesPresenter.present : SourcesDocument -> Result<SourcesPresentation, SourcePresentationFailure>
SourcesPresentation.view : SourcesPresentation -> SourceSections
Sources.init : RequestId -> SourcesModel * SourcesEffect list
Sources.update : RequestId -> SourcesMessage -> SourcesModel -> SourcesModel * SourcesEffect list
Sources.state : SourcesModel -> SourcesState

Api.readContract : HttpClient -> CancellationToken -> Request<'a> -> Async<Result<'a, ReadFailure>>
SourcesRuntime.command :
    (Request<SourcesDocument> -> Async<Result<SourcesDocument, ReadFailure>>)
    -> SourcesEffect -> Cmd<SourcesMessage>
SourcesView.render : SourcesState -> (SourcesMessage -> unit) -> Node
```

Presentation preserves the whole served document. It either associates each
source with its served category once, in category/source order, or returns a
named failure containing the offending records. Duplicate category identifiers
are refused before missing-category validation; neither duplication nor silent
drop is a permitted presentation. A blank link has no Visit affordance, matching
the C# presentation policy, while its original wire value stays in the document.
No label, reference, date or source identity is inferred.

A network/interruption failure and HTTP 5xx may retry. HTTP refusals outside
5xx, decode faults and presentation faults may not. HTTP classification belongs
to the read failure policy and is exercised across the status range. The view
renders the stored state and does not group, decode, classify retry or present
responses. Existing non-Sources callers retain their compatibility result until
the parent migration applies this approved pattern everywhere.

Sources application integration changes the existing surface case to `Surface.Sources of SourcesModel`. The application maps only `SourcesEffect.Read of RequestId` into its existing `Effect.ReadSources of RequestId`, interprets it through `SourcesRuntime.command`, and passes the read-only `Sources.state` projection to `SourcesView.render`. The old generic `LoadState<SourcesDocument>` path and its grouping/card renderers are removed. No other surface changes in this exemplar.

## Current disposition

Ops owner directives 570f5b9 and 7572dae supersede the earlier style-first order:
the domain, data structures and algebras must be signed off first. The Sources
checkpoint is preserved, not submitted as completed style approval. Its 30
properties and 108 regression tests pass; the new FCS gate covers the named
production exemplars and self-tests module/member order and forbidden syntax.
Comprehensive lexical/local shadowing, arbitrary fake recursion and whole-client
closure remain open. No additional view or exemplar code follows before the
domain package.
