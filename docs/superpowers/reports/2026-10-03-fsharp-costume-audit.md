> Owner, 2026-10-03: "where is the jsonpath behavior defined? How do I know it really only supports json?" … "it's not just there. I'm sure that's not the only place stuff went wrong in this style and we can't have nonsense like this."

# F# costume-type audit: `lane/codex/CX-FSHARP-STYLE` @ 35f6797

Scope: every `.fs` file under `client-fsharp*/` at `origin/lane/codex/CX-FSHARP-STYLE` 35f6797. That covers the generator (9 files), Core (17), the client shell and Styles (8), and the tests (29). It also covers the generated contract `obj/Contract/Wire.g.fs` (997 lines) and `Vocabulary.g.fs` (57 lines). I regenerated both from `contracts/openapi.yaml` with the branch's own generator. Generated line numbers below are `Wire.g.fs:<n>` in that output. The read was read-only, and I built nothing beyond the generator.

**The defect class is a type wearing a costume.** A named type (or a field whose name claims a meaning) that guarantees nothing beyond the primitive it carries. Its name promises an invariant that nothing checks, so readers and callers trust a guarantee that doesn't exist.

**Counts.** 89 instances across 42 files: 27 production or generated files and 15 test files.

| # | Pattern | Count |
|---|---|---|
| 1 | Costume wrappers | 9 |
| 2 | Open doors | 8 |
| 3 | Stringly fields | 16 |
| 4 | Raw numbers with meaning | 8 |
| 5 | Option or bool standing in for a case | 8 |
| 6 | Library leakage outside the one adapter | 19 (11 production, 8 test files) |
| 7 | Tests that prove pass-through | 9 |
| 8 | Generated types inherit the costume | 12 |

Some shapes are already right, and the fixes should copy them. `RequestId` (Loading.fs:10-14) is private and is produced only by `initial` and `next`, so it really is monotonic. `NonEmpty` (NonEmpty.fs:3-7) guarantees non-emptiness by its structure. **`DocumentPath` (ContractModel.fs:9-11, 66-75) is already the correct JsonPath shape**: a private list of structured `Member | Item` steps, with `root` as the empty list and rendering in one place. The JSON side reinvented the path as a string beside it. The generated `Element`, `PositionRef` and `TextRef` are real tagged unions, which shows the generator can emit cases instead of option bags.

## 1. Costume wrappers (9)

| file:line | Claims | Guarantees | Fix |
|---|---|---|---|
| Core/WireFailure.fs:3, 10 `JsonPath` | A JSONPath into the served JSON answer | Any string. `ofLibraryPath "hello"` compiles. Its only producer copies `JsonException.Path` text (JsonAdapter.fs:36). | A private list of `Property of WireField \| Index of ArrayIndex`, built by the decoder as it descends, never parsed from library text (shape below). |
| ContractGenerator/ContractModel.fs:3, 59 `ContractDocument` | An OpenAPI contract document | Any text. `ofText ""` is a "contract document". | Delete it. The file adapter returns raw text and the OpenAPI reader returns the typed document (see "Hand-rolled where a library exists"). |
| ContractGenerator/ContractModel.fs:5, 63 `GeneratedSource` | F# source that is the contract | Any text, and no test compiles it | Make it private, produced only by `ContractEmitter.emit`. Add a law that parses the output with FCS and gets zero diagnostics. |
| Wire.g.fs:328-329 `EraId` (emitted by ContractEmitter.fs:24, 41) | The id of an Era node | Any JSON string, including `""`. It has no door and no accessor, so the client can neither build nor read one. | Generate a validating door from the contract's id grammar, or (until the contract states one) a non-empty `NodeId<Era>` built on one shared id type. |
| Wire.g.fs:462-463 `NarrativeId` | The id of a Narrative node | Same as EraId. Also, `NarrativePosition.NarrativeId: string` (Wire.g.fs:469) carries the same concept untyped. | Same as EraId, and use the same type at every site of the concept. |
| Wire.g.fs:589-590 `PolityId` | The id of a Polity node | Same as EraId | Same as EraId |
| Core/Exploration.fs:19-22, 30-35 `Resolved` | An element resolved on artifact root `root` | Nodes are checked against `root` (line 33). **Edges are stamped with whatever root the caller passes (line 35), unchecked.** `root` is a bare string. | Make root an `ArtifactRoot` type. Either have edges carry a root the server states, or split `Resolved` so an edge does not claim a root it was never checked against. |
| Core/SourcesPresentation.fs:7-8, Styles/SourceSectionsView.fs:8 `SourcesPresentation` | Sections that passed the category checks | The wrapper is private, but its payload `SourceSections` is a public record. The view takes `SourceSections`, so anyone can render unchecked sections (the test fixture does: SourcesFixtures.fs:28). | Make `SourceSections` private, or have the view accept only `SourcesPresentation`. |
| ContractGenerator/ContractModel.fs:7, 78-93 `SchemaName` | A unique F# type name for a schema | It is a non-empty identifier (good), but uniqueness is not enforced. `"cited-by"` and `"cited_by"` both become `CitedBy`, and **ContractReader.fs:28 `List.distinctBy` silently drops the second schema.** | Add `DuplicateName of DocumentPath * DocumentPath` to `ContractError`, raised when two wire names map to one `SchemaName`. |

