# CONTRACT-1b (client) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Blazor client compiles against records generated at build from `contracts/openapi.yaml`, names kinds with the generated `EdgeKind`/`NodeKind`, and loses every hand-written copy of the wire — `Dtos.cs`, `EdgeKindId`, the startup version check — with no behaviour change.

**Architecture:** An MSBuild target runs NSwag (a local dotnet tool) before compile and feeds `obj/…/Wire.g.cs` to the compiler; nothing generated is committed. The only hand-written contract code is `client/Contract/EdgeKinds.cs`, which reads the derived `graph-vocabulary.json` (embedded at build) for duals. A usage test forbids generating types nobody reads; Stryker covers the hand-written file and the retyped call sites.

**Tech Stack:** .NET 10.0.401, Blazor WebAssembly; NSwag (`NSwag.ConsoleCore` dotnet tool); Stryker.NET (`dotnet-stryker`); xUnit; Reqnroll 3.3.4; Playwright (`tests/ux`).

**Spec:** `docs/superpowers/specs/2026-09-26-contract-from-graph-types-design.md` — §6, §7, §10, §11. **Prerequisite:** `2026-09-26-contract1a-server.md` complete (it produces `contracts/openapi.yaml`, `aqc.schema.json`, `graph-vocabulary.json`).

## Global Constraints

