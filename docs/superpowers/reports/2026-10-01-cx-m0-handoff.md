# CX-M0: WSL cleanup implementation and remaining gate

Implementation pushed in map-generator: `lane/codex/CX-M0`, range `6608db4..9f99a90`. **Not ready to land: one required fixture gate remains red.**

The instructed Stage 1 fast-forward and TOOLCHAIN-1 cherry-pick are included. Seven broken Windows-specific atlas paths now consume one workspace git dependency pinned at `e49593567e05bf61210a842b68ef85bb6c75514e`, with Cargo.lock committed. It compiled against the then-current atlas main; the current `0887c03` main has the identical graph-types tree (`d8e4ef52320bcf5853bebf1c4011c85e11ed527c`). No compatibility fallback or graph type change was needed. The Linux launcher uses the isolated target directory, detaches the process, records its PID, and refuses to stop a foreign executable. The Windows path remains available but was not executed on WSL. The unrelated PDF is removed from HEAD only.

Red-before-green evidence: the original dependency failed manifest loading; the fixed workspace builds. Scratch launcher tests for start/repeated start/stop and refusal to kill a foreign PID failed before the launcher change and pass afterward. They live at `/tmp/codex-CX-M0-demo-test.py`.

Required gates already run against this implementation:

- Rust workspace: **187 passed, 1 failed, 1 ignored**. The sole failure is `map-encoders::tests::limb_fixtures_match_rust`.
- Haskell contract suite: **368 examples, zero failures**. Both contract check and vocabulary commands passed; semver shell laws passed **6/6**.
- `make contract-gates` reaches the same failing Rust fixture.
- `make demo`, HTTP `/api/meta` on 8090 and `make stop` passed. No Codex demo remains running.
- Full browser golden check has not been run; the required fixture gate is already red. Run remaining gates after the fixture question is resolved. No fixture has been re-blessed.

The failing law compares pretty-printed floating-point JSON byte-for-byte against the committed Windows-produced fixture. A scratch regeneration found **23 numeric differences, maximum absolute difference 2.220446049250313e-16**, with **zero shape/key/boolean differences**. The committed fixture is unchanged. The focused law was rerun on 2026-10-01 and remains red, independently of the missing-path problem.

A clean worktree also needs ignored `data/canon/` generated using `map-compile build` from committed vendor inputs; no source refresh was performed. Cabal initially omitted tests during dependency solving; the ignored `contracts/runner/cabal.project.local` supplies `tests: True` and `jobs: 4`. Logs and scratch numeric probe are under `/tmp/codex-CX-M0-*`.

The remaining failure is in `crates/map-encoders/src/tests.rs`, outside CX-M0's explicit allowed files. AGENTS.md says to work exactly to the item's files and never widen scope. Proposed owner ruling **O-M0-LIMB**: authorize a scoped follow-up to make the cross-platform fixture law compare numerical geometry with a justified bound while retaining exact structure/discrete values and the existing fixture. Do not blindly re-bless or change geometry to fit floating-point text. This proposal requires numerical-error justification and red tests for meaningful drift; no tolerance is selected here.

The queue migration omitted this existing blocker and the analysis handoffs; they are being restored on ops. CX-M1/CX-M2 remain dependent on CX-M0's reviewed landing. The owner's errata seed is recorded: **1446/1406 BC first, possibly 1200 BC; Edom extent and early Davidic content; local geography only; findings cite Scripture or atlas facts and do not fix data.**

Authored cleanup commit `59a9006`: 285 inserted / 26 deleted text lines (including 218 lockfile lines), plus PDF removal; detached-launch follow-up `9f99a90`: 1 insertion / 1 deletion. Imported Stage 1 and TOOLCHAIN-1 changes are separate from those figures. No shared locks held.

Handoff: Claude can review the path/launcher implementation now, but should not land it as gate-clean. Resolve O-M0-LIMB, finish the scoped fixture follow-up, run remaining required gates, then move CX-M0 to review. Do not merge old atlas queue commits from `lane/codex/CX-M0`; take this report alone if needed. All further queue edits live on ops.