## 2. Open doors (8)

| file:line | Claims | Guarantees | Fix |
|---|---|---|---|
| Core/WireFailure.fs:10 `JsonPath.ofLibraryPath` | Turns a library path into a JsonPath | `string -> JsonPath`, total | Delete it. Paths come only from `JsonPath.root \|> property \|> index`. |
| ContractModel.fs:59 `ContractDocument.ofText` | Text becomes a contract | Total | Delete it, together with the type (row 1.2). |
| ContractModel.fs:63 `GeneratedSource.ofText` | Text becomes generated source | Total. Tests use it to build expected values (pattern 7). | Make it private to the emitter. |
| Core/Exploration.fs:59-60 `Trail.beginAt` / `Trail.follow` | A trail of steps | `resume` checks one root (line 79), but the public `follow` appends any step on any root. The invariant is enforced on one door and bypassed on the other. | `follow : Step -> Trail -> Result<Trail, TrailFailure>` returns `RootChanged`, or `Trail` is only extended inside `Explore`. |
| Wire.g.fs:837, 844, 851, 858, 872, 888, 895, 916, 930, 937, 965, 992 generated `Reads.*` | A typed request for a node, event, chapter, … | Every identity parameter is `string`, for example `nodeRecord (id: string)` and `elements (ids: string list)`. Any text becomes a request. | Make the parameters the identity types (pattern 8). For `elements`, use `NonEmpty<NodeOrEdgeId>` (the contract says `minItems: 1`, openapi.yaml:199). |
| Core/Model.fs:30-34 `Routes.parse` (concord) | A Book of Concord reference | Any query text becomes `Route.Concord(Some reference)` | Parse into `ConcordRef` (it already exists in the contract: part, article, paragraph) or refuse to `NotFound`. |
| ContractEmitter.fs:41 identity converter template | Wire value becomes an identity | `Deserialize<string>` then wraps it, with no check | The generated decoder calls the identity's validating door and maps its failure to `WireFailure.InvalidIdentity`. |
| client-fsharp/Main.fs:13 `Unchecked.defaultof<HttpClient>` | An injected HTTP client | `null` until Blazor injects one | Keep it, because it is framework injection, but confine it to the shell. List it as the one sanctioned `Unchecked` in the source law. |

## 3. Stringly fields (16)

