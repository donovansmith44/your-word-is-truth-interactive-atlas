# TOOLCHAIN-1 + CONTENTION-1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pin the Rust toolchain in both repos (TOOLCHAIN-1) and move the eight wall-clock test gates into a serialized, process-isolated step so they stop flipping on identical code (CONTENTION-1) — the two prerequisites every later batch of the relational-artifact migration depends on.

**Architecture:** TOOLCHAIN-1 is a `rust-toolchain.toml` at each repo root plus a test that proves the compiler that built the test IS the pinned version. CONTENTION-1 marks the eight timing tests `#[ignore]` with one exact reason string, adds `scripts/timing-gates.sh` that runs each of them in its own `cargo test` process with `--test-threads=1`, and adds a `check` mode plus a self-test proving the script's list and the tree's ignored tests are the same multiset (so a test cannot be quietly un-gated). Ceilings are not touched.

**Tech Stack:** rustup 1.97.1 (`x86_64-pc-windows-gnu`), cargo integration tests (std only in `graph-types`), Git Bash shell scripts (the repo's existing `scripts/*.sh` convention).

**Spec:** `docs/superpowers/specs/2026-09-14-relational-artifact-design.md` — §3.2 (toolchain pin), §8 rows 0 and 1 (batch definitions and gates), §12 (measurements).

## Global Constraints

- Pinned toolchain: `1.97.1` exactly (spec §3.2). Host triple on this machine: `x86_64-pc-windows-gnu`.
- `graph-types` stays zero-dependency (spec §1.2) — any test added there uses `std` only.
- Ceilings are NOT loosened, ever (spec §8 row 1): artifact load 4 s, full-graph conformance 60 s, perf_smoke 75/75/50/30/30/50 ms. Only WHERE they run changes.
- Standing counting procedure (`server/Cargo.toml` header comment): a full-suite count is run from `server/` and reported per command, never as a bare total. This plan changes it from two commands to three and updates that comment.
- Batches ship green on their own; each ends with a commit whose message names the batch.
- Commit attribution lines (from the session's system reminder) go at the end of every commit message:
  `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W`.
- Every fresh shell needs `$env:Path = "$env:Path;$env:LOCALAPPDATA\Microsoft\dotnet;$env:USERPROFILE\.cargo\bin"` (README "Prerequisites"). In Git Bash: `export PATH="$PATH:$HOME/.cargo/bin"`.
- The map project lives at `C:\Users\donov\scratch\map-generator` (its own git repo; its eight crates path-depend on this worktree's `graph-types`).

---

## File Structure

| Path | Responsibility |
|---|---|
| `rust-toolchain.toml` (repo root, NEW) | the pin; rustup reads it for every `cargo` run under `server/` and `graph-types/` |
| `graph-types/tests/toolchain_pin.rs` (NEW) | proves the file exists, pins the exact version, and the compiler that built the test is that version |
| `C:\Users\donov\scratch\map-generator\rust-toolchain.toml` (NEW) | same pin, other repo |
| `C:\Users\donov\scratch\map-generator\crates\map-types\tests\toolchain_pin.rs` (NEW) | same proof, other repo |
| `server/atlas-graph/tests/artifact_conformance.rs:100-101` (MODIFY) | 1 timing test gains `#[ignore = REASON]` |
| `server/atlas-server/tests/graph_conformance.rs:138-139` (MODIFY) | 1 timing test gains `#[ignore = REASON]` |
| `server/atlas-server/tests/perf_smoke.rs` (MODIFY, 6 sites) | 6 timing tests gain `#[ignore = REASON]` |
| `scripts/timing-gates.sh` (NEW) | runs the 8 gates serialized, one cargo process each; `check` mode reconciles its list against the tree |
| `scripts/timing-gates-selftest.sh` (NEW) | attempts the bypasses (drop a name, add a stray `#[ignore]`, drift the reason) and requires each to be refused |
| `server/Cargo.toml` header comment (MODIFY) | standing counting procedure becomes three commands |
| `README.md` (MODIFY) | new "Test suite" section before "## Dev loop" |
| `.superpowers/sdd/2026-08-17-bible-atlas-m1/progress.md` (APPEND) | ledger entries for both batches, with the ten-run evidence |

The exact ignore reason, used verbatim everywhere (the script greps for it byte-for-byte):

```
wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)
```

---

### Task 1: TOOLCHAIN-1 — pin this repo

**Files:**
- Create: `rust-toolchain.toml`
- Test: `graph-types/tests/toolchain_pin.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: the pin file; Task 2 copies its content byte-for-byte.

- [ ] **Step 1: Install the exact toolchain (network, one time)**

Run from anywhere:

```bash
export PATH="$PATH:$HOME/.cargo/bin"
rustup toolchain install 1.97.1-x86_64-pc-windows-gnu --component llvm-tools
rustup toolchain list
```

Expected: the list now shows both `stable-x86_64-pc-windows-gnu (default)` and `1.97.1-x86_64-pc-windows-gnu`. (`llvm-tools` is what `server/setup-toolchain.ps1` needs when it is ever re-run under the pinned toolchain; the already-provisioned `~/.cargo/atlas-tools/llvm-dlltool.exe` keeps working regardless.)

- [ ] **Step 2: Write the failing test**

Create `graph-types/tests/toolchain_pin.rs`:

```rust
//! TOOLCHAIN-1 (spec §3.2): the toolchain pin is a law, not a convention.
//!
//! Every content-addressed id in this project was, until the relational
//! artifact cutover, a `DefaultHasher` output -- an algorithm Rust's own
//! docs say "may change between releases." A `rustup update` could move
//! every id with zero data change. These tests make the pin load-bearing:
//! the file must exist, must name the exact version, and the compiler
//! that BUILT THIS TEST must be that version.

use std::path::{Path, PathBuf};
use std::process::Command;

const PINNED: &str = "1.97.1";

fn repo_root() -> PathBuf {
    // graph-types/ sits at the repo root; the pin sits beside it.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn channel_from(toml: &str) -> Option<String> {
    toml.lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("channel"))
        .and_then(|rest| rest.trim().strip_prefix('='))
        .map(|v| v.trim().trim_matches('"').to_string())
}

#[test]
fn rust_toolchain_toml_pins_the_exact_version() {
    let path = repo_root().join("rust-toolchain.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must exist at the repo root: {e}", path.display()));
    let channel = channel_from(&text).expect("rust-toolchain.toml must declare `channel = \"...\"`");
    assert_eq!(
        channel, PINNED,
        "channel must be the exact version {PINNED:?}, never a moving name like \"stable\""
    );
}

#[test]
fn the_compiler_that_built_this_test_is_the_pinned_version() {
    // `CARGO` is the cargo binary that performed this build; its sibling
    // `rustc` is the compiler that built this test binary.
    let rustc = Path::new(env!("CARGO")).with_file_name(if cfg!(windows) { "rustc.exe" } else { "rustc" });
    let out = Command::new(&rustc)
        .arg("--version")
        .output()
        .unwrap_or_else(|e| panic!("could not run {}: {e}", rustc.display()));
    let v = String::from_utf8_lossy(&out.stdout);
    assert!(
        v.starts_with(&format!("rustc {PINNED} ")),
        "this test was built by {v:?}; the pin requires rustc {PINNED}"
    );
}

#[test]
fn rustup_resolves_the_pin_from_the_repo_root() {
    // The proxy in ~/.cargo/bin resolves the toolchain from the working
    // directory; from the repo root it must land on the pin.
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).expect("a home dir");
    let proxy = Path::new(&home)
        .join(".cargo")
        .join("bin")
        .join(if cfg!(windows) { "rustc.exe" } else { "rustc" });
    let out = Command::new(&proxy)
        .arg("--version")
        .current_dir(repo_root())
        .output()
        .unwrap_or_else(|e| panic!("could not run the rustup proxy {}: {e}", proxy.display()));
    let v = String::from_utf8_lossy(&out.stdout);
    assert!(
        v.starts_with(&format!("rustc {PINNED} ")),
        "rustc resolved from the repo root is {v:?}; expected the pin {PINNED}"
    );
}

