# Exploration vocabulary and declaration cleanup

Continuation from `ffb1757`, following the owner's newer instructions on Explore,
Explorable, separate Focus/Frontier, resume only, duplicate declarations and zero
unused code. This is a compiling author checkpoint, not a completed correction or a request for whole-domain sign-off.

## Tool survey and choice

- [FSharpPlus](https://github.com/fsprojects/FSharpPlus/tree/v1.9.1), Apache-2.0,
  already pinned at 1.9.1: maintained library with ReaderT, StateT and ResultT.
  Selected for the reader/state/async-result composition. The existing small
  domain CE adapter delegates return/bind to those library operations and defers
  evaluation; it does not implement another transformer library. Public signatures
  retain the owner's `Explore` name. No replay operation.
- [FsToolkit.ErrorHandling](https://github.com/demystifyfp/FsToolkit.ErrorHandling),
  MIT, maintained async-result builders: good for result-based async effects,
  but does not supply the complete reader/state transformer composition needed
  here. No second dependency added beside the already installed FSharpPlus.
- [F# compiler service](https://fsharp.github.io/fsharp-compiler-docs/fcs/project.html),
  MIT, Microsoft's maintained compiler tooling, already pinned at 43.12.401:
  selected for resolved declaration/use analysis and the duplicate-name syntax
  law. MSBuild supplies actual compiler flags and references; FCS resolves symbols.
  Python's standard subprocess/JSON/tempfile APIs orchestrate the compiler only.
- [FSharpLint](https://fsprojects.github.io/FSharpLint/how-tos/rule-configuration.html),
  MIT, published community linter: its documented rules do not provide whole-client
  unused public declaration checking. FS1182 addresses local unused values rather
  than public APIs; neither by itself meets the owner's gate.
- [Ionide analyzers](https://github.com/ionide/ionide-analyzers), MIT, maintained
  analyzer framework: useful typed-code rules, but its published analyzer set does
  not supply this application's cross-project/no-test-call policy. The compiler's
  language-service unused analysis deliberately excludes public APIs. We therefore
  compose FCS's resolved symbol definitions/uses with the repository's scope rule,
  without writing an F# parser, resolver or compiler.

Generated wire sources are build outputs, checked by the existing generated-type
laws; this declaration gate inspects authored client/core/generator source and
excludes all tests as callers. Compiler failures fail analysis rather than
silently producing an empty unused set. This is an unreferenced-symbol gate, not
proof that every dynamic execution path is reachable. The single pending marker exempts its body and unused arguments, not an entire file. A compiled adversarial fixture pins public/internal/private functions, an unused type/field/case, a caller present only in tests, a phantom function type parameter, a live caller, an entry point and the one pending exemption. Transitive dead-call cycles and complete phantom-type-parameter analysis are not yet proved; zero-dead-code closure remains open.

## Scope and remaining obligations

The current pinned contract does not yet contain NodeId, EdgeId, ArtifactRoot or
the split cursor/reference identities from unapproved WIREID. This change must not
invent them or consume its changes-requested producer branch. Existing generated
PositionRef/Element/NodeRecord/EdgeRecord are available now. Missing future Year
and FOCUS-3 producer fields remain dependencies, not permission to copy contracts.

The previous skeleton duplicated the live exploration implementation. There is
now one concrete Explorable over served graph records, one Trail and one Explore
in their actual files. Focus wraps only the current Explorable; Frontier owns
neighbour windows. Graph reading and JSON are admission adapters. Unicode run
composition moved with text-span admission. Merely moving an old implementation
does not certify its invariants, typed failure completeness or style: those
remaining gaps must be named, tested and completed before sign-off.

## Changes and evidence

| Check | Result |
|---|---|
| [Duplicate declaration red](evidence/2026-10-03-fsharp-vocabulary/duplicates-red.log) | New FCS syntax property fails on the two Resolved and Trail declaration homes before migration. Its generated self-check permits a type/module companion in one file and refuses the same concept in two files. |
| [Deferred CE red](evidence/2026-10-03-fsharp-vocabulary/explore-red.log) | Four algebra/root properties pass; constructing the old CE eagerly evaluates its body, so the fifth property fails. |
| [Trail bound red](evidence/2026-10-03-fsharp-vocabulary/trail-red.log) | The five CE/root properties pass after the CE change; generated walks beyond the named retention budget fail before bounded retention. |
| [Regression green](evidence/2026-10-03-fsharp-vocabulary/regression-green.log) | 150 tests pass, no skips. Includes six generated exploration laws and two declaration properties; older example tests remain to convert. |
| [Debug WASM](evidence/2026-10-03-fsharp-vocabulary/wasm-green.log) | Build succeeds with zero warnings/errors. |
| [Skeleton inventory](evidence/2026-10-03-fsharp-vocabulary/skeleton-green.json) | 61 unique pending calls, zero inventory failures. Reduction from 69 includes deletion of the parallel uncalled trail/resolution model, not implementation of all those operations. Explore.links is now explicitly pending. |
| [Declaration checker fixture](evidence/2026-10-03-fsharp-vocabulary/declaration-selftest-green.json) | All eight expected offenders detected, the live entry/caller and one pending body excluded correctly. |
| [Mandatory unused gate](evidence/2026-10-03-fsharp-vocabulary/unused-red.json) | Intentionally RED: exit 1, zero compiler errors, 61 pending bodies, 294 unreferenced declarations/parameters. This is the remaining worklist, not a waiver or a zero-dead-code claim. |

Both runtime opening and saved restoration now use Resume, with no separate
Begin wrapper or replay. The library composes reader/state/result bind; the thin
CE adapter adds deferred execution. Its first generic Return invocation compiled
but failed in Debug (`Dynamic invocation of Return is not supported`); the
concrete nested constructor now supplies Return while library ReaderT.bind
continues to compose the transitions, and the whole generated suite exercises it.
No broad generic builder was added. The actual presenter now accepts Focus,
which contains only the current Explorable; Frontier is a different private type.

One retention path is used for Follow and Resume. The current named client
policy is 40 retained steps plus their source, including the dual steps recorded
by Back. Generated walks test the complete retained sequence. No claim of a
configurable preference or a new saved-format feature is made.

The duplicate pre-domain exploration/graph files and root JSON/text-run files
are deleted. Their live callers use the actual domain/exploration/admission files.
Generated TextRef, BibleRef and ConcordRef replace the placeholder reference
parameters, and pinned provenance uses its actual current wire string shape.
Unknown future contract identities/roles/levels/marks remain explicit dependencies.

The declaration tool initially expanded foreign compiler metadata while walking
owners of referenced types, reaching excessive memory. That check was stopped by
its own PID; no other process was stopped. Owner traversal is now restricted to
authored source locations, generated record methods do not count as live callers,
and each compiler-analysis subprocess has a 4 GiB managed heap ceiling and a
120-second timeout. The bounded check and its synthetic fixture finish normally.
No fresh Cargo, AOT, full browser or mutation gate was run.

## Review status and exact next step

14b: one live Explorable/Trail/Explore, one shared retention path, no replay,
no duplicate reference-forwarding functions, generated references reused.
24a/24b: the duplicate *name* category is enumerated by FCS in the normal suite;
this does not assert that every remaining semantic duplicate or dead category is
closed. The unused gate is still red. In particular, the old string Failure was
moved out of Loading but has not yet been replaced by the structured read/wire
failure model. Text-run conversion still has the old clamping behavior. Frontier
admission and Explore.links remain pending, and the trail still lacks admitted
generated edge evidence. These prevent sign-off.

Next: use the compiled unused inventory to remove unneeded APIs and consolidate
the live failure/admission door; preserve the owner's required model signatures
behind the one marker while finishing real Focus/Frontier/links integration.
Complete transitive reachability and generic-parameter gate coverage, then
request Claude/owner review of the whole compiling domain. Do not expand views or
claim 100% client parity. The existing node/reference producer work cannot be
substituted with copied contracts.

Claude review sweep: all stale `review` entries were compared with their latest
recorded verdicts and fetched heads. F1/F6/EDGES, F2/FSTYLE, NOBLURB, LICBOC, F39,
PROVENANCE and RULE1 already have approvals/landings. WIREID remains changes
requested at 3fee0dc under F-WI-6; the fetched head has no submitted correction.
No previous approval was inferred for a new head and no already-approved gate
was rerun merely because an old Status line remains in the queue.
