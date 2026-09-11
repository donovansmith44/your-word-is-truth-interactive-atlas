#!/usr/bin/env bash
# THE CONTRACT GATE (batch CDC-1; INVERTED in fix round 3).
#
# Owner order 2: "we will get to a point where we cannot push or do
# anything if our contracts don't agree. that is target shape."
#
# =====================================================================
# WHY THIS SCRIPT WAS TURNED INSIDE OUT
# =====================================================================
# Three reviews beat this gate. Rounds 1 and 2 closed each demonstrated
# probe and the next review defeated its classmates. The third review named
# the species exactly, and it was not "we missed a case":
#
#   EVERY PROTECTION WAS A NEGATIVE GUARD CONDITIONED ON SOMETHING THAT
#   COULD ITSELF BE REMOVED OR RENAMED.
#
#   * `[ -f "$d/RECEIVED.md" ] || continue`  -- delete the marker and every
#     received-suite protection switched off. Leg 0 then PRINTED
#     "no received suite is disarmed" while the suite was disarmed.
#   * `contracts/SUITES` one-word edit -- the graph-primary suite, the
#     owner's ruled #1 juncture, left four legs and the gate said PASSED.
#   * rename the `.feature` files -- the suite vanished from ROOTS, only
#     leg 3 objected, calling it "provider drift", and the remedy the gate
#     itself printed (`ATLAS_BLESS_PACT=1 ...`) shrank the pact from 175,656
#     to 34,257 bytes and turned the gate green.
#
# Guard-by-guard fixes twice closed the probe and left the class alive,
# because in every case THE GATE ASKED "IS ANYTHING WRONG?" of a set the
# author controlled the membership of. An empty set has nothing wrong with
# it.
#
# So the question is now the other way round:
#
#   DERIVE, FROM COMMITTED GIT CONTENT, THE COMPLETE SET OF EXPECTATIONS
#   THAT EXIST -- every scenario in every suite -- AND REQUIRE EVIDENCE
#   THAT EACH ONE WAS EXECUTED AND PASSED.
#
# Under that shape:
#   - deleting a marker, a registry row, a directory or a `.feature` file
#     REDUCES nothing the gate consults and produces MISSING COVERAGE;
#   - renaming launders nothing: the inventory is content-derived from
#     `git`, and the count must reconcile against the runner's own results;
#   - there is no guard left to defeat, because protection is not
#     conditional on any artifact in the working tree.
#
# =====================================================================
# THE TRUSTED BASE -- stated explicitly, because everything else is derived
# =====================================================================
# A gate cannot verify everything; it can only be honest about the floor it
# stands on. This gate assumes exactly four things, and nothing else:
#
#  1. `git` itself, and the object database. The inventory, the registry,
#     the suite set and the file modes are all read from `git ls-tree` /
#     `git archive` / `git show` against HEAD and the base. If git lies, or
#     the repository is rewritten under us, nothing here holds.
#     WHY DEFENSIBLE: git is what the push publishes. Grading anything else
#     grades a document that will not be shipped -- which was exactly review
#     H-R2-3 (the fixture grader read `old` from git and `new` from disk).
#
#  2. The POSIX toolchain this script is written in (bash, awk, sort, comm,
#     find, tar, mktemp) and the shell it is invoked by.
#     WHY DEFENSIBLE: it is the interpreter of the gate itself. Someone who
#     can substitute `bash` can edit this file.
#
#  3. `cargo` and `cabal` AT FIXED ABSOLUTE PATHS, resolved from a list
#     compiled into this file. The gate reads NO environment variable that
#     selects a tool (review H-R2-1: `CARGO=`/`CABAL=` are gone, not
#     validated), so PATH and the environment cannot substitute either one.
#     Someone who can overwrite `~/.cargo/bin/cargo` already owns the
#     machine that runs the gate.
#     WHY DEFENSIBLE: it is a smaller assumption than the previous rounds'
#     ("whatever `$CARGO` points at, if it greets us correctly"), and it is
#     the same assumption the compiler makes.
#
#  4. `contract-runner`, BUILT BY THIS SCRIPT from the vendored Haskell
#     source under `contracts/runner/` using assumption 3. It is the tag
#     oracle, the fixture grader (review H-R2-2: no more `python` off PATH)
#     and the executor. It is not looked up on PATH and cannot be passed in.
#     WHY DEFENSIBLE: its source is in the repository, under the same review
#     as everything else, and a change to it is a change to a diff a human
#     reads. A binary found on PATH is not.
#
# NOT trusted, and therefore derived or refused: the working tree, the
# contents of `contracts/SUITES`, the presence of any marker file, the
# existence of any directory, the name of any file, `python`, `sha256sum`
# (calibrated on unpredictable input), and every environment variable.
#
# There is no --allow, no advisory mode and no skip flag. `--fast` skips
# only leg 3 and EXITS 3, never 0.
#
# Usage:
#   scripts/contract-gate.sh              full gate (what pre-push runs)
#   scripts/contract-gate.sh --fast       skip the recorders while iterating
#   scripts/contract-gate.sh --base REF   semver base (default: auto)
set -uo pipefail
cd "$(dirname "$0")/.."