| file:line | Claims | Guarantees | Fix |
|---|---|---|---|
| Core/WireFailure.fs:5 `JsonDiagnostic.Message` | Why decoding failed | Library English, which tests pin verbatim (WireLaws.fs:25, 39) | A closed union of failure kinds (shape below). |
| ContractModel.fs:13 `YamlDiagnostic.Message`, YamlAdapter.fs:32 | Why the YAML failed | Library English. **YamlAdapter.fs:32 also fabricates a diagnostic: `Line = 1; Column = 1; Message = "expected one document"`.** | `InvalidYaml of TextPosition` plus a closed reason. Multiple documents become their own `ContractError` case with no invented position. |
| Core/Loading.fs:5-8 `Failure = Transport of string \| Contract of string \| ArtifactMoved of string * string` | The app's failure vocabulary | Free text, built at 15 sites: Exploration.fs:36, 81, 118; Graph.fs:26, 27, 28, 35, 42; Model.fs:231, 234, 270; Presentation.fs:33; Json.fs:7; Api.fs:11, 12 | A closed union, for example `ElementMissing of NodeId`, `CardinalityMismatch of requested: int * answered: int`, `WrongCorpus of Corpus * Corpus`, `NoOpening of ReadingLocation`, `TextUnitWithoutText of NodeId`, `ArtifactMoved of ArtifactRoot * ArtifactRoot`, `Read of ReadFailure`. |
| Core/Api.fs:11-12, Core/Json.fs:7 | Keeps the read failure | **Erases** the typed `ReadFailure`/`WireFailure` into strings via `WireFailure.render` and `ReadFailure.describe` | Carry `ReadFailure` inside `Failure`. Render only in the view. |
| Core/ReadFailure.fs:10-11 `Unreachable of diagnostic: string`, `Cancelled of diagnostic: string` | Why the transport failed | The exception's `.Message` (HttpAdapter.fs:21-22) | Make them nullary, `Unreachable \| Cancelled`. The message is a log concern, not a domain value. |
| Core/ReadFailure.fs:6 `HttpRejection.Reason: string option` | The HTTP reason phrase | Server free text | Drop it. `Status` plus the typed `ErrorBody.Code` carry the meaning. |
| ContractGenerator/FileAdapter.fs:8 `FileUnavailable of FileOperation * path: string * diagnostic: string` | A file failure | Path text plus exception text | `FileUnavailable of FileOperation * ContractPath * FileFault`, where `FileFault = NotFound \| AccessDenied \| IoFailure`. |
| ContractGenerator/Vocabulary.fs:7, 22, 34 `generate : string -> Result<string, string>` | Generates the edge-kind dual table | Anything to anything, with string errors. The owner already called out this exact shape in the parent's Generator.fs. | `VocabularySource -> Result<GeneratedSource, ContractError>`. It should be a decoder over a typed vocabulary record. |
| ContractModel.fs:21-27 `ContractError` payloads `MissingField of _ * string`, `UnsupportedType of _ * string list`, `InvalidReference of _ * string`, `UnsupportedParameterLocation of _ * string` | Typed reader failures | The "kind" lists are closed JSON Schema vocabulary carried as text | Use `JsonSchemaType` and `ParameterLocation` unions. Keep the raw offending scalar only where it is truly unknown input (`Unrecognised of RawScalar`). |
| ContractModel.fs:35, 37, 46, 52, 54 `WireName: string` ×3, `discriminator: string`, `Operation.Path: string` | Wire spellings and a path template | Any text. `Path` has structure (`/api/node/{id}/edges`) but its parameters are matched by string `.Replace` (ContractEmitter.fs:74). | Use `WireName` (non-empty, private). Make the path template a list of `Literal \| Parameter of ParameterName` segments, and check that every path parameter appears in the template. |
| ContractModel.fs:29 `Scalar of string`, YamlAdapter.fs:11, ContractReader.fs:129, 133 | A YAML scalar | YAML null becomes `""` (YamlAdapter.fs:11), and `required: True` or `yes` are silently `false` (`= "true"`, ContractReader.fs:133) | Replaced wholesale by the OpenAPI library's typed `Required` and `Type` (see "Hand-rolled where a library exists"). |
| Core/Model.fs:15, 71, 253, 278 `Concord of string option`, `Reference: string option`, opening `(string * …)` | A Concord reference | Any text | `ConcordRef` from the contract. |
| Exploration.fs:21-22, Loading.fs:8, Graph.fs:46, and generated `Version: string` (pattern 8) | An artifact root | Any text | `ArtifactRoot` (private, from the wire door only). |
| Core/SourcesPresentation.fs:5, 37; Styles/SourceCardView.fs:23 `Visit: string option` | A visitable link | Any non-blank text goes into `href`, **including `javascript:` URLs** | `Visit: HttpUrl option`, where the door is `Uri.TryCreate` with an absolute http or https scheme. |
| client-fsharp/View.fs:347-349 `"popover-field-" + caption` | A test identifier | A display caption reused as an identifier | Give `FieldName` a `testId` beside `caption`. |
| Core/Model.fs:24, 42; client-fsharp/Runtime.fs:14 | The wire spelling of `BookId` and `Corpus` | Obtained by a JSON round trip through the serializer: `Json.decode<BookId>(Json.encode book)`, `JsonSerializer.Deserialize<string>(Json.encode corpus)` | Have the emitter generate total `wire : T -> WireName` and `ofWire : WireName -> Result<T, UnknownCase>` per enumeration. |

## 4. Raw numbers with meaning (8)

| file:line | Claims | Guarantees | Fix |
|---|---|---|---|
| Core/WireFailure.fs:5 `Line: int64 option; Byte: int64 option` | A source position | Zero-based (WireLaws.fs:24 expects `Line = Some 0L`). Negative values are representable. | `TextPosition = { Line: LineNumber; Column: ByteColumn }`. Both are private and one-based, normalised once in the adapter. |
| ContractModel.fs:13 `YamlDiagnostic.Line: int; Column: int` | A source position | **One-based** (YamlDotNet `Mark`). The same word "Line" means a different origin and type than in JsonDiagnostic. | The same `TextPosition` as above. |
| ContractModel.fs:9, 69 `Item of int`, `itemAt : int -> …` | A sequence index | Negative is representable | `Item of ArrayIndex` (≥ 0). |
| Core/Graph.fs:45 `Cursor: int option`; Wire.g.fs:272-273, 311-312 `Next/Previous: int option` | A page cursor | Any int, interchangeable with a count | `PageCursor` (private, from the wire only). |
| Core/AnchoredText.fs:9-12 | Text offsets | **Unicode scalar offsets and UTF-16 offsets are both `int`.** `utf16` silently clamps out-of-range spans into range (line 10), so a bad span from the server becomes a plausible wrong highlight. | Use `ScalarOffset` and `Utf16Offset`. The conversion returns `Result<_, SpanOutOfText>`, and the span type carries `Start <= End`. |
| Core/Model.fs:7, 25 `ReadingLocation.Chapter: int` | A chapter number (≥ 1) | Checked only in `Routes.parse`, while the record is public, so `{ Chapter = -5 }` compiles | `ChapterNumber` (private, ≥ 1). Make `ReadingLocation` private, or build it from validated parts. |
| Core/Model.fs:281 `concordPageSize = 20`; Wire.g.fs:980 `n: int option` | A page size | Any int | `PageSize` with a bounded door (1..max). |
| Core/ReadFailure.fs:6, 20-22 `Status: HttpStatusCode` | An HTTP refusal | Any int can be cast to the enum (`enum<HttpStatusCode>(…)`, HttpLaws.fs:22). `HttpRejected` with status 200 is representable, and retry is decided by int arithmetic. | `Refusal = ClientError of ClientStatus \| ServerError of ServerStatus`, classified once in HttpAdapter. `canRetry` becomes a match. |

