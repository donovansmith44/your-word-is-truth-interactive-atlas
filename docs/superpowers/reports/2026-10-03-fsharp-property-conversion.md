# F# property-only test checkpoint

The current F# client test assembly contains **164 property tests**, all passing.
The remaining ContractTests, ExplorationTests and ViewTests examples have been
converted or consolidated into generated laws. A compiled-assembly gate rejects
an example test anywhere in that assembly. This advances the owner's
property-only directive; it does not establish whole-client parity, domain
sign-off, zero unused declarations or complete semantic newspaper order.

Base: `456a508` on `lane/codex/CX-FSHARP-wire`, approved producer base `da7e00d`.
Only the claimed F# test files/project and this report/evidence changed. No
application, generated contract, C#, server, data or Claude worktree edit.

## Changed laws

Contract tests now vary both closed text-reference cases, book vocabulary,
reference fields, identities, unknown discriminators, null whitespace envelopes,
artifact roots and Concord text. The committed text fixture and **both** contents
fixtures are compared as whole JSON documents under generated artifact roots.
The null-heading Concord test asserts its complete generated TextWindow.
The two fixed required-string examples were removed because the existing
ContractShapeTests property enumerates every required field of every generated
record, refusing omission and null. That broader property still passes.

Exploration tests now vary edge kinds, identities, artifact roots, arbitrary saved
journey kinds/lengths and failure payloads. They assert complete results/trails
and exact read lists. Resume has an independent retained-path expectation in
addition to equivalence with the step algebra. Failure tests assert that the
continuation did not run and that only the requested graph read occurred.
Zero-step Back and failed whole-journey Resume gained generated laws. The three
fixed monad examples were removed: ExploreLaws already supplies generated left
identity, right identity and associativity over successful/failed walks. Its
six existing properties still pass. The unused test-only `run` helper was deleted.

All 22 DOM/entry tests now run with generated inputs. Served node/anchor/event ids,
roots, source titles/descriptions/links, failure payloads, continuation handles
and text suffixes vary. Whole markup expectations remain explicit. Keyboard
activation varies Enter/Space and unrelated letters; the shared header is checked
across all six current route cases. The real WebAssembly entry component reads
and renders a generated Sources response, then clears it after navigation.
These checks cover the existing UI, without adding a view or a pending feature.

## Property-only closure

`PropertyTests.fs` uses the installed CLR reflection API and the actual Xunit
FactAttribute/FsCheck PropertyAttribute types. It enumerates declared methods in
the compiled F# test assembly, requires a nonempty property inventory, and compares
the entire example-method list plus discovered-test/property counts. Attribute
spelling, qualification or an F# alias cannot bypass a type comparison; a derived
Fact/Theory attribute is included through assignability. No source-name parser,
new library or permissive whitelist was introduced.

Library fit: Xunit 2.9.3 and FsCheck.Xunit 3.4.0 are already installed/attributed;
CLR reflection is the framework API for the exact metadata this gate checks.
FCS remains the existing source-order tool. No new parser, generator, serializer
or general-purpose machinery was written or new dependency adopted.

The gate was run with a temporary Fact added to the real test assembly. It failed
with **164 properties versus 165 tests**. The temporary source is retained as
`property-only-counterexample.txt`; it is absent from the final test project.
The final gate passes with **164 properties versus 164 tests**, zero examples.
This is a genuine category red/green for the new gate. The law conversion changes
no application code and does not claim a production-behavior red/green fix.
Initial annotation/parenthesization compile errors during conversion are not
counted as behavior reds.

## Verification

| Executed gate | Result |
| --- | --- |
| ExplorationTests + ExploreLaws + SourceOrderTests | 30 pass / 0 fail / 0 skip |
| ContractTests + ContractShapeTests + SourceOrderTests | 23 pass / 0 fail / 0 skip |
| ViewTests + SourceOrderTests | 35 pass / 0 fail / 0 skip |
| Temporary example through compiled property-only gate | 1 real failure |
| Final full normal F# suite | **164 pass / 0 fail / 0 skip**, 28.7 seconds |
| Source-order inventory within full suite | **67 authored .fs + 2 generated .fs**, green |
| Generated Wire.g.fs and Vocabulary.g.fs | byte-identical to `456a508` |
| Final authored test source scan | no Fact/Theory/example-data attributes |
| `git diff --check` | clean |

The suite count falls from 167 to 164: two redundant required-string rows were
removed, two contents rows became one property enumerating both fixtures, two
keyboard rows became one generated property, and the compiled category gate adds
one property. No count inflation or skip is claimed. Structural repository gates
use exhaustive one-pass properties; behavior laws use generated inputs.

Evidence is in `evidence/2026-10-03-fsharp-property-conversion/`: complete scoped
and full logs, the failing gate/counterexample, source hashes and checkpoint
metadata. Both generated-file hashes match the previous integration checkpoint.
The latest unused-symbol measurement remains **294 unused / 61 pending / zero
compiler errors** at `456a508`; it was not rerun for this test-only change.
Its known reachability limitations remain open. The source-order gate is still
syntactic: arbitrary match-pattern shadowing, aliases, overloads and whole
compiler-symbol/type ordering are not declared closed by a green inventory.

No AOT, new Rust target, browser differential, mutation or parity gate was run.
The normal suite built its referenced Debug/WASM projects successfully; no
separate new WASM publish is claimed. Claude holds the ordinary F3 heavy/contract
locks; they are not mutation locks and were left untouched.

## Handoff

Resume newly submitted Claude exact-head reviews first, then structured Failure,
frontier/explorer integration, unused reachability and remaining semantic order
corrections under CX-FSHARP. All 61 pending bodies and the whole-domain sign-off
restriction remain intact. Full 100% parity and C# retirement remain unfinished.
Frozen STYLE `35f6797`, parent `88f0d26`, isolated order `79671ab` and R2 `d17644f`
are unchanged. No Codex lock/server/live build remains.

CX-I3 retains the owner's request to clear obsolete, stopped, owned mutation
results and unnecessary build output after retaining compact outcomes. Current
WSL space is about **772 GB free**, but the Windows VHD-host C: volume has only
**3.1 GB free**. Check both before growth-heavy work. This continuation performs
no new bulk cleanup, retention automation, raw/cache/other-agent deletion or VHD
compaction; prior cleanup inventories remain retained.