FAST=0
BASE=""
BASE_EXPLICIT=0
while [ $# -gt 0 ]; do
  case "$1" in
    --fast) FAST=1; shift;;
    --base) BASE="${2:?--base needs a ref}"; BASE_EXPLICIT=1; shift 2;;
    *) echo "unknown argument: $1" >&2; exit 2;;
  esac
done

fail=0
step() { printf '\n=== %s ===\n' "$1"; }
check() { if [ "$1" -ne 0 ]; then echo "FAILED: $2" >&2; fail=1; fi; }

TMPDIRS=()
cleanup() { for d in "${TMPDIRS[@]:-}"; do [ -n "$d" ] && rm -rf "$d"; done; }
trap cleanup EXIT

# Machine output from the runner is consumed STRUCTURALLY: every field is
# percent-escaped by `Tags.escField`, so a tab or a newline cannot occur
# inside one and `-F'\t'` is a total parse. The only lexing this script does
# is stripping the CR that a Windows text handle appends to a line, and
# folding the `\` the Windows runtime emits in paths -- neither of which can
# appear inside an escaped field.
# `grep -c .` prints 0 and EXITS 1 on an empty file, so the obvious
# `$(grep -c . f || echo 0)` yields a TWO-LINE string ("0" then "0") and
# every later arithmetic test on it is a syntax error. One helper, used
# everywhere a count is taken.
count_lines() { awk 'NF{n++} END{print n+0}' "$1" 2>/dev/null || echo 0; }
count_stdin() { awk 'NF{n++} END{print n+0}'; }

# The backslash fold is applied to the PATH FIELD ONLY. The Windows runtime
# emits `dir\file.feature` for a path built from a forward-slashed argument,
# and both the inventory and the results must key on the same spelling --
# but a blanket `tr '\\' '/'` would also rewrite a backslash inside a
# SCENARIO NAME, which is the same class of mistake as re-lexing the
# oracle's answer (review H-R2-4): it edits data while trying to normalise
# structure.
norm_rows() { tr -d '\r' | awk -F'\t' -v OFS='\t' 'NF>=2 { gsub(/\\/, "/", $1); print }'; }

# ---------------------------------------------------------------------
# TOOLCHAIN (review H-R2-1) -- resolved, never accepted
# ---------------------------------------------------------------------
# Round 1 answered `CARGO=/usr/bin/true` by requiring `$CARGO --version` to
# begin with "cargo". Round 2's reviewer wrote a three-line shim that says
# it. Round 2 answered THAT by requiring `test result: ok. N passed` in the
# output. Round 3's reviewer wrote a six-line shim that prints it -- and the
# gate then reported "leg 3 (CLI recorder): 4 test(s) passed" for a recorder
# that has exactly one test, and did not notice.
#
# Identity-by-greeting and identity-by-echoed-summary are the same mistake:
# both ask the tool to describe itself. The fix is not a better question. It
# is to stop letting the environment choose who answers.
#
# There is no CARGO or CABAL variable here. The paths are compiled in, tried
# in order, and the first that exists AND identifies itself wins. The name
# check survives only as a diagnostic for a broken install.
resolve_tool() { # <name> <candidate...> -> absolute path on stdout
  local want="$1"; shift
  local c out
  for c in "$@"; do
    [ -x "$c" ] || continue
    out="$("$c" --version 2>/dev/null)" || continue
    case "$out" in "$want"*) printf '%s\n' "$c"; return 0;; esac
  done
  return 1
}
CARGO="$(resolve_tool cargo "$HOME/.cargo/bin/cargo" /usr/local/bin/cargo /usr/bin/cargo)" || {
  echo "FAILED: no cargo found at any of the paths this gate is willing to use." >&2
  echo "  The gate does not read \$CARGO, and does not search \$PATH: both are ways" >&2
  echo "  for a substituted toolchain to answer for the real one (review H-R2-1)." >&2
  exit 1
}
CABAL="$(resolve_tool cabal "$HOME/.local/bin/cabal" /usr/local/bin/cabal /usr/bin/cabal)" || {
  echo "FAILED: no cabal found at any of the paths this gate is willing to use." >&2
  exit 1
}
step "toolchain"
echo "cargo:  $CARGO"
echo "cabal:  $CABAL"
echo "(resolved from compiled-in paths; \$CARGO and \$CABAL are not read)"

step "building the contract runner"
( cd contracts/runner && "$CABAL" build all >/dev/null 2>&1 )
check $? "contracts/runner does not build"
RUNNER="$(cd contracts/runner && "$CABAL" list-bin contract-runner 2>/dev/null)"
if [ -z "${RUNNER:-}" ] || [ ! -x "$RUNNER" ]; then
  echo "FAILED: no contract-runner binary" >&2
  exit 1
fi