## 5. Option or bool standing in for a case (8)

| file:line | Claims | Guarantees | Fix |
|---|---|---|---|
| Core/AnchoredText.fs:5, 23-27; View.fs:207 `IsWordsOfChrist: bool` in `TextPiece list list` | Runs of red or plain text | Each piece repeats the flag, and the view trusts `first.IsWordsOfChrist` of a list that could mix flags | `Run = Red of NonEmpty<Piece> \| Plain of NonEmpty<Piece>`. |
| ContractModel.fs:52 `Parameter.Required: bool` | Optionality | Duplicates `TypeShape.Optional`, so the two can disagree | Use one representation. Parameter shape becomes `Optional _` or not. |
| ContractReader.fs:95-104 `operation … option` | No operation | `None` means "not a GET" or "no 200 JSON body". Both are silently skipped, so a typo in the contract drops an endpoint. | Return `Unsupported of DocumentPath * reason` or an explicit `Skipped` case reported by the generator. |
| Core/Sources.fs:9-10, 34-35 `RetryableFailure of ReadFailure \| ReadRejected of ReadFailure` | Retryable vs terminal | Both cases take the same payload, so `RetryableFailure(InvalidAnswer …)` is representable. Classification is a bool (`canRetry`). | Split the payload types: `RetryableFailure of TransientFailure \| ReadRejected of TerminalFailure`. |
| Core/Graph.fs:46 `Root: string option` | The pinned root | `None` means "first page, root not yet known" | `Reading = FirstPage of … \| LaterPage of ArtifactRoot * PageCursor * …`. |
| Core/Model.fs:15, 278 `Concord of string option`, `Bible of ReadingLocation option` | Where to open | `None` means "the default opening" | `Opening = DefaultOpening \| At of ConcordRef` (and the same for the Bible). |
| Wire.g.fs:498-516 generated `NodeRecord`; Core/Presentation.fs:30-33 | A node of a kind | `Kind` plus 9 independent detail options. `Kind = Place` with `Polity = Some _` is representable, and `TextUnit` without `Text` fails only at runtime (Presentation.fs:33). | Have the contract and generator emit a tagged `NodeRecord` by kind, as they already do for `Element`. |
| Core/JsonAdapter.fs:36 `if isNull error.Path then "$"` | The failure's location | When the library doesn't know the location, **the code claims the root** | `TextPosition` for syntax failures. A path only where the decoder actually knows it. |

## 6. Library leakage (19)

Adapters that own a library: HttpAdapter (HttpClient), JsonAdapter (System.Text.Json), YamlAdapter (YamlDotNet), FileAdapter (System.IO). Their `try`s are sanctioned. Everything below is outside them, or an adapter that leaks.

| file:line | Leak | Fix |
|---|---|---|
| ContractGenerator/Vocabulary.fs:4, 8-9, 34 | System.Text.Json in a non-adapter module, and a **catch-all `with error -> Error error.Message`** that also swallows the `NullReferenceException` from a missing `GetString()` | Decode the vocabulary through the one JSON door into a typed record. |
| ContractGenerator/Program.fs:16-17 | `File.ReadAllText` and `File.WriteAllText` bypass FileAdapter, so exceptions are unhandled | Route them through FileAdapter. |
| ContractEmitter.fs:41 → Wire.g.fs (3 converters) | Emits `raise (JsonException …)` and `JsonConverter` subclasses into the generated contract | Generated decoders return `Result` (see "Hand-rolled where a library exists"). |
| ContractEmitter.fs:55 → Wire.g.fs `RequestEncoding` | `JsonDocument` and `JsonSerializer` round trips inside the generated contract to spell query values | Generated `wire` functions per enumeration and identity. |
| Core/Model.fs:4, 24, 42 | System.Text.Json in the domain model (route parsing) | Generated `ofWire` and `wire`. |
| client-fsharp/Runtime.fs:4, 14 | System.Text.Json in the command interpreter | Generated `Corpus.wire`. |
| Core/ReadFailure.fs:3, 6 | `System.Net.HttpStatusCode` in the domain failure | `Refusal` union (row 4.8). |
| Core/Graph.fs:3-4, 8, 13; Core/Api.fs:4, 6 | `HttpClient` and `CancellationToken` thread through a Core domain module | `Graph.explorer` takes a `Read` function. Only HttpAdapter sees `HttpClient`. |
| client-fsharp/Main.fs:13 | `Unchecked.defaultof` | Sanctioned shell injection (row 2.8). |
| Core/JsonAdapter.fs:25-30 | Catches only `JsonException`. `NotSupportedException` and `InvalidOperationException` from the serializer escape the door. | Use the library decoders below, which return `Result` with no exception path. Until then, catch the documented set. |
| Core/HttpAdapter.fs:11, 20-22 | Catches two exception types. Others (for example an invalid URI) escape. GraphTests.fs:68 relies on `InvalidOperationException` propagating. | Catch the documented `GetAsync` set, and map to a closed `TransportFault`. |

Test files with `mutable`, `failwith`, partial access or `Unchecked` (count per file):

