# F# named-wire generation checkpoint

Author work in progress on `lane/codex/CX-FSHARP-wire`. The approved producer
baseline is **da7e00d**, the controller's squashed WIREID landing. Existing F#
sources were carried unchanged from **88f0d26** in **a3bb8c3**; generator changes
follow that carry-forward. The old unsquashed lane could not merge cleanly with
squashed main, so that isolated merge was aborted. No producer, C# application,
contract, artifact or other agent worktree was edited or resolved.

The generated definitions compile and **12 scoped properties pass**, but the
whole app does not yet compile against these types. Its Core build reports
**20 FS0001 errors** at existing consumer signatures. This is not a full-domain
sign-off, review-ready application or parity claim.

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
