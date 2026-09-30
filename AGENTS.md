# Working in this repository

Two agents work here around the clock:

- **Claude** is the controller and Lane A (client architecture).
- **Codex** runs Lanes B–D (maps, analyses, infrastructure).

The owner (Donovan) makes the rulings. Agents propose.

## Read first, every session
1. `docs/PRINCIPLES.md`. It binds every change. A plan or brief that conflicts with it is wrong.
2. `docs/superpowers/plans/2026-09-30-v1-roadmap.md`: the goal, the dates and the lanes.
3. `.superpowers/QUEUE.md`: the only list of work.
4. `.superpowers/LOCKS.md`: how to take a lock before touching a shared artifact.

## "go"
If the owner's whole message is "go" or "continue", read your GO file and follow it:
- Claude: `.superpowers/GO-claude.md`
- Codex: `.superpowers/GO-codex.md`

## The queue
- **Taking an item:** take the first `ready` item in your lane whose dependencies are `done`. Claim it by setting its status to `claimed:<agent>:<ISO time>` in a one-line commit and pushing. If the push is rejected, fetch and read the queue again: someone else may have taken it.
- **What an item gives you:** its base commit, the files it may touch, the gates it runs, and when it counts as done. Touch nothing else.
- **Finishing:** set it to `review` with the commit range. The other agent reviews it (PRINCIPLES 14b), then marks it `done`. Nothing is done on its author's word alone.
- **New work:** anything you find that needs doing goes into the queue as `proposed`. Never do it on the side.
- **Blocked on the owner:** write the question under **OWNER QUESTIONS**, mark the item `blocked:owner`, and take the next item. Never sit idle, and never guess on a ruling.
- **Running out of tokens:** before you stop, write a `handoff:` line on the item saying where you are and the exact next step, then commit and push.

## Lanes and the files each may touch
- **Lane A (Claude):** `server/`, `client/`, `graph-types/`, `contracts/`, `data/`, `tests/`, `docs/superpowers/{specs,plans}`.
- **Codex:**
  - all of the `map-generator` repo
  - in this repo, only `scripts/backup/`, `docs/superpowers/reports/`, `.superpowers/analysis/`, and whatever files a claimed item explicitly names
- **FOCUS batches assigned to Codex** name their files in the item.

## Critical sections (PRINCIPLES 21)
Take the lock first, following `.superpowers/LOCKS.md`. The locks are:
- **`heavy`:** full `cargo test --workspace`, the timing gates, the full Playwright suite, the mutation run.
- **`contract`:** regenerating `contracts/openapi.yaml` or `aqc.schema.json`, re-blessing pacts or fixtures, rebuilding `data/compiled`, anything that moves the version root, appending to `relations!`.
- **`land`:** landing commits onto `worktree-bible-atlas-m1`.

## One machine, shared (PRINCIPLES 19, 20, 23)
- **Working copies:** every item gets its own worktree (`git worktree add ../wt/<item-id>`) and its own target directory (`CARGO_TARGET_DIR=~/.cache/cargo-target/<agent>-<item-id>`). Use `cargo -j 4`.
- **Memory is the ceiling.** Pair unlike work: at most one Rust-heavy build per agent at a time, and prefer C# beside Rust.
- **Data:** never link `data/raw` into a worktree; copy it. Before any `git worktree remove`, check for zero symlinks inside the worktree.
- **Ports:**
  - Claude uses 8000 and 5000; Codex uses 8100 and 5100. The map-generator workbench uses 8090.
  - Never touch port 8080, the owner's demo.
  - Stop processes by the PID that owns the port, never by name, and only processes you started.
- **Scripts:** never edit a script while an instance of it is running.

## Commits and branches
- **Commits:** small, one behaviour each, with a message that states what is now true.
- **Codex:** pushes to `lane/codex/<item-id>` and never pushes to `worktree-bible-atlas-m1`.
- **Landing:** Claude lands reviewed work by cherry-picking it, holding the `land` lock. Claude's own work lands the same way, after Codex has reviewed it.

## Never
- Edit the KJV text.
- Force-push or rewrite history.
- Delete `data/raw` or `data/cache`.
- Commit a secret. Backup keys live only in the owner's password manager.
- Run the mutation gate outside the owner's window (`.superpowers/MUTATION-GATE-DEBT.md`).
- Re-bless a golden map view or a map fixture without the owner's approval.