#[test]
fn channel_parser_reads_the_toml_shape_we_write() {
    assert_eq!(channel_from("[toolchain]\nchannel = \"1.97.1\"\n").as_deref(), Some("1.97.1"));
    assert_eq!(channel_from("[toolchain]\n  channel=\"stable\"\n").as_deref(), Some("stable"));
    assert_eq!(channel_from("[toolchain]\nprofile = \"minimal\"\n"), None);
}
```

- [ ] **Step 3: Run it to verify it fails**

Run from `server/` (the workspace that resolves `-p atlas-graph-types`):

```bash
cd server && cargo test -p atlas-graph-types --test toolchain_pin
```

Expected: `rust_toolchain_toml_pins_the_exact_version` FAILS with "rust-toolchain.toml must exist at the repo root"; `rustup_resolves_the_pin_from_the_repo_root` FAILS only if `stable` has moved past 1.97.1 (today it is 1.97.1, so it may already pass — that is fine; the file test is the one that must be red); `channel_parser_reads_the_toml_shape_we_write` passes.

- [ ] **Step 4: Write the pin**

Create `rust-toolchain.toml` at the repo root:

```toml
# TOOLCHAIN-1 (docs/superpowers/specs/2026-09-14-relational-artifact-design.md §3.2).
# Every content-addressed id was, until the relational-artifact cutover, a
# `DefaultHasher` output -- an algorithm Rust documents as unspecified and
# free to change between releases. Without this pin a routine `rustup update`
# could move every pid, every edge id and the version root with ZERO data
# change, silently breaking map-generator's C6 stale-check and bibex's
# byte-pinned transcripts. The exact version, never a channel name.
# Bump deliberately, in its own batch, with the ids re-blessed on purpose.
#
# graph-types/tests/toolchain_pin.rs makes this file load-bearing.
[toolchain]
channel = "1.97.1"
components = ["llvm-tools"]
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd server && cargo test -p atlas-graph-types --test toolchain_pin
```

Expected: `test result: ok. 4 passed`. Also confirm the override is active:

```bash
rustup show active-toolchain
```

Expected (run from the repo root or `server/`): `1.97.1-x86_64-pc-windows-gnu (overridden by '...\rust-toolchain.toml')`.

- [ ] **Step 6: Prove no behaviour change — the standing block**

```bash
cd server && cargo test --workspace 2>&1 | grep -E "^test result|FAILED|panicked" ; cargo test -p atlas-graph-types 2>&1 | grep -E "^test result"
```

Expected: every `test result: ok.` line; zero `FAILED`. The `atlas-graph-types` count is +4 (the new file). The workspace count is unchanged. (The pre-existing CONTENTION-1 flakes may fire here — that is exactly what Task 3 fixes; record the run either way and do not re-run to "get green".)

- [ ] **Step 7: Commit**

```bash
git add rust-toolchain.toml graph-types/tests/toolchain_pin.rs
git commit -F - <<'EOF'
build(toolchain): TOOLCHAIN-1 -- pin rustc 1.97.1 and make the pin load-bearing

