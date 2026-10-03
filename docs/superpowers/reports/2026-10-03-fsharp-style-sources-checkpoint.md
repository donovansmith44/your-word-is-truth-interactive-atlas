# Sources exemplar checkpoint

Base `2e85a19`; work is isolated on `lane/codex/CX-FSHARP-STYLE`.

Sources now enters a private `SourcesModel`, advances through exhaustive messages,
and stores one prepared presentation. The presenter validates duplicate and
unlisted categories, preserves the whole served document and order, and prepares
Visit affordances once. The view neither groups records nor classifies retry.
The command interpreter uses the generated request descriptor and delivers the
whole typed completion. The application delegates to these owners; the old
Sources LoadState, grouping and card renderer are removed.

HTTP library calls and exceptions have one internal adapter. Read failures remain
network, cancellation, HTTP rejection (status/reason/typed error body), or invalid
answer. Sources may retry network/cancellation/5xx; invalid answers, other HTTP
refusals and inconsistent category presentation are terminal. This policy is
proposed in the style spec, not a claim of malformed-answer parity with C#.
Non-Sources operations keep the existing compatibility Failure until migration.

**Verified:** 30 exemplar properties pass, zero failures/skips. They include the
complete Sources state/message matrix (153 pairs per generated trial; reflection
checks all five state cases), all HTTP rejection statuses, wire round trips and
failures, Cmd interpretation, complete category/card markup, application rejection,
and >=10,000-turn paging with varied full windows. The paging law checks whole
pending/ready state and effect values on each turn: one capped request and exactly
one prior/current page, never accumulated history. It exercises existing behavior;
no paging feature was added. The final pinned regression gate:
108 pass, zero failures/skips, including converted Sources DOM/startup properties. The tests build Debug WASM;
there is no fresh AOT/browser differential/mutation/full-parity claim.

FSharp.Compiler.Service is now explicitly pinned to 43.12.401 in both test projects,
matching FSharp.Core 10.1.401. Its downloaded nuspec declares MIT. The source gate
walks FCS syntax rather than matching words inside literals; self-properties check
exception flow, mutation, mutable containers, partial access, module/member order
and harmless string literals. Its scope is the named exemplar production files.
This is not comprehensive semantic proof of arbitrary local shadowing, fake
recursion or all legacy files. Initial gate false positives (a read parameter,
a separate JsonPath module and the proven nonempty head binding) were corrected
by lexical module grouping, parameter exclusion and qualified partial access.
The member-order self-property then caught members in ObjectModel rather than
SynTypeDefn's trailing list; that real checker red was fixed. Scanning/caching
references once per binding reduced the source suite from about 23s to about 4s.

**Red evidence:** presenter/update placeholders gave 3 failures then the exhaustive
transition property still failed alone; typed HTTP placeholder gave 4 failures;
Cmd.none gave 1; empty renderer gave 2; application integration gave 1 before
production migration. The empty-render run was a deliberate substitution after
fixture compilation was corrected, not a claim that compile errors are behavioral
reds. Generator/decoder evidence remains in the previous checkpoint report.
Compact behavioral failure names and summaries follow; full temporary logs are in
`~/mut/codex-CX-FSHARP-STYLE-logs`.


## sources-red.log

```text
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.SourcesLaws.the source model has a whole-value transition for every message and reachable state [42 ms]
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.SourcesLaws.the source presentation preserves the whole document and every card in served order [4 ms]
---- Assert.Equal() Failure: Values differ
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.SourcesLaws.repeated category identities refuse their whole records before grouping [2 ms]
---- Assert.Equal() Failure: Values differ
Failed!  - Failed:     3, Passed:    13, Skipped:     0, Total:    16, Duration: 1 s - BibleAtlas.FSharp.StyleGeneration.Tests.dll (net10.0)
```

## sources-transition-red.log

```text
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.SourcesLaws.the source model has a whole-value transition for every message and reachable state [126 ms]
---- Assert.Equal() Failure: Collections differ
Failed!  - Failed:     1, Passed:     0, Skipped:     0, Total:     1, Duration: 172 ms - BibleAtlas.FSharp.StyleGeneration.Tests.dll (net10.0)
```

