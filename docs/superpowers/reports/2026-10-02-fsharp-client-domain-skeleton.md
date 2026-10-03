# Whole-domain compiling skeleton

Owner ops `e65c377` explicitly requires all real modules, complete visible
representations and operation signatures, green compilation and one counted
placeholder marker before further laws or implementation. This checkpoint starts
at `884644e`; it completes the pre-review §0 file map, preserving the earlier
implemented NonEmpty/Positive/Rooted/PageCache/admission leaves. It does not add
views, effects, contract generation behavior, caches or behavioral test laws.
The [domain tour](../specs/2026-10-03-fsharp-domain.md) links every declaration.

There are 25 compiling files in the domain map, including the marker file and
admission leaves. All operations expose parameter/result signatures. The four
historical proposal `.fsi` files remain deleted. The marker is internal, owns
exactly one `NotImplementedException` construction/raise and receives the named
operation. It has **69 call sites**. The inventory gate lists each one, refuses a
missing/uncompiled/out-of-order module, a placeholder outside the inventory,
duplicate operation names and unmarked throwing/default bodies in this module
set. It reports a count rather than passing off pending behavior as completed.
The existing app never calls the marker; executing pending operations is not a
supported feature. This is the owner's authorized skeleton exception to the
ordinary implementation/no-dead-code gates, not a landing claim.

The complete client builds with the new types in the actual Core compile list.
The pending producers are explicit type parameters, not shadow declarations of
wire ids, cursors or Year/FOCUS-3 vocabulary. This permits an honest build against
the branch's pinned contract while making the missing inputs visible. It does
not establish concrete producer admission, reachable refusals, lawful bounds or
structural closure for every generic instantiation. Those must be reviewed when
the generated producer types bind and the laws/operations are filled. In
particular, projection/comparison predicates are trusted adapter inputs in the
skeleton; they are not yet a public production escape hatch or proof of closure.

Distinct Bible/Concord navigation instances, node-based passage/year identity,
one-span dated events versus undated events, short true concurrency, separate
stories, rooted edge-bearing neighbour entries, last-N trails and one focus-change
operation follow the owner's rulings. Calendar/festival/weekday/text-sequence
facts belong to the producer's time evidence. No client Year/passage mint,
calendar conversion, chronology inference or fictional server history is added.
The two cursor parameters refer to producer identities rather than new scalar
wrappers; reading/neighbours use the same future EdgePageCursor. ElementPageCursor
and WindowCursor remain the generated dependencies of their producer reads;
there is no new domain cursor type.

Tool choice for the inventory gate: Python's maintained standard-library
`xml.etree.ElementTree` reads the project file; `pathlib` enumerates files and
`re` searches the single literal marker, producing JSON with the standard `json`
library (PSF permissive license, existing Python toolchain). This is an inventory
scan, not a handwritten F# or XML parser. The compiler and the existing pinned
FSharp.Compiler.Service source-order property check F# syntax/order. Existing
library/admission/cache surveys stand; no new parser, decoder, validator framework
or combinator machinery/package is added. Scalar/span admission remains marked
pending, except the existing tested leaves and trivial immutable projections.

Evidence retained below (absolute machine paths inside compiler logs are expected):

| Gate | Observed result |
|---|---|
| [Inventory red](evidence/2026-10-03-fsharp-skeleton/skeleton-red.json) | Before new source: 17 missing modules. |
| [Inventory green](evidence/2026-10-03-fsharp-skeleton/skeleton-green.json) | All 25 files compiled in dependency order; 69 uniquely named pending operations; no failures. |
| [Debug WASM build](evidence/2026-10-03-fsharp-skeleton/wasm-build.log) | 0 warnings / 0 errors, the whole client builds. |
| [Existing regression suite](evidence/2026-10-03-fsharp-skeleton/regression.log) | 142 pass, 0 fail, 0 skip, including existing leaf/cache properties and source-order check. No pending operation is tested or claimed implemented. |

Reproduce from this lane worktree with `. ~/.bible-atlas-env`:

```sh
python3 scripts/fsharp-client/domain-skeleton.py
dotnet build client-fsharp/BibleAtlas.FSharp.Client.fsproj --no-restore
dotnet test client-fsharp.Tests/BibleAtlas.FSharp.Tests.fsproj --no-restore
```

14b: no second wire vocabulary or duplicate root/query cache identity; shared
container navigation and page/window refusal categories are declared once.
24a/24b: this is a design skeleton, not a bug fix/category-closure assertion.
Private representations show the intended doors, but pending checks and generic
producer parameters cannot substantiate enforcement yet. The prior costume
inventory, source-wide keyword/member-order closure, concrete adapter review,
whole-client property conversion and 100% parity remain open. No fresh Rust,
AOT, browser, mutation or shared-artifact gate was run; ordinary provenance heavy
lock held by Claude was respected. Windows C: was approximately 3.5 GB free,
so checks used the existing small Debug F# outputs. No C#/server/contracts/data
edit, owner port use or Codex lock.

Next: Claude/owner review the compiling real-file skeleton. Fill laws and pending
bodies after sign-off, and bind the authoritative generated producer shapes as
they land. Before further CX-FSHARP implementation, re-review Claude's exact F-86
provenance closure and A-WIRE-IDENTITIES when its implementation is submitted.

Author integration addendum, 2026-10-03: fast-forwarded the skeleton lane from
3bceb6c to pushed **eb77e95**, the separately authored account correction.
Single-verse and multi-verse runs now share the private historical-account door;
ElementId and UnitReference are left to their generated producer. See
[the account correction report](2026-10-03-fsharp-account-skeleton.md).
This is author integration and validation, not the required other-agent approval
or owner sign-off. The pending operation count remains **69**.

The integrated tree independently rebuilds Debug WASM with **0 warnings / 0
errors** and runs the existing **142 tests with 0 failures / 0 skips**. Both
normal account/position compiler consumers pass; the removed ElementId and
UnitReference consumers fail FS0039, and the private HistoricalAccount constructor
consumer fails FS1093. Pending operations are never invoked by those programs.
Compact logs and exact source head are retained in
[evidence/2026-10-03-fsharp-skeleton-integration/integration.json](evidence/2026-10-03-fsharp-skeleton-integration/integration.json).

Handoff: Claude reviews the corrected compiling skeleton, then Donovan signs off
the actual types and signatures before behavioral laws or pending bodies resume.
Reviewed WIREID is changes requested at 3fee0dc and has not been consumed. This
skeleton still binds missing producer identities/vocabularies through explicit
parameters; concrete producer binding/admission and complete client parity remain
open. No shared contract/artifact, Rust/AOT/mutation gate, server or lock touched.