rust-toolchain.toml at the repo root names the exact version (never a
channel). graph-types/tests/toolchain_pin.rs proves the file exists,
pins that version, the compiler that built the test IS that version,
and the rustup proxy resolves it from the repo root. Spec §3.2: a
`rustup update` could otherwise move every content-addressed id with
zero data change.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
EOF
```

---

### Task 2: TOOLCHAIN-1 — pin map-generator

**Files:**
- Create: `C:\Users\donov\scratch\map-generator\rust-toolchain.toml`
- Test: `C:\Users\donov\scratch\map-generator\crates\map-types\tests\toolchain_pin.rs`

**Interfaces:**
- Consumes: the pin content from Task 1 Step 4 (identical bytes).
- Produces: nothing downstream; the map project's own build now resolves the same toolchain.

- [ ] **Step 1: Confirm the map project's state before touching it**

```bash
cd /c/Users/donov/scratch/map-generator && git status --short && git log --oneline -1
```

Expected: a clean tree (or only files you did not create — if anything is dirty, stop and report; do not commit someone else's work). Latest commit `6608db4` or later.

- [ ] **Step 2: Write the failing test**

Create `crates/map-types/tests/toolchain_pin.rs` — identical to Task 1's file except the root path, because `crates/map-types/` is two levels below the repo root:

```rust
//! TOOLCHAIN-1 (atlas spec 2026-09-14-relational-artifact-design §3.2):
//! map-generator path-depends on atlas-graph-types from eight crates and
//! pins the atlas version root (C6). The two repos must build with the
//! same compiler, or the root the atlas computes and the root this repo
//! recomputes could disagree with ZERO data change.

use std::path::{Path, PathBuf};
use std::process::Command;

const PINNED: &str = "1.97.1";

fn repo_root() -> PathBuf {
    // crates/map-types/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn channel_from(toml: &str) -> Option<String> {
    toml.lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("channel"))
        .and_then(|rest| rest.trim().strip_prefix('='))
        .map(|v| v.trim().trim_matches('"').to_string())
}

#[test]
fn rust_toolchain_toml_pins_the_exact_version() {
    let path = repo_root().join("rust-toolchain.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must exist at the repo root: {e}", path.display()));
    let channel = channel_from(&text).expect("rust-toolchain.toml must declare `channel = \"...\"`");
    assert_eq!(channel, PINNED, "channel must be the exact version {PINNED:?}, never \"stable\"");
}

#[test]
fn the_compiler_that_built_this_test_is_the_pinned_version() {
    let rustc = Path::new(env!("CARGO")).with_file_name(if cfg!(windows) { "rustc.exe" } else { "rustc" });
    let out = Command::new(&rustc).arg("--version").output()
        .unwrap_or_else(|e| panic!("could not run {}: {e}", rustc.display()));
    let v = String::from_utf8_lossy(&out.stdout);
    assert!(v.starts_with(&format!("rustc {PINNED} ")), "built by {v:?}; the pin requires rustc {PINNED}");
}

#[test]
fn rustup_resolves_the_pin_from_the_repo_root() {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).expect("a home dir");
    let proxy = Path::new(&home).join(".cargo").join("bin")
        .join(if cfg!(windows) { "rustc.exe" } else { "rustc" });
    let out = Command::new(&proxy).arg("--version").current_dir(repo_root()).output()
        .unwrap_or_else(|e| panic!("could not run the rustup proxy {}: {e}", proxy.display()));
    let v = String::from_utf8_lossy(&out.stdout);
    assert!(v.starts_with(&format!("rustc {PINNED} ")), "resolved {v:?} from the repo root; expected {PINNED}");
}
```

- [ ] **Step 3: Run it to verify it fails**

```bash
cd /c/Users/donov/scratch/map-generator && cargo test -p map-types --test toolchain_pin
```

Expected: `rust_toolchain_toml_pins_the_exact_version` FAILS ("must exist at the repo root").

- [ ] **Step 4: Write the pin (same bytes as Task 1, different comment header)**

Create `C:\Users\donov\scratch\map-generator\rust-toolchain.toml`:

```toml
# TOOLCHAIN-1 (atlas spec docs/superpowers/specs/2026-09-14-relational-artifact-design.md §3.2).
# This repo path-depends on atlas-graph-types from eight crates and pins the
# atlas version root (contract C6). Both repos must build with the SAME
# compiler: the atlas's content-addressed ids are (until its cutover) a
# `DefaultHasher` output, unspecified across Rust releases. Exact version,
# never a channel name; bump in lockstep with the atlas, deliberately.
#
# crates/map-types/tests/toolchain_pin.rs makes this file load-bearing.
[toolchain]
channel = "1.97.1"
components = ["llvm-tools"]
```

- [ ] **Step 5: Run the tests to verify they pass, then the map project's whole suite**

```bash
cd /c/Users/donov/scratch/map-generator && cargo test -p map-types --test toolchain_pin && cargo test --workspace 2>&1 | grep -E "^test result|FAILED|panicked"
```

Expected: `3 passed` for the new file; every workspace `test result: ok.`; zero `FAILED`.

- [ ] **Step 6: Commit in the map repo**

```bash
cd /c/Users/donov/scratch/map-generator
git add rust-toolchain.toml crates/map-types/tests/toolchain_pin.rs
git commit -F - <<'EOF'
build(toolchain): TOOLCHAIN-1 -- pin rustc 1.97.1 in lockstep with the atlas

Same pin as the atlas repo (its spec §3.2): this repo recomputes the
atlas version root from a path dependency, and until the atlas cutover
that root is a DefaultHasher output. The two compilers must be the
same compiler. crates/map-types/tests/toolchain_pin.rs makes the file
load-bearing.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
EOF
```

---

### Task 3: CONTENTION-1 — gate the eight wall-clock tests and run them serialized

**Files:**
- Modify: `server/atlas-graph/tests/artifact_conformance.rs:100` (the `#[test]` above `serialized_artifact_is_admitted_and_loads_under_the_committed_ceiling`)
- Modify: `server/atlas-server/tests/graph_conformance.rs:138` (the `#[test]` above `the_full_real_graph_is_admitted_the_in_memory_store_answers_match_the_model_exactly`)
- Modify: `server/atlas-server/tests/perf_smoke.rs:83,93,103,118,132,152` (the six `#[test]` lines)
- Create: `scripts/timing-gates.sh`
- Test: `scripts/timing-gates-selftest.sh`

