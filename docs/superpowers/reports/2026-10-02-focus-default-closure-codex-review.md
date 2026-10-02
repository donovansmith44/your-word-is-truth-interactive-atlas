# Codex re-review at ea0ba5d

Verdict: **approved at ea0ba5d**. The F-18 first-page-default site raised in the preceding review is closed. F-74 and the previously verified resource/root/interaction closures remain intact. No new findings.

Reviewed `320afac..ea0ba5d` on `lane/claude/FIX6`, extending the cumulative FOCUS-1/FOCUS-6/fix reviews. The reviewed stack is `678a0d2..ea0ba5d`; this pass independently verifies the new increment rather than rerunning every earlier gate. Current main AGENTS/GO-codex/PRINCIPLES/LOCKS were checked, ops was pulled, and remote locks were empty before builds. No application changes are included in this report branch.

## F-18 site closure

`ServedPages.FirstPage` is deleted. The contract generator discovers query cursors through OpenAPI operations and emits `BibleAtlas.Client.Contract.PagedReads.FirstPage` from the published default. Missing, inconsistent or non-integral defaults refuse generation. The existing build appends this generated declaration to untracked `Wire.g.cs`; the page-store boundary consumes that generated value. Inspection of the rebuilt output confirms the actual document produces `FirstPage = 0`.

The old two-path list is also gone from `FirstPageTests`; the law discovers the document's operations and checks their cursor defaults against the generated value. The generator tests demonstrate a nonzero default and refusal of inconsistent/missing values.

Four independent generator probes use three unfamiliar operations:

- Uniform defaults of 7 and 27 each generate the complete expected consumer declaration with that value, without a manual C# constant edit.
- Adding an unfamiliar third operation with a conflicting default fails generation.
- Adding an unfamiliar third operation without a default fails generation.

All four pass. This verifies the requested one-declaration/document-discovery closure rather than merely checking that two handwritten zeros agree.

`IExplorableClient.DefaultPageSize` is deleted too. Requests now use the client's existing `Affordances.PageSize`, the owner's interaction policy. The server's unpublished `DEFAULT_EDGE_LIMIT` and any broader disposition of original F-18 remain existing queue matters; this report closes the newly introduced first-cursor restatement site and credits removal of the duplicate client page-size declaration. It does not declare all old queue debt resolved.

## Regression checks and principles

The ten earlier independent client probes pass at this head: original Back alias, failed opening/retry, bounded initial read and surface transition; serialized More; 20/40 rendering/position; same-id/new-root presentation; mentions root consistency; full round trips over 300/3,000/30,000/301 rows after eviction; and refusal/renewal in the rendered production person section. The scratch fake signatures were updated for deletion of `IExplorableClient.DefaultPageSize`; expected behavior was unchanged.

The required 14b D.R.Y./Haskell-bar pass finds the previously reported duplication removed. Contract-derived configuration now enters through generated code, while the client retains its interaction policy. There is one cache canonicalization boundary and one generated first-page value. The 24a/24b category check covers discovery of unfamiliar operations and refusal of incomplete/disagreeing declarations. Generation occurs in the build tool, adding no runtime parsing, graph scan, request or unbounded cache under 25/27–27g.

No Rust, server, graph-types, committed contract or data file changed in `320afac..ea0ba5d`. The independently verified F-74 HTTP/both-store behavior at `320afac` therefore has no new server implementation to recheck in this increment. The client cache and navigation paths are covered by the rerun client suite and independent probes.

The incremental added-comment scan finds **six AAA markers and zero added non-AAA comments**. `git diff --check` passes. Known F-55/F-63/F-65/F-66/F-72/F-73 and the broader original F-18 remain separately tracked; no new instance of those known findings is reported here.

## Verification and limits

| Independent check at ea0ba5d | Result |
| --- | --- |
| Full client suite | 733 passed |
| Full client contract suite | 55 passed |
| Prior client behavior/resource probes | 10 passed |
| Generator change/discovery/refusal probes | 4 passed |

Fresh .NET builds ran sequentially in detached `/home/donovan/w/A-F1-default-review`. Independent scratch project: `/tmp/codex-ea0ba5d-review/Review.csproj`. Logs: `/tmp/codex-ea0ba5d-{client-tests,contract-tests,repros}.log`. No Rust build was needed for this client/generator-only increment; no Rust target or shared lock was reused for a build. No owner port, server, fixture, compiled artifact or application source was changed by this review.

Author evidence on ops records client 733, contract 55, graph-types 133 and contract gate PASS. Graph-types/export/full workspace/browser gates were **not independently rerun at this head**. Prior independent `320afac` evidence remains in `2026-10-02-focus-first-page-codex-review.md`: graph-types 133, actual-artifact HTTP walks/probes 4, predecessor law 1 and every-cursor/limit parity law 1 passed. Previously recorded browser failures remain known debt; approval does not claim an all-green browser run.

Handoff: **A-F1/A-F6 stack through ea0ba5d passes Codex review**. Claude can integrate FIX6 and land the reviewed tree under the established lock/squash protocol. If another change alters that tree, provide its head for review. No review locks or servers held; other Codex lane handoffs remain on ops.
