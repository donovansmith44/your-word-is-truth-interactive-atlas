# Codex re-review at eea9023

Verdict: **changes requested for F-74, the new backward-cursor contract**. The remaining F-68 cursor-history and F-70 person-mentions findings are **closed by this head**. Their independently reproduced failures now pass. F-67/F-69's previously reviewed closures remain intact.

Reviewed `8bf2292..eea9023` on `lane/claude/F1-int`, against the prior whole-stack review of `678a0d2..caddd75` and the fix/PAGES20 review at `8bf2292`/`f9789f1`. This pass includes PAGES20, NOFIELD, FIX3, FIX4 and FIX5. The remote integration head was re-fetched and remained `eea9023` before this report. No application fix is included in the review branch.

## F-74 — Important: null conflates no previous page with the cursor of the first page

Sites at the reviewed head:

- `server/atlas-contract/src/graph.rs:169,184,290`: both operation descriptions publish the first-page test; the elements handler discards the zero predecessor.
- `graph-types/src/adjacency.rs:105` and `server/atlas-graph/src/sqlite/snapshot.rs:253`: both stores answer null when the preceding page is the first page.
- `contracts/openapi.yaml:189,440`: the committed API document repeats the same guarantee.

The published contract says that absence of `previous` means the current page is the first page. But a client can follow a non-null `next` to the second page and receive `previous: null` there too. These are two different navigation states collapsed into the same value: no preceding page, and a preceding page whose starting cursor is represented locally by null.

Two independent tests called the real router over the committed artifact, without mocked responses or owner ports:

| Read | First page | Page obtained by following `next` |
| --- | --- | --- |
| Aaron, `mentioned-in`, limit 20 | 20 entries, next 20, previous null | 20 entries, next 41, previous null |
| 201 element ids | 200 elements, next 200, previous null | 1 element, next null, previous null |

Both tests fail the documented law that a non-first page identifies its preceding page. The green author HTTP/store tests intentionally expect null on **both** first and second pages, so they do not enforce the advertised boundary.

The current UI's More/Less behavior passes: `PageWindow` maintains local counters and treats null as the first-page request when going back. This finding concerns the new public protocol; a consumer following its documented availability test hides backward navigation one page too early. It is separate from F-68's resource bound and F-70's artifact consistency.

Proposed closure: give every non-first page an explicit usable predecessor, reserving null for no predecessor, consistently across elements and both neighbour stores. A cursor of zero can identify the first page; canonicalize its null/zero request aliases at the page-store boundary so the owner's one-cursor-per-page requirement still holds. Gate the public law for first/second/third/partial final pages, and the round trip from the second page to the first. If a different navigation representation is chosen, publish its semantics explicitly rather than making consumers reconstruct them from an ambiguous nullable integer.

## Closures verified

- **F-68:** `_earlier` is gone. The retained block list and its predecessor/successor cursors are bounded. Independent complete forward/backward walks at 300, 3,000 and 30,000 rows, plus a 301-row partial final page, hold at most four cursor slots and recover every expected window after the 32-page cache has evicted earlier pages. The author's per-turn allocation/read law also passes in the 728-test client suite. The visible 20/40 window and position line remain correct.
- **F-70:** the raw-client window overload is deleted; the internal opener is reached through `Paging.Window(PresentationRequest, EdgeKind)`. `PersonCardAndMentionsSection` receives the resolved current element and uses this door. An independent rendered production-section test changes the artifact before More: all 20 retained entries stay on root A, no root-B entry is rendered, and renewal is requested exactly once. The component reproduction also passes. The enumerating door/root law covers opening, More, Less, retry and cached revisits.
- **F-67:** the independent delayed two-click component reproduction still has one page in flight and finishes on the requested rows 21–60. No stale completion overwrites the result.
- **Original F-57–F-60:** the original Back alias, failed initial load/retry, bounded initial read and surface-transition reproductions were adapted only for the newer element-page envelope and rerun; all four pass. Shared identity/comparer closure remains covered by the full client suite.
- **FIX4/FIX5:** the first page has one cache cursor; legacy section context is supplied as a required argument; the invalid `RefsList.OnMoved` parameter is removed. The component parameter law and client suite pass. Browser changes wait for section presence before reading their order.
- **NOFIELD:** type descriptions remain attributes, field doc comments are removed, and the new wire/schema source law passes. This does not declare all older plain comments or A-STRIP closed.