**Interfaces:**
- Consumes: nothing from Tasks 1–2 (independent; the toolchain pin just makes the timings comparable across runs).
- Produces: `scripts/timing-gates.sh` with two modes — `bash scripts/timing-gates.sh check` (exit 0 iff the script's gate list and the tree's `#[ignore = REASON]` tests are the same multiset AND no other `#[ignore` attribute exists under `server/`) and `bash scripts/timing-gates.sh` (runs the eight gates serialized; prints `TIMING GATES: 8/8 passed`; exit 1 otherwise). Task 4's counting procedure and Task 5's evidence depend on those exact outputs. Env override `TIMING_GATES_TREE=<dir>` points `check` at a different tree (used by the self-test only). Env override `TIMING_GATES_LOG_DIR=<dir>` redirects logs (default `.superpowers/sdd/2026-08-17-bible-atlas-m1/logs-timing-gates/`).

The eight gates, in the order the script runs them (package, test binary, test name):

| # | Package | Binary | Test | Ceiling |
|---|---|---|---|---|
| 1 | atlas-graph | artifact_conformance | serialized_artifact_is_admitted_and_loads_under_the_committed_ceiling | 4 s load |
| 2 | atlas-server | graph_conformance | the_full_real_graph_is_admitted_the_in_memory_store_answers_match_the_model_exactly | 60 s |
| 3 | atlas-server | perf_smoke | scene_time_full_span_completes_within_smoke_threshold | 75 ms |
| 4 | atlas-server | perf_smoke | scene_time_nt_window_completes_within_smoke_threshold | 75 ms |
| 5 | atlas-server | perf_smoke | scene_scripture_chapter_completes_within_smoke_threshold | 50 ms |
| 6 | atlas-server | perf_smoke | xrefs_for_verse_completes_within_smoke_threshold | 30 ms |
| 7 | atlas-server | perf_smoke | text_window_completes_within_smoke_threshold | 30 ms |
| 8 | atlas-server | perf_smoke | chapter_window_completes_within_smoke_threshold | 50 ms |

The untimed neighbours stay in the default run: `graph_service_from_artifact_serves_the_same_answers_as_from_sources` (artifact_conformance) and `the_real_kjv_derived_graph_is_admitted_the_in_memory_store_answers_match_the_model_exactly` (graph_conformance) carry no `Instant` and are not touched.

- [ ] **Step 1: Write the self-test first (it is the failing test for the script)**

Create `scripts/timing-gates-selftest.sh`:

```bash
#!/usr/bin/env bash
# CONTENTION-1: every way to quietly un-gate a wall-clock test is ATTEMPTED
# here and must be REFUSED by `timing-gates.sh check`. In the CDC-1
# tradition: a gate that has not tried to bypass itself is a hope.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GATES="$ROOT/scripts/timing-gates.sh"
REASON='wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)'

pass=0; fail=0
expect_ok()      { if "$@" >/dev/null 2>&1; then pass=$((pass+1)); else fail=$((fail+1)); echo "FAIL (expected ok):      $*" >&2; fi; }
expect_refused() { if "$@" >/dev/null 2>&1; then fail=$((fail+1)); echo "FAIL (expected refused): $*" >&2; else pass=$((pass+1)); fi; }

# A synthetic tree whose ignored tests exactly match the script's list.
mk_tree() { # <dir> ; writes one .rs per gate name given on stdin
  local dir="$1"; mkdir -p "$dir/src"
  while read -r name; do
    printf '#[ignore = "%s"]\nfn %s() {}\n' "$REASON" "$name" >> "$dir/src/gates.rs"
  done
}

tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT

# 0. The real tree reconciles.
expect_ok bash "$GATES" check

# 1. Control: a synthetic tree with exactly the script's names reconciles.
bash "$GATES" list > "$tmp/names"
mk_tree "$tmp/t1" < "$tmp/names"
expect_ok env TIMING_GATES_TREE="$tmp/t1" bash "$GATES" check

# 2. ATTEMPT: drop one gated test from the tree (someone deleted the
#    #[ignore] line, or renamed the test) -> the script would still "run"
#    a name that no longer exists. Must be refused.
mk_tree "$tmp/t2" < <(sed '1d' "$tmp/names")
expect_refused env TIMING_GATES_TREE="$tmp/t2" bash "$GATES" check

# 3. ATTEMPT: gate a NINTH test with the right reason but never add it to
#    the script -> it is ignored by the default run and run by nobody.
mk_tree "$tmp/t3" < "$tmp/names"
printf '#[ignore = "%s"]\nfn a_ninth_gate_nobody_runs() {}\n' "$REASON" >> "$tmp/t3/src/gates.rs"
expect_refused env TIMING_GATES_TREE="$tmp/t3" bash "$GATES" check

# 4. ATTEMPT: reason drift -- keep the test ignored but change the reason
#    string so it stops matching the grep. Must be refused: NO #[ignore
#    attribute under the tree may carry any other reason.
mk_tree "$tmp/t4" < <(sed '1d' "$tmp/names")
printf '#[ignore = "flaky, look later"]\nfn %s() {}\n' "$(sed -n '1p' "$tmp/names")" >> "$tmp/t4/src/gates.rs"
expect_refused env TIMING_GATES_TREE="$tmp/t4" bash "$GATES" check

# 5. ATTEMPT: a bare #[ignore] with no reason at all.
mk_tree "$tmp/t5" < "$tmp/names"
printf '#[ignore]\nfn quietly_disabled() {}\n' >> "$tmp/t5/src/gates.rs"
expect_refused env TIMING_GATES_TREE="$tmp/t5" bash "$GATES" check

# 6. ATTEMPT: duplicate a name in the tree (two tests, one listing) -- a
#    SET comparison would pass this; the comparison must be a MULTISET.
mk_tree "$tmp/t6" < "$tmp/names"
printf '#[ignore = "%s"]\nfn %s() {}\n' "$REASON" "$(sed -n '1p' "$tmp/names")" >> "$tmp/t6/src/gates.rs"
expect_refused env TIMING_GATES_TREE="$tmp/t6" bash "$GATES" check

echo "timing-gates selftest: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
```

