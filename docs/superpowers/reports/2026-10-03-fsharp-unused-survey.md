# Unused-symbol gate: library survey supplement

Research under CX-FSHARP, on the isolated account/report lane after **910b445**.
The restarted headless session owns implementation in `~/w/CX-FSHARP`.
This supplements its vocabulary report; it changes no application, gate or package.

Use the already pinned **FSharp.Compiler.Service 43.12.401** for semantic symbols
and project analysis. Its editor unused-declaration helper is insufficient for
the owner’s ops **1b05c04** requirement. No complete replacement rule was identified
in the catalogues surveyed below. That is a bounded survey result, not a claim
that no such tool exists anywhere.

## Candidates, licences and fit

| Candidate | Licence / maintenance evidence checked | Fit and decision |
|---|---|---|
| F# compiler warnings | Compiler included with the existing SDK; maintained Microsoft toolchain | FS1182 is documented as an opt-in unused-variable warning. Retain ordinary compiler checks; do not treat this warning as the whole exported-symbol gate. |
| FSharp.Compiler.Service 43.12.401 | MIT in the installed package metadata; package source commit **e34a38d2ae1fc26406a317517196e55c68ff83ab** | Selected semantic engine, already installed. Its `UnusedDeclarations` editor helper is file-local and intentionally narrower than this task. |
| FSharpLint | MIT; inspected **d2607f5**, repository activity 2026-09-02 | Style, hints and typed/untyped lint rules. Rule inventory did not identify the requested cross-project, tests-excluded reachability policy. The similarly named UselessBinding rule detects self-bindings, not the owner’s category. No additional installation proposed. |
| FSharp.Analyzers.SDK / fsharp-analyzers | MIT; inspected **ae2ee72**, changelog 0.39.2 dated 2026-09-12 | Existing CLI and typed/untyped traversal framework, not itself an unused-code rule. Prefer its collectors if additional general traversal machinery is needed. Current source pins FCS 43.12.400 and FSharp.Core 10.1.400; compatibility with this repository’s 43.12.401 pin has not been tested. |
| Ionide.Analyzers | MIT; inspected **093975d**, changelog 0.19.0 dated 2026-09-10 | Published rules address particular style/performance defects. No requested whole-client rule identified. IgnoreFunction checks an ignored function value, not unreferenced definitions. |
| G-Research.FSharp.Analyzers | Apache-2.0; inspected **7ab2d89**, repository activity 2026-09-12 | Specific API/lifetime/style rules. No requested rule identified; UnionCaseAnalyzer concerns name shadowing, not unused union cases. |
| WoofWare.FSharpAnalyzers | MIT; inspected **bb09a06**, repository activity 2026-09-29 | Specific async/API rules, maintained for its author’s personal use. No requested rule identified in its ten inventoried analyzer sources. |

Sources: [compiler options](https://learn.microsoft.com/en-us/dotnet/fsharp/language-reference/compiler-options),
[FCS project analysis](https://fsharp.github.io/fsharp-compiler-docs/fcs/project.html),
[FSharpLint](https://github.com/fsprojects/FSharpLint/tree/d2607f5c68fe0e8c0366e815692cb8d314e254a3),
[analyzer SDK installation](https://ionide.io/FSharp.Analyzers.SDK/content/getting-started/Installing%20Analyzers.html),
[SDK package pins](https://github.com/ionide/FSharp.Analyzers.SDK/blob/ae2ee72d6bde9ad62f9a47ea3f070c9a5117e347/Directory.Packages.props),
[Ionide rules](https://github.com/ionide/ionide-analyzers/tree/093975da0ecd9fce77d3a52e70a014f70ab450ef/src/Ionide.Analyzers),
[G-Research rules](https://github.com/G-Research/fsharp-analyzers/tree/7ab2d8906209560ed96808a27e57f41fe05000fa/src/FSharp.Analyzers),
[WoofWare rules](https://github.com/Smaug123/WoofWare.FSharpAnalyzers/tree/bb09a06c936fd07e9ced75fdcedab7715615033d/WoofWare.FSharpAnalyzers).

## Exact installed helper limitation

The package’s repository metadata points to the dotnet monorepo. Its pinned
[ServiceAnalysis.fs](https://github.com/dotnet/dotnet/blob/e34a38d2ae1fc26406a317517196e55c68ff83ab/src/fsharp/src/Compiler/Service/ServiceAnalysis.fs)
shows that the Boolean argument means **isScriptFile**. Ordinary project analysis
considers definitions private to a file; record/union/interface/module/class
entities, constructors and override/interface implementations are skipped.
It gathers uses from that file alone. Passing true would change script handling,
not turn this into cross-project application reachability.

These facts were checked against the exact package source, rather than inferred
from the XML summary. Source hashes, licence metadata, package pins, dated
changelog headings and untruncated rule-path inventories are retained in
[sources.json](evidence/2026-10-03-fsharp-unused-survey/sources.json).
Inventory coverage is not a runtime compatibility result.

## Closure requirements for the implementation owner

Reuse FCS for parsing, name resolution and project references. The application
policy must distinguish a referenced declaration from one reached by a real
screen or the owner’s model: references entirely within a dead island must not
certify it as live. Tests and audit/probe callers do not supply that evidence.
Do not silently skip generic parameters, modules, types or union cases.
Framework dispatch and generated identities need explicit semantic handling,
not a blanket exemption for all public APIs or all override methods.

Generated property probes should vary names and file/project placement and assert
whole diagnostic inventories for: exported/internal dead declarations; test-only
callers; unreachable chains and recursive islands; live callbacks and cross-project
calls; unused type parameters; dead cases/modules; and compiler failures. The
single pending marker must not launder an unrelated dead sibling or wrapper;
its exemption must remain counted and shrink to zero. These are required future
checks, not tests executed by this research report.

No candidate was installed, no analyzer was implemented or run, and no build,
Rust target, lock or live worktree was touched. The CLI/native audit belongs in
tooling; no compiler/analyzer package is proposed for the WASM runtime. Claude
and the implementation session can consume this supplement without cherry-picking
any application code. Full parity and owner sign-off remain open.