## http-red.log

```text
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.HttpLaws.a served HTTP refusal preserves its status reason and whole typed body [164 ms]
---- Assert.Equal() Failure: Values differ
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.HttpLaws.the Sources read delivers every field through the typed contract boundary [22 ms]
---- Assert.Equal() Failure: Values differ
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.HttpLaws.a successful null response remains a typed contract failure [3 ms]
---- Assert.Equal() Failure: Values differ
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.HttpLaws.transport failures preserve their category and diagnostic [4 ms]
---- Assert.Equal() Failure: Values differ
Failed!  - Failed:     4, Passed:     0, Skipped:     0, Total:     4, Duration: 274 ms - BibleAtlas.FSharp.StyleGeneration.Tests.dll (net10.0)
```

## command-red.log

```text
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.CommandLaws.a Sources command reads its generated descriptor once and returns the entire typed message [75 ms]
---- Assert.Single() Failure: The collection was empty
Failed!  - Failed:     1, Passed:     0, Skipped:     0, Total:     1, Duration: 134 ms - BibleAtlas.FSharp.StyleGeneration.Tests.dll (net10.0)
```

## view-red.log

```text
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.SourcesViewLaws.only a prepared retryable source failure offers Retry [316 ms]
---- Assert.Equal() Failure: Collections differ
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.SourcesViewLaws.the Sources view renders every prepared category and complete card without regrouping [17 ms]
---- Assert.Equal() Failure: Collections differ
Failed!  - Failed:     2, Passed:     0, Skipped:     0, Total:     2, Duration: 385 ms - BibleAtlas.FSharp.StyleGeneration.Tests.dll (net10.0)
```

## sources-app-red.log

```text
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.SourcesAppLaws.the application refuses terminal source answers without offering a network retry [348 ms]
---- Assert.Equal() Failure: Collections differ
Failed!  - Failed:     1, Passed:     0, Skipped:     0, Total:     1, Duration: 392 ms - BibleAtlas.FSharp.StyleGeneration.Tests.dll (net10.0)
```

## style-source-fixed.log

```text
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.SourceLaws.the source gate refuses helpers placed above their first module or member caller [35 ms]
---- Assert.Equal() Failure: Collections differ
  Name = "Detail0 before Intent" }]
Failed!  - Failed:     1, Passed:     3, Skipped:     0, Total:     4, Duration: 4 s - BibleAtlas.FSharp.StyleGeneration.Tests.dll (net10.0)
```

**14b:** one Sources owner for transitions, one presenter for associations and
Visit policy, one interpreter for commands; generated records remain the wire
vocabulary. Sources presentation keeps an original document plus composed cards;
this is a view of served values, not a second domain authority. The property
oracle independently describes whole output values. The source checker is a
library test adapter, not application domain code.

**24a/24b:** raw Sources documents enter successful application state through the
presenter only; private model/presentation constructors fence direct installation.
Completion tickets prevent departed, duplicate and stale reads from altering the
page. HTTP exceptions belong to one internal adapter. Old Sources sites are all
migrated. Existing non-Sources textual failures, Vocabulary generation, request
encoding, trail/resources, every-file newspaper order and full parity remain open.

**Owner steering received while verifying this checkpoint:** ops 570f5b9 and
7572dae require domain/data structures/algebras first. Stop further exemplar or
view code; retain this checkpoint, do not consume it into the parent or call it
review-ready. Draft the sign-off package on the parent lane next. Later exemplars
must consume the approved domain through one wire admission boundary.

**Disk:** ops b1a9928 records periodic mutation/build output retention and actual
cleanup: 1,336,476,418 bytes of stopped own build output, plus four empty finished
review worktrees after zero-symlink checks. Pushed reports, raw/cache copies and
active audit scratch were retained. Guest cleanup does not shrink the VHD; C:
remains about 3.5 GB free. Claude's code, processes and tidy script were preserved.
No Codex lock/server remains.