- [ ] **Step 2: Run the self-test to verify it fails**

```bash
bash scripts/timing-gates-selftest.sh
```

Expected: every case FAILS (the script does not exist yet): `timing-gates selftest: 0 passed, 7 failed`, exit 1.

- [ ] **Step 3: Mark the eight tests ignored, with the exact reason**

In each of the eight sites, change the bare attribute to the two-attribute form. Example for `server/atlas-graph/tests/artifact_conformance.rs:100`:

```rust
#[test]
#[ignore = "wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)"]
fn serialized_artifact_is_admitted_and_loads_under_the_committed_ceiling() {
```

Apply the identical `#[ignore = "..."]` line directly under the `#[test]` line of:

- `server/atlas-server/tests/graph_conformance.rs:138` → `the_full_real_graph_is_admitted_the_in_memory_store_answers_match_the_model_exactly`
- `server/atlas-server/tests/perf_smoke.rs:83` → `scene_time_full_span_completes_within_smoke_threshold`
- `server/atlas-server/tests/perf_smoke.rs:93` → `scene_time_nt_window_completes_within_smoke_threshold`
- `server/atlas-server/tests/perf_smoke.rs:103` → `scene_scripture_chapter_completes_within_smoke_threshold`
- `server/atlas-server/tests/perf_smoke.rs:118` → `xrefs_for_verse_completes_within_smoke_threshold`
- `server/atlas-server/tests/perf_smoke.rs:132` → `text_window_completes_within_smoke_threshold`
- `server/atlas-server/tests/perf_smoke.rs:152` → `chapter_window_completes_within_smoke_threshold`

(Line numbers are as of commit `61b49db`; match on the function name, not the number.)

Then add this paragraph to the top-of-file doc comment of each of the three files, so a reader of the test learns where it runs:

```rust
//! CONTENTION-1 (spec 2026-09-14-relational-artifact-design §8): the
//! timing gate(s) in this file are `#[ignore]`d in the default run and
//! executed by `scripts/timing-gates.sh` -- one cargo process per gate,
//! `--test-threads=1`, after the parallel suite -- because three
//! wall-clock ceilings were resolving BOTH WAYS on identical code under
//! parallel load (progress.md, 2026-09-08 through 2026-09-10). The
//! ceilings themselves are unchanged. `timing-gates.sh check` refuses
//! any #[ignore] in server/ that is not one of the listed gates.
```

- [ ] **Step 4: Write the script**

Create `scripts/timing-gates.sh`:

```bash
#!/usr/bin/env bash
# CONTENTION-1 (docs/superpowers/specs/2026-09-14-relational-artifact-design.md §8):
# the eight wall-clock gates, run SERIALIZED -- one `cargo test` process per
# gate, `--test-threads=1`, never concurrently with the parallel suite.
# The ceilings live in the tests and are NOT touched here.
#
#   bash scripts/timing-gates.sh          run all eight; exit 1 on any failure
#   bash scripts/timing-gates.sh check    reconcile this list against the tree
#   bash scripts/timing-gates.sh list     print the gate names, one per line
#
# `check` is the anti-bypass law: every `#[ignore = REASON]` test under the
# tree must be listed here (multiset-equal), and NO other `#[ignore` attribute
# may exist under the tree at all. scripts/timing-gates-selftest.sh attempts
# each bypass and requires refusal.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TREE="${TIMING_GATES_TREE:-$ROOT/server}"
LOG_DIR="${TIMING_GATES_LOG_DIR:-$ROOT/.superpowers/sdd/2026-08-17-bible-atlas-m1/logs-timing-gates}"
REASON='wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)'

# package | test binary | test name -- ORDER IS THE RUN ORDER.
GATES=(
  "atlas-graph|artifact_conformance|serialized_artifact_is_admitted_and_loads_under_the_committed_ceiling"
  "atlas-server|graph_conformance|the_full_real_graph_is_admitted_the_in_memory_store_answers_match_the_model_exactly"
  "atlas-server|perf_smoke|scene_time_full_span_completes_within_smoke_threshold"
  "atlas-server|perf_smoke|scene_time_nt_window_completes_within_smoke_threshold"
  "atlas-server|perf_smoke|scene_scripture_chapter_completes_within_smoke_threshold"
  "atlas-server|perf_smoke|xrefs_for_verse_completes_within_smoke_threshold"
  "atlas-server|perf_smoke|text_window_completes_within_smoke_threshold"
  "atlas-server|perf_smoke|chapter_window_completes_within_smoke_threshold"
)

names_in_script() { printf '%s\n' "${GATES[@]}" | awk -F'|' '{print $3}' | sort; }

# Every `fn NAME` that directly follows a `#[ignore = REASON]` attribute.
names_in_tree() {
  grep -rn --include='*.rs' --exclude-dir=target -A 2 -F "#[ignore = \"$REASON\"]" "$TREE" 2>/dev/null \
    | grep -oE '(^|[-:])fn [A-Za-z0-9_]+' | sed -E 's/^.*fn //' | sort
}

