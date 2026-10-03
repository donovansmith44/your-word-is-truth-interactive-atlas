# Working in this repository

Two agents work here around the clock:

- **Claude** is the controller and Lane A (client architecture).
- **Codex** runs Lanes B–D (maps, analyses, infrastructure).

The owner (Donovan) makes the rulings. Agents propose.

## Read first, every session
1. `docs/PRINCIPLES.md`. It binds every change. A plan or brief that conflicts with it is wrong.
2. `docs/superpowers/plans/2026-09-30-v1-roadmap.md`: the goal, the dates and the lanes.
3. The queue: `QUEUE.md` on the `ops` branch (worktree `~/src/bible-atlas-ops`; `git -C ~/src/bible-atlas-ops pull` first). It is the only list of work. It lives off the code branch so `worktree-bible-atlas-m1`'s history holds code changes only (owner, 2026-10-01).
4. `.superpowers/LOCKS.md`: how to take a lock before touching a shared artifact.

## The machine
- **WSL2 (Ubuntu 22.04), 16 threads, 25 GB for WSL.** Repos: `~/src/bible-atlas`, `~/src/map-generator`.
- **Toolchains are user-local.** Run `. ~/.bible-atlas-env` in the same shell before any `cargo`, `dotnet`, `node`/`npx`, `ghc` or `cabal` command; non-interactive shells load none of them otherwise. It provides Rust 1.97.1 (pinned by `rust-toolchain.toml`), .NET 10, Node 24, GHC 9.6.7 + cabal (ghcup). `restic`, `bats`, `jq` and `codex` are in `~/.local/bin`.
- **Pushing:** WSL pushes to both repos through the GitHub CLI (`gh auth setup-git`, owner-authenticated). Verified 2026-09-30 with a push and a lock take/release.

## "go"
If the owner's whole message is "go" or "continue", read your GO file and follow it:
- Claude: `.superpowers/GO-claude.md`
- Codex: `.superpowers/GO-codex.md`

## The queue
- **Taking an item:** take the first `ready` item in your lane whose dependencies are `done`. Claim it by setting its status to `claimed:<agent>:<ISO time>` in a one-line commit on `ops` and pushing `ops` (both agents push `ops`; never force). If the push is rejected, fetch and read the queue again: someone else may have taken it.
- **What an item gives you:** its base commit, the files it may touch, the gates it runs, and when it counts as done. Touch nothing else.
- **Finishing:** set it to `review` with the commit range. The other agent reviews it (PRINCIPLES 14b), then marks it `done`. Nothing is done on its author's word alone.
- **Reviewing:** the 14b pass (D.R.Y., the Haskell bar) and the 24a category pass (a bug is a category: was the failed abstraction named, the side chosen, every site migrated, and the category CLOSED so an offender cannot be written — 24b). Offenders you find go under FINDINGS in the queue; the owner decides. Never fix one on the side.
- **New work:** anything you find that needs doing goes into the queue as `proposed`. Never do it on the side.
- **Blocked on the owner:** write the question under **OWNER QUESTIONS**, mark the item `blocked:owner`, and take the next item. Never sit idle, and never guess on a ruling.
- **Running out of tokens:** before you stop, write a `handoff:` line on the item saying where you are and the exact next step, then commit and push.

