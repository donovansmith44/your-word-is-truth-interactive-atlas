#!/usr/bin/env bash
# PRINCIPLES 3a: the batch's ONE mutation gate, sharded across throwaway git
# worktrees so it finishes in a fraction of the serial time. One invocation of
# cargo-mutants per crate per shard, each with its own `.cargo/mutants.toml`
# (the crate's scope and the covering test targets that kill its mutants) and
# its own CARGO_TARGET_DIR; the shards' outcomes are merged into ONE summary.
#
#   bash scripts/mutants-parallel.sh -n 8 -b 78f51ff        the batch's gate
#   bash scripts/mutants-parallel.sh -n 2 -F 'merge\.rs'    one file, 2 shards
#   bash scripts/mutants-parallel.sh -n 1 -F 'merge\.rs'    the same, serial
#
#   -n N      shards (default 8)
#   -b REF    score only the lines changed since REF (cargo-mutants `--in-diff`)
#   -F RE     cargo-mutants `--re`, matched against the names `--list` prints,
#             which begin with the file path -- the only way to narrow a run to
#             one file, since `-f` is UNIONED with the declared scope
#   -c CRATE  restrict to one crate's config (repeatable)
#   -k        keep the worktrees
#
# THE GATE SCORES HEAD, not the working tree: a shard is `git worktree add HEAD`,
# so a config, a fixture or a new test that is still uncommitted is not in the
# run. Commit first, then run.
#
# WHY WORKTREES AND NOT `--jobs`: `--jobs` needs the copied trees cargo-mutants
# builds for itself, and for this repository they cannot build at all. `server/`
# is the workspace root, so a copy holds `server/**` and nothing else, while
# `atlas-graph` depends on `path = "../../graph-types"` -- a sibling of the
# workspace root. Measured at cargo-mutants 27.1.0: the unmutated baseline fails
# in under a second with `failed to read <tmp>/graph-types/Cargo.toml`, before a
# mutant is applied. `--in-place` is therefore forced, `--in-place` and `--jobs`
# are exclusive, and a worktree per shard is what remains.
#
# WHY THE DATA IS COPIED AND NEVER LINKED: `git worktree remove --force`
# FOLLOWS an NTFS junction or symlink and deletes the real files through it --
# that is how 374 MB of `data/raw` was destroyed on 2026-09-28. A shard gets its
# own robocopy'd COPY of the two gitignored inputs, `data/raw` and `data/cache`;
# everything else its tests read -- `data/compiled`, `data/curated`,
# `contracts/`, `tests/fixtures/`, `rust-toolchain.toml` -- is tracked and so is
# already in the checkout. Nothing outside a shard's own worktree is ever
# written, which is why this script does NOT point the tests at the main
# repository's data.
#
# `data/cache` is copied rather than left to materialise itself because
# `decompress_verified` unpacks a section through a tmp path that carries no
# process id: the 58 `bibex` processes `atlas-cli`'s suite spawns in parallel
# then race on the same `<logical>.sqlite.tmp` and the baseline goes red
# ("required section core unavailable: cannot find the file"). Handing every
# shard a warm cache means no test materialises anything.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SHARD_ROOT="C:/mut"
SHARDS=8
BASE=""
EXAMINE_RE=""
KEEP=0
ONLY_CRATES=()
# scoop's gcc: the pinned x86_64-pc-windows-gnu toolchain ships no `dlltool`,
# so `libsqlite3-sys`'s bundled C build fails without it on PATH.
export PATH="$HOME/scoop/apps/gcc/current/bin:$PATH"

while [ $# -gt 0 ]; do
  case "$1" in
    -n) SHARDS="$2"; shift 2;;
    -b) BASE="$2"; shift 2;;
    -F) EXAMINE_RE="$2"; shift 2;;
    -c) ONLY_CRATES+=("$2"); shift 2;;
    -k) KEEP=1; shift;;
    *) echo "unknown argument $1" >&2; exit 64;;
  esac
done

