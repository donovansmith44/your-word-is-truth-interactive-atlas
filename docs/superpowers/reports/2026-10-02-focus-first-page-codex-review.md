# Codex re-review of FIX6 at 320afac

Verdict: **F-74's predecessor behavior is fixed; changes requested for a new site of existing F-18 under PRINCIPLES 6/14b/25**. No runtime regression reproduced. No duplicate finding number is assigned to the existing paging-default category.

Reviewed `eea9023..320afac` on `lane/claude/FIX6`, with the earlier FOCUS-1/FOCUS-6/fix reviews as context. The review head is **320afac**; `lane/claude/F1-int` still points to **eea9023**, so this verdict covers FIX6 as an extension of the reviewed stack. Read current main AGENTS, GO-codex, PRINCIPLES and LOCKS, pulled ops, and checked remote locks before testing.

## F-18 additional site — Important: the first-page default is hand-declared on the client

At this head, `graph-types/src/adjacency.rs:13` declares `Cursor::FIRST`. Both operations publish its value through generated OpenAPI (`server/atlas-contract/src/graph.rs:195,415`). But `client/Exploring/ServedPages.cs:9` independently declares `public const int FirstPage = 0` and uses it to canonicalize cache keys at line 21.

`client.Tests/Contract/FirstPageTests.cs:9,20–28` proves that the literal currently equals the defaults on two hand-listed paths. That is a useful drift check, but it verifies a second maintained declaration rather than deriving the consumer's value from its contract. `client.ContractGenerator/ContractGeneration.cs` generates DTOs with client classes disabled; `Program.cs` emits those DTOs and does not emit query-parameter defaults. The paging default therefore does not enter the client through the existing generated contract code.

This is a new site in F-18's already-filed category, **client restatement of server paging defaults**, introduced by FIX6. The finding is architectural under the expressly required 14b pass; all current runtime tests are green. It does not dispute zero as the correct first-page cursor, the owner's 20/40 interaction policy, or the need to canonicalize the aliases.

Proposed closure: generate the consumed paging default from the committed OpenAPI at build time, and make the page-store boundary consume that generated value. Discover the relevant paged operations from the document/type structure instead of a second maintained path list; gate the declared common default or generate per-operation defaults if they differ. Include a generator test showing that a changed contract default changes generated consumer code without a manual C# constant edit. This is the same one-declaration guarantee requested by F-18, not a proposal for another server endpoint or per-page metadata field.

## F-74 behavior closure passes

The shared `Cursor` type makes the first request total and centralizes predecessor semantics. Both stores and the element read use it. Only the first page returns null; the second names zero explicitly. Nullable/zero request aliases share one client cache entry, and `PageWindow` consults the served predecessor when deciding how to move back.

The two independent real-router failures from the previous review were retargeted to this head and extended with whole-response round trips. Both now pass:

| Artifact read | First page | Second page | Reading its predecessor |
| --- | --- | --- | --- |
| Aaron `mentioned-in`, limit 20 | 20 entries; next 20; previous null | 20 entries; next 41; previous 0 | Exactly the first response |
| 201 element ids | 200 elements; next 200; previous null | 1 element; next null; previous 0 | Exactly the first response |

Two further independent tests walk/retrace every page of the real neighbour read at limits 1, 20 and 200, and the real element read over 401 ids (three pages, partial final page). Each checks the complete predecessor response and verifies that omitted cursor and explicit zero return the same first response. All four pass.

The author's both-store law now enumerates **positions**, including edge positions, and checks first/second/third/partial-final pages. Its former expectation of null on page two is removed. The independent run passes. The separate every-cursor/every-limit store parity law passes, including limit zero; this internal zero-width case keeps the current cursor as its predecessor when earlier entries exist. HTTP neighbour reads clamp zero to one, so that internal behavior is not a new HTTP defect.

## Earlier closures and principles

The ten previous independent client probes were rerun. The fake's predecessor was updated to the new explicit-zero wire semantics; no expected rendering/root outcomes were relaxed. They pass: original Back alias, initial failure/retry, initial bounded read and surface transition; serialized More; 20/40 rendering and position; same-id/new-root presentation; root consistency in mentions; complete 300/3,000/30,000/301-row round trips after cache eviction; and refusal/renewal in the rendered production person section. F-68/F-70 remain behaviorally closed.

The predecessor algorithm is shared rather than copied between three read paths, and a `NonZeroUsize` reaches the store-specific backward lookup only after zero-width handling. The Cursor migration through adapters, CLI and served readers is consistent; no new domain derivation or geometry pipeline change was found. Indexed backward reads and bounded client navigation remain the 27/27b/27d/27e direction. The remaining 14b hit is the client restatement described above; the constant-equality law does not replace mechanical generation under principle 6.

The added-comment scan over the full increment finds **six AAA markers and zero new non-AAA comments**. `git diff --check` passes. Known F-55/F-63/F-65/F-66/F-72/F-73 and older comment debt are not re-filed or waived.

## Independent checks and limits

| Exact head 320afac | Result |
| --- | --- |
| Full client suite | 730 passed |
| Full client contract suite | 55 passed |
| Graph-types default-feature suite | 133 passed |
| Prior independent client behavior/resource probes | 10 passed |
| Actual-artifact HTTP predecessor probes and full walks | 4 passed |
| Both-store predecessor category law | 1 passed |
| Every-cursor/every-limit store parity law | 1 passed |

The author's full workspace, export/contract and browser gates were read from ops, not independently rerun: workspace 1,489/0; graph-types 133; client 730; contract 55; contract gate PASS; browser 456 pass / 3 skip / 5 fail, with the reported carried failures and XSCRIPT hover-dismiss failures recorded under F-55. This review does not certify a green browser suite or reclassify those known failures.

Detached worktree: `/home/donovan/w/A-F1-cursor-review`. Its own Rust target: `/home/donovan/mut/codex-A-F1-cursor-review`. Used the pinned environment, `nice -n 10`, `-j4`, one Rust build at a time, and sequential .NET builds. Independent scratch projects are `/tmp/codex-320afac-review/Review.csproj` and `/tmp/codex-320afac-server-review/Cargo.toml`; logs `/tmp/codex-320afac-{client-tests,contract-tests,graph-types,repros,cursor-repros,sqlite-previous,sqlite-limits}.log`. The real-router tests use the committed artifact's serving loader, without raw fallback or owner ports. No app source, fixture or compiled artifact was edited; no lock or server is held.

Handoff: close F-74's behavioral finding. Before landing, resolve the newly introduced F-18 default declaration or record the owner's disposition; then provide the next head. The earlier resource/root regressions need no further instance fixes. Report and ops verdict will be pushed on the review lane and ops respectively.
