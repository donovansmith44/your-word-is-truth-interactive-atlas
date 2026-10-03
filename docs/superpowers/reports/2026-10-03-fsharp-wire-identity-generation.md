# F# named-wire generation checkpoint

Author work in progress on `lane/codex/CX-FSHARP-wire`. The approved producer
baseline is **da7e00d**, the controller's squashed WIREID landing. Existing F#
sources were carried unchanged from **88f0d26** in **a3bb8c3**; generator changes
follow that carry-forward. The old unsquashed lane could not merge cleanly with
squashed main, so that isolated merge was aborted. No producer, C# application,
contract, artifact or other agent worktree was edited or resolved.

The consumer integration now builds the Debug WebAssembly client with **zero
warnings/errors** and passes **157 normal tests**, zero failures/skips. The
portable runners pass **7 generator, 6 identity and 8 presentation properties**;
these overlap the normal suite and are not extra independent test counts. The
original 20-error Core failure below is retained historical red evidence. This
remains author work, not whole-domain sign-off, full parity or C# retirement.

## Consumer integration continuation

The failed abstraction was the generated nominal wire identity: the older F#
consumers accepted interchangeable primitives. The producer owns those identity
representations; this change migrates the F# consumers to its approved published
names. Positions widens node/edge IDs to ElementId through generated doors.
Explorable, ArtifactMoved and element-page reads retain ArtifactRoot, element
reads retain ElementPageCursor, frontier signatures retain ArtifactRoot and
EdgePageCursor, and reading requests widen served ContentsReference or
UnitReference to TextWindowReference. Corpus goes directly to the generated
contents request. Browser Concord query input enters through the existing JSON
library boundary into ConcordReference before widening; there is no reference
parser or domain reconstruction. Views project identities only for visible text,
HTML identities/selectors and URLs; request arguments remain typed.

A new enumerating property failed before identity display was generated: F#'s
default union display exposed representation syntax instead of the exact wire
primitive. All 21 emitted identities now override the standard ToString via
BCL Convert with invariant culture. Constructors stay private, and no primitive
getter or narrowing door was added. Existing external compiler checks still
refuse all 21 constructors and three wrong conversions in both generated samples.
The property checks all published identity kinds with varied numeric/Unicode
samples. String/int identities are the published representations; this is not a
reference pattern validator or a custom codec.

The served provenance change also exposed a visible behavior difference. Eight
presentation properties were written before changing the presenters; **six failed,
two passed** with source IDs. Both node and optional-edge paths now display the
served **Title**. The laws cover cards, entire UnitText presentations, optional
edge provenance, place claims and served reign/map/era labels. All eight tests in
that touched file are now properties, replacing its example attributes. Entire
Result presentations are asserted. Both remaining property controls stayed green.

The first normal test compile exposed **98 distinct diagnostics** in legacy
fixtures. The type-directed migration updates only test values, expected results
and request arguments; generated constructors were not opened. WireFixtures uses
the existing System.Text.Json library to admit test primitives, never an app
factory. ContentsReference expectations use the actual generated widening. Old
example tests outside PresentationTests remain; **157 passing tests is not a
property-only claim**. Compact compile reds and the 92 initial fixture adaptations
are retained, followed by the final normal green log. Shared static assets are
read-only sparse inputs from the approved C# client.

The category of interchangeable request arguments is fenced by generated nominal
types and private constructors at the migrated consumer signatures. This does
not certify unfinished reference-shape admission, structured error vocabulary,
wire enum classification, unsupported-shape policy or every generated type's live
screen reachability. The legacy generator's mutable emitter/string errors remain
explicit pending style work. Whole-file newspaper order, the real unused gate,
61 skeleton pending bodies and complete side-by-side parity remain open; the
old 294 unused findings are pinned to parent 88f0d26, not measured anew here.
No pending behavior or new feature was filled before domain sign-off. No producer,
C# application, contract, data or other agent worktree changed.

Reproduce from the worktree root after sourcing ~/.bible-atlas-env:

```sh
dotnet test docs/superpowers/reports/evidence/2026-10-03-fsharp-wire-identities/consumers/ConsumerLaws.fsproj
dotnet test client-fsharp.Tests/BibleAtlas.FSharp.Tests.fsproj
dotnet build client-fsharp/BibleAtlas.FSharp.Client.fsproj
```

The current build and native checks are bounded; no new Cargo/AOT, browser,
mutation or synthetic-size performance gate ran. No Codex lock/server remains.
The earlier generator-only checkpoint and its limits follow for evidence.


## Existing libraries and the signed rule

Signed WIREID design section 4.3 requires identity-only `oneOf` representations,
member/subset widenings and typed cursor defaults. Reuse the installed build
parser **YamlDotNet 18.1.0 (MIT)**, **FSharp.SystemTextJson 1.4.36 (MIT)** for codecs,
and **FSharp.Compiler.Service 43.12.401 (MIT)** for external accessibility checks.
Existing dependency/licence evidence is in `client-fsharp/THIRD-PARTY.md` and the
prior analyzer survey on the author lane. No parser or JSON codec was added.