# The two feature states of graph-types are ONE score, not either of them: the
# crate keeps a `canon-ids` arm and a `not(canon-ids)` arm of the same
# functions, and a mutation of whichever arm is switched off is a no-op that
# survives every test. `graph-types/.cargo/mutants.toml` states why the union is
# the only honest reading; the merge below is where the union is taken.
GRAPH_TYPES_FEATURES=("--features serde,openapi" "--all-features")

wanted() {
  [ ${#ONLY_CRATES[@]} -eq 0 ] || printf '%s\n' "${ONLY_CRATES[@]}" | grep -qx "$1"
}

# The one way a shard worktree is ever deleted, before a run and after it. A
# reparse point inside it would make either deletion reach the real files it
# points at -- `git worktree remove --force` and `rm -rf` both follow one -- so
# zero is the only count this proceeds on.
remove_worktree() {
  local wt="$1" links
  [ -d "$wt" ] || return 0
  links=$(powershell -NoProfile -Command \
    "(Get-ChildItem '$wt' -Recurse -Force -Attributes ReparsePoint -ErrorAction SilentlyContinue | Measure-Object).Count" | tr -d '\r')
  if [ "$links" != "0" ]; then
    echo "REFUSING to delete $wt: $links reparse point(s) -- 'cmd /c rmdir' each one first" >&2
    exit 70
  fi
  git -C "$ROOT" worktree remove --force "$wt" >/dev/null 2>&1
  rm -rf "$wt"
  git -C "$ROOT" worktree prune
}

# name <tab> directory to run in <tab> config path <tab> extra cargo-mutants args
runs() {
  for config in "$ROOT"/server/*/.cargo/mutants.toml; do
    crate="$(basename "$(dirname "$(dirname "$config")")")"
    wanted "$crate" && printf '%s\tserver\t%s/.cargo/mutants.toml\t\n' "$crate" "$crate"
  done
  if wanted graph-types; then
    local state=0
    for features in "${GRAPH_TYPES_FEATURES[@]}"; do
      state=$((state + 1))
      printf 'graph-types-%s\tgraph-types\t.cargo/mutants.toml\t%s\n' "$state" "$features"
    done
  fi
}

OUT="$SHARD_ROOT/out"
rm -rf "$OUT"
mkdir -p "$OUT"
if [ -n "$BASE" ]; then
  git -C "$ROOT" diff --relative=server "$BASE..HEAD" -- server > "$OUT/server.diff"
  git -C "$ROOT" diff --relative=graph-types "$BASE..HEAD" -- graph-types > "$OUT/graph-types.diff"
fi

# One jobserver slot per core, shared out between the shards, so eight
# concurrent `cargo build`s do not each believe they own all sixteen.
JOBSERVER_TASKS=$(( $(nproc) / SHARDS ))
[ "$JOBSERVER_TASKS" -lt 1 ] && JOBSERVER_TASKS=1

echo "=== $SHARDS shards x $(runs | wc -l) crate runs, base '${BASE:-none}', re '${EXAMINE_RE:-none}'"
START=$(date +%s)

shard() {
  local i="$1" wt="$SHARD_ROOT/s$1" target="$SHARD_ROOT/t$1"
  remove_worktree "$wt"
  # The target directory OUTLIVES the worktree: a shard's path is the same on
  # every run, so cargo's cache is still warm and a re-run pays no cold build.
  mkdir -p "$target"
  # `core.autocrlf=false`: the global setting is true, which checks the
  # `.feature` files out as CRLF and reddens `aqc_corpus_generation` before any
  # mutant is applied.
  if ! git -C "$ROOT" -c core.autocrlf=false worktree add --detach --quiet "$wt" HEAD; then
    echo "shard $i: worktree add failed" >> "$OUT/exits.txt"
    return
  fi
  local gitignored
  for gitignored in data/raw data/cache; do
    # MSYS_NO_PATHCONV: Git Bash otherwise rewrites `/E` into the path `E:/`
    # and robocopy rejects it as an invalid parameter.
    MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' \
      robocopy "$(cygpath -w "$ROOT/$gitignored")" "$(cygpath -w "$wt/$gitignored")" \
        /E /NFL /NDL /NJH /NJS /NP /MT:8 >/dev/null 2>&1
    # Robocopy reports what it did, not whether it failed: 0-7 are successes.
    if [ $? -ge 8 ]; then
      echo "shard $i: robocopy of $gitignored failed" >> "$OUT/exits.txt"
      return
    fi
  done
  local name dir config features args
  while IFS=$'\t' read -r name dir config features; do
    args=(--in-place --config "$config" --shard "$i/$SHARDS"
          --jobserver-tasks "$JOBSERVER_TASKS" -o "$OUT/$name.s$i")
    [ -n "$BASE" ] && args+=(-D "$OUT/$dir.diff")
    [ -n "$EXAMINE_RE" ] && args+=(-F "$EXAMINE_RE")
    # shellcheck disable=SC2086 -- $features is a word list on purpose
    ( cd "$wt/$dir" && CARGO_TARGET_DIR="$target" cargo mutants "${args[@]}" $features \
      ) > "$OUT/$name.s$i.log" 2>&1 \
      || echo "$name shard $i: cargo-mutants exit $?" >> "$OUT/exits.txt"
  done < <(runs)
}

# cargo-mutants numbers shards from zero: `--shard k/n` refuses k == n.
for i in $(seq 0 $((SHARDS - 1))); do shard "$i" & done
wait
ELAPSED=$(($(date +%s) - START))

MERGED="$ROOT/mutants.out-merged"
rm -rf "$MERGED"
mkdir -p "$MERGED"
for outcome in caught missed unviable timeout; do
  cat "$OUT"/*/mutants.out/"$outcome.txt" 2>/dev/null | sort -u > "$MERGED/$outcome.txt"
done
# THE UNION, and the whole reason graph-types is run twice. A mutant of a
# `#[cfg(feature = ...)]` arm that is switched OFF is not applied to anything the
# compiler sees, so it survives every test and is reported MISSED; the run that
# switches that arm ON is the one whose verdict is real. So: a name any run
# caught is caught, a name any run found unviable is unviable, and only what
# every run merely missed is a survivor.
for outcome in missed timeout; do
  cat "$MERGED/caught.txt" "$MERGED/unviable.txt" | sort -u > "$MERGED/decided.tmp"
  grep -vxF -f "$MERGED/decided.tmp" "$MERGED/$outcome.txt" > "$MERGED/$outcome.tmp"
  mv "$MERGED/$outcome.tmp" "$MERGED/$outcome.txt"
done
grep -vxF -f "$MERGED/caught.txt" "$MERGED/unviable.txt" > "$MERGED/unviable.tmp"
mv "$MERGED/unviable.tmp" "$MERGED/unviable.txt"
rm -f "$MERGED/decided.tmp"

{
  echo "# Merged mutation gate: $SHARDS shards, ${ELAPSED}s wall clock"
  echo
  printf '%-48s %7s %7s %7s %8s %8s\n' file tested caught missed timeout unviable
  for outcome in caught missed timeout unviable; do
    awk -v o="$outcome" -F: '{print $1"\t"o}' "$MERGED/$outcome.txt"
  done | sort | awk -F'\t' '
    { n[$1]++; c[$1"\t"$2]++ }
    END {
      for (f in n) printf "%-48s %7d %7d %7d %8d %8d\n", f, n[f], c[f"\tcaught"], c[f"\tmissed"], c[f"\ttimeout"], c[f"\tunviable"]
    }' | sort
  echo
  printf '%-48s %7d %7d %7d %8d %8d\n' TOTAL \
    "$(cat "$MERGED"/{caught,missed,timeout,unviable}.txt | wc -l)" \
    "$(wc -l < "$MERGED/caught.txt")" "$(wc -l < "$MERGED/missed.txt")" \
    "$(wc -l < "$MERGED/timeout.txt")" "$(wc -l < "$MERGED/unviable.txt")"
  if [ -f "$OUT/exits.txt" ]; then echo; echo "non-zero exits:"; cat "$OUT/exits.txt"; fi
} | tee "$MERGED/summary.txt"

if [ "$KEEP" -eq 0 ]; then
  for i in $(seq 0 $((SHARDS - 1))); do remove_worktree "$SHARD_ROOT/s$i"; done
fi

[ ! -s "$MERGED/missed.txt" ] && [ ! -s "$MERGED/timeout.txt" ]