# =====================================================================
# THE COMMITTED TREE -- the only thing this gate grades
# =====================================================================
# `git archive HEAD -- contracts` materialises exactly what a push would
# publish. Every derivation below reads THIS, never the working tree:
# the suite set, the registry, the received-suite list, and the scenario
# inventory. An edit that is not committed cannot change what the gate
# believes exists -- which is what makes `rm RECEIVED.md`, a registry word
# and a `mv *.feature *.feature.bak` inert rather than exempting.
HEADSHA="$(git rev-parse HEAD 2>/dev/null)"
if [ -z "${HEADSHA:-}" ]; then
  echo "FAILED: this is not a git repository with a HEAD, so nothing can be derived from committed content." >&2
  exit 1
fi
# ---------------------------------------------------------------------
# LEG 8 -- FILE MODES, and the symlink question, settled
# ---------------------------------------------------------------------
# BEFORE the archive, deliberately. A symlink entry makes `git archive |
# tar -x` fail on Windows, so auditing after materialising produced a
# refusal with the WRONG REASON ("could not materialise contracts/ from
# HEAD"). Fail-closed, so never a green lie -- but a message that names the
# wrong event is precisely how C-R2-3 happened, and my own self-test (B25)
# caught this only because it asserts WHY the gate refused.
#
# The round-2 reviewer's symlink probe was INCONCLUSIVE: MSYS `ln -s`
# deep-copies rather than links, so the attack could not be run on this
# platform and was honestly reported as unproven rather than closed.
#
# It is settled here without needing the platform to cooperate, because git
# records the mode regardless of what the filesystem did: a symlink is a
# `120000` blob whose CONTENT is a path, and a submodule is a `160000`
# gitlink. Either one under `contracts/` would let a `.feature` file point
# somewhere the inventory does not follow -- or make `git archive`'s output
# differ from what a checkout produces. Neither is a thing a contract corpus
# has any use for, so both are refused outright, by mode, from the object
# database. `git update-index --add --cacheinfo 120000,<sha>,<path>` creates
# one on Windows with no filesystem symlink at all.
step "leg 8/8: committed file modes under contracts/"
badmodes="$(git ls-tree -r "$HEADSHA" -- contracts 2>/dev/null \
            | awk '$1 != "100644" && $1 != "100755" { print "    " $1 " " $4 }')"
if [ -n "$badmodes" ]; then
  echo "FAILED: contracts/ contains committed entries that are not regular files:" >&2
  printf '%s\n' "$badmodes" >&2
  echo "  A symlink (120000) or a gitlink (160000) in a contract corpus is a way to" >&2
  echo "  make what the gate reads differ from what a consumer checks out." >&2
  echo "CONTRACT GATE: FAILED" >&2
  exit 1
fi
echo "every committed entry under contracts/ is a regular file (no symlinks, no gitlinks)"

HEADTREE="$(mktemp -d)"; TMPDIRS+=("$HEADTREE")
if ! git archive "$HEADSHA" -- contracts 2>/dev/null | tar -x -C "$HEADTREE" 2>/dev/null; then
  echo "FAILED: could not materialise contracts/ from HEAD ($HEADSHA)." >&2
  echo "  The gate grades committed content. If it cannot read committed content," >&2
  echo "  it has nothing to say and must not say PASSED." >&2
  exit 1
fi
if [ ! -d "$HEADTREE/contracts" ]; then
  echo "FAILED: HEAD carries no contracts/ directory at all." >&2
  exit 1
fi

# ---------------------------------------------------------------------
# LEG 7 -- WHAT IS GRADED IS WHAT WILL BE PUSHED (review H-R2-3)
# ---------------------------------------------------------------------
# `contract-semver-gate.sh:330-331` read `old` from `git show "$base:$f"`
# and `new` from `cat "$f"`. A `git stash`, a partial `git checkout --`
# while debugging a red, or an interrupted rebase produces a worktree that
# differs from HEAD, and the gate then signed off on a change it did not
# read: the reviewer re-blessed a fixture IN A COMMIT, reverted only the
# working-tree copy, and got `diff requires patch, declared patch`, exit 0,
# while `git show HEAD:...` still held the re-bless.
#
# The grader now reads both sides from git (that fix is in the semver gate),
# and this leg states the same rule for the whole gate, out loud: a pre-push
# gate grades commits. If the tree disagrees with HEAD on any path the gate
# grades, the run is reporting on a document nobody will receive.
#
# Scoped to the paths the gate actually grades, so unrelated work in
# progress does not block a push about contracts.
#
# UNTRACKED FILES COUNT (found by this round's own probe A9e, which nobody
# had run). `git diff --name-only HEAD` reports only TRACKED paths, so a
# brand-new `contracts/atlas-edge/SOMETHING.feature` sitting in the working
# tree was invisible to this leg entirely -- the coverage reconciliation
# caught it as an UNKNOWN scenario, but leg 7, whose whole job is "the tree
# agrees with HEAD", said the tree agreed. A leg that announces a positive
# result it did not establish is the exact species this round exists to
# remove, so the second half is `git ls-files --others`.
step "leg 7/8: the working tree agrees with HEAD on every graded path"
drift="$(git diff --name-only HEAD -- contracts data/exports 2>/dev/null)"
untracked="$(git ls-files --others --exclude-standard -- contracts data/exports 2>/dev/null)"
if [ -n "$drift" ] || [ -n "$untracked" ]; then
  echo "FAILED: these graded paths differ from HEAD, so the gate would grade something else:" >&2
  [ -n "$drift" ] && printf '%s\n' "$drift" | sed 's/^/    modified:  /' >&2
  [ -n "$untracked" ] && printf '%s\n' "$untracked" | sed 's/^/    untracked: /' >&2
  echo "  A pre-push gate grades what will be pushed. Commit these, or restore them." >&2
  fail=1
