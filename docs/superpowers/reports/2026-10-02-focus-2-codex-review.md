# FOCUS-2 and house-style review

Verdict: **APPROVED** for `b3d7cfa..ace063d` on `lane/claude/F2`. This includes the FOCUS-2 implementation, the owner's verse-words ruling, ruled section ordering, and the merged A-FSTYLE work through `b2ff768`. No new blocking finding. No code, fixture, compiled artifact or Claude worktree was edited by this review.

## Independent evidence

| Check | Result |
|---|---|
| `dotnet test client.Tests` | 737 passed |
| `dotnet test client.ContractTests` | 55 passed |
| Rust `graph_api`, `contract_generation`, `contract_coverage`, `no_served_label_composition` | 123 + 13 + 3 + 3 passed |
| graph-types `cargo test --all-features` | 148 passed |
| Additional external xUnit/bUnit probes | 5 passed |

The independent probes cover repeated text-unit neighbours sharing one batched hydration and cached page; retry after failed word hydration; artifact movement between adjacency and words followed by renewal; Unicode scalar offsets and the served citation kind in UnitTextView; and stable ruled section ordering. Probe source: `/tmp/codex-F2-review`. Logs: `/tmp/codex-F2-{client,contract,rust,graph-types,probes}.log`. The first probe assertion used array equality inside a tuple; correcting the probe's equality comparison made it pass without an application change.

Rust work ran with `CARGO_TARGET_DIR=/home/donovan/mut/codex-A-F2-review`, `nice -n 10`, and `-j 4`, sequentially. Raw/cache inputs were copied into this worktree, never linked. The graph API run took 112 seconds, including its shared real-atlas initialization. No heavy/contract/land lock was taken. Claude's contract lock for A-NOBLURB was respected.

## Correctness and scope

Verse and Concord paragraph openings use their served node references. Text is presented from the served UnitText, with anchors retaining their served edge kind. The context hatch uses the served BibleRef and preserves split/follow URL behavior. Text-unit neighbour pages hydrate words in one element request, deduplicate their ids, cache by root and request, and refuse root mismatches. Paging remains bounded through the existing PageWindow and ServedPages doors; failed hydration does not poison the memo.

The owner-authorized removals in the plan are accepted, including verse parallels, passage membership, the book chip and the obsolete verse endpoint. Passage-only providers and their retirement ratchets remain deferred to their named FOCUS batches. FOCUS-2 does not close those remaining categories by itself.

The style work defines the new FocusView classes and composes row/edge steps under one Stepped fragment. Overpass and Atkinson Hyperlegible are bundled with their license texts and the recorded owner exception; font declarations use the two CSS tokens. ViewStyleLawTests and TypefaceLawTests pass. The ops queue records the owner's visual sign-off. This review does not substitute a new visual ruling for that sign-off.

## Principles 14b, 24a/24b and 27–27g

The D.R.Y./Haskell pass found no new blocker: the contract remains the vocabulary; labels and references use the existing compiled/reference readers; Fields and Stepped factor the repeated render structure; new Entry words are optional for non-text neighbours; presentation dispatch remains over the closed kind/form vocabulary.

The category pass checked text-unit legacy deletion, compiled references, compiled token offsets, reference/whole-read ratchets, live edge steps, and batched verse-word presentation against the plan and close report. Deletion and store laws enumerate their categories; the two legacy ratchets explicitly name the surviving files and retirement batches rather than claiming completion. Known F-63, F-65 and F-75 remain on ops and are not re-filed. The label-root defect remains the approved A-F39 item. Provenance naming remains the approved O-PROVENANCE follow-up.

Interaction ordering and presentation stay on the client; data-only references and citation labels stay in compilation. The new words query reads compiled, indexed token rows. Existing resource/root closures are preserved by the additional probes. Map tiling and future-size gate debt are unchanged by this batch. The application diff adds no prohibited comments; additions in test modules are AAA markers.

## Gate limits and handoff

I did not repeat the full workspace, full Playwright, timing, or mutation runs. The author supplied those gate results on ops; the latest full Playwright result there is 447 passed / 2 skipped / 3 carried failures. Existing browser flakes and deferred mutation are not newly introduced findings. The independent checks above exercised the changed server and client seams.

Claude: land the reviewed FOCUS-2/style tree using the normal land lock and squash protocol. Update the close report's final counts and remove its superseded FRONTIER-ORDER-1-pending-ruling statement: the ruled order and owner-approved look are now in the reviewed tree. A-NOBLURB's later backend removal is outside this pinned range and needs its own review. Codex will build the owner-requested F# Bolero/Elmish client in separate projects/worktree, consuming landed FOCUS changes as inputs; the C# client remains the parity reference.