# Any #[ignore attribute at all (bare, or with a different reason).
foreign_ignores() {
  grep -rn --include='*.rs' --exclude-dir=target -E '^\s*#\[ignore' "$TREE" 2>/dev/null \
    | grep -vF "#[ignore = \"$REASON\"]" || true
}

check() {
  local ok=0
  local foreign; foreign="$(foreign_ignores)"
  if [ -n "$foreign" ]; then
    echo "REFUSED: #[ignore] attributes under $TREE that are not the CONTENTION-1 gate reason:" >&2
    echo "$foreign" >&2; ok=1
  fi
  # MULTISET comparison: `diff` on the sorted, non-deduplicated lists.
  if ! diff <(names_in_script) <(names_in_tree) >"$LOG_DIR/.check.diff" 2>/dev/null; then
    echo "REFUSED: the gate list in $0 and the #[ignore = REASON] tests under $TREE differ (< script, > tree):" >&2
    cat "$LOG_DIR/.check.diff" >&2; ok=1
  fi
  [ "$ok" -eq 0 ] && echo "timing-gates check: ${#GATES[@]} gates reconcile with the tree"
  return "$ok"
}

run() {
  command -v cargo >/dev/null || { echo "cargo not on PATH (export PATH=\"\$PATH:\$HOME/.cargo/bin\")" >&2; exit 2; }
  check || { echo "not running gates whose list does not reconcile" >&2; exit 1; }
  local stamp; stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  local passed=0 failed=0 i=0
  for g in "${GATES[@]}"; do
    i=$((i+1))
    IFS='|' read -r pkg bin name <<<"$g"
    local log="$LOG_DIR/$stamp-$i-$bin-$name.log"
    echo "== gate $i/${#GATES[@]}: $pkg::$bin::$name"
    # One process per gate. --exact so a name is a name, not a prefix.
    if ( cd "$ROOT/server" && cargo test -p "$pkg" --test "$bin" -- --ignored --exact "$name" --test-threads=1 ) >"$log" 2>&1 \
       && grep -qF "test $name ... ok" "$log" \
       && grep -qE "^test result: ok\. 1 passed" "$log"; then
      passed=$((passed+1)); grep -E "ARTIFACT LOAD|CONFORMANCE|took|ceiling" "$log" | head -3 | sed 's/^/   /'
    else
      failed=$((failed+1)); echo "   FAILED -- see $log"; grep -E "panicked|exceeding|over the" "$log" | head -3 | sed 's/^/   /'
    fi
  done
  echo "TIMING GATES: $passed/${#GATES[@]} passed (serialized, one process each; logs in $LOG_DIR)"
  [ "$failed" -eq 0 ]
}

mkdir -p "$LOG_DIR"
case "${1:-run}" in
  run)   run ;;
  check) check ;;
  list)  printf '%s\n' "${GATES[@]}" | awk -F'|' '{print $3}' ;;
  *)     echo "usage: $0 [run|check|list]" >&2; exit 2 ;;
esac
```

- [ ] **Step 5: Run the self-test to verify it passes**

```bash
bash scripts/timing-gates-selftest.sh
```

Expected: `timing-gates selftest: 7 passed, 0 failed`, exit 0. If case 0 (the real tree) is the one failing, the diff it prints names the test whose `#[ignore]` line was mistyped in Step 3 — fix the attribute, not the script.

- [ ] **Step 6: Prove the default run no longer executes the gates, and the script does**

```bash
cd server && cargo test -p atlas-server --test perf_smoke 2>&1 | grep -E "^test result"
```

Expected: `test result: ok. 0 passed; 0 failed; 6 ignored`.

```bash
cd .. && bash scripts/timing-gates.sh
```

Expected: eight `== gate` blocks each followed by a timing line, then `TIMING GATES: 8/8 passed`, exit 0. Wall time ≈ 3–4 minutes (the 60 s conformance dominates). If any gate fails here in ISOLATION, that is a real regression, not contention — stop and report it with the log; do not adjust a ceiling.

- [ ] **Step 7: Commit**

```bash
git add scripts/timing-gates.sh scripts/timing-gates-selftest.sh \
        server/atlas-graph/tests/artifact_conformance.rs \
        server/atlas-server/tests/graph_conformance.rs \
        server/atlas-server/tests/perf_smoke.rs
git commit -F - <<'EOF'
test(gates): CONTENTION-1 -- the eight wall-clock gates run serialized, one process each

Three ceilings (artifact load 4s, full-graph conformance 60s, perf_smoke
30-75ms) were resolving both ways on identical code under parallel
load. The gates are now #[ignore = REASON] in the default run and
executed by scripts/timing-gates.sh after it: one cargo process per
gate, --test-threads=1, --exact. Ceilings unchanged.

`timing-gates.sh check` is the anti-bypass law: the script's list and
the tree's ignored tests must be MULTISET-equal and no other #[ignore]
may exist under server/. timing-gates-selftest.sh attempts six bypasses
(drop a name, add a ninth gate, drift the reason, bare #[ignore],
duplicate a name) and requires each refused.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
EOF
```

---

### Task 4: CONTENTION-1 — the counting procedure and the README

**Files:**
- Modify: `server/Cargo.toml` (the `STANDING COUNTING PROCEDURE` comment block near the top, currently lines ~20–33)
- Modify: `README.md` (insert a section immediately before the line `## Dev loop`)

**Interfaces:**
- Consumes: the exact summary line `TIMING GATES: 8/8 passed` and the `check` mode from Task 3.
- Produces: the three-command counting procedure Task 5 and every later batch report follow.

- [ ] **Step 1: Write the failing check — the README must describe the procedure**

Add to `scripts/timing-gates-selftest.sh`, just before the final `echo`:

```bash
# 7. The README and the workspace manifest must name the serialized step,
#    or the standing counting procedure silently drops 8 gates.
expect_ok grep -qF 'scripts/timing-gates.sh' "$ROOT/README.md"
expect_ok grep -qF 'scripts/timing-gates.sh' "$ROOT/server/Cargo.toml"
```