- `docs/PRINCIPLES.md` binds: TDD; no code a test does not need; `why` comments only; tests carry only `// Arrange` `// Act` `// Assert`.
- **Tests as documentation (PRINCIPLES 15–18):** every assertion is whole-body — `Assert.Equal(expected, actual)` on the entire value (records, arrays, dictionaries, canonical JSON strings), never a field picked out of it; one behaviour per test, named as a sentence; no bare numbers — `DeclaredEdgeKinds = 46`, `DeclaredSymmetricKinds = 6`, `DeclaredNodeKinds = 15` are the constants this plan uses; files in newspaper order — public members first, private fields and helpers below their first caller (C# static field initialisers run in textual order, so dependent fields are declared after what they read, at the bottom).
- **D2:** generated C# is never committed; `client/obj/` is already gitignored.
- **D4:** generated type names are the server's (`NodeCard`, not `NodeCardDto`); nothing is remapped.
- **D5:** zero behaviour change. The Playwright suite under `tests/ux` (minus the one spec D7 deletes) is the proof and runs at the end of every task that touches a page or component.
- **D7:** the client performs no startup contract check of any kind.
- **§10:** no generated type without a reader; fields are not pruned.
- All `dotnet` commands run from the repo root unless stated. The API must be running on :8000 and the client dev server on :5000 for Playwright (the harness the repo already uses).
- Commit per task; push at the end (owner authorization on record).

---

### Task 1: Local dotnet tools — NSwag and Stryker

**Files:**
- Create: `.config/dotnet-tools.json`

- [ ] **Step 1: Create the manifest and install both tools**

```bash
dotnet new tool-manifest
dotnet tool install NSwag.ConsoleCore
dotnet tool install dotnet-stryker
dotnet tool restore
dotnet nswag version
dotnet stryker --version
```
Expected: `.config/dotnet-tools.json` lists `nswag.consolecore` and `dotnet-stryker` with exact versions; both commands print a version.

- [ ] **Step 2: Commit**

```bash
git add .config/dotnet-tools.json
git commit -m "tools: nswag and dotnet-stryker as local dotnet tools"
```

---

### Task 2: Generate the wire records at build

**Files:**
- Create: `client/Contract/nswag.json`
- Modify: `client/BibleAtlas.Client.csproj`
- Test: `client.ContractTests/WireFixtureTests.cs`

**Interfaces:**
- Produces: namespace `BibleAtlas.Client.Contract` with one record per schema in `openapi.yaml` (`NodeCard`, `NodeRef`, `EdgePage`, `EdgeEntry`, `EdgeSummaryEntry`, `TextUnit`, `TextWindow`, `Contents`, `ContentsRoot`, `ContentsChild`, `Contract`, `PersonLife`, `Chapter`, `Verse`, `VerseDetail`, `CrossRef`, `CatechismRef`, `CatechismItem`, `PlaceDetail`, `EventDetail`, `NarrativeEventPositions`, `Polities`, `Polity`, `PolityDelta`, `LandMask`, `KretzmannChapter`, `Scene`, `ScenePlace`, `QuietPlace`, `SceneEvent`, `SceneArrow`, `SceneNarrative`, `VerseGroup`, `CanonBook`, `Narrative`, `Era`, `Landmark`, `SourcesDocument`, `SourceCategory`, `SourceEntry`, `ProvenanceEntry`, `TimeRange`, `ErrorBody`, `ErrorInner`, …) and enums `NodeKind`, `EdgeKind`.

- [ ] **Step 1: Write the failing test**

`client.ContractTests/WireFixtureTests.cs` (the project already computes `RepoRoot` in `Steps/AqcSteps.cs`; reuse that helper or duplicate its three lines):
```csharp
using System.Text.Json;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.ContractTests;

public sealed class WireFixtureTests
{
    [Theory]
    [InlineData("node-place-hazor-1.json", typeof(NodeCard))]
    [InlineData("node-event-ab-ur.json", typeof(NodeCard))]
    [InlineData("edges-hazor-1-site-of.json", typeof(EdgePage))]
    public void A_recorded_server_body_round_trips_through_the_generated_record_unchanged(string fixture, Type record)
    {
        // Arrange
        var json = FixtureBody(fixture);
        var expected = Canonical(JsonDocument.Parse(json).RootElement);
        // Act
        var actual = Canonical(JsonSerializer.SerializeToElement(JsonSerializer.Deserialize(json, record), record, WithoutNulls));
        // Assert
        Assert.Equal(expected, actual);
    }

    [Fact]
    public void The_site_of_page_for_hazor_lists_events_under_a_typed_kind()
    {
        // Arrange
        var json = FixtureBody("edges-hazor-1-site-of.json");
        // Act
        var page = JsonSerializer.Deserialize<EdgePage>(json)!;
        var kinds = (page.Kind, page.Entries.Select(e => e.Node.Kind).Distinct().ToArray());
        // Assert
        Assert.Equal((EdgeKind.SiteOf, new[] { NodeKind.Event }), kinds);
    }

    private static readonly JsonSerializerOptions WithoutNulls = new() { DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull };

    private static string FixtureBody(string name)
    {
        var text = File.ReadAllText(Path.Combine(AqcSteps.RepoRoot, "contracts", "atlas-graph-contract", "fixtures", name));
        var root = JsonDocument.Parse(text).RootElement;
        return root.TryGetProperty("body", out var body) ? body.GetRawText() : text;
    }

    private static string Canonical(JsonElement element) => JsonSerializer.Serialize(element);
}
```
`WithoutNulls` mirrors the server's `skip_serializing_if = "Option::is_none"`; a fixture that omits an optional field must come back without it. `FixtureBody` unwraps the pact's `{status, body}` envelope when a fixture carries one.

- [ ] **Step 2: Run to verify it fails**

Run: `dotnet test client.ContractTests --filter WireFixtureTests`
Expected: compile error — namespace `BibleAtlas.Client.Contract` / type `NodeCard` not found.

- [ ] **Step 3: `nswag.json`**

`client/Contract/nswag.json`:
```json
{
  "runtime": "Net100",
  "documentGenerator": {
    "fromDocument": { "url": "../../contracts/openapi.yaml", "output": null }
  },
  "codeGenerators": {
    "openApiToCSharpClient": {
      "generateClientClasses": false,
      "generateDtoTypes": true,
      "namespace": "BibleAtlas.Client.Contract",
      "classStyle": "Record",
      "generateNativeRecords": true,
      "jsonLibrary": "SystemTextJson",
      "arrayType": "System.Collections.Generic.IReadOnlyList",
      "arrayInstanceType": "System.Collections.Generic.List",
      "arrayBaseType": "System.Collections.Generic.List",
      "dictionaryType": "System.Collections.Generic.IReadOnlyDictionary",
      "dictionaryInstanceType": "System.Collections.Generic.Dictionary",
      "generateNullableReferenceTypes": true,
      "generateOptionalPropertiesAsNullable": true,
      "generateDataAnnotations": false,
      "generateDefaultValues": true,
      "generateJsonMethods": false,
      "enumNameGeneratorType": null,
      "excludedTypeNames": [],
      "output": "$(Output)"
    }
  }
}
```

- [ ] **Step 4: The build target**

In `client/BibleAtlas.Client.csproj`, inside `<Project>`:
```xml
<PropertyGroup>
  <ContractDocument>$(MSBuildProjectDirectory)/../contracts/openapi.yaml</ContractDocument>
  <ContractVocabulary>$(MSBuildProjectDirectory)/../contracts/atlas-graph-contract/fixtures/graph-vocabulary.json</ContractVocabulary>
  <GeneratedWire>$(IntermediateOutputPath)Contract/Wire.g.cs</GeneratedWire>
</PropertyGroup>

<ItemGroup>
  <EmbeddedResource Include="$(ContractVocabulary)" LogicalName="graph-vocabulary.json" />
</ItemGroup>

<Target Name="GenerateContract" BeforeTargets="CoreCompile"
        Inputs="$(ContractDocument);Contract/nswag.json"
        Outputs="$(GeneratedWire)">
  <MakeDir Directories="$(IntermediateOutputPath)Contract" />
  <Exec Command="dotnet nswag run Contract/nswag.json /variables:Output=$(GeneratedWire)" WorkingDirectory="$(MSBuildProjectDirectory)" />
</Target>

<Target Name="IncludeGeneratedContract" AfterTargets="GenerateContract" BeforeTargets="CoreCompile">
  <ItemGroup>
    <Compile Include="$(GeneratedWire)" />
  </ItemGroup>
</Target>
```
(The embedded vocabulary is Task 3's input; it is declared here so the csproj is touched once.)

- [ ] **Step 5: Build and run the test**

Run: `dotnet build client && dotnet test client.ContractTests --filter WireFixtureTests`
Expected: `obj/Debug/net10.0/Contract/Wire.g.cs` exists (untracked); three tests pass. If NSwag rejects an OpenAPI 3.1 construct in the document, record the exact error in the ledger and stop — that is a spec-level ruling (utoipa's document version), not a plan-level fix. Run `git status` and confirm nothing under `client/obj` is listed.

- [ ] **Step 6: Commit**

```bash
git add client/Contract/nswag.json client/BibleAtlas.Client.csproj client.ContractTests/WireFixtureTests.cs
git commit -m "client: wire records generated at build from contracts/openapi.yaml (NSwag); AGC fixtures deserialise into typed kinds"
```

---

### Task 3: `EdgeKinds` — duals and labels from the derived vocabulary

**Files:**
- Create: `client/Contract/EdgeKinds.cs`
- Test: `client.Tests/Contract/EdgeKindsTests.cs`

**Interfaces:**
- Produces: `EdgeKinds.Label(this EdgeKind) -> string`, `EdgeKinds.Dual(this EdgeKind) -> EdgeKind`, `EdgeKinds.IsSymmetric(this EdgeKind) -> bool`, `EdgeKinds.Parse(string label) -> EdgeKind`.

- [ ] **Step 1: Write the failing tests**

`client.Tests/Contract/EdgeKindsTests.cs`:
```csharp
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class EdgeKindsTests
{
    private const int DeclaredSymmetricKinds = 6;

    [Fact]
    public void Dual_is_an_involution()
    {
        // Arrange
        var kinds = Enum.GetValues<EdgeKind>();
        // Act
        var twice = kinds.Select(k => k.Dual().Dual()).ToArray();
        // Assert
        Assert.Equal(kinds, twice);
    }

    [Fact]
    public void Member_of_and_contains_are_each_others_dual()
    {
        // Arrange
        var contains = EdgeKind.Contains;
        // Act
        var dual = contains.Dual();
        // Assert
        Assert.Equal(EdgeKind.MemberOf, dual);
        Assert.Equal(contains, dual.Dual());
    }

    [Fact]
    public void Symmetric_kinds_are_their_own_dual()
    {
        // Arrange
        var symmetric = Enum.GetValues<EdgeKind>().Where(k => k.IsSymmetric()).ToArray();
        // Act
        var duals = symmetric.Select(k => k.Dual()).ToArray();
        // Assert
        Assert.Equal(DeclaredSymmetricKinds, symmetric.Length);
        Assert.Equal(symmetric, duals);
    }

    [Fact]
    public void Label_round_trips_through_Parse_for_every_kind()
    {
        // Arrange
        var kinds = Enum.GetValues<EdgeKind>();
        // Act
        var back = kinds.Select(k => EdgeKinds.Parse(k.Label())).ToArray();
        // Assert
        Assert.Equal(kinds, back);
        Assert.Equal("member-of", EdgeKind.MemberOf.Label());
    }

    [Fact]
    public void Parse_rejects_an_undeclared_label()
    {
        // Arrange
        var label = "cited";
        // Act
        var act = () => EdgeKinds.Parse(label);
        // Assert
        Assert.Throws<FormatException>(act);
    }
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `dotnet test client.Tests --filter EdgeKindsTests`
Expected: compile error — `EdgeKinds` not found.

- [ ] **Step 3: Implement from the embedded vocabulary**

`client/Contract/EdgeKinds.cs`:
```csharp
using System.Reflection;
using System.Runtime.Serialization;
using System.Text.Json;

namespace BibleAtlas.Client.Contract;

public static class EdgeKinds
{
    public static string Label(this EdgeKind kind) => Labels[kind];

    public static EdgeKind Parse(string label) =>
        ByLabel.TryGetValue(label, out var kind) ? kind : throw new FormatException($"'{label}' is not a declared edge kind");

    public static EdgeKind Dual(this EdgeKind kind) => Duals[kind];

    public static bool IsSymmetric(this EdgeKind kind) => Duals[kind] == kind;

    private const string VocabularyResource = "graph-vocabulary.json";

    private static readonly IReadOnlyDictionary<EdgeKind, string> Labels =
        Enum.GetValues<EdgeKind>().ToDictionary(k => k, k =>
            typeof(EdgeKind).GetField(k.ToString())!.GetCustomAttribute<EnumMemberAttribute>()!.Value!);

    private static readonly IReadOnlyDictionary<string, EdgeKind> ByLabel =
        Labels.ToDictionary(p => p.Value, p => p.Key);

    private static readonly IReadOnlyDictionary<EdgeKind, EdgeKind> Duals = LoadDuals();

    private static IReadOnlyDictionary<EdgeKind, EdgeKind> LoadDuals()
    {
        using var stream = typeof(EdgeKinds).Assembly.GetManifestResourceStream(VocabularyResource)!;
        using var doc = JsonDocument.Parse(stream);
        var duals = new Dictionary<EdgeKind, EdgeKind>();
        foreach (var r in doc.RootElement.GetProperty("relations").EnumerateArray())
        {
            var forward = Parse(r.GetProperty("forward").GetString()!);
            var inverse = Parse(r.GetProperty("inverse").GetString()!);
            duals[forward] = inverse;
            duals[inverse] = forward;
        }
        foreach (var s in doc.RootElement.GetProperty("symmetric").EnumerateArray())
        {
            var kind = Parse(s.GetProperty("label").GetString()!);
            duals[kind] = kind;
        }
        return duals;
    }
}
```
The vocabulary is derived from the same `openapi.yaml` the enum came from (CONTRACT-1a Task 7), so the two cannot disagree without the server-side regenerate-and-diff gate failing first; `Dual_is_an_involution` over `Enum.GetValues` is the client-side totality check.

- [ ] **Step 4: Run**

Run: `dotnet test client.Tests --filter EdgeKindsTests`
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add client/Contract/EdgeKinds.cs client.Tests/Contract/EdgeKindsTests.cs
git commit -m "client: EdgeKinds -- Label/Parse from the generated enum, Dual/IsSymmetric from the derived vocabulary"
```

---

### Task 4: Retype the graph client onto the generated types; delete `EdgeKindId`

**Files:**
- Modify: `client/IExplorableClient.cs`, `client/GraphExplorableClient.cs`, `client/AtlasClient.cs` (`NodeEdges`, `NodeCard`, `ConcordUnit`, `Contents`), `client/Explore/EdgeSectionRegistry.cs`, every caller of `Graph.Edges(...)` / `Atlas.NodeEdges(...)` / `EdgeSectionRegistry.*.EdgeKind`
- Test: `client.Tests/Contract/EdgeSectionRegistryTests.cs`

- [ ] **Step 1: Write the failing test**

`client.Tests/Contract/EdgeSectionRegistryTests.cs`:
```csharp
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class EdgeSectionRegistryTests
{
    [Fact]
    public void The_registry_holds_exactly_the_four_section_policies_keyed_by_generated_kinds()
    {
        // Arrange
        var expected = new Dictionary<EdgeKind, EdgeSectionSpec>
        {
            [EdgeKind.Cites]         = new(EdgeKind.Cites,         SectionStyle.Quiet,    InitialClamp: 3,  SectionOrder.VotesRanked),
            [EdgeKind.Mentions]      = new(EdgeKind.Mentions,      SectionStyle.Standard, InitialClamp: 50, SectionOrder.Canonical),
            [EdgeKind.MentionedIn]   = new(EdgeKind.MentionedIn,   SectionStyle.Standard, InitialClamp: 12, SectionOrder.Canonical),
            [EdgeKind.CommentedOnBy] = new(EdgeKind.CommentedOnBy, SectionStyle.Standard, InitialClamp: 20, SectionOrder.Canonical),
        };
        // Act
        var actual = EdgeSectionRegistry.ByKind;
        // Assert
        Assert.Equal(expected, actual);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `dotnet test client.Tests --filter EdgeSectionRegistryTests`
Expected: compile error — `ByKind` is keyed by `EdgeKindId`, not `EdgeKind`.

- [ ] **Step 3: Retype**

`client/IExplorableClient.cs` — delete the `EdgeKindId` record struct (and its `why` comment, which no longer applies) and retype:
```csharp
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public interface IExplorableClient
{
    Task<NodeCard> Card(string id);

    Task<EdgePage> Edges(string id, EdgeKind kind, int? cursor = null, int limit = 20);

    Task<TextWindow> Reading(string fromRef, int n, string dir = "onward", string corpus = "bible");
}
```
`client/GraphExplorableClient.cs`: `NodeCardDto`→`NodeCard`, `EdgePageDto`→`EdgePage`, `TextWindowDto`→`TextWindow`; `Uri.EscapeDataString(kind.Value)` → `Uri.EscapeDataString(kind.Label())`; drop the `Wire.Options` argument from each `GetFromJsonAsync` (generated records carry explicit `JsonPropertyName`s).
`client/AtlasClient.cs`: `NodeEdges(string nodeId, EdgeKind kind, …)` with `kind.Label()`; return types `EdgePage`, `NodeCard`, `TextWindow`, `Contents`.
`client/Explore/EdgeSectionRegistry.cs`:
```csharp
public sealed record EdgeSectionSpec(EdgeKind EdgeKind, SectionStyle Style, int InitialClamp, SectionOrder Order);

public static readonly EdgeSectionSpec Cites         = new(EdgeKind.Cites,         SectionStyle.Quiet,    InitialClamp: 3,  SectionOrder.VotesRanked);
public static readonly EdgeSectionSpec Mentions      = new(EdgeKind.Mentions,      SectionStyle.Standard, InitialClamp: 50, SectionOrder.Canonical);
public static readonly EdgeSectionSpec MentionedIn   = new(EdgeKind.MentionedIn,   SectionStyle.Standard, InitialClamp: 12, SectionOrder.Canonical);
public static readonly EdgeSectionSpec CommentedOnBy = new(EdgeKind.CommentedOnBy, SectionStyle.Standard, InitialClamp: 20, SectionOrder.Canonical);

public static readonly IReadOnlyDictionary<EdgeKind, EdgeSectionSpec> ByKind = new Dictionary<EdgeKind, EdgeSectionSpec>
{
    [Cites.EdgeKind] = Cites, [Mentions.EdgeKind] = Mentions, [MentionedIn.EdgeKind] = MentionedIn, [CommentedOnBy.EdgeKind] = CommentedOnBy,
};
```
Then `dotnet build client` and follow the compiler through every `Graph.Edges(…, new EdgeKindId("…"))` / `Atlas.NodeEdges(…, "…")` call site in `Explore/PopoverSectionProviders.cs`, `Components/PersonMentionsList.razor` and any other: replace the literal with the named member (`EdgeKind.CatechismLink`, `EdgeKind.ParticipatesIn`, …). No string literal that equals a declared label may remain outside `Contract/`:
```bash
grep -rnE "\"(contains|member-of|attested-in|attests|follows-in|precedes-in|dated-by|dates|located-at|site-of|mentions|mentioned-in|cites|cited-by|quotes|quoted-by|confesses|confessed-in|fulfilled-in|fulfills|prefigures|prefigured-by|named-after|namesake-of|justified-by|justifies|comments-on|commented-on-by|spoken-by|speech-of|spoken-at|site-of-speech|derived-from|derives|occurs-in|words|parent-of|child-of|participates-in|participants|analogous-to|catechism-link|corresponds-to|parallel|temporal-adjacency|partner-of)\"" client --include=*.cs --include=*.razor | grep -v "client/Contract/"
```
Expected: no output (test ids that happen to contain a label word are not quoted labels; if one is, it is a test id string and stays).

- [ ] **Step 4: Verify**

Run: `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests`
Expected: green. Then the popover Playwright specs: `npx playwright test tests/ux --grep "popover|person|catechism|xrefs"` — green.

- [ ] **Step 5: Commit**

```bash
git add -A client client.Tests
git commit -m "client: the graph client speaks generated NodeCard/EdgePage/TextWindow and EdgeKind; EdgeKindId and every label literal gone"
```

---

### Task 5: The rename sweep — every `*Dto` becomes the generated type; `Dtos.cs` and `Wire.cs` die

**Files:**
- Delete: `client/Dtos.cs`, `client/Wire.cs`
- Modify: every consumer (Pages, Components, Explore, `AtlasClient.cs`, `MapInterop.cs`, `LocalStore.cs` if it used `Wire.Options`, tests)

- [ ] **Step 1: The mapping**

Client name → generated name (server's, per CONTRACT-1a Task 4 and Appendix A):

| client | generated | client | generated |
|---|---|---|---|
| `TimeRangeDto` | `TimeRange` | `EraDto` | `Era` |
| `BookTocEntry` | `CanonBook` | `LandmarkDto` | `Landmark` |
| `NarrativeOut` | `Narrative` | `SourcesDocumentOut` | `SourcesDocument` |
| `ProvenanceEntryDto` | `ProvenanceEntry` | `SourceCategoryDto` | `SourceCategory` |
| `SourceEntryDto` | `SourceEntry` | `ChapterOut` | `Chapter` |
| `HeadingDto` | `Heading` | `VerseOut` | `Verse` |
| `PlaceRefDto` | `PlaceRef` | `PersonRefDto` | `PersonRef` |
| `WordsOfChristSpanDto` | `WordsOfChristSpan` | `VerseEventDto` | `VerseEvent` |
| `VerseDetail` | `VerseDetail` (unchanged) | `CrossRefOut` | `CrossRef` |
| `CatechismRefDto` | `CatechismRef` | `CatechismProofVerseDto` | `CatechismProofVerse` |
| `CatechismItemDetail` | `CatechismItem` | `EventDetail` | `EventDetail` (unchanged) |
| `EventPlaceDto` | `EventPlace` | `EventWitnessDto` | `EventWitness` |
| `EventAnalogueDto` | `EventAnalogue` | `NarrativePositionDto` | `NarrativePosition` |
| `NarrativeAdjacentEventDto` | `NarrativeAdjacentEvent` | `TimelinePositionDto` | `TimelinePosition` |
| `NarrativeEventPositionsResult` | `NarrativeEventPositions` | `PlaceDetail` | `PlaceDetail` (unchanged) |
| `PlaceHistoryOut` | `History` | `DateClaimOut` | `DateClaim` |
| `PolityEraOut` | `Polity` | `PolityDeltaDto` | `PolityDelta` |
| `PolitiesOut` | `Polities` | `LandMaskOut` | `LandMask` |
| `KretzmannChapterOut` | `KretzmannChapter` | `KretzmannChapterItemOut` | `KretzmannChapterItem` |
| `KretzmannChapterVerseOut` | `KretzmannChapterVerse` | `BookMetaDto` | `BookMeta` |
| `EdgeSummaryEntryDto` | `EdgeSummaryEntry` | `NodeCardDto` | `NodeCard` |
| `PersonLifeDto` | `PersonLife` | `NodeRefDto` | `NodeRef` |
| `EdgeEntryDto` | `EdgeEntry` | `EdgePageDto` | `EdgePage` |
| `TextUnitDto` | `TextUnit` | `TextWindowDto` | `TextWindow` |
| `ContractDto` | `Contract` | `ContentsOut`/`ContentsRootOut`/`ContentsChildOut` | `Contents`/`ContentsRoot`/`ContentsChild` |
| `Scene`, `ScenePlace`, `QuietPlace`, `SceneEvent`, `VerseGroup`, `SceneArrow`, `SceneNarrative` | same names, new namespace | | |

- [ ] **Step 2: Sweep**

```bash
git rm client/Dtos.cs
# add `using BibleAtlas.Client.Contract;` to client/_Imports.razor and to each .cs file the compiler reports
python - <<'EOF'
import re,pathlib
m = {"TimeRangeDto":"TimeRange","EraDto":"Era","BookTocEntry":"CanonBook","LandmarkDto":"Landmark","NarrativeOut":"Narrative",
"SourcesDocumentOut":"SourcesDocument","ProvenanceEntryDto":"ProvenanceEntry","SourceCategoryDto":"SourceCategory","SourceEntryDto":"SourceEntry",
"ChapterOut":"Chapter","HeadingDto":"Heading","VerseOut":"Verse","PlaceRefDto":"PlaceRef","PersonRefDto":"PersonRef","WordsOfChristSpanDto":"WordsOfChristSpan",
"VerseEventDto":"VerseEvent","CrossRefOut":"CrossRef","CatechismRefDto":"CatechismRef","CatechismProofVerseDto":"CatechismProofVerse","CatechismItemDetail":"CatechismItem",
"EventPlaceDto":"EventPlace","EventWitnessDto":"EventWitness","EventAnalogueDto":"EventAnalogue","NarrativePositionDto":"NarrativePosition",
"NarrativeAdjacentEventDto":"NarrativeAdjacentEvent","TimelinePositionDto":"TimelinePosition","NarrativeEventPositionsResult":"NarrativeEventPositions",
"PlaceHistoryOut":"History","DateClaimOut":"DateClaim","PolityEraOut":"Polity","PolityDeltaDto":"PolityDelta","PolitiesOut":"Polities","LandMaskOut":"LandMask",
"KretzmannChapterOut":"KretzmannChapter","KretzmannChapterItemOut":"KretzmannChapterItem","KretzmannChapterVerseOut":"KretzmannChapterVerse","BookMetaDto":"BookMeta",
"EdgeSummaryEntryDto":"EdgeSummaryEntry","NodeCardDto":"NodeCard","PersonLifeDto":"PersonLife","NodeRefDto":"NodeRef","EdgeEntryDto":"EdgeEntry","EdgePageDto":"EdgePage",
"TextUnitDto":"TextUnit","TextWindowDto":"TextWindow","ContractDto":"Contract","ContentsOut":"Contents","ContentsRootOut":"ContentsRoot","ContentsChildOut":"ContentsChild"}
pat = re.compile(r"\b(" + "|".join(sorted(m, key=len, reverse=True)) + r")\b")
for root in ["client","client.Tests","client.ContractTests"]:
    for p in pathlib.Path(root).rglob("*"):
        if p.suffix in (".cs",".razor") and "obj" not in p.parts and "bin" not in p.parts and p.name != "Wire.g.cs":
            s = p.read_text(encoding="utf-8"); t = pat.sub(lambda x: m[x.group(1)], s)
            if s != t: p.write_text(t, encoding="utf-8")
EOF
dotnet build client
```
Follow the compiler: positional-construction sites (`new NodeCardDto(a, b, c)`) become object initialisers on the generated records (`new NodeCard { Id = a, Kind = b, … }`) — expect these mainly in tests; `List<T>` parameters become `IReadOnlyList<T>`. `Wire.Options` has no remaining reader once `GetFromJsonAsync` calls drop it: check `grep -rn "Wire.Options" client client.Tests`; `MapInterop.cs` serialises `Scene` for JS with `JsonSerializer.Serialize(scene, Wire.Options)` — the generated `Scene` carries `[JsonPropertyName]`s, so `JsonSerializer.Serialize(scene)` yields identical snake_case keys; change the call and delete `client/Wire.cs`.

- [ ] **Step 3: Verify — the whole surface**

Run: `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests`, then the full Playwright suite: `npx playwright test tests/ux`.
Expected: green throughout — this is D5's proof for the rename. The world-map specs in particular prove the `MapInterop` serialisation is byte-identical.

- [ ] **Step 4: Commit**

```bash
git add -A client client.Tests client.ContractTests
git commit -m "client: every wire type is the generated one; Dtos.cs and Wire.cs deleted (D4)"
```

---

### Task 6: D7 — the startup check dies

**Files:**
- Modify: `client/App.razor`
- Delete: `client/Pages/ContractMismatch.razor`, `client/AqcContract.cs`, `tests/ux/contract-versioning.spec.ts`
- Modify: `client/AtlasClient.cs` (remove `Contract()`), `client.ContractTests/Steps/AqcSteps.cs` (remove the three client-acceptance steps), `contracts/atlas-query-contract/features/versioning.feature` (remove the scenarios that bind them), `contracts/atlas-query-contract/CHANGELOG.md`
- Test: `client.Tests/Contract/StartupTests.cs`

- [ ] **Step 1: Write the failing test**

`client.Tests/Contract/StartupTests.cs` (a source-scan test in the repo's existing style — see `client.Tests/State/ConformanceTests.cs` for how it locates the client directory):
```csharp
namespace BibleAtlas.Client.Tests.Contract;

public sealed class StartupTests
{
    [Fact]
    public void The_client_performs_no_contract_check_at_startup()
    {
        // Arrange
        var app = File.ReadAllText(Path.Combine(ConformanceTests.ClientRoot, "App.razor"));
        // Act
        var mentionsContract = app.Contains("Contract", StringComparison.Ordinal) || app.Contains("@code", StringComparison.Ordinal);
        // Assert
        Assert.False(mentionsContract, "App.razor must be the Router alone (spec D7)");
        Assert.False(File.Exists(Path.Combine(ConformanceTests.ClientRoot, "Pages", "ContractMismatch.razor")));
        Assert.False(File.Exists(Path.Combine(ConformanceTests.ClientRoot, "AqcContract.cs")));
    }
}
```
(If `ConformanceTests` does not expose a `ClientRoot`, add `internal static readonly string ClientRoot` there from the path logic it already has.)

- [ ] **Step 2: Run to verify it fails**

Run: `dotnet test client.Tests --filter StartupTests`
Expected: FAIL — `App.razor` contains `@code` and `Contract`.

- [ ] **Step 3: Delete**

`client/App.razor` becomes exactly:
```razor
<Router AppAssembly="@typeof(App).Assembly" NotFoundPage="typeof(Pages.NotFound)">
    <Found Context="routeData">
        <RouteView RouteData="@routeData" DefaultLayout="@typeof(MainLayout)"/>
        <FocusOnNavigate RouteData="@routeData" Selector="h1" />
    </Found>
</Router>
```
```bash
git rm client/Pages/ContractMismatch.razor client/AqcContract.cs tests/ux/contract-versioning.spec.ts
```
In `client/AtlasClient.cs` delete `Contract(CancellationToken)`. In `client.ContractTests/Steps/AqcSteps.cs` delete the steps bound to `the server advertises AQC version "..." through "..."` (Given, :171, and Then, :543), `the client accepts the advertised range` (:550) and `the client rejects the advertised range` (:556). In `contracts/atlas-query-contract/features/versioning.feature`, delete the scenarios whose steps are those four; keep every scenario that only pins what the **server** advertises (they are also run by the Rust harness). Add to the AQC `CHANGELOG.md`: "client-side acceptance of the advertised range removed (D7: agreement is proven in CI); server advertisement unchanged" and bump `VERSION` minor.

- [ ] **Step 4: Verify**

Run: `dotnet test client.Tests --filter StartupTests && dotnet test client.ContractTests && (cd server && cargo test -p atlas-contract --test aqc_cucumber 2>&1 | grep -E "test result|FAILED") && npx playwright test tests/ux`
Expected: green; the Rust cucumber still runs the remaining versioning scenarios; Playwright green minus the deleted spec.

- [ ] **Step 5: Commit**

```bash
git add -A client client.ContractTests client.Tests tests/ux contracts/atlas-query-contract
git commit -m "client: no startup contract check -- App.razor is the Router; ContractMismatch, AqcContract, AtlasClient.Contract and their scenarios deleted (D7)"
```

---

### Task 7: No generated type without a reader

**Files:**
- Test: `client.Tests/Contract/GeneratedUsageTests.cs`
- Modify: `client/Contract/nswag.json` (`excludedTypeNames`)

- [ ] **Step 1: Write the failing test**

```csharp
using System.Text.Json;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class GeneratedUsageTests
{
    private static readonly string RepoRoot = Path.GetFullPath(Path.Combine(ConformanceTests.ClientRoot, ".."));

    private static IEnumerable<string> GeneratedTypeNames()
    {
        using var schema = JsonDocument.Parse(File.ReadAllText(Path.Combine(RepoRoot, "contracts", "atlas-query-contract", "aqc.schema.json")));
        using var nswag = JsonDocument.Parse(File.ReadAllText(Path.Combine(RepoRoot, "client", "Contract", "nswag.json")));
        var excluded = nswag.RootElement.GetProperty("codeGenerators").GetProperty("openApiToCSharpClient").GetProperty("excludedTypeNames")
            .EnumerateArray().Select(e => e.GetString()!).ToHashSet();
        return schema.RootElement.GetProperty("$defs").EnumerateObject().Select(p => p.Name).Where(n => !excluded.Contains(n));
    }

    private static string ClientSource() =>
        string.Join("\n", Directory.EnumerateFiles(ConformanceTests.ClientRoot, "*.*", SearchOption.AllDirectories)
            .Where(f => (f.EndsWith(".cs") || f.EndsWith(".razor")) && !f.Contains($"{Path.DirectorySeparatorChar}obj{Path.DirectorySeparatorChar}"))
            .Select(File.ReadAllText));

    [Fact]
    public void Every_generated_type_is_referenced()
    {
        // Arrange
        var source = ClientSource();
        // Act
        var unread = GeneratedTypeNames().Where(n => !System.Text.RegularExpressions.Regex.IsMatch(source, $@"\b{n}\b")).ToArray();
        // Assert
        Assert.Empty(unread);
    }

    [Fact]
    public void Every_excluded_type_is_unreferenced()
    {
        // Arrange
        using var nswag = JsonDocument.Parse(File.ReadAllText(Path.Combine(RepoRoot, "client", "Contract", "nswag.json")));
        var excluded = nswag.RootElement.GetProperty("codeGenerators").GetProperty("openApiToCSharpClient").GetProperty("excludedTypeNames").EnumerateArray().Select(e => e.GetString()!);
        var source = ClientSource();
        // Act
        var referenced = excluded.Where(n => System.Text.RegularExpressions.Regex.IsMatch(source, $@"\b{n}\b")).ToArray();
        // Assert
        Assert.Empty(referenced);
    }
}
```
A generated type referenced only by another generated type (e.g. `ErrorInner` inside `ErrorBody`) counts as unread by this scan; that is correct — if nothing hand-written reads `ErrorBody`, both are excluded together.

- [ ] **Step 2: Run to see the unread list**

Run: `dotnet test client.Tests --filter GeneratedUsageTests`
Expected: FAIL naming the unread types — at least `Contract`, `ErrorBody`, `ErrorInner`, and whichever of the map/detail types no client code reads after Task 5.

- [ ] **Step 3: Exclude exactly those**

Put the reported names into `excludedTypeNames` in `nswag.json`; `dotnet build client` (the generator re-runs because `nswag.json` is an input); rerun the tests.
Expected: both pass; the build still succeeds (excluding a type something referenced would have failed compilation, which is the point).

- [ ] **Step 4: Commit**

```bash
git add client/Contract/nswag.json client.Tests/Contract/GeneratedUsageTests.cs
git commit -m "client: no generated type without a reader -- excludedTypeNames is the declared consumption, GeneratedUsageTests keeps it honest"
```

---

### Task 8: Mutation coverage, the closing verification, push

**Files:**
- Create: `client.Tests/stryker-config.json`

- [ ] **Step 1: Stryker, scoped to this batch's hand-written code**

`client.Tests/stryker-config.json`:
```json
{
  "stryker-config": {
    "project": "BibleAtlas.Client.csproj",
    "mutate": [
      "**/Contract/EdgeKinds.cs",
      "**/IExplorableClient.cs",
      "**/GraphExplorableClient.cs",
      "**/Explore/EdgeSectionRegistry.cs",
      "!**/obj/**"
    ],
    "thresholds": { "high": 100, "low": 100, "break": 100 },
    "reporters": ["progress", "html", "json"]
  }
}
```
Run: `cd client.Tests && dotnet stryker`
Expected: mutation score 100 for the listed files. A surviving mutant is either killed by a new test (add it, in the AAA style) or recorded in this file under `"ignore-mutations"` with a reason — never silenced with a `// Stryker disable` comment.

- [ ] **Step 2: The whole client surface, once more**

Run: `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests && npx playwright test tests/ux`
Expected: green. Record the counts in the ledger beside CONTRACT-1a's standing-block numbers.

- [ ] **Step 3: Confirm nothing generated is tracked**

Run: `git status --porcelain | grep -E "obj/|Wire.g.cs"`
Expected: no output.

- [ ] **Step 4: Commit and push**

```bash
git add client.Tests/stryker-config.json
git commit -m "client: Stryker scoped to the contract batch's hand-written files at 100%"
git push origin worktree-bible-atlas-m1
```

---

## Self-review against the spec

- **Coverage:** §6.1–6.2 (Task 2), §6.3 (Task 3 — table replaced by the derived vocabulary, which is DRYer than the spec's hand table and satisfies the same tests), §6.4 (Task 4), §6.5 and §11 client rows (Tasks 4–6), D7 (Task 6), §10 usage law and Stryker scope (Tasks 7–8), §9 ripple (`Wire.Options`, conformance scans — none named the deleted files, re-checked in Task 5).
- **Placeholders:** none. The one conditional (NSwag rejecting an OpenAPI 3.1 construct) is a named stop with its escalation, not a "handle it".
- **Type consistency:** `EdgeKinds.Label/Parse/Dual/IsSymmetric` (Task 3) are what Task 4's client code and Task 8's mutation scope name; `NodeCard`/`EdgePage`/`TextWindow`/`Contents` (Task 2) are what Tasks 4–5 retype onto; `ConformanceTests.ClientRoot` (Task 6) is what Task 7 reuses.
