# Owed mutation gates

**Owner directive, 2026-09-29 07:27 local:** "if we run into any mutation
gates before 9am tomorrow morning, skip it. I don't want to spend time
waiting on tests to run when I have a lot of tokens to spend left."

**Window: 2026-09-29 07:27 → 2026-09-30 09:00 local.** A batch that closes in
this window does NOT run its mutation gate. It records the debt below and
closes, reviews and pushes as normal. Every other gate still runs (contract,
semver, timing, Playwright) — they are minutes, not hours.

**Deferred, not cancelled.** PRINCIPLES 3 still binds: 100% on every line a
batch changed, minus recorded equivalents. The debt is paid after the window.
Each row carries the BASE COMMIT (PRINCIPLES 22) so the run measures exactly
the lines that batch changed, however many batches have landed since.

Run one with:
`bash scripts/mutants-parallel.sh -n 4 --base <base>` (N=4 per R46; memory is
the ceiling on this box), then Stryker per
`.superpowers/sdd/2026-09-26-contract1b-client/task-8-stryker-report.md`.
Gates are a critical section (PRINCIPLES 21): one at a time, never two at once.

| batch | base commit | HEAD at close | Rust owed | C# owed | paid |
|---|---|---|---|---|---|
| CONTRACT-1 (1a+1b) | 78f51ff | 13111dd | no — RAN, 1376 mutants, 41/41 killed | no — RAN, 146/146 | 2026-09-29 |