## Lanes and the files each may touch
- **Lane A (Claude):** `server/`, `client/`, `graph-types/`, `contracts/`, `data/`, `tests/`, `docs/superpowers/{specs,plans}`.
- **The backend** (PRINCIPLES 26): closed over the data — the server composes over the compiled artifact; a domain fact in server code (an id, a name, a date, a list, a special case, a pinned inventory) is an offender; it moves into `data/` with provenance and the code reads it through the graph.
- **Tools vs the system** (PRINCIPLES 26a): the ETL, the compile binary and `scripts/` are tools that shape raw sources into the artifact, once; the served system reads the artifact and never parses a raw file or scans text.
- **The client** (PRINCIPLES 25): composes over the generated contract types and nothing else; no domain parsing, formatting, scanning or arithmetic on the client; less client code is the direction.
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
- **Working copies:** every item gets its own worktree, `git -c core.autocrlf=false worktree add -b lane/<agent>/<item-id> ~/w/<item-id> <base>` (map-generator items: `~/w/mg-<item-id>`), and its own target directory, `CARGO_TARGET_DIR=~/mut/<agent>-<item-id>`. Use `cargo -j 4`, prefixed `nice -n 10`. Delete the target directory when the item lands.
- **Memory is the ceiling.** Pair unlike work: at most one Rust-heavy build per agent at a time, and prefer C# beside Rust.
- **The mutation run owns the machine.** Only one agent ever runs it (it holds `heavy`, with "mutation" in the lock message). While a `heavy` lock says "mutation", the other agent runs no `cargo`, `dotnet` or Playwright at all; only reading, reviews, analyses and docs. The run uses `scripts/mutants-parallel.sh -n 3` in WSL (not 4: each shard's test process holds a whole real graph, and 8 shards exhausted 31 GB on Windows). Before starting, `free -g` must show at least 18 GB available; otherwise wait. Stryker (C#) runs after the Rust shards finish, not beside them.
- **Data:** never link `data/raw` or `data/cache` into a worktree; copy them (`cp -r ~/src/bible-atlas/data/raw/. <wt>/data/raw/`, same for `data/cache`). Before any `git worktree remove`, `find <wt> -type l | wc -l` must print 0.
- **Ports:**
  - Claude uses 8000 and 5000; Codex uses 8100 and 5100 for servers it starts by hand (`atlas-server --port 8100`). The map-generator workbench uses 8090.
  - The Playwright suite always binds 8000 and 5000 (its config reuses a server already there). Whoever holds `heavy` runs it; stop your own servers on those ports first.
  - Never touch port 8080, the owner's demo.
  - Stop processes by the PID that owns the port, never by name, and only processes you started.
- **Scripts:** never edit a script while an instance of it is running.

## Code
- **Use tools that exist; do not hand-roll** (owner, 2026-10-03: "we use tools that exist rather than handrolling wherever possible"). Before writing a parser, decoder, codec, serializer, validator, file or format handling, a combinator set or any other general-purpose machinery, search for maintained, permissively licensed libraries and tools that already do it. Record the survey in the spec or plan: the candidates, their licences, maintenance, fit, the choice and why. Hand-rolling needs a written reason that every candidate was rejected. A library's errors map into our closed domain types at one door per library. Existing hand-rolled machinery is replaced wherever a surveyed library fits.
- **No comments in application code** (PRINCIPLES 9, owner 2026-09-30): not `//`, `///`, `//!` or `<!-- -->`, and no "why" exemption. Tests carry only `// Arrange`, `// Act`, `// Assert`. A description the published contract needs is a `#[schema(description = "…")]` attribute. Every review greps the diff for added comment lines.
- **Licensing** (owner 2026-09-29): ingest nothing that isn't public domain, CC0, or attribution-only permissive (CC BY 4.0, MIT, BSD, Apache-2.0). ShareAlike/copyleft (CC BY-SA, ODbL, GPL), NonCommercial, NoDerivatives and unlicensed sources are out. Cite the license; record attribution in `LICENSES.md`.

## Commits and branches
- **Commits (owner, 2026-09-30: "start squashing"):** one commit per task on the main branch, squashed at landing (`git merge --squash` or `cherry-pick -n` of the lane range), with a message that states what is now true. Lane branches may hold work-in-progress commits; they are never rewritten, only squashed when landed. Red-before-green evidence lives in the task report and the ledger, not in separate commits. Queue edits go to the `ops` branch, never to `worktree-bible-atlas-m1`; the code branch carries no queue or ledger commits.
- **Codex:** pushes to `lane/codex/<item-id>` and never pushes to `worktree-bible-atlas-m1`.
- **Landing:** Claude lands reviewed work by cherry-picking it, holding the `land` lock. Claude's own work lands the same way, after Codex has reviewed it.

## Never
- Edit the KJV text.
- Force-push or rewrite history. The one exception is taking a lock (`LOCKS.md`), whose `--force-with-lease=<ref>:` only ever creates a branch that doesn't exist.
- Delete `data/raw` or `data/cache`.
- Commit a secret. Backup keys live only in the owner's password manager.
- Run the mutation gate outside the owner's window (`.superpowers/MUTATION-GATE-DEBT.md`).
- Re-bless a golden map view or a map fixture without the owner's approval.