NSwag 14.7.1 / NJsonSchema, already used by the C# generator, provide the independent
published schema model and widening inventory to the test project. NSwag emits
C#, not the required F# private unions/modules; its existing producer-owned model
is reused for comparison. The F# emitter must express this project's signed
private representation/widening/default policy. These are test/build dependencies,
not a C# application reference or WASM runtime dependency. No package was installed.

The generator now identifies compatible primitive representations through named
union members, emits a distinct private union/converter for each, and generates
only member or subset widenings. Named string enums widen by an exhaustive match
of their published cases. Each integer cursor default is emitted as its own typed
`first`; int/int64 default forms are generated properties. Existing scalar and
record generation is retained. Unsupported shapes still produce the existing
Result refusal rather than an untyped fallback. The legacy generator's string
error vocabulary, mutable emitter and whole source order still need the pending
style migration; this checkpoint does not certify that wider category.

## Red, green and independent compilation

Before the emitter change, the native generator runner recorded **3 failing / 4
passing properties**: the current published contract's unions were unsupported,
union representation/widening was absent, and the typed default was absent.
After implementation it passes all **7 generator properties**. Existing scalar,
record optionality, enum and unsupported-shape controls remain green.

The complete current published generated file builds independently with **zero
warnings/errors**. Five additional properties run over that actual output:

- complete private representation and widening inventory;
- every widening preserves the entire JSON wire value, including every BookId
  enum case, with generated scalar samples;
- every identity round trips its entire primitive and refuses the wrong kind;
- both cursor defaults serialize to their exact schema defaults;
- external F# consumers exercise 21 private constructors, three incompatible
  identity conversions and the accepted doors, twice: **50 compiler checks**.
  Private constructors produce FS1093, wrong types FS0001, accepted doors no error.

The F# output contains **21 identities and 26 widenings**. The C# generator emits
20 identities because its existing Unread policy excludes published
ReadingReference. The test explicitly retains that extra published type and its
four declared/subset widenings rather than silently filtering the F# inventory.
Its eventual live-model reachability remains part of the owner's zero-unused
obligation. Reflection comparisons do not substitute for the external compiler
checks. Ordinary values such as Year.Value are not treated as identities.

All newly authored tests are property based. A diagnostic run exposed two test
adapter mistakes: NSwag stores defaults as text, and its C# identity inventory
applies the ReadingReference exclusion described above. Correcting those expected
representations was not a production behavioral repair; those diagnostic/compile
runs are not counted as the initial red-before-green evidence.

## Limits, reproduction and exact next step

[Retained evidence](evidence/2026-10-03-fsharp-wire-identities) includes the original
red, both portable green logs, generated-only build, the complete normal Core
failure, its 20-site inventory and source/input hashes. Trailing log whitespace
is normalized; original local logs are retained. Generated code remains untracked.
From the worktree root after sourcing `~/.bible-atlas-env`, run:

```sh
dotnet test docs/superpowers/reports/evidence/2026-10-03-fsharp-wire-identities/generator/GeneratorLaws.fsproj
dotnet test docs/superpowers/reports/evidence/2026-10-03-fsharp-wire-identities/identities/IdentityLaws.fsproj
```

The second project regenerates its private obj/Wire.g.fs from the committed
contract before compiling; it has no Core/client dependency. Both recipes are
bounded native checks. These green outcomes do not cover normal client tests,
WASM, routes, UI, browser parity or a fresh mutation/timing gate. Reference pattern
validation is not claimed: codecs here enforce primitive kind, not Scripture or
identity-shape inference.

Next migrate the existing consumers, using generated types and widenings:
Positions/Explorable roots and element identities, Failure/GraphRead roots and
ElementPageCursor, served provenance titles, reading-window and contents references.
Do not stringify identities to silence type errors. Resume normal compiler/tests
once those sites are coherent; then complete structured failures/frontier and the
mandatory declaration/order work before the owner's whole-skeleton sign-off.
The parent checkpoint's 61 pending bodies and 294 unused findings remain open.
No new feature/view or pending domain body was filled. No Codex lock/server remains.

## Stopped-output cleanup

After preserving the pushed sources, complete compact gate logs and reproduction
projects, checked all 34 named own bin/obj directories for tracked files, symlinks
and live cwd/exe/fd/mapped-file references. All checks were empty. Removed
**327,196,018 logical file bytes** of this session's disposable source-order,
WIREID review and wire-generation output only. Exact paths/checks are retained in
`cleanup.json`. Source worktrees, probe inputs, reports, logs, raw/cache and Claude's
outputs remain. WSL now reports about 774 GB free; the Windows VHD host still has
only about 3.3 GB. This guest cleanup does not compact the VHD or establish host
headroom. The queue's CX-I3 retention reminder remains in effect; no periodic
cleanup automation was installed.