else
  echo "contracts/ and data/exports/ are byte-identical to HEAD (no modifications, no untracked files)"
fi

# =====================================================================
# SUITES -- derived from committed content, classified by a registry that
# is read from committed content and can only ever ADD obligations
# =====================================================================
# `contracts/SUITES` is read from HEAD, not from disk, so editing it in the
# working tree changes nothing at all. Editing it IN A COMMIT is caught by
# the semver gate's harness ratchet (a suite's harness strength may never
# decrease between the base and HEAD), and by the coverage reconciliation
# below, which does not consult the registry when deciding what must exist.
REGISTRY="$HEADTREE/contracts/SUITES"
[ -f "$REGISTRY" ] || { echo "FAILED: HEAD carries no contracts/SUITES -- the gate cannot tell which harness owns which suite" >&2; exit 1; }

harness_of() { # <dir>
  awk -v want="$1" '$1 !~ /^#/ && NF>=2 && $1==want {print $2; found=1} END{if(!found) print ""}' "$REGISTRY" | head -1
}

suite_root() { # <dir relative to the repo root>
  local d="$1"
  while [ -n "$d" ] && [ "$d" != "." ] && [ "$d" != "contracts" ]; do
    if [ -f "$HEADTREE/$d/VERSION" ] || [ -f "$HEADTREE/$d/RECEIVED.md" ]; then printf '%s\n' "$d"; return; fi
    d="$(dirname "$d")"
  done
  printf '%s\n' "$1"
}

ROOTS=()
while IFS= read -r f; do
  [ -z "$f" ] && continue
  r="$(suite_root "$(dirname "$f")")"
  case " ${ROOTS[*]:-} " in *" $r "*) ;; *) ROOTS+=("$r");; esac
done < <( cd "$HEADTREE" && find contracts -name '*.feature' -not -path 'contracts/runner/*' 2>/dev/null | sort )

if [ "${#ROOTS[@]}" -eq 0 ]; then
  echo "FAILED: HEAD carries no contract suites under contracts/ -- an empty corpus is a broken gate, not a passing one" >&2
  exit 1
fi

# `aqc-dual` is accepted ONLY where the vendored parser genuinely cannot
# read the corpus, and that is now tested by ASKING THE PARSER rather than
# by grepping the directory (review C-R2-2). The round-2 derivation was
# `grep -r 'Scenario Outline' "$d"` over EVERY file in the directory, which
# a three-line NOTES.md satisfied -- so the graph-primary suite was excused
# from four legs by a sentence in a note. A parse failure is a fact about
# the corpus that no adjacent file can manufacture.
parser_can_read() { # <dir under HEADTREE>
  ( cd "$HEADTREE" && "$RUNNER" tags "$1" >/dev/null 2>&1 )
}

RUN_ROOTS=()      # we are the provider: legs 0,1,2,4 and per-scenario coverage
CHECK_ROOTS=()    # we are the consumer: legs 0,1,2 and suite-level coverage
DUAL_ROOTS=()     # run by their own harnesses; coverage is a committed count
for d in "${ROOTS[@]}"; do
  h="$(harness_of "$d")"

  if [ -f "$HEADTREE/$d/RECEIVED.md" ] && [ "$h" != "contract-runner" ]; then
    echo "FAILED: $d carries RECEIVED.md at HEAD but is registered '$h' in contracts/SUITES." >&2
    echo "  A suite we received is another repo's expectations OF US. It is executed" >&2
    echo "  here or the gate fails; there is no registration that exempts it." >&2
    fail=1
    h=contract-runner   # gate it anyway, so this run still executes it
  fi
  if [ "$h" = "aqc-dual" ] && parser_can_read "$d"; then
    echo "FAILED: $d is registered 'aqc-dual', but the vendored parser reads it fine." >&2
    echo "  aqc-dual exists for corpora Gherkin/Parse.hs CANNOT parse (Scenario Outline)." >&2
    echo "  'some other harness owns this' is a claim; 'the parser fails on this' is a fact." >&2
    fail=1
    h=contract-runner
  fi

  case "$h" in
    contract-runner)          RUN_ROOTS+=("$d"); CHECK_ROOTS+=("$d");;
    contract-runner-consumer) CHECK_ROOTS+=("$d");;
    aqc-dual)                 DUAL_ROOTS+=("$d");;
    "")
      echo "FAILED: the suite $d is not registered in contracts/SUITES." >&2
      echo "  Every suite must declare which harness runs it, so a new one cannot" >&2
      echo "  silently escape the gate the way contracts/atlas-edge once did." >&2
      fail=1;;
    *)
      echo "FAILED: the suite $d declares unknown harness '$h' in contracts/SUITES." >&2
      fail=1;;
  esac
