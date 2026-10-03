# CX-FSHARP foundation checkpoint

This is a work-in-progress checkpoint, not a completed rewrite or a parity verdict. CX-FSHARP remains claimed. Claude's C# client and ongoing branches are untouched.

Reference: ace063d4799a6447920add63336824a00cd34ef7, the approved FOCUS-2/style tree. The client/contracts trees of the landed e558f07 match this reference byte for byte. Full FOCUS-2 review is separately delivered as 894d6d6 on lane/codex/A-F2-review.

The separate client has an immutable model, closed messages, a pure update returning typed effects, one Elmish command interpreter, Bolero views, routing and a standalone WebAssembly entry point. The generator builds all 102 published schemas, discriminated/closed vocabularies and 22 JSON GET request signatures into untracked output; it links no C# client project. The exploration algebra is Reader/StateT over Async/Result with tested identities, associativity, failure short-circuiting, Back/duality, batched Resume and artifact-root renewal. Loading rejects stale completions, Retry advances the identity, the private cache evicts at its capacity, and text composition uses served scalar offsets and anchors only.

Sources and the shared header are the first rendered slice. Reader/Concord startup state is implemented; their views and the remaining routes/interactions are not implemented. No C# component or state object is wrapped.

## Verification

- F# unit/contract/algebra/state/transport/bUnit: **49 passed**, zero skipped, log `/tmp/codex-fsharp-foundation-tests.log`.
- SDK build: zero warnings/errors, `/tmp/codex-fsharp-assets-build.log`.
- Release WebAssembly publish: passed, `/tmp/codex-fsharp-publish.log`.
- Native WebAssembly AOT publish: passed, including the client/core and FSharp.SystemTextJson; `/tmp/codex-fsharp-aot.log`, then incremental final publish `/tmp/codex-fsharp-aot-final.log`. Installed wasm-tools for SDK 10.0.401, runtime packs 10.0.12. Bolero.Templates 0.25.17 installed earlier; runtime packages pinned in project files.
- Chromium development smoke: **3 passed**, `/tmp/codex-fsharp-browser-boot.log`.
- Chromium published AOT smoke: **3 passed**, `/tmp/codex-fsharp-browser-aot.log`.
- Both smoke runs render every record from the complete pinned compiled Sources document, exercise a failed HTTP read and Retry, reject malformed required fields, and compare all **18 shared assets** against the pinned SHA-256 hashes. API requests are mocked; this is not live graph/side-by-side parity.
- Inventory: **235 entries**, **217 pending**, 18 shared assets; `--require-complete` intentionally fails. No source/component/UX entry is marked verified.

