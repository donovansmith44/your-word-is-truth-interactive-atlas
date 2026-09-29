# RAW-INTEGRITY Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `data/raw` the integrity the compiled artifact already has: a committed merkle manifest whose leaves are the fetched files and whose per-directory nodes roll up to one root per dataset, a verifier that refuses a drifted or half-written tree, a fetch script that checks what it downloaded instead of trusting any file that exists, and one archive copy outside every git worktree.

**Why now:** on 2026-09-28 a throwaway worktree junctioned `data/raw/*` and `git worktree remove --force` followed the junctions, emptying nine dataset directories (408 MB → 34 MB). Nothing detected it, nothing could prove what had been lost, and the refetch could not be shown to have restored the same bytes. A second failure in the same hour made the other half of the case: the refetch died mid-copy and left `brain-fuel-bible/lexicon` half-populated and `morph/` empty, a state `fetch-raw.ps1`'s own `if (-not (Test-Path …))` guard would have accepted forever.

**Architecture:** A new `graph-types` module computes the tree (pure, zero-dependency, reusing the crate's own SHA-256 and the `root_of_lines` construction the compiled manifest already uses); `bibex` gains a raw section in `verify` plus an explicit `raw bless` writer; `data/fetch-raw.ps1` calls the verifier; `scripts/archive-raw.ps1` makes the off-worktree copy. The raw root is recorded in `data/compiled/manifest.toml` as provenance but is deliberately NOT part of its root preimage.

**Tech Stack:** Rust 1.97.1 workspace under `server/`; `graph-types` (zero non-optional deps); `atlas-cli` (`bibex`); PowerShell for the fetch/archive scripts.

**Spec:** none — this plan is its own design record; §Design decisions below is the spec surface, and the owner has ruled on the two open ones (D1, D2).

## Global Constraints

- `docs/PRINCIPLES.md` binds: TDD, whole-body assertions, named constants, `why` comments only, zero dead code, 100% mutation coverage on changed lines at the batch close.
- **The compiled manifest root does not move.** `data/compiled/manifest.toml`'s `root` is computed from `manifest_lines` today; the new `raw_root` field is data, never preimage. `server/atlas-graph/src/sqlite/manifest.rs`'s self-verification must still pass on the committed artifact untouched, and `bibex verify` on today's `data/compiled` must stay green.
- **Vendored raw data under `data/raw/**` is never edited** — this plan only reads it.
- **`data/raw/*` stays gitignored** except `README.md` and the new `MANIFEST.toml` (`.gitignore` gains one `!` line).
- Hashing 297 MiB is seconds; the verifier must stay under the timing budget of the gate it joins (measure, record in `server/BENCHMARKS.md`).
- Commit per task; do not push without the owner's word.

## Design decisions (recorded, not re-opened)

- **D1 — the raw root is a sibling of the compiled chain, not a link in it.** Making the compiled root depend on the raw root would break the property `server/atlas-graph/src/sqlite/manifest.rs:1-3` is built on (a byte-identical rebuild on another day yields the same root), because a refetch that changes nothing compiled would still move it. Instead `manifest.toml` carries `raw_root` as an informational field, excluded from `manifest_lines`, so "which inputs produced this artifact" is answerable without coupling the two identities.
- **D2 — leaves carry the full SHA-256, nodes carry the 128-bit domain-prefixed hash.** A raw file's bytes are its whole identity (there is no logical/transport split as in the compiled sections), so corruption detection belongs at full width; the roll-up mirrors `root_of_lines` exactly so there is one node construction in the repo, not two.
- **D3 — `bytes` is recorded per file but excluded from every preimage**, being redundant with the leaf hash; it exists so a human reading `MANIFEST.toml` can see the shape, and so the fetch script can reject a truncated download cheaply before hashing.
- **D4 — this manifest is a pin that stays read.** `contracts/atlas-graph-contract/graph/version-root.feature:7-13` refuses a routinely re-blessed hash ("a check that is re-blessed as a matter of routine stops being read — this project has the scar"). Raw inputs change only on a deliberate refetch, which has happened twice since 2026-08-17; when the root moves, the diff is the thing the owner wants to read. The writer is therefore a separate explicit verb (`bibex raw bless`), never a `--fix` on the verifier.

## Deletion inventory

- `data/fetch-raw.ps1`'s `Fetch` early-return (`:9`, `if (Test-Path $path) { "have"; return }`) — replaced by a verify-then-skip, so an existing file is skipped only when it matches the manifest.
- `data/fetch-raw.ps1`'s two `if (-not (Test-Path …))` vendoring guards (`:60`, `:79`) — replaced by subtree-hash guards; a half-copied `lexicon/` must re-copy.
- `data/raw/README.md:38-50`'s hand-maintained byte-size listing — superseded by `MANIFEST.toml`; the prose keeps the *shape* documentation, which is its stated purpose, and gains one line pointing at the manifest for sizes.
- Nothing else. `server/atlas-graph/tests/version_root_regression.rs`'s `EXPECTED_VERSION_HEX` stays: it pins compiled output, which is a different claim, and this plan gives its failure message the diagnosis it lacks (Task 5).

---

### Task 1: The tree — `graph-types` computes leaves, nodes and a root

**Files:**
- Create: `graph-types/src/raw_manifest.rs`, `graph-types/tests/raw_manifest_vectors.rs`
- Modify: `graph-types/src/lib.rs` (module declaration)

**Interfaces:**
- Produces: `RawLeaf { name: String, sha256: String, bytes: u64 }`, `RawNode { name: String, hash: String, children: Vec<RawEntry> }`, `RawEntry` (leaf or node), `fn node_hash(entries: &[RawEntry]) -> String`, `fn leaf_line(&RawLeaf) -> String`, `fn node_line(&RawNode) -> String`.
- Consumes: the crate's own `sha256` module (`graph-types/src/sha256.rs`) and whatever `sections.rs` names its 128-bit domain-prefixed helper (`sha256_prefixed_128`) and its line roll-up (`root_of_lines`) — reuse both; declare a `RAW_DOMAIN_PREFIX` distinct from the sections' prefix so a raw node hash can never collide with a section hash.

- [ ] **Step 1: Write the failing vectors**

`graph-types/tests/raw_manifest_vectors.rs`: hand-computed vectors, whole-body, no I/O (this module is pure):
- a single leaf's line is `<name>|<64 hex>\n` (D3: no `bytes`)
- a node over two leaves equals `sha256_prefixed_128(RAW_DOMAIN_PREFIX, "a|<hash-a>\nb|<hash-b>\n")`
- **order is by name, not insertion** — assert a node built from `[b, a]` equals one built from `[a, b]`
- a nested node's line is `<name>|<node hash>\n`, so a change deep in a subtree moves exactly its ancestors' hashes and nothing else (assert the sibling's hash is unchanged — this is the property that makes the tree worth having)
- the empty node has a defined hash (state it; an empty fetched directory is a real state and must not be a panic)

Run: `cd graph-types && cargo test --test raw_manifest_vectors` → fails to compile (`raw_manifest` does not exist).

- [ ] **Step 2: Implement**

Pure, no `std::fs` in this module — the tree is computed from values so it is testable without a filesystem and reusable by the verifier and the writer alike. Reuse the sections' helpers; if `sha256_prefixed_128` is private, make it `pub(crate)`-to-`pub` in the same commit with a one-line why.

Run: the vectors pass. Also `cargo test --all-features` and `cargo test --no-default-features` (the crate's zero-dep promise).

- [ ] **Step 3: Commit**

`graph-types: a raw input tree -- per-file SHA-256 leaves, name-ordered directory nodes, one root`

---

### Task 2: Walking the tree — `atlas-cli` reads `data/raw` into it

**Files:**
- Create: `server/atlas-cli/src/raw.rs`
- Test: `server/atlas-cli/tests/raw_walk.rs`

**Interfaces:**
- Produces: `fn walk(root: &Path) -> io::Result<RawNode>`, `fn read_manifest(path: &Path) -> Result<RawManifest>`, `fn write_manifest(path: &Path, &RawManifest)`, `RawManifest { schema: u32, root: String, datasets: Vec<RawNode> }`.

- [ ] **Step 1: Write the failing test**

`server/atlas-cli/tests/raw_walk.rs` against a temp fixture tree (three files in two directories, one empty directory):
- `walk` produces the same root as the Task 1 values computed by hand from the same bytes (whole-body)
- a byte flipped in one file moves that file's leaf, its directory node and the root, and no sibling hash
- a file added and a file removed both move the root
- symlinks and junctions are **not followed** — assert a junction into a sibling tree is recorded as what it is or skipped, never silently inlined. (This is the specific mechanism that caused the 2026-09-28 loss; the walker must not be the thing that re-creates it. Decide and state which, in the report.)
- the walk is deterministic across two runs (same root) and ordering does not depend on the filesystem's enumeration order

Run: `cargo test -p atlas-cli --test raw_walk` → fails to compile.

- [ ] **Step 2: Implement, then measure**

TOML shape (`data/raw/MANIFEST.toml`), mirroring `data/compiled/manifest.toml`'s spirit:
```toml
schema = 1
root = "<32 hex>"
# one [[dataset]] per top-level entry of data/raw, name-ordered
[[dataset]]
name = "geo"
hash = "<32 hex>"
[[dataset.file]]
name = "ancient.jsonl"
sha256 = "<64 hex>"
bytes = 9180734
```
Record the wall-clock of a full walk of the real tree (~297 MiB / 7,400 files) in the report; if it is over a second, say whether the gate should hash or only stat-and-spot-check, with numbers.

- [ ] **Step 3: Commit**

`atlas-cli: walking data/raw into the input tree; the manifest is TOML beside the README`

---

### Task 3: `bibex verify` refuses a drifted or half-written raw tree

**Files:**
- Modify: `server/atlas-cli/src/commands/verify.rs`
- Test: `server/atlas-cli/tests/verify_raw.rs`

- [ ] **Step 1: Write the failing test**

Extend the existing verify tests' style (`verify.rs:204-214` exits `integrity_failed` = 6):
- a tree matching `MANIFEST.toml` verifies, exit 0, and the JSON form (`verify.rs:225-255`) carries the raw root
- a flipped byte exits 6 and the message **names the file**, not just the root (this is the diagnosis the compiled-only chain cannot give)
- a missing file, an extra file, and a truncated file are each distinguished in the message
- a missing `MANIFEST.toml` is **not** a failure: it reports "raw: unrecorded" and leaves the exit code alone, so a fresh clone with no raw tree still verifies its compiled artifact (`data/raw` is gitignored; the verifier must stay useful without it)
- an empty `data/raw` is likewise "unrecorded"/skipped, never a false pass

Run: `cargo test -p atlas-cli --test verify_raw` → fails.

- [ ] **Step 2: Implement**

One raw section added to the existing report, the same shape as the compiled sections. Remediation string beside `verify.rs:49`'s: what to run to restore (the archive) and what to run if the change was deliberate (`bibex raw bless`).

- [ ] **Step 3: The writer**

`bibex raw bless` — walks, writes `MANIFEST.toml`, and prints the root plus a diff summary against the previous manifest (files added / removed / changed, by name). Per D4 it is a separate verb, never wired into `verify`. It refuses to write when the walk found zero files, so a blessing cannot record an emptied tree as the truth (the 2026-09-28 failure mode, one command later).

- [ ] **Step 4: Commit**

`atlas-cli: verify reads data/raw against its manifest and names the file that moved` and `atlas-cli: raw bless records the input tree, and refuses to record an empty one`

---

### Task 4: The fetch script verifies what it downloaded

**Files:**
- Modify: `data/fetch-raw.ps1`
- Test: `data/tests/fetch-raw-guards.Tests.ps1` (or, if no PowerShell test harness exists in this repo, a Rust test in `atlas-cli` over the script's guard logic extracted into a verifiable form — decide, and say which in the report; do not leave the guards untested)

- [ ] **Step 1: The guards**

- `Fetch` (`:7-12`): when `MANIFEST.toml` knows the file, skip only if size and hash match; re-download if not; report which. When the manifest does not know it (a genuinely new source), fetch and say so.
- The two `brain-fuel-bible` vendoring blocks (`:60`, `:79`): the guard becomes "does this subtree match its recorded node hash", so the half-copied `lexicon/` + empty `morph/` state re-copies instead of being skipped.
- At the end: run the verifier and fail loudly on mismatch, naming the files.

- [ ] **Step 2: Prove it on the real failure**

Reproduce 2026-09-28's state deliberately in a scratch copy — delete half of `lexicon/`, empty `morph/` — and show the script re-copies rather than skipping. Paste the before/after.

- [ ] **Step 3: Commit**

`data: fetch-raw verifies what it downloaded and re-copies a half-written subtree`

---

### Task 5: The compiled artifact records which inputs built it

**Files:**
- Modify: `server/atlas-graph/src/sqlite/manifest.rs`, `server/atlas-graph/src/sqlite/writer.rs`, `server/atlas-graph/tests/version_root_regression.rs`
- Test: `server/atlas-graph/tests/manifest_raw_provenance.rs`

- [ ] **Step 1: The failing tests**

- `manifest.toml` round-trips a `raw_root` field
- **the manifest root is unchanged** by its presence (whole-body: the committed artifact's root is still `18c7546b435791ee9422be8f32fbb5fa` — this is D1's proof, and the test that must not be allowed to go green by accident)
- `read_manifest` accepts a manifest with no `raw_root` (every artifact built before this plan)
- `version_root_regression.rs`'s failure message gains the diagnosis: when the roots disagree, it says whether the **raw** root also moved, so "corrupt input" and "deliberate content change" are distinguishable at the point of failure

- [ ] **Step 2: Implement, regenerate, prove the root held**

`bibex verify` green on the untouched committed artifact; `git diff data/compiled/manifest.toml` shows exactly one added field.

- [ ] **Step 3: Commit**

`atlas-graph: the manifest records the raw root as provenance, outside its own preimage`

---

### Task 6: The copy that survives a worktree removal

**Files:**
- Create: `scripts/archive-raw.ps1`
- Modify: `docs/PRINCIPLES.md` (the worktree rule), `data/raw/README.md` (one line pointing at the manifest)

- [ ] **Step 1: The script**

Archives the datasets that cannot be re-derived from a pinned source — `geo`, `concord`, `kretzmann` (the three live-source trees), plus the three surviving zips — to a path **outside every git worktree**, default `%USERPROFILE%\backups\bible-atlas-raw\raw-<root>-<date>.zip`, named by the manifest root so the archive says which tree it holds. Then it verifies the archive round-trips to that same root, and prints what to run to restore. It refuses to archive a tree that does not match `MANIFEST.toml`.

- [ ] **Step 2: The rule that was missing**

`docs/PRINCIPLES.md` gains: a throwaway git worktree never has ignored data linked into it (junction or symlink) — it copies what it needs, or the links are removed with `cmd /c rmdir` *before* `git worktree remove`, because `--force` follows a junction and deletes through it. Cite the 2026-09-28 loss as the why.

- [ ] **Step 3: Run it, verify, commit**

Paste the archive path, its size, and the verified root. `scripts: archive-raw keeps an off-worktree copy of the inputs that cannot be refetched`

---

## Self-review against the goal

- **Coverage:** merkle tree (Tasks 1–2), verification + diagnosis (Task 3), the trust-any-existing-file bug and the half-written-subtree bug (Task 4), provenance without coupling (Task 5), durability + the rule (Task 6).
- **The two real failures of 2026-09-28 are each pinned by a test:** the junction-following walk (Task 2 Step 1) and the half-copied subtree (Task 4 Step 2).
- **Placeholders:** none. Three decisions are delegated with a required answer in the report: whether the walker skips or records junctions; whether the gate hashes fully or spot-checks (with numbers); and where the PowerShell guards are tested.