done

# VACUITY (review C-R2-2 §A.15). `ROOTS` was already refused when empty and
# `RUN_ROOTS` was not, so a configuration that executed NOTHING printed
# `suites (executed): none` and passed. A gate that runs nothing is not a
# gate that found nothing wrong.
if [ "${#RUN_ROOTS[@]}" -eq 0 ]; then
  echo "FAILED: no suite is EXECUTED by this gate (RUN_ROOTS is empty)." >&2
  echo "  Legs 1 and 2 parse; only leg 4 executes. A configuration in which nothing" >&2
  echo "  executes must never read as success." >&2
  fail=1
fi

# A registry row pointing at a directory that does not exist at HEAD.
while read -r rdir rharness; do
  case "$rdir" in ''|\#*) continue;; esac
  [ -n "${rharness:-}" ] || continue
  if [ ! -d "$HEADTREE/$rdir" ]; then
    echo "FAILED: contracts/SUITES lists $rdir, which does not exist at HEAD." >&2
    echo "  Deleting a contract suite is a cross-consumer break, not a tidy-up." >&2
    fail=1
  fi
done < <(grep -vE '^[[:space:]]*(#|$)' "$REGISTRY")

echo "suites (executed):     ${RUN_ROOTS[*]:-none}"
echo "suites (checked only): ${CHECK_ROOTS[*]:-none}"
echo "suites (own harness):  ${DUAL_ROOTS[*]:-none}"

# =====================================================================
# THE INVENTORY -- every expectation that EXISTS, from committed content
# =====================================================================
# One row per scenario: `<suite-relative path>\t<scenario>\t<tags>`, every
# field percent-escaped by the runner. This is the set the whole gate is now
# organised around: it is computed before anything runs, from `git archive
# HEAD`, and nothing that happens later can shorten it.
INV="$(mktemp)"; TMPDIRS+=("$INV")
: > "$INV"
inv_ok=1
for d in "${ROOTS[@]}"; do
  case " ${DUAL_ROOTS[*]:-} " in *" $d "*) continue;; esac   # counted, not parsed -- see below
  if ! ( cd "$HEADTREE" && "$RUNNER" tags "$d" 2>&1 ) | norm_rows >> "$INV"; then
    echo "FAILED: could not read the committed corpus of $d -- the inventory is incomplete." >&2
    echo "  An unparseable suite is not a suite with no expectations." >&2
    fail=1; inv_ok=0
  fi
done
INV_N="$(count_lines "$INV")"
echo "inventory (committed): $INV_N scenario(s) across $(( ${#ROOTS[@]} - ${#DUAL_ROOTS[@]} )) parseable suite(s), ${#DUAL_ROOTS[@]} run by their own harness"
if [ "$INV_N" -eq 0 ] && [ "$inv_ok" -eq 1 ]; then
  echo "FAILED: the committed inventory is EMPTY -- there is nothing to prove coverage of." >&2
  fail=1
fi

# ---------------------------------------------------------------------
# LEG 0 -- A RECEIVED SUITE MAY NOT BE DISARMED
# ---------------------------------------------------------------------
# The received-suite list is derived from HEAD, so `rm RECEIVED.md` in the
# working tree removes nothing from it (review C-R2-1: that one `rm` used to
# switch off all three received-suite protections at once, after which leg 0
# printed `no received suite is disarmed` while the suite WAS disarmed).
#
# Both trees are asked: HEAD because that is what will be pushed, and the
# working tree because a developer wants to know now. `--forbid` uses the
# runner's exit code, and an unparseable corpus fails it too.
step "leg 0/8: received suites carry no @target"
received=()
for d in "${ROOTS[@]}"; do
  [ -f "$HEADTREE/$d/RECEIVED.md" ] && received+=("$d")
done
if [ "${#received[@]}" -eq 0 ]; then
  echo "FAILED: HEAD carries no RECEIVED suite at all." >&2
  echo "  contracts/atlas-edge is map-generator's expectations of us and the reason" >&2
  echo "  leg 0 exists; if it is genuinely gone, that is a cross-repo negotiation." >&2
  fail=1
fi
disarmed=0
for d in "${received[@]:-}"; do
  [ -z "$d" ] && continue
  for where in HEAD worktree; do
    if [ "$where" = HEAD ]; then
      hits="$( cd "$HEADTREE" && "$RUNNER" tags "$d" --forbid target 2>&1 )"; rc=$?
    else
      hits="$( "$RUNNER" tags "$d" --forbid target 2>&1 )"; rc=$?
    fi
    if [ "$rc" -ne 0 ]; then
      echo "FAILED: the RECEIVED suite $d is disarmed, missing or unparseable at $where -- we may not withdraw another repo's guarantee:" >&2
      printf '%s\n' "$hits" | grep -E 'FORBIDDEN|^tags:' | sed 's/^/    /' >&2
      echo "  If their expectation of us is genuinely wrong: break, report, and coordinate a bump on BOTH sides." >&2
      fail=1; disarmed=1
    fi
  done