Red-before-green logs: generator `/tmp/codex-fsharp-generator-red.log`; application entry `/tmp/codex-fsharp-entry-red.log`; text `/tmp/codex-fsharp-anchors-red.log`; interrupted reads/Concord budget `/tmp/codex-fsharp-interruption-red.log`; required strings `/tmp/codex-fsharp-required-red.log`. The initial browser stylesheet red is retained in the task narrative: build links had the wrong development content root. The final MSBuild ContentRoot metadata resolves development and publish from the same source; it introduces no filesystem link or maintained asset copy. SDK task behavior was checked against [DefineStaticWebAssets](https://github.com/dotnet/sdk/blob/v10.0.401/src/StaticWebAssetsSdk/Tasks/DefineStaticWebAssets.cs).

The decoder fix closes a configuration category: serialization's omit-null option also made missing reference fields skippable in FSharp.SystemTextJson. Read and write options are now separate; missing required strings and explicit nulls fail, while absent optional fields still decode. [The converter's field helper](https://github.com/Tarmil/FSharp.SystemTextJson/blob/v1.4.36/src/FSharp.SystemTextJson/Helpers.fs) supplies the mechanism; tests and browser rejection prove it. An interrupted HTTP task is converted to an explicit failure inside the task boundary before entering Async; it cannot leave Elmish waiting without a completion.

## New existing-client finding

**Required wire fields are not enforced at the C# HTTP boundary.** At ace063d, RequiredResponses.GetRequired checks only that the outer record is non-null. An external test sends `{"id":"source","category":"text","title":"Title","license":"PD"}` to a typed SourceEntry read. It succeeds with required `what_it_is`, `what_we_built` and `licenses_row_key` null. Whole serialized result asserted; **1 probe passed** in `/tmp/codex-fsharp-required-csharp-probe.log`; source `/tmp/codex-F2-review/RequiredFieldProbe.cs`. This predates the FOCUS-2 diff and does not change its approved review verdict. No C# fix was made.

Category: structurally incomplete contract records entering successful client state. Sites: client/RequiredResponses.cs, generated record constructor/JSON metadata in client.ContractGenerator, and every AtlasClient/GraphExplorableClient read using that door. Proposed closure: derive required/nullable enforcement from the contract at generation and apply it at the single deserialization boundary; enumerate required-field omissions/nulls across generated schemas, including nested records and discriminated cases. Root-null guards alone do not close this category. The F# equivalent is refused; owner decides how intended malformed-answer parity and C# closure are recorded.

## Remaining work and handoff

Next: Reader/Concord whole-view tests and rendering, contents/pickers, MVU focus with bounded neighbour windows and word hydration, selection/storage/saved journeys, Kretzmann, World/map/time and split/follow. Consume reviewed FOCUS changes, retain a pinned reference, then run the actual clients side by side, the existing UX inventory, resource/usage/source laws and all completion gates. Root LICENSES.md/runtime distribution attribution needs reconciliation with Claude's ongoing licensing change; dependency notes are in client-fsharp/THIRD-PARTY.md. F# mutation tooling/coverage remains to be established under the owner's window/protocol.

There is no Codex-held lock and no remaining browser/server process from this checkpoint. The C# client is not retired. No completion note or review status is issued for this partial slice.

## Focus-state follow-up

After the foundation checkpoint, the typed graph interpreter and Elmish focus states bring the F# suite to **65 passed**, zero skipped (`/tmp/codex-fsharp-focus-state-green.log`). Red logs: `/tmp/codex-fsharp-graph-red.log`, `/tmp/codex-fsharp-focus-state-red.log`, `/tmp/codex-fsharp-focus-runtime-red.log`.

Graph.explorer implements the one Resolve read through generated generic element requests. It walks protocol pages, checks the returned identities/cardinality and root, rejects nonterminal empty pages and extra pages, and never admits a missing element. Resolved.ofElement also checks the node record's own version against the page root. Positions is the one kind/id comparison door; labels may change during renewal. FocusState is a closed union retaining the exact opening or traversal for Retry. Stale/closed requests cannot replace focus. Runtime interprets opening/Follow/Back/Renew through the tested algebra; the Back command makes no HTTP call.

This follow-up is state/transport groundwork. Its newest code has the 65-test build gate; the recorded AOT/browser gates above belong to the earlier 49-test foundation, and will be repeated after the next rendered slice. Focus UI, frontiers and Reader/Concord rendering remain pending. A-NOBLURB entered review during this work; Codex switches to that review before extending the migration.

## Reader/Concord text follow-up

A-NOBLURB is reviewed and approved separately as **d37b68d**, `lane/codex/A-NOBLURB-review`; the F# branch consumed its exact tree **e57542c** in merge **9855596** and the parity ledger deliberately re-pins it. No new implementation edit was made in C# or the shared contract/artifact paths.

Reader and Concord now render complete served UnitText, inline anchors, scalar Unicode offsets and red-letter spans. Reader labels come from served Contents, and headings use the served event, kind and continuation. Rows, headings and anchors emit typed OpenPosition messages; Enter/Space activate and unrelated keys do not. Both routes expose contents/text failures through the existing Retry view. This is a text-view slice; full routes remain pending in the parity inventory.

**80 F# tests pass**, `/tmp/codex-fsharp-reading-options-green.log`; initial six missing-view reds are `/tmp/codex-fsharp-reading-view-red.log`. **2 Chromium development reading checks pass**, `/tmp/codex-fsharp-reading-browser.log`, including whole visible text, anchors/red-letter runs and exact request sequences. The browser mock was corrected to the generated `/api/text` path and `n` parameter; those initial harness errors were not application fixes.

The browser exposed an actual decoder category: explicitly null optional records were rejected. Red whole-Concord response and generated-shape laws are `/tmp/codex-fsharp-reading-options-red.log`. The one JSON door now enables `deserializeNullAsNone` for skippable options; read/write settings remain separate. A law walks all **79 generated records**, checking all **84 optional fields** as exactly None, and all **245 required fields** with both omission and null rejected. Valid baseline records and mutated complete records are compared as whole values. The shared converter option closes this category across records, primitives, enums and lists instead of weakening required-field handling. Installed option signature and [upstream customizing documentation](https://github.com/Tarmil/FSharp.SystemTextJson/blob/v1.4.36/docs/Customizing.md) were inspected.

Native AOT and published-browser results for this slice will be recorded after completion. No side-by-side, full-UX, map or retirement claim is made. Paging, contents/pickers and focus presentation are the next work.