| File | Count and sites |
|---|---|
| client-fsharp.Tests/ViewTests.fs | 12 `mutable` (26, 36, 56, 68, 90, 109, 138, 148, 160, 172, 184, 195) |
| client-fsharp.Tests/ModelTests.fs | 4 `failwith` (262, 273, 291, 292) |
| client-fsharp.Tests/ContractShapeTests.fs | 4: `failwith` 88; `Array.exactlyOne` 62, 98; `Array.head` 86 |
| client-fsharp.Tests/GraphTests.fs | 2 `mutable` (59, 60) |
| client-fsharp.Tests/ExplorationTests.fs | 2: `mutable` 54; `failwithf` 91 |
| client-fsharp.Tests/PresentationTests.fs | 2 `Option.get` (33, 56) |
| client-fsharp.Tests/StyleGeneration/SourcesDom.fs | 2 `Unchecked.defaultof` (22, 23; `nameof` only, harmless) |
| client-fsharp.Tests/SourceOrderTests.fs | 1 `failwith` (47) |

The exemplar gate `SourceLaws.fs:36-43` checks only 15 hand-picked files. It does not see Vocabulary.fs, generator Program.fs, Model.fs, Runtime.fs, Graph.fs, Exploration.fs, View.fs or any adapter, so none of the leaks above can fail it.

## 7. Tests that prove pass-through (9)

| file:line | What it proves | Why it can't catch an invalid value | Fix |
|---|---|---|---|
| Tests/StyleGeneration/WireLaws.fs:24-25, 38-39 | The diagnostic equals `ofLibraryPath "$"` plus the library's English sentence | The expected value is built with the same open door, and the message pins library wording | Expect a structured `WireFailure`, for example `WrongKind(JsonPath.root \|> property WireField.Title, JsonKind.String, JsonKind.Number)`. |
| Tests/StyleGeneration/ReaderLaws.fs:19, 23, 35-38, 83 (and :73) | The reader's names equal `SchemaName.create` of the same wire text | It uses the code under test to build the expectation. :73 pins YamlDotNet's English. | Write expected names as literals (`"Entry"`), and add a collision case. |
| Tests/GeneratorTests.fs:15, 36, 50 | Emitted text equals an `ofText` string | String equality. The generated source is never compiled. | Add an FCS parse/type-check law over the emitted source. |
| Tests/StyleGeneration/IdentityLaws.fs:11-29 | **Any** `NonNull<string>` (including `""`) round-trips into an identity | The law requires the costume. A validating identity would fail it. | Split the law: valid inputs round-trip, and generated invalid inputs return `Error InvalidIdentity`. |
| Tests/ContractShapeTests.fs:47-70 | Same for EraId, NarrativeId and PolityId over arbitrary text | Only `null`, `{}`, `[]` and the wrong primitive kind count as invalid | The same split, per identity. |
| Tests/StyleGeneration/WireLaws.fs:10-16 | `SourceEntry` round-trips arbitrary text into `Id`, `Category` and `Link` | A link of `"x:link"` is accepted as a link | Add invalid-link and blank-id cases. |
| Tests/StyleGeneration/SourcesFixtures.fs:15-20; SourcesLaws.fs:11-13 | `Visit` equals the input link verbatim | No non-http, relative or `javascript:` case | Add generated bad-scheme links that expect `Visit = None` or a typed refusal. |
| Tests/ExplorationTests.fs:39, 64, 69; GraphTests.fs:34, 40, 46, 56; PresentationTests.fs:27; TransportTests.fs:39, 46; ModelTests.fs:34, 105 | Failures equal `Contract "<English sentence>"` | They detect only wording changes, never the wrong kind of failure | Assert closed failure cases (row 3.3). |
| Tests/ModelTests.fs:82-85 | Any `NonNull<string>` Concord reference round-trips through the URL | The law requires `Route.Concord` to accept anything | Use a generated `ConcordRef`, plus refusal of malformed `ref=`. |

## 8. Generated contract types (12)

The generator turns a scalar schema into a named identity only when the schema is a top-level `type: string` with nothing else. There are 3 (`EraId`, `NarrativeId`, `PolityId`). All three are costumes (rows 1.4-1.6). Every other identity, root, cursor and bound in the contract arrives as a bare primitive.