## Principles 14b, 24a/24b and 27–27g

The paging changes reduce duplicated machinery: one root-aware window entry point, one bounded page store, and shared `PageControls` for generic and person views. Position/window policy is interaction-dependent client state; backward edge discovery is an indexed store read. Compiled edge counts remain the server's summary source. This pass introduces no map geometry change and does not claim a new 27g map proof.

The Haskell-bar issue is F-74: an optional cursor carries two distinct meanings. The UI can compensate using its local counters, but the public value does not carry the distinction its description promises. Closure must hold at the protocol boundary and across both stores, rather than patching a particular button. No additional D.R.Y. offender was found in this increment.

The rename-aware added-comment scan found **54 AAA markers and zero added non-AAA comments** across client, client tests, server, graph-types and browser tests. `git diff --check` passes. Previously filed F-55/F-63/F-65/F-66/F-72/F-73 remain existing queue items and are not re-reported or waived.

## Independent verification and limits

| Check at eea9023 | Result |
| --- | --- |
| Client suite | 728 passed |
| Client contract suite | 55 passed |
| Independent bUnit/resource probes, including four original reproductions | 10 passed |
| Both-store predecessor walk (`atlas-graph`, `sqlite_laws`) | 1 passed; confirms the implementation's chosen null semantics |
| Artifact HTTP predecessor walk (`atlas-contract`, `graph_api`) | 1 passed; same limitation |
| Wire/schema doc-comment laws | 2 passed |
| Independent documented predecessor availability tests, actual artifact router | 2 reproduced failures, F-74 |

Full workspace, timing, contract-export and browser gates were read from the author's ops entry, **not independently rerun**: 1,489 Rust, 728 client, 55 contract; contract gate PASS; timing 11/11; browser 457 pass / 4 skipped / 3 known reds. This report does not label those browser reds new regressions or claim an all-green browser run.

Tests used the detached worktree `/home/donovan/w/A-F1-final-review`; Rust used its own `/home/donovan/mut/codex-A-F1-final-review`, `nice -n 10`, `-j4`, one Rust build at a time. .NET projects ran sequentially. Logs: `/tmp/codex-eea9023-{client-tests,contract-tests,repros,sqlite-previous,http-previous,doc-comment-law,cursor-repros}.log`. Independent scratch projects: `/tmp/codex-eea9023-review/Review.csproj` and `/tmp/codex-eea9023-server-review/Cargo.toml`. The latter uses the reviewed workspace lockfile's package versions and calls `load_from_data_dir(...).into_router(None)`; an initial scratch attempt incorrectly used the raw-data fallback and was corrected before recording the two protocol failures. No app sources, compiled artifacts or fixtures were edited; no shared lock or server is held.

## Reproducing F-74

With a net10.0/Rust setup sourced from `~/.bible-atlas-env`, the scratch Rust crate references the reviewed `atlas-contract` path, plus axum 0.8, tokio 1 (macros/rt-multi-thread), tower 0.5 (util), http-body-util 0.1 and serde_json 1. Its router is the actual serving path:

```rust
let app = atlas_contract::load::load_from_data_dir(
    std::path::Path::new("/home/donovan/w/A-F1-final-review/data/compiled"),
).unwrap().into_router(None);
```

Send each request using `tower::ServiceExt::oneshot`, require HTTP 200, and decode its JSON body. First request `/api/node/Person:aaron_1/edges?kind=mentioned-in&limit=20`, then the same request with `cursor` equal to its `next`. Assert the second response's `previous` is non-null: **fails**. Repeat for `/api/elements?ids=` with 201 copies of `text-unit:GEN.1.1`, passing its first response's `next` as cursor: the same assertion **fails**. Both are the first/second-page distinction directly stated in the published operation descriptions.

Handoff: F-68/F-70 residual closures pass. Keep A-F1/A-F6 in review for F-74; reconcile the predecessor protocol and its boundary laws, then provide the new integrated head. No new owner ruling or fixture blessing was taken by this review; no locks held.
