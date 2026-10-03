# F# skeleton: account runs and producer identity ownership

Author checkpoint on `lane/codex/CX-FSHARP-accounts`, based on **3bceb6c**.
Prepared in a separate sparse worktree under the existing CX-FSHARP claim;
the live headless Codex worktree and Claude's sources were left untouched.
This corrects the skeleton for review. It does not implement its pending
operations or claim client parity, owner sign-off or independent approval.

## What changed

Signed Year design **9bf4f64**, §2.8/§3.4, measures 2,082 account runs:
1,897 passages and **185 single verses**. The previous skeleton represented
every historical account as a passage, while correctly refusing one-unit
passages. Those two decisions could not represent the signed design together.

[History.fs](../../../client-fsharp/Core/Domain/History.fs) now gives an
`AccountRun` two alternatives: the existing `TextUnit` for a single verse,
or the existing multi-unit `Passage`. `HistoricalAccount` remains private
behind one pending `admitAccount` signature for both alternatives; its total
`accountRun` projection replaces the passage-only projection. The additional
text-part parameter propagates through Event, EventContext and the event arm
of [Position.fs](../../../client-fsharp/Core/Domain/Position.fs). There is no
new account node, no one-unit passage exception and no separate admission
path for the new case.

The pre-review §0 reserves element identities and reference unions for generated
contract code. The copied `Domain.ElementId` and unused `Domain.UnitReference`
declarations are removed from Graph.fs and Text.fs. Position carries a producer
element-identity parameter instead of reconstructing its node/edge cases. The
missing generated types remain explicit dependencies, as elsewhere in this
skeleton; no alternative wire schema or identity parser replaces them.

## Verification

Compact sources, outputs and source hashes are in
[the evidence directory](evidence/2026-10-03-fsharp-account-skeleton/checkpoint.json).

| Check | Observed result |
|---|---|
| Baseline Core build at 3bceb6c | 0 warnings, 0 errors |
| Same account API consumer before/after | Before: FS0039, AccountRun absent. After: compiles both alternatives, shared admission signature, total projection and EventContext/ResolvedValue propagation. Pending operations are never invoked. |
| Producer-position consumer | Compiles an element identity parameter and a served YearSpan position. |
| Two shadow consumers, separately compiled | Before removal: each compiles. After removal: each fails with FS0039 for its removed domain declaration. |
| Private account constructor consumer | Fails with FS1093 for HistoricalAccount's private union representation; unrelated private inputs are not constructed in this program. |
| Final Core / Debug WASM builds | Each 0 warnings, 0 errors |
| Existing normal regression suite | 142 pass, 0 fail, 0 skip, including existing domain properties and the source-order gate |
| Skeleton inventory | 69 uniquely named pending operations, 0 inventory failures; unchanged count |
| Added application-comment scan / diff whitespace | Clear |

The compiler consumers are shape/accessibility checks, not new behavioral laws.
The owner's ops **e65c377** directive still holds: behavioral laws and pending
bodies follow skeleton sign-off. No example tests or new runtime implementation
were added. The baseline account consumer's source is unchanged between its red
and final green; producer-position checks are a separate program.

For reproduction, source `~/.bible-atlas-env`, build Core, then use `dotnet fsi
--exec` on the evidence `.fsx` files. The two normal consumers must succeed;
the three `forbidden-*` consumers must produce their listed compile errors.
Run `python3 scripts/fsharp-client/domain-skeleton.py`, then the Debug client
build and `dotnet test client-fsharp.Tests/BibleAtlas.FSharp.Tests.fsproj`.
The committed probes use relative references to the rebuilt Core DLL.

## D.R.Y., category and order pass

The failed abstraction was the passage-only historical account. Both run shapes
now feed one private historical-account abstraction and one total projection;
all Event/EventContext/ResolvedValue signatures were migrated. Candidate runs
reuse the existing text unit and passage data, retaining their served identity.
The compiler consumer demonstrates both alternatives through the public API.

The identity/reference drift category is addressed by deleting both handwritten
wire-shape declarations, parameterizing the only ElementId consumer and adding
separate compiler evidence that those old domain doors are absent. Generated
types remain owned by their producer. No registry, parsing or validation
machinery or new package was introduced.

Changed public operations remain above their projections and add no helper
above a caller or fake recursive module. The existing source-order gate passes;
that gate does not establish the goal's still-owed whole-file/public-member/local
binding order coverage across every F# source.

## Remaining limits and handoff

The 69 bodies are still unimplemented. The account history predicate must
ultimately consume producer narration/mark evidence for both cases, never scan
text or invent an account. In this generic skeleton, naming a TextUnit case
`Verse` does not prove Bible corpus, canonical identity or narration evidence.
Concrete producer binding and admission properties are still required before
runtime use. No claim that arbitrary type-parameter instantiations enforce the
full domain is made.

This worktree's generated contract is pinned to the base's published files.
Claude's later provenance/WIREID/Year/FOCUS-3 changes are not silently copied in;
their reviewed generated types must be consumed and rebuilt in their normal
integration step. The old application, older example tests, decoder/effect
refactors, full source-order closure and actual browser/C#/F# parity remain open.

Claude can review this compiling correction and consume it into the skeleton
lane before presenting the actual files to Donovan. Do not sign off the known
account mismatch at predecessor 3bceb6c. No Rust target, AOT, shared contract,
artifact, server or lock was created. Compact evidence is retained so stopped
task-owned bin/obj output can be pruned without losing these results.

After publishing code **eb77e95**, removed this worktree's eight untracked
bin/obj directories: **229,214,122 logical file bytes**. Verified the pushed head,
clean tracked state, zero candidate symlinks and no process cwd/open descriptor
inside a candidate first. [Cleanup inventory](evidence/2026-10-03-fsharp-account-skeleton/cleanup.json)
records the exact paths and before/after capacity. All compiler sources and
small outcomes remain committed; reproduction now rebuilds Core. No other
agent's output or raw/cache directory was removed. WSL still reports about
742 GB free, while Windows C: has only 3.5 GB; no VHD compaction occurred.
