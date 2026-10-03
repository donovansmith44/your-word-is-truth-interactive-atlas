# Newspaper-order gate coverage

Author work in progress, separate `lane/codex/CX-FSHARP-order`, base **ffb1757**.
The whole-client order and parity goals remain unfinished. No pending domain
body or feature implementation is authorized by this structural checkpoint.

## Library choice before implementation

Use the already installed **FSharp.Compiler.Service 43.12.401**, MIT, for parsing
and full syntax traversal. `ParsedInput.fold` visits compiler nodes and their ancestor paths; they replace the existing reflection-based AST
walker. This is a repository-specific order rule over the compiler’s syntax,
not an additional parser or resolver. No package is added to the WASM client.

The [official AST guide](https://fsharp.github.io/fsharp-compiler-docs/fcs/untypedtree-apis.html)
documents these APIs; installed XML metadata was checked for the exact version.
The [unused-symbol survey](2026-10-03-fsharp-unused-survey.md) records the MIT
licence and maintained compiler/package evidence and examines FSharpLint and
Ionide tools. Neither inspected catalogue supplied this exact caller-before-helper
policy. Use the existing compiler traversal rather than extend the handwritten
reflection walker. Syntax-only name matching must not be presented as complete
compiler-resolved dependency or execution-order proof.

## Red evidence before changing the gate

Six new properties generate names and public/internal visibility. A native
runner links the actual SourceOrderTests.fs without building Core or WASM.
The original helper produces **5 failing / 3 passing / 0 skipped** properties:
public/internal helpers, qualified module calls, type members and local helper
chains are missed; parameter shadowing produces a false positive. Existing
private-helper and record-fixture controls plus the new nested-module control pass.

An independent reflection consumer of the original helper also returns no
findings for actual Rooted.fs at both the base and pushed parent 88f0d26. That file
places public `admit` above public `map2`, which calls it. Both snapshots share a
source hash. This is a real omitted ordering relationship, not an assertion that
the full application was analyzed or that its behavior is wrong.

## Scoped implementation and evidence

The gate now uses compiler ancestor paths to identify binding scopes, function
heads, module qualification and member receivers. It inspects public and internal
helpers, member helpers, local chains and the existing shared test fixtures.
A private record names each binding's range, scope, owner and helper classification.
Function parameters shadow outer helper names in the generated control.

The first expanded inventory incorrectly classified ordinary local/module data as
helpers. Two additional generated properties first failed (**2 fail / 0 pass**),
then passed after distinguishing function bindings, type members and shared test
fixtures from ordinary data. This distinction keeps input bindings above the
computation that consumes them without calling them helper functions.

The committed native runner passes **11 properties / 0 fail / 0 skip**: ten gate
properties and one complete real-Rooted snapshot comparison. The reproduction
links the actual test source. It does not build Core or WASM and does not run the
whole-client zero-findings property. All new tests are property based; no
application behavior or domain placeholder was filled.

Evidence is in [the retained directory](evidence/2026-10-03-fsharp-order-coverage):
`red.log`, `data-red.log`, `portable.log`, `actual-snapshots.json`,
`expanded-inventory.json` and `checkpoint.json`, plus the portable runner sources. Log trailing whitespace is normalized; the
original local logs are retained outside git.
The inventory retains every enumerated source hash and its entire diagnostic list.
At this checkpoint it records **62 authored .fs files / 22 syntax diagnostics / 9
files with diagnostics** in the separate base worktree, and **64 / 26 / 12** in the
read-only parent at 88f0d26. These are triage candidates, not 26 independently
proved application defects. They include Rooted, Position, Explore, Trail, loading,
text-span admission, the generator and existing test fixtures. Generated .fs files
are excluded from this external inventory.

From the worktree root, after sourcing `~/.bible-atlas-env`:

```sh
ATLAS_ORDER_PARENT_SOURCE=/home/donovan/w/CX-FSHARP \
ATLAS_ORDER_REPORT=/tmp/atlas-order-inventory.json \
dotnet test docs/superpowers/reports/evidence/2026-10-03-fsharp-order-coverage/OrderCoverage.fsproj \
  --filter 'FullyQualifiedName~the order gate|FullyQualifiedName~the expanded gate records'
```

Omit `ATLAS_ORDER_PARENT_SOURCE` to check only this worktree. The additional root
is a source snapshot, so reproducing these exact counts requires its recorded
commit. Preserve compact outcomes before removing the runner's disposable bin/obj.

## Remaining obligations and handoff

The wider category remains open. This checker is syntax based: arbitrary local,
lambda and pattern shadowing, aliases, overloads and full qualified symbol identity
still require coverage and compiler-resolved analysis. Same-scope name matching
cannot certify all dependencies, mutual recursion or execution order. The
whole-client zero-findings property has not passed; its generated-file path also
requires the normal contract build before execution. No full gate, browser parity,
independent review or owner domain sign-off is claimed.

The next step is to cover binding/qualification ambiguities with generated laws
and use compiler symbol identities where syntax is insufficient; then triage and
migrate actual files, preserving evaluation order and public contracts. Do not add
spurious recursive declarations just to silence the gate. The parent checkpoint's
294 unused findings, 61 pending bodies, structured failure/frontier integration and
100% F# parity remain separate unfinished obligations. This branch is an author
checkpoint under CX-FSHARP, not a landing recommendation.