| Wire.g.fs / generator line | Claims | Guarantees | Fix |
|---|---|---|---|
| 328, 462, 589; ContractEmitter.fs:24, 41 | Named identities | Any string. No door, no accessor. | A validating door plus accessor, and invalid-input laws (pattern 7). |
| 37 `Id/Ref/Version: string` fields, e.g. NodeRef.Id 519, EdgeRef.Id 295, NodeRecord.Id 506, MissingElement.Id 441, SourceEntry.Id/Category 671-672, ContentsChild.Id/Ref 157/160, TextUnit.Ref 711, TextWindow.Next 715 | Node, edge, source and category identities | Bare `string`. A source's `Category` and a category's `Id` aren't even the same type nominally. | Contract-side identity schemas (the WIREID spec on `lane/claude/WIREID-spec` d64ece1), emitted as validated identities. |
| 469, 628 (vs 286, 210) | A narrative | `NarrativeId` in two places, `string` in two others | One type per concept. |
| ContractReader.fs:206-212 drops `minimum` (36 in openapi.yaml), `minItems` (2), `maxItems`; Point 560-561 | Non-negative counts and offsets; a 2-coordinate point; a non-empty id list | `int`, `float list`, `string list` | The generator emits bounded types or refuses the keyword: **no silently dropped schema keyword**. |
| ContractReader.fs:209 | `format: int64` → `Int64` | Any other format (e.g. `int16`, `double`) silently becomes `Int32` | Make the format a closed union, with `UnsupportedFormat` as an error. |
| 858 `contents (corpus: string)`, 937 `nodeEdges (id: string)`, … | Typed requests | String path parameters, while a `Corpus` enum exists | Make the contract's path parameter `$ref: Corpus`, and use typed identities for ids. |
| `Version: string` in Contents 152, EdgePage 274, ElementPage 313, NodeRecord 515, KretzmannChapter 403, TextWindow 717 | An artifact root | Any text | `ArtifactRoot`. |
| 498-516 `NodeRecord` | A node of a kind | Kind plus 9 options (row 5.7) | Tagged by kind. |
| 719-724 `TimeRange`, 768-772 `Year` | A span of years | `From` and `To` unordered. `Label` is free text that duplicates `Value`. | `YearSpan` with `From <= To`. The label is display text derived in one place, or explicitly server-owned `DisplayText`. |
| 551-552, 607-608, 658-659 `Lat/Lon: float`; 445, 625, 645 `Color: string` | Coordinates and colours | Unbounded float, any text | `Latitude` (-90..90), `Longitude` (-180..180), `ColorToken` from a closed palette. |
| 733, 396 `IsContinuation: bool`; 534 `Eternal: bool`; 537 `Gender: string option` | Heading kind, lifespan kind, gender | Bool flags and free text over closed vocabularies | Use `HeadingRole = Opening \| Continuation`, a lifespan union, and a `Gender` enum in the contract. |
| 6-12 `Anchor.Start/End: int`, 763-767 `WordsOfChristSpan` | Scalar-offset spans | `int`, with no `Start <= End` | `ScalarSpan`, validated at the wire door (row 4.5). |

## The corrected shapes

### JsonPath: structured steps, `$` is the empty list, rendered in one place

```fsharp
/// Generated from openapi.yaml: every property name the contract defines, with its wire spelling.
[<RequireQualifiedAccess>]
type WireField = Id | Category | Title | License | LicensesRowKey | Link | WhatItIs | WhatWeBuilt (* … *)

type ArrayIndex = private ArrayIndex of int          // invariant: >= 0

type PathStep =
    | Property of WireField                          // only names the contract has
    | Index of ArrayIndex

type JsonPath = private JsonPath of PathStep list    // [] is "$"

module ArrayIndex =
    let ofPosition (position: int) : Result<ArrayIndex, NegativeIndex> =
        if position >= 0 then Ok(ArrayIndex position) else Error(NegativeIndex position)

module JsonPath =
    let root = JsonPath []
    let property field (JsonPath steps) = JsonPath(steps @ [Property field])
    let index position (JsonPath steps) = JsonPath(steps @ [Index position])
    /// The one place "$", "." and "[n]" exist.
    let render (JsonPath steps) =
        steps |> List.fold (fun text step ->
            match step with
            | Property field -> text + "." + WireField.wire field
            | Index (ArrayIndex position) -> text + $"[{position}]") "$"
```

The decoder builds the path as it descends: each generated field decoder adds `property WireField.X`, and each list decoder adds `index`. There is no `string -> JsonPath`, so `JsonPath.ofLibraryPath "hello"` cannot be written. "Only supports JSON" now means something concrete: a path can name only fields the contract defines. This is the shape `DocumentPath` already has (ContractModel.fs:9-11), so the two should share one `Path<'step>`.

### JsonDiagnostic becomes a closed union of failure kinds

```fsharp
type LineNumber = private LineNumber of int64        // >= 1, normalised from the library once
type ByteColumn = private ByteColumn of int64        // >= 1
type TextPosition = { Line: LineNumber; Column: ByteColumn }

[<RequireQualifiedAccess>]
type JsonKind = Object | Array | String | Number | Boolean | Null

type WireFailure =
    | NotJson of TextPosition                        // the parser's position; the library exposes no typed reason, so we claim none
    | NullAnswer
    | MissingField of JsonPath * WireField
    | WrongKind of JsonPath * expected: JsonKind * actual: JsonKind
    | UnknownCase of JsonPath * Vocabulary           // value outside a closed enum (BookId, EdgeKind, …)
    | UnknownTag of JsonPath * Discriminator         // tagged union with an unlisted tag
    | OutOfBounds of JsonPath * Bound                // minimum / minItems / maxItems from the contract
    | InvalidIdentity of JsonPath * IdentityFailure  // an identity's own door refused the value

module WireFailure =
    let render failure = (* the one place that turns a failure into prose, used only by the view and logs *)
```