done
[ "$disarmed" -eq 0 ] && [ "${#received[@]}" -gt 0 ] \
  && echo "no received suite is disarmed (${#received[@]} suite(s), checked at HEAD and in the working tree)"

step "leg 1/8: totality (check)"
for d in "${CHECK_ROOTS[@]:-}"; do
  [ -z "$d" ] && continue
  "$RUNNER" check "$d"; check $? "totality: $d"
done

step "leg 2/8: vocabulary drift (vocab)"
for d in "${CHECK_ROOTS[@]:-}"; do
  [ -z "$d" ] && continue
  "$RUNNER" vocab "$d"; check $? "vocabulary drift: $d"
done

# ---------------------------------------------------------------------
# LEG 3 -- PROVIDER DRIFT.
# ---------------------------------------------------------------------
if [ "$FAST" -eq 0 ]; then
  step "leg 3/8: provider drift (the recorders)"

  # THE PACT FINGERPRINT, fail-closed, and now with a hasher CALIBRATED ON
  # UNPREDICTABLE INPUT (review L-R2-2). A `sha256sum` on PATH that prints a
  # constant satisfied a before/after comparison permanently. Asking it
  # three fixed questions would repeat the H-R2-2 mistake -- constants can
  # be answered by a `case` -- so it is asked about two byte strings
  # generated at run time: it must give them DIFFERENT digests and must give
  # the same string the same digest twice.
  HASHER=""
  for h in sha256sum shasum sha1sum md5sum; do
    command -v "$h" >/dev/null 2>&1 || continue
    _c="$(mktemp -d)"; TMPDIRS+=("$_c")
    head -c 64 /dev/urandom > "$_c/a" 2>/dev/null || date +%s%N > "$_c/a"
    head -c 64 /dev/urandom > "$_c/b" 2>/dev/null || { sleep 0; date +%s%N%N > "$_c/b"; }
    a1="$("$h" < "$_c/a" 2>/dev/null)"; a2="$("$h" < "$_c/a" 2>/dev/null)"; b1="$("$h" < "$_c/b" 2>/dev/null)"
    if [ -n "$a1" ] && [ "$a1" = "$a2" ] && [ "$a1" != "$b1" ]; then HASHER="$h"; break; fi
    echo "  note: '$h' did not discriminate two random inputs; not using it as the fingerprint tool" >&2
  done
  if [ -z "$HASHER" ]; then
    echo "FAILED: no hashing tool that demonstrably discriminates is available." >&2
    echo "  The pact fingerprint is what proves the gate did not rewrite its own" >&2
    echo "  evidence; a constant-printing hasher would make it always agree." >&2
    fail=1
  fi
  pact_fingerprint() {
    find contracts/pacts -type f -name '*.json' -print0 2>/dev/null \
      | sort -z | xargs -0 "$HASHER" | "$HASHER"
  }
  before=""
  if [ -n "$HASHER" ]; then
    before="$(pact_fingerprint)"
    if [ -z "$before" ]; then
      echo "FAILED: could not fingerprint contracts/pacts (empty result from $HASHER)." >&2
      fail=1
    fi
  fi

  # EVIDENCE TIED TO THE ACTUAL SUITE (review H-R2-1, second half). Round 2
  # required `test result: ok. N passed`, N >= 1, and then printed
  # "leg 3 (CLI recorder): 4 test(s) passed" for a recorder that has ONE
  # test -- it collected evidence and never checked it for plausibility.
  # The named tests are now bound: each must appear with `... ok`, and the
  # count must be exactly the number named. A summary line on its own proves
  # nothing about which tests ran.
  require_named_tests() { # <label> <log> <expected-name>...
    local label="$1" log="$2"; shift 2
    local want=("$@") missing=() n t
    for t in "${want[@]}"; do
      grep -qE "^test ${t} \.\.\. ok$" "$log" || missing+=("$t")
    done
    n="$(grep -cE '^test .+ \.\.\. ok$' "$log" 2>/dev/null || echo 0)"
    if [ "${#missing[@]}" -ne 0 ]; then
      echo "FAILED: $label did not run the tests this gate depends on:" >&2
      printf '    missing: %s\n' "${missing[@]}" >&2
      echo "  A summary line is a claim about work; a named passing test is the work." >&2
      sed 's/^/    /' "$log" | tail -15 >&2
      return 1
    fi
    if [ "$n" -ne "${#want[@]}" ]; then
      echo "FAILED: $label reported $n passing test(s); this recorder has exactly ${#want[@]}." >&2
      echo "  A count that does not match the suite is evidence about some other suite." >&2
      return 1
    fi
    echo "  $label: ${#want[@]}/${#want[@]} named tests passed"
    return 0
  }

  http_log="$(mktemp)"; cli_log="$(mktemp)"
  ( cd server && env -u ATLAS_BLESS_PACT "$CARGO" test -p atlas-server --test contract_pact ) >"$http_log" 2>&1
  check $? "provider drift: the HTTP pact no longer matches the live graph"
  require_named_tests "leg 3 (HTTP recorder)" "$http_log" \
    the_recorded_pact_still_matches_the_live_graph \
    the_assembled_app_is_not_hollow_on_any_derived_index \
    request_keys_are_read_out_of_step_lines_exactly \
    the_published_vocabulary_is_drawn_from_the_macros || fail=1

  ( cd server && env -u ATLAS_BLESS_PACT "$CARGO" test -p atlas-cli --test contract_pact_cli ) >"$cli_log" 2>&1
  check $? "provider drift: the CLI pact no longer matches the real bibex binary"
  require_named_tests "leg 3 (CLI recorder)" "$cli_log" \
    the_recorded_cli_pact_still_matches_the_real_binary || fail=1
  rm -f "$http_log" "$cli_log"

  if [ -n "$HASHER" ] && [ -n "$before" ]; then
    after="$(pact_fingerprint)"
    if [ -z "$after" ] || [ "$before" != "$after" ]; then
      echo "FAILED: contracts/pacts was REWRITTEN while this gate was running (or could not be re-read)." >&2
      echo "  The gate verifies evidence; it must never be the thing that produces it." >&2
      fail=1
    fi
  fi
