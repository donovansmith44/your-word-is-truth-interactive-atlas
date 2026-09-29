# The next mutation run

**Owner, 2026-09-29:** mutation gates are not run per batch while the owner is
spending tokens on implementation ("if we run into any mutation gates before
9am tomorrow morning, skip it"), and there is nothing to "make up" afterwards —
"running them once or twice gives enough information". A mutation run measures
the CURRENT tests against the CURRENT code; it does not care which batch changed
a line. One run over the union of every batch since the last run is the same
measurement as one run per batch, and it also sees the seams between batches.

So: **one row, not a ledger.** When the next run happens, it is based here.

| last run | base for the next run | batches since |
|---|---|---|
| CONTRACT-1, 2026-09-29 (Rust 1376 mutants 41/41 killed; C# 146/146) | `13111dd` | FOCUS-0 (closed 2026-09-29, base `dd72809`), … (append as they land) |

Command: `bash scripts/mutants-parallel.sh -n 4 --base 13111dd` (N=4 per R46),
then Stryker per `.superpowers/sdd/2026-09-26-contract1b-client/task-8-stryker-report.md`.
It is a critical section (PRINCIPLES 21): one holder, never two at once.
Survivors get tests, never `#[mutants::skip]`; equivalents get a written reason.