There is no `Message: string` and no `Line: int64 option`. A syntax failure has a position and no path. A value failure has a path and no invented position. Neither case can be empty or fabricated.

## Closure

**Category: costume types.** These are nominal types and named fields whose guarantee is no stronger than the primitive underneath. They include wrappers with total doors, string or int fields over structured or closed vocabularies, failures carrying library prose, and absence encoded as a fabricated value (`"$"`, `""`, `Line = 1`).

**Law: no named type may guarantee less than its name claims.** The following rules make that mechanically enforced rather than a matter of review:

1. **Source law: private wrappers with one validating door.** This is an FCS gate over *every* `.fs` under `client-fsharp*/` and the generated `Wire.g.fs`, not a hand-picked list. Every single-case union or record whose payload is a primitive (`string`, `int`, `int64`, `float`, `bool`) must:
   - be `private`;
   - have its module expose a door typed `primitive -> Result<T, F>`, where `F` is a closed union with no `string` payload;
   - have no other public function returning `T` from a primitive.

   `ofX`, `create` or `from` with a total signature fails the build. The single sanctioned exception, `Main.fs` injection, is named in the gate.
2. **Source law: no bare primitives in domain records or failures.** In Core and the generated contract, a record field or union case typed bare `string` or numeric fails the gate, unless its type is `DisplayText`. `DisplayText` explicitly means server prose that is shown and never inspected. Any type whose name ends in `Failure`, `Error` or `Diagnostic` may carry no `string` at all. Library text never crosses an adapter.
3. **Enumerating property law: every door refuses.** Reflection enumerates every door the source law finds. Each door must have a registered FsCheck generator of *invalid* inputs, and the test fails if a type has none. The law asserts `Error` for invalid inputs and a round trip for valid ones. A new wrapper without an invalid case cannot pass.
4. **Test law: expectations aren't built with the door under test.** In tests, the expected side of an assertion may not call the door of the type under test. Paths are built structurally (`JsonPath.root |> property …`), and failures are asserted as cases, never as English.
5. **Generator law: no silent keyword.** Every JSON Schema keyword present in `openapi.yaml` is either emitted as a type constraint or rejected with `ContractError.UnsupportedKeyword of DocumentPath * Keyword`. A test walks the contract and asserts that every keyword was consumed. This closes the 36 dropped `minimum`s and the 2 dropped `minItems`.
6. **Domain wrappers carry structure, not text.** When a value has parts (a path, a reference, a span, a template), its type is those parts. Its text form is produced by one `render` per type.

These laws belong in the domain spec (`docs/superpowers/specs/2026-10-03-fsharp-domain.md`), next to the domain, data structures and algebras the owner signs off. They are part of the domain model, not a style afterthought.

## Hand-rolled where a library exists

> Owner, 2026-10-03: "I'm guessing there are fs libraries that have proper json handling and we shouldn't handroll all this for json, yaml and files, no?"

Yes. I checked licences and dates against the NuGet package metadata (nuspec and registration) and GitHub on 2026-10-03.

| Library | Version (published) | Licence | Targets / WASM fit | Maintenance |
|---|---|---|---|---|
| Thoth.Json.Core + Thoth.Json.System.Text.Json | 0.9.1 / 0.4.0 (2026-06-03) | MIT (LICENSE.txt in the package; GitHub `thoth-org/Thoth.Json` MIT) | netstandard2.0, pure F# over System.Text.Json. Runs in Blazor WebAssembly (Bolero), with no reflection-based serializer. | Active (repo pushed 2026-09-18, not archived). Pre-1.0, so pin the version. |
| Thoth.Json.Net | 12.0.0 (2024-05-23) | MIT | netstandard2.0, Newtonsoft.Json | Legacy line. **Not recommended**: it adds Newtonsoft. |
| Microsoft.OpenApi + Microsoft.OpenApi.YamlReader | 3.10.2 (2026-08-20) | MIT | net8.0 / netstandard2.0. Used only in the build-time generator, so WASM doesn't matter. | Active (Microsoft). Reads OpenAPI 3.1 (our contract is `openapi: 3.1.0`) and 3.2. `Microsoft.OpenApi.Readers` (1.6.31) is the v1 line; v2+ folded reading into the core package, with YAML through `YamlReader` (SharpYaml). |
| FsToolkit.ErrorHandling | 5.2.0 (2026-02-27) | MIT | netstandard2.0, pure F#. WASM fine. | Active. |
| Argu (optional) | 6.2.5 (2024-12-07) | MIT | netstandard2.0. Generator CLI only. | Stable. |
| Bolero routing (already a dependency) | 0.25.65 | Apache-2.0 | It is the WASM framework | Active. |

What this replaces, place by place:

| file:line | Hand-rolled today | Library | What it becomes |
|---|---|---|---|
| Core/JsonAdapter.fs:8-38, Json.fs, WireDecoder.fs | FSharp.SystemTextJson reflection deserializer, with `JsonException`s caught into `Message: string` and a `"$"` path | Thoth.Json.Core + .System.Text.Json | **Generated decoders and encoders** (`Decoder<SourceEntry>` and so on) emitted from `openapi.yaml` by ContractEmitter. Each field decoder calls the domain door (`ArrayIndex.ofPosition`, identity doors, bound checks) with `Decode.andThen`, so validation happens *during* decoding rather than after. There is no exception path except `JsonDocument.Parse` syntax errors, and those are caught in one place → `NotJson of TextPosition`. |
| ContractEmitter.fs:34-49 (identity converters, `IdentityConverters.all`), Wire.g.fs converters | Hand-written `JsonConverter` subclasses that `raise` | Thoth codecs | Generated `Decode`/`Encode` per identity. The converter registry disappears. |
| ContractEmitter.fs:55 `RequestEncoding`, Model.fs:24, 42, Runtime.fs:14 | Spelling enums by a JSON round trip | None needed | The emitter generates `wire`/`ofWire` per enumeration (it already has every `WireName`). Thoth encoders reuse them. |
| ContractGenerator/YamlAdapter.fs:8-34, ContractReader.fs:1-253, ContractModel.fs:29 `YamlValue` | Walks raw YamlDotNet nodes by hand; all scalars are strings, `null` becomes `""`, `required = "true"` | Microsoft.OpenApi + YamlReader | `OpenApiDocument.Parse`/`Load` gives a typed `OpenApiDocument` plus `OpenApiDiagnostic`. ContractReader becomes a total mapping `OpenApiDocument -> Result<ContractModel, ContractError>` over typed `OpenApiSchema` (`Type` as `JsonSchemaType` flags including `Null`; `Required`, `Enum`, `Discriminator.Mapping`, `Minimum`, `MinItems`, `MaxItems`, `Pattern`, `Format`), and that mapping makes generator law 5 easy to keep. YamlAdapter, `YamlValue`, `fields`/`scalar`/`sequence`/`descendant` and the YamlDotNet dependency are deleted. |
| ContractGenerator/Vocabulary.fs:7-34 | `JsonDocument` walk with a catch-all | Thoth (the same decoders) | A `Decoder<GraphVocabulary>` for the vocabulary fixture, so errors are the same closed `WireFailure`. |
| ContractGenerator/Results.fs:3-10; ContractReader.fs:250-253 `traverse`; YamlAdapter.fs:15, 19-23 folds; Vocabulary.fs:26-30 fold; ReaderLaws.fs:21 | A minimal `ResultBuilder` (no `For`, `Combine` or `Zero`) and hand-written traverse folds | FsToolkit.ErrorHandling | `result { }`, `List.traverseResultM`, and `validation { }` / `List.traverseValidationA` where the generator should report **every** bad schema at once, not just the first. |
| ContractGenerator/FileAdapter.fs:12-26; Program.fs:7-19 | A thin `File.*` adapter (fine), bypassed by Program.fs:16-17; positional `argv` matching | System.IO stays the library; Argu is optional | Keep one thin `FileAdapter` (read and write → `Result<_, FileFailure>`, closed `FileFault`) and route Program.fs through it. Argu would replace the positional `match arguments with [| … |]` with a typed argument union. That is useful but not required. |
| Core/Model.fs:19-49 `Routes.parse`/`url`; Main.fs:23-27 custom `IRouter` | Hand-split path and query string | Bolero `Router.infer` with `[<EndPoint>]` | A typed `Route` union routed by Bolero. Segments that are domain values (`BookId`, `ConcordRef`) still go through their doors via a custom segment parser. |

**Recommended set:** Microsoft.OpenApi + Microsoft.OpenApi.YamlReader (generator), Thoth.Json.Core + Thoth.Json.System.Text.Json (wire; they replace FSharp.SystemTextJson and all generated converters), FsToolkit.ErrorHandling (both projects), and Bolero's own router. Drop YamlDotNet and FSharp.SystemTextJson. Record each licence in `client-fsharp/THIRD-PARTY.md`.

**What still has to be our own types.** A library replaces machinery, not meaning:

- **Thoth's errors are themselves partly costumed.** `DecoderError = string * ErrorReason`, where the path is a `string` and `BadPrimitive of string * …` carries expectations as text. So **the one JSON door maps Thoth's errors into our closed `WireFailure`**. The path comes from our own generated field decoders as they descend, not from parsing Thoth's path string. Parsing that string back would rebuild today's costume.
- **`OpenApiDiagnostic.Errors` is a list of `OpenApiError` with string `Pointer`/`Message`.** The one OpenAPI door maps it to `ContractError.InvalidDocument of NonEmpty<DocumentPath>`, and every unsupported construct to a named `ContractError` case.
- **The domain types are ours:** identities, `ArtifactRoot`, `PageCursor`, `ChapterNumber`, `ConcordRef`, spans, `YearSpan`, `HttpUrl`, `Failure`, `ReadFailure`, `WireFailure`, `ContractError`, `JsonPath`/`DocumentPath`, `TextPosition`. So are all of the closure laws above. One adapter per library is the only module that may name that library's types or catch its exceptions.

Context: on the parent `lane/codex/CX-FSHARP`, `client-fsharp.ContractGenerator/Generator.fs` still has 14 `invalidOp` calls. On this STYLE branch the reader returns the closed `ContractError` instead. That is the right direction, but it is still hand-walked YAML.
