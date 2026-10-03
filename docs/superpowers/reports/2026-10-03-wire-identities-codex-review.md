# A-WIRE-IDENTITIES: Codex review

Reviewed 2026-10-03: exact implementation **3fee0dc**, range
`7247d81..3fee0dc`. This incorporates implementation `97b940a`, description
pin `9d4c6d6`, and the already approved provenance repair `7247d81`.
**CHANGES REQUESTED: the existing F-WI-6 reference-construction category remains
open.** The generated identities themselves pass the independent checks below.
No application, contract or artifact change was made by this review.

1. **Important — four transport reads still accept the wrong identity category.**
   The signed design §4.2 requires `AtlasClient.SceneScripture(BibleReference)`
   and `Chapter`, `ChapterText`, `KretzmannChapter(ChapterReference)`.
   `client/AtlasClient.cs` still publishes `SceneScripture(string)` (line 57),
   `Chapter(string, int)` (line 74), `ChapterText(string, int)` (line 87), and
   `KretzmannChapter(string, int)` (line 103). The latter three still construct
   `{book}.{chapter}` inside the transport, precisely the F-WI-6 offender named
   in the signed design. Naming the server parameter schema does not migrate
   these independent custom-client signatures.

   An external consumer compiled against the actual built client assembly can
   supply `ArtifactRoot.Value` to all four methods. The retained probe compiles
   delegates and reflects the four signatures; it never invokes the delegates
   or makes an HTTP request. This demonstrates an open compile-time category,
   not a corrupt artifact or a newly observed runtime regression. The behavior
   predates this change; its closure is explicitly part of this item.

   Closure: use the generated accepted reference types at every transport read;
   route unavoidable legacy construction through the owner's approved fenced,
   shrinking door. Enumerate every named-reference request signature from the
   contract and independently reject incompatible references at those actual
   doors. The owner's O3 exception authorizes fenced legacy node-id construction;
   it does not waive the §4.2 reference transport migration. The controller
   chooses the implementation or records an explicit owner disposition; this
   review does not expand the frozen C# UI or fix callers on the side.

2. **Important — the promised identity extraction guard is absent.**
   Signed §4.2 explicitly confines identity `.Value` reads to `AtlasClient` and
   `GraphExplorableClient` when writing transport URLs and requires a source law
   in `OneDoorLawTests`. That file still contains only the pre-existing catch
   and neighbour-paging guards. The new `IdentityTypesTests` law looks for
   literal `Deserialize<NamedIdentity>` calls; it cannot detect extracting an
   identity's primitive and reusing it as another request argument. Likewise
   `ReferenceParsingLawTests` recognizes `CanonRef`/`LegacyNodeIds` calls, so it
   passes while the three unlisted transport interpolations remain. No claim
   is made that every `.Value` in the client is an identity extraction: nullable
   scalars and state values legitimately have that member too.

   Closure: implement the signed identity-specific source/semantic guard using
   the generated identity inventory, with independent invalid examples covering
   extraction and reuse outside the approved transport/legacy boundary. This
   is the 24b protection for the same open reference/identity category; do not
   treat a literal decoder-name regex as category closure.

The 14b pass finds one producer-owned wire identity vocabulary, schema-derived
C# identities and widenings, and the approved bounded legacy door. Existing
Rust `VerseId`/`ScriptureRef` parsing is reused; union and cursor declarations
compose those owners. No new prohibited application comment line is added
(only permitted Arrange/Act/Assert test markers). Existing library adapters and
licenses are preserved. The 24a/24b pass identifies the remaining wrong-side
raw-reference transport sites above: most consumers migrated, but all sites
and the future-offender guard are not closed, so the item cannot be marked done.

Independent validation:

- Complete C# client suite: **738 passed, 0 failed, 0 skipped**. Contract suite:
  **88 passed, 0 failed, 0 skipped**. Builds ran at `9d4c6d6`; the subsequent
  provenance merge to exact `3fee0dc` changes none of their application,
  generator, test or contract-test source files. Two existing xUnit1031
  warnings remain at `ViewRegistryConformanceTests.cs` lines 137/141.
- A separate compilation refuses `ArtifactRoot -> NodeId`,
  `ElementPageCursor -> EdgePageCursor`, and `VerseReference -> ChapterReference`.
  The generated nominal distinctions and union widening matrix are working.
- The external four-method transport probe compiles and retains the complete
  reflected signatures, establishing the residual gap independently of the
  authored tests. No HTTP requests or live-server assertions were made.
- All **7 compiled artifact files** and **120 of 124 contract files** have the
  same Git blob identities as `7247d81`. Only the authorized OpenAPI, AQC schema,
  VERSION and CHANGELOG differ. Existing AQC/AGC fixtures and pact bytes are
  therefore unchanged. C# contract tests independently deserialize/round-trip
  their enumerated recorded bodies; this is not a fresh live producer export
  of every route.

Evidence is retained under `evidence/2026-10-03-wire-identities/`: complete test
logs, positive transport probe source/output, forbidden conversion source and
compiler output, exact revisions, artifact comparison and SHA-256 inventory.
No fresh Cargo target, full workspace, timing, browser, mutation or AOT gate
was run by Codex because the host VHD remains capacity constrained. Author
exact-head full-gate publication was still absent from the queue at review
publication; these independent checks do not substitute for those gates.

Handoff: repair/dispose the two residual signed obligations, publish the exact
new head and gates, then request re-review. Do not land or consume the wire
migration as category-closed. Codex holds no lock or server; this report branch
contains review evidence only.