else
  step "leg 3/8: SKIPPED (--fast)"
fi

# ---------------------------------------------------------------------
# LEG 4 -- EXPECTATION DRIFT, and the execution record
# ---------------------------------------------------------------------
# The runner now writes one row per scenario it actually executed. That file
# -- the executor's own account of its own work -- is what leg 6 reconciles
# against the committed inventory.
step "leg 4/8: expectations (run --replay)"
RESULTS="$(mktemp)"; TMPDIRS+=("$RESULTS")
: > "$RESULTS"
for d in "${RUN_ROOTS[@]:-}"; do
  [ -z "$d" ] && continue
  one="$(mktemp)"; TMPDIRS+=("$one")
  "$RUNNER" run --replay contracts/pacts --exports data/exports --results "$one" "$d"
  check $? "expectations: $d"
  norm_rows < "$one" >> "$RESULTS"
done

# =====================================================================
# LEG 6 -- COVERAGE. THE LEG THIS ROUND EXISTS FOR.
# =====================================================================
# Every scenario in the committed inventory must appear in the executor's
# own results with an acceptable verdict. This is the inversion: the gate no
# longer asks "did anything go wrong?" of a set the author controls, it
# asks "did everything that exists actually run?" of a set derived from git.
#
#   MISSING   in the inventory, not in the results -> a committed
#             expectation that did not execute. Deleting a marker, a
#             registry row or a `.feature` file lands here.
#   UNKNOWN   in the results, not in the inventory -> something executed
#             that is not committed. (Leg 6's tree check makes this nearly
#             unreachable; it is kept because "nearly" is not "cannot".)
#   FAILED    executed and red.
#
# A `@target` scenario on a suite we version may be red -- that is the
# documented disclosure mechanism, it is counted and printed, and the semver
# classifier makes ADDING the tag a MAJOR event. On a RECEIVED suite leg 0
# has already refused it outright.
step "leg 6/8: coverage -- every committed expectation executed"
cov_missing="$(mktemp)"; TMPDIRS+=("$cov_missing")
cov_unknown="$(mktemp)"; TMPDIRS+=("$cov_unknown")

inv_keys="$(mktemp)"; TMPDIRS+=("$inv_keys")
res_keys="$(mktemp)"; TMPDIRS+=("$res_keys")
awk -F'\t' 'NF>=2 {print $1 "\t" $2}' "$INV" | sort -u > "$inv_keys"
awk -F'\t' 'NF>=3 {print $1 "\t" $2}' "$RESULTS" | sort -u > "$res_keys"

