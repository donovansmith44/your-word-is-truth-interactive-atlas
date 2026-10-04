# F# browser schema-validator fit

The pinned MIT-source JsonSchema.Net validator executes the same eight native
schema properties in Chromium WebAssembly. All **701 trials** pass, including
all **35 original query fixtures**. This is a test-host fit checkpoint, not
application adoption or full client parity. Base: `ade5e14`; approved producer:
`da7e00d`; lane: `lane/codex/CX-FSHARP-wire`.

## One property suite and one input preparation tool

[SurveyInputs.fs](../../../client-fsharp.Tests/SchemaSpike/SurveyInputs.fs)
owns native preparation of the original OpenAPI and fixture JSON, using the
installed YamlDotNet and System.Text.Json libraries. Both the native tests and
the export tool call it. The browser receives the resulting JSON; it does not
read raw YAML or repository files.

The standalone Bolero/Elmish host compiles
[SchemaLaws.fs](../../../client-fsharp.Tests/SchemaSpike/SchemaLaws.fs) directly.
It initializes that suite with the prepared inputs and discovers its compiled
`PropertyAttribute` methods. FsCheck's existing `Check.Method` runs every
discovered property with its declared `MaxTest`. There is no second handwritten
property inventory or second set of schema assertions. The full native
property names appear in the browser report. The Python driver launches the
browser and records results; semantic assertions remain FsCheck properties.

## Executed gates

| Gate | Result |
|---|---|
| Native shared input/schema suite, including property-only gate | 9 pass, 0 fail, 1.3365 s |
| Final Debug WASM host build | 0 warnings/errors, 8.27 s |
| Final Chromium schema properties | 8 pass; fixture property once, seven generated properties 100 times each |
| Fixture acceptance | All 35 original query answers accepted |
| Deliberately invalid artifact-root control | Fixture property falsified after 1 trial; driver correctly returns success for expected failure |
| Browser faults and HTTP refusals | None in final positive or negative-control runs |
| Final newspaper-order suite | 13 pass, 0 fail, 2.4751 s; 77 authored and 2 generated F# sources |

The final positive property run takes **8,536 ms** inside the host and
**10.374 s** including browser navigation. These are suite timings, not a
per-answer latency budget. The control intercepts only the browser's input
response and replaces one fixture's artifact root with `root-0`; it leaves
repository fixtures and the exported input unchanged. The expected full
fixture result fails at `contents-bible`, proving the browser executes the
actual assertion rather than merely displaying a successful build.

Initial host compilation failures were incorrect project-path and F# call/init
syntax, not production behavioral reds. The first browser execution received
an **empty HTTP 200** for `survey.json` because the generated content's root
was incorrectly inferred as `wwwroot`. Explicit MSBuild `ContentRoot` fixes
that test-host mapping; the final asset is **373,584 bytes**, SHA-256
`99dfa81a250b300d564f4d23370473c40578e4ade1e6d44014e646ee442baee2`.
That derived input is regenerated, not committed as another source of truth.

## Footprint and license

The four validator runtime WebCIL resources total **1,033,577 encoded bytes**
in this untrimmed Debug host: JsonSchema.Net, JsonPointer.Net, Json.More and
Humanizer. Separate symbols and the property's FsCheck/xUnit/YAML dependencies
are not included in that subtotal. The complete Debug property host loads
**42,378,486 encoded resource bytes**. Neither number is a published client
size measurement; no trim, AOT, shipping comparison or future-size budget ran.

The source pin remains `399f198431f65cf6896fe6038f833ef6d0b27a39`, with
the existing three build-only SourceLink updates and no library-code changes.
The [prior native fit report](2026-10-03-fsharp-json-validation.md) records
the source license, dependency licenses and reproducible source checkout/patch.
The NuGet validator binaries' OSMFEULA remains excluded. The MIT-source test
dependency has not been added to the application.

## Reproduce

Prepare the pinned source checkout and SourceLink patch as described in the
prior report. From this lane worktree:

```bash
. /home/donovan/.bible-atlas-env
dotnet test client-fsharp.Tests/SchemaSpike/SchemaSpike.fsproj -p:JsonSchemaSource=/tmp/codex-json-schema-fit-source -p:GeneratePackageOnBuild=false --logger 'console;verbosity=normal'
dotnet build client-fsharp.Tests/SchemaSpike/Wasm/WasmFit.fsproj -p:JsonSchemaSource=/tmp/codex-json-schema-fit-source -p:GeneratePackageOnBuild=false
dotnet run --project client-fsharp.Tests/SchemaSpike/Wasm/WasmFit.fsproj --no-build --no-restore --urls http://127.0.0.1:5100
```

Run the driver from another shell using an already installed Chromium:

```bash
python3 client-fsharp.Tests/SchemaSpike/Wasm/run-probe.py --browser /home/donovan/.cache/ms-playwright/chromium-1234/chrome-linux64/chrome --output /tmp/schema-browser.json
python3 client-fsharp.Tests/SchemaSpike/Wasm/run-probe.py --browser /home/donovan/.cache/ms-playwright/chromium-1234/chrome-linux64/chrome --invalidate-root --expect failed --output /tmp/schema-control.json
```

Require port 5100 to be free first; stop only the process started for this host.
The browser path is an explicit local tool selection, not an application path.

## Limits and handoff

[Compact evidence and source hashes](evidence/2026-10-03-fsharp-schema-wasm-fit/manifest.json)
retain native/build/order results, browser resource measurements, the genuine
negative-control failure and initial infrastructure failures. Saved logs
normalize trailing whitespace; original-output hashes are retained. The normal
190-property application suite and 271-unused/59-pending inventory belong to
the preceding `ade5e14` checkpoint and were not rerun here. No application
behavior changed. Newspaper-order syntax coverage still has the previously
recorded alias/overload/compiler-symbol limitations; it is not full semantic
order closure.

Schema/type bindings, nested concrete discriminator enforcement, precise
structured diagnostics and one application validation door remain unfinished.
The original base discriminator schema still accepts an incomplete child;
the concrete schema and generated serializer reject it. The fit does not
justify dropping concrete serializer validation. Frontier/read integration,
root-aware unused checking, domain sign-off and full parity remain open.

No original schema/fixture, C# client, shared artifact or Claude source changed.
Parent `88f0d26`, order `79671ab`, frozen STYLE `35f6797` and R2 `d17644f`
remain unchanged. The owned 5100 host is stopped and no Codex lock remains.
CX-I3 retains the owner's instruction to prune obsolete stopped mutation
results and disposable build output after preserving compact outcomes. Current
WSL free space is about **780 GB**, but the Windows VHD host has only
**1.9 GB** free. No fresh Cargo/AOT target, raw/cache deletion, other-agent
cleanup or VHD compaction occurred.