Run `bash scripts/timing-gates-selftest.sh` — expected: `7 passed, 2 failed` (the two new greps).

- [ ] **Step 2: Update the standing counting procedure in `server/Cargo.toml`**

Replace this exact text:

```
# STANDING COUNTING PROCEDURE (established by review at f988f1b, restated by
# every batch that reports a full-suite total): a full-suite count is TWO
# commands, both run from server/, summed together --
#   cargo test --workspace
#   cargo test -p atlas-graph-types
```

with:

```
# STANDING COUNTING PROCEDURE (established by review at f988f1b, restated by
# every batch that reports a full-suite total; amended by CONTENTION-1,
# 2026-09-14): a full-suite count is THREE commands, the first two run from
# server/ and the third from the repo root, AFTER the first two finish --
#   cargo test --workspace
#   cargo test -p atlas-graph-types
#   bash ../scripts/timing-gates.sh      # the 8 wall-clock gates, serialized
# The eight gates are `#[ignore = "wall-clock gate: ..."]` in the first
# command (they show as "ignored" there, never as passed) and are counted
# ONLY from the third command's own summary line, `TIMING GATES: N/8
# passed`. Report all three per-command numbers and the section count;
# `bash ../scripts/timing-gates.sh check` must print "8 gates reconcile"
# in the same report, or the count is not a count.
```

- [ ] **Step 3: Add the README section**

Insert immediately before the line `## Dev loop` in `README.md`:

```markdown
## Test suite

Three commands, in this order, the third only after the first two have
finished (it measures wall-clock ceilings and must not share the machine
with a parallel build):

```powershell
cd server
cargo test --workspace              # parallel; the 8 wall-clock gates show as "ignored" here
cargo test -p atlas-graph-types     # graph-types is a path dep, not a workspace member
bash ../scripts/timing-gates.sh     # the 8 gates, serialized, one process each -> "TIMING GATES: 8/8 passed"
```

The timing gates (artifact load ≤ 4 s, full-graph conformance ≤ 60 s,
six perf_smoke thresholds) live in the tests themselves and are never
loosened; `scripts/timing-gates.sh check` refuses any `#[ignore]` under
`server/` that is not one of the eight, and
`scripts/timing-gates-selftest.sh` attempts the bypasses. Report a
full-suite count as the three per-command numbers, never a bare total
(see the `STANDING COUNTING PROCEDURE` comment in `server/Cargo.toml`).
The client suites (`dotnet test client.Tests`, `dotnet test
client.ContractTests`) and the Playwright UX suite (`tests/ux/`) are
separate and unchanged.

```

- [ ] **Step 4: Run the self-test to verify it passes**

```bash
bash scripts/timing-gates-selftest.sh
```

Expected: `9 passed, 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add server/Cargo.toml README.md scripts/timing-gates-selftest.sh
git commit -F - <<'EOF'
docs(gates): CONTENTION-1 -- the standing count is three commands now

The counting procedure in server/Cargo.toml and a new README "Test
suite" section name the serialized timing step; the self-test greps
for both so the procedure cannot silently drop the eight gates.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
EOF
```

---

### Task 5: CONTENTION-1 acceptance — ten consecutive full runs, zero flips

**Files:**
- Create: `.superpowers/sdd/2026-08-17-bible-atlas-m1/logs-contention1/run-NN-{workspace,types,timing}.log` (30 logs) and `summary.txt`
- Append: `.superpowers/sdd/2026-08-17-bible-atlas-m1/progress.md`

**Interfaces:**
- Consumes: the three-command procedure from Task 4.
- Produces: the evidence the spec's §8 row 1 gate demands ("ten consecutive full runs, zero flips, ceilings unchanged"), and the ledger entry later batches cite.

- [ ] **Step 1: Write the runner (throwaway, lives in the log dir, not committed as a tool)**

Create `.superpowers/sdd/2026-08-17-bible-atlas-m1/logs-contention1/ten-runs.sh`:

```bash
#!/usr/bin/env bash
# CONTENTION-1 acceptance: ten consecutive full runs of the three-command
# procedure. Nothing else may run on the machine (no dotnet, no other cargo).
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
OUT="$(cd "$(dirname "$0")" && pwd)"
export PATH="$PATH:$HOME/.cargo/bin"
: > "$OUT/summary.txt"
for n in $(seq -w 1 10); do
  echo "=== run $n: $(date -u +%FT%TZ)" | tee -a "$OUT/summary.txt"
  ( cd "$ROOT/server" && cargo test --workspace )        >"$OUT/run-$n-workspace.log" 2>&1
  ( cd "$ROOT/server" && cargo test -p atlas-graph-types ) >"$OUT/run-$n-types.log" 2>&1
  ( cd "$ROOT" && bash scripts/timing-gates.sh )           >"$OUT/run-$n-timing.log" 2>&1
  {
    printf 'workspace: '; grep -cE '^test result: ok\.' "$OUT/run-$n-workspace.log" | tr -d '\n'; printf ' ok-sections, '
    grep -E '^test result' "$OUT/run-$n-workspace.log" | awk '{p+=$4; f+=$6; i+=$8} END {printf "%d passed %d failed %d ignored\n", p, f, i}'
    printf 'types:     '; grep -E '^test result' "$OUT/run-$n-types.log" | awk '{p+=$4; f+=$6} END {printf "%d passed %d failed\n", p, f}'
    printf 'timing:    '; grep -E '^TIMING GATES' "$OUT/run-$n-timing.log" || echo "NO SUMMARY LINE"
    grep -hE 'ARTIFACT LOAD|CONFORMANCE: .*wall time|took [0-9.]+m?s' "$OUT/run-$n-timing.log" | sed 's/^/   /'
  } | tee -a "$OUT/summary.txt"