# Only the suites we EXECUTE are reconciled per scenario. Consumer suites
# are not ours to run (their provider is somebody else's server) and are
# held to legs 0/1/2 plus the semver ratchet; that boundary is derived from
# the registry AT HEAD and cannot be widened by a working-tree edit.
run_prefixes="$(printf '%s\n' "${RUN_ROOTS[@]:-}" | sed '/^$/d')"
in_run_scope() { # <key path>
  local p="$1" r
  while IFS= read -r r; do
    [ -z "$r" ] && continue
    case "$p" in "$r"/*) return 0;; esac
  done <<< "$run_prefixes"
  return 1
}

while IFS= read -r k; do
  [ -z "$k" ] && continue
  in_run_scope "${k%%$'\t'*}" || continue
  grep -Fqx "$k" "$res_keys" || printf '%s\n' "$k" >> "$cov_missing"
done < "$inv_keys"
while IFS= read -r k; do
  [ -z "$k" ] && continue
  grep -Fqx "$k" "$inv_keys" || printf '%s\n' "$k" >> "$cov_unknown"
done < "$res_keys"

n_expected="$(while IFS= read -r k; do [ -z "$k" ] && continue; in_run_scope "${k%%$'\t'*}" && printf 'x\n'; done < "$inv_keys" | count_stdin)"
n_missing="$(count_lines "$cov_missing")"
n_unknown="$(count_lines "$cov_unknown")"
n_failed="$(awk -F'\t' '$3=="failed" || $3=="skipped"' "$RESULTS" | count_stdin)"
n_targetred="$(awk -F'\t' '$3=="target-red"' "$RESULTS" | count_stdin)"
n_passed="$(awk -F'\t' '$3=="passed" || $3=="target-met"' "$RESULTS" | count_stdin)"

echo "  committed expectations in executed suites: $n_expected"
echo "  executed and green: $n_passed    @target red (disclosed): $n_targetred    red: $n_failed"
if [ "$n_missing" -gt 0 ]; then
  echo "FAILED: MISSING COVERAGE -- $n_missing committed expectation(s) did not execute:" >&2
  sed 's/^/    /' "$cov_missing" | head -20 >&2
  echo "  These scenarios are in HEAD. Something removed them from the run without" >&2
  echo "  removing them from the repository: a marker file, a registry row, a" >&2
  echo "  directory, or a file extension. None of those is a bump class." >&2
  fail=1
fi
if [ "$n_unknown" -gt 0 ]; then
  echo "FAILED: $n_unknown scenario(s) executed that are not in HEAD:" >&2
  sed 's/^/    /' "$cov_unknown" | head -20 >&2
  echo "  The gate grades committed content; an uncommitted expectation is not one." >&2
  fail=1
fi
if [ "$n_failed" -gt 0 ]; then
  echo "FAILED: $n_failed executed expectation(s) are red or never ran." >&2
  fail=1
fi
if [ "$n_missing" -eq 0 ] && [ "$n_unknown" -eq 0 ] && [ "$n_expected" -gt 0 ]; then
  echo "  coverage reconciles: $n_expected/$n_expected committed expectations executed"
fi

# ---------------------------------------------------------------------
# LEG 5 -- VERSION NEGOTIATION.
# ---------------------------------------------------------------------
# Base resolution (reviews M-1, M-R2-2, L-R2-1):
#   * an EXPLICIT --base that does not resolve is a hard failure. It used to
#     be skipped silently, so `--base 14323d` (one character short) quietly
#     classified against a different, weaker base;
#   * the `HEAD~1` last resort is GONE. It made leg 5 classify a single
#     commit on any clone where the branch refs do not resolve -- which is
#     this worktree's own situation for four of the six candidates -- and
#     "no base could be resolved" is a fact worth stopping for.
step "leg 5/8: semver"
if [ "$BASE_EXPLICIT" -eq 1 ] && ! git rev-parse --verify --quiet "$BASE" >/dev/null 2>&1; then
  echo "FAILED: --base '$BASE' does not resolve to a commit." >&2
  echo "  An explicitly requested base that is silently ignored classifies against" >&2
  echo "  something the caller did not ask for. The fallback chain is for the" >&2
  echo "  ABSENCE of an argument, not for a typo in one." >&2
  fail=1
fi
resolve_base() {
  local b out
  for b in "$BASE" "@{upstream}" origin/main origin/master main master; do
    [ -z "$b" ] && continue
    git rev-parse --verify --quiet "$b" >/dev/null 2>&1 || continue
    case "$b" in
      main|master|origin/main|origin/master)
        out="$(git merge-base HEAD "$b" 2>/dev/null)" && [ -n "$out" ] && { printf '%s\n' "$out"; return 0; };;
      *)
        out="$(git rev-parse "$b" 2>/dev/null)" && [ -n "$out" ] && { printf '%s\n' "$out"; return 0; };;
    esac
  done
  return 1
}
RESOLVED="$(resolve_base || true)"
if [ -z "${RESOLVED:-}" ]; then
  echo "FAILED: no base commit could be resolved for the semver gate." >&2
  echo "  Pass one: scripts/contract-gate.sh --base <ref>. The previous rounds fell" >&2
  echo "  back to HEAD~1 here, which classified one commit and printed the shape of" >&2
  echo "  a full classification (review M-R2-2)." >&2
  fail=1
else
  echo "semver base: $RESOLVED"
  bash scripts/contract-semver-gate.sh --runner "$RUNNER" "$RESOLVED"; check $? "contract semver gate"
fi

printf '\n'
if [ "$fail" -ne 0 ]; then
  echo "CONTRACT GATE: FAILED" >&2
  exit 1
fi
if [ "$FAST" -eq 1 ]; then
  echo "CONTRACT GATE (--fast): legs 0, 1, 2, 4, 5, 6, 7, 8 passed; leg 3 NOT RUN."
  echo "  Run without --fast before pushing." >&2
  exit 3
fi
echo "CONTRACT GATE: PASSED"
