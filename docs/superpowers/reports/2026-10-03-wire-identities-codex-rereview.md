# A-WIRE-IDENTITIES: repaired-head Codex re-review

**APPROVED for this item at exact dcbc1ea**, `7247d81..dcbc1ea`.
The repair is `3fee0dc..dcbc1ea`. It closes both residual obligations recorded
in [the prior review](2026-10-03-wire-identities-codex-review.md), F-WI-6.
No new finding. This author-independent verdict permits the controller's normal
locked, squashed landing; Codex has not landed or changed the application.

## What now closes the category

All four outstanding custom transport methods accept the signed generated
reference types: Chapter, ChapterText and KretzmannChapter take ChapterReference;
SceneScripture takes BibleReference. They no longer compose a chapter reference.
Every affected legacy caller now reaches them through the owner's one approved
LegacyNodeIds door. The existing ratchet explicitly accounts for the 15 previously
hidden constructions across seven files; those legacy views remain scheduled for
retirement rather than being described as removed.

Every generated identity Value getter carries Experimental("ATLASWIRE"). The
compiler refuses extraction outside an opt-in. The client-wide guard enumerates
the schema's identity inventory and permits the diagnostic opt-in only in
AtlasClient.cs and GraphExplorableClient.cs. The generated adapter's own opt-in
is private to its generated block; test projects opt in to inspect wire values.
Ordinary served properties such as Year.Value are not identity extraction.
The law lives in IdentityTypesTests instead of the design's named OneDoorLawTests;
it enforces the intended rule at the compiler and source boundary.

The D.R.Y./14b pass finds one schema-derived diagnostic declaration and one
legacy construction door. The custom transport interfaces compose producer
identities; no additional domain parser or copied reference type is introduced.
The 24a/24b pass finds every residual reference-read caller migrated and the
future primitive-parameter/value-opt-in guards in place. The permitted legacy
exception remains counted and fenced. No prohibited application comment is added;
the diff adds only permitted Arrange/Act/Assert markers in tests. Pre-existing
comments and previously filed whole-client findings are outside this repair.

## Independent evidence

- **740 client tests and 88 contract tests pass**, zero failures/skips, at code
  identical to dcbc1ea. The review branch's merge retains the earlier report but
  changes no application source. The same two pre-existing xUnit1031 warnings
  remain in ViewRegistryConformanceTests.
- The identical prior external consumer is now refused: four ATLASWIRE errors,
  three CS1501 old-overload errors and one CS1503 string/reference mismatch.
- A second external consumer compiles the four valid nominal delegates and
  checks the entire reflected signature list. No delegate is invoked and no
  HTTP request is made. Its complete identity comparison contains exactly the
  20 expected generated identities, each fenced with ATLASWIRE.
- A separate forbidden program tries every one of those 20 getters, plus an
  ArtifactRoot at each of the four repaired reference doors. The full compiler
  result matches **20 ATLASWIRE + 4 CS1503 errors**, no unexpected diagnostics.
- The repair changes no server, graph-types, contract or compiled artifact blob
  relative to 3fee0dc. Across the full item, all seven compiled files and 120 of
  124 contract files are unchanged from 7247d81; the four changes are the approved
  OpenAPI, AQC schema, VERSION and CHANGELOG. Fixtures and pact bytes remain
  identical. This comparison is not a fresh live export of every route.

[Retained evidence](evidence/2026-10-03-wire-identities-repair) includes complete
logs, forbidden/positive source consumers, project recipes, whole diagnostic
comparison, artifact comparison, exact revisions and SHA-256 metadata. Log
trailing whitespace is normalized; original local logs remain outside git.
The recipes reference this review worktree's built Debug client assemblies.
Build the client first, then run the nominal consumer and build the deliberately
forbidden consumer; the latter must fail with its complete recorded diagnostic set.
The earlier consumer source is retained in the prior review's evidence directory.

No fresh Cargo, browser, timing, mutation or AOT gate was run by Codex. The ops
queue publishes the author's exact-head gates: server 1537 pass / 11 ignored,
graph-types 136 default and 150 feature pass, timing 11/11, contract gate pass,
and the 21 id-touching browser specs 206 pass / 1 skip. Producer code and schema
are unchanged by this repair; these published outcomes complement the independent
C# and compiler checks above. Host C: still has about 3.5 GB free; no new Rust/AOT
target, owner port, shared artifact or other agent's worktree was touched.

Handoff: Claude may land exact dcbc1ea under the usual protocol. Then consume
its approved producer vocabulary into the F# generator, including identity-only
oneOf widening and cursor defaults specified in design section 4.3; full F# domain
sign-off and parity remain unfinished. Codex holds no lock or server.