done
echo "=== flips: $(grep -cE ' [1-9][0-9]* failed|NO SUMMARY LINE|TIMING GATES: [0-7]/8' "$OUT/summary.txt")" | tee -a "$OUT/summary.txt"
```

- [ ] **Step 2: Run it (long; ~10 × (parallel suite + ~4 min) — run in the background and do nothing else on the machine meanwhile)**

```bash
bash .superpowers/sdd/2026-08-17-bible-atlas-m1/logs-contention1/ten-runs.sh
```

Expected in `summary.txt`: ten blocks, each `... 0 failed 8 ignored` for the workspace, `0 failed` for types, `TIMING GATES: 8/8 passed` for timing, and a final `=== flips: 0`. The eight gate timings per run are also captured, so the report can state the observed spread against each ceiling (e.g. load 2.1–2.8 s vs 4 s).

- [ ] **Step 3: If `flips` is not 0**

Do NOT re-run to get a clean ten. Read the failing log. Two possibilities, and the summary distinguishes them:

- a gate failed in the serialized step → contention is not the cause; it is a real regression or an under-margined ceiling. Report it to the owner with the measured value; a ceiling is changed only by the owner, in its own batch.
- the parallel suite failed → an unrelated flake (the ledger names `world-map WORLD-10b` as a known 1-in-8 client-side flake, which is not in this suite; a Rust-side flake here is new). Report it as found; it is outside CONTENTION-1's scope but inside its evidence.

Either way the batch report states what happened; this step never edits a threshold.

- [ ] **Step 4: Append the ledger entries**

Append to `.superpowers/sdd/2026-08-17-bible-atlas-m1/progress.md` (fill the numbers from `summary.txt`; keep the shape):

```markdown
## TOOLCHAIN-1: COMPLETE (2026-09-XX)
- rust-toolchain.toml pins 1.97.1 in BOTH repos (atlas <sha>, map-generator <sha>); graph-types/tests/toolchain_pin.rs (+4) and map-types/tests/toolchain_pin.rs (+3) make the pin load-bearing: file exists, exact version, the compiler that built the test IS 1.97.1, the rustup proxy resolves it from the repo root.
- No behaviour change. Standing block: workspace N/0 across S sections, types N+4, timing (pre-CONTENTION-1, still in the parallel run at this point): as measured.

## CONTENTION-1: COMPLETE (2026-09-XX)
- The 8 wall-clock gates are #[ignore = REASON] in the default run and executed by scripts/timing-gates.sh (one cargo process each, --test-threads=1, --exact). Ceilings UNCHANGED (4s / 60s / 75,75,50,30,30,50ms).
- Anti-bypass: `timing-gates.sh check` (multiset reconciliation + no foreign #[ignore]); timing-gates-selftest.sh 9/9 (six bypasses attempted and refused, plus README and Cargo.toml naming the step).
- ACCEPTANCE: ten consecutive full runs, logs-contention1/summary.txt -- flips: 0. Observed spreads: load X.X–X.X s (ceiling 4 s); conformance XX–XX s (60 s); perf_smoke JHN.3 chapter XX–XX ms (50 ms); ... (one line per gate).
- Standing counting procedure is now THREE commands (server/Cargo.toml, README "Test suite"). New canonical block: workspace N passed / 0 failed / 8 ignored across S sections; types N; TIMING GATES 8/8.
```

- [ ] **Step 5: Commit the evidence and the ledger**

```bash
git add .superpowers/sdd/2026-08-17-bible-atlas-m1/logs-contention1/summary.txt \
        .superpowers/sdd/2026-08-17-bible-atlas-m1/logs-contention1/ten-runs.sh \
        .superpowers/sdd/2026-08-17-bible-atlas-m1/progress.md
git commit -F - <<'EOF'
chore(ledger): TOOLCHAIN-1 + CONTENTION-1 close -- ten consecutive full runs, zero flips

summary.txt holds the ten-run evidence (per-run counts and every gate's
measured time against its unchanged ceiling). progress.md records both
batches and the new three-command standing count.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
EOF
```

(If `.superpowers/` is gitignored in this repo — check with `git check-ignore -v .superpowers/sdd/2026-08-17-bible-atlas-m1/progress.md` — commit only what is tracked and say so in the report; the ledger's own convention governs.)

---

## Self-review (done while writing)

**Spec coverage.** §3.2 toolchain pin → Tasks 1–2 (both repos, both load-bearing). §8 row 0 gate ("full suite green; no behaviour change") → Task 1 Step 6, Task 2 Step 5. §8 row 1 ("serialized, process-isolated step; ceilings NOT loosened"; gate "ten consecutive full runs, zero flips") → Tasks 3–5; process isolation = one `cargo test` per gate with `--test-threads=1`; no ceiling constant is edited anywhere in this plan. §12 asks for measurements by the same methods — Task 5 captures every gate's time per run. §1.2 zero-dep `graph-types` → Task 1's test uses `std` only.

**Placeholders.** The ledger template in Task 5 Step 4 uses `XX`/`<sha>` for numbers that do not exist until the run happens; the step says to fill them from `summary.txt`. No other placeholders.

**Type/name consistency.** REASON string is byte-identical in Task 3 Step 1 (selftest), Step 3 (attributes), Step 4 (script), and Task 4 Step 2 (Cargo.toml quotes its prefix only). Test names in the Task 3 table, the script's `GATES` array, and the attribute sites match `artifact_conformance.rs:101`, `graph_conformance.rs:139`, `perf_smoke.rs:84,94,104,119,133,153` as read at commit `61b49db`. The summary line `TIMING GATES: N/8 passed` is produced by Task 3's script and consumed by Task 4's Cargo.toml text and Task 5's runner grep.
