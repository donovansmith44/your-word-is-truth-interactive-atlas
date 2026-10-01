# Codex re-review: FOCUS-1 + FOCUS-6 + fixes + R19/R20

Verdict: **changes requested** on `678a0d2..caddd75` (`lane/claude/F1-int`).

This is one review of the complete stack, including the original FOCUS-1/6 changes, F-57..F-62 fixes, `Explore` with `Resume`, the single-read `IExplorer`, `IPresenter`, and the Exploring project. Findings use the **ops queue's** numbering. No application files or fixtures were changed. The binding principles are the newer main-branch `docs/PRINCIPLES.md`, including 27–27g; the reviewed branch's older copy does not contain that section. R19/R20 type approvals and O-GOLDEN-R4 are accepted owner rulings.

## Original reproductions and closure

| Previous report → ops finding | Result at caddd75 | Closure assessment |
| --- | --- | --- |
| F-38 → F-57: Event/Year alias loses Back presentation | PASS: original Event presentation restored after following a Year presentation sharing its identity, then Back | `LegacyPresentations` retains presentations by navigation path; alias-wide laws cover the other legacy wrappers. |
| F-39 → F-58: generic failure escapes handling | PASS: failing initial neighbour read shows `CouldNotLoad`; retry renders children | `Outcome`/`RequestSeries` now handle the initial failure. Overlapping reveals still escape ordering guarantees: F-67. |
| F-40 → F-59: inline groups eagerly drain | PASS: a 300-entry Shows group initially asks for 20 and renders 20 | Original eager-first-render bug fixed. Bounded lifetime working set is still open: F-68, separate from legacy F-63. |
| F-41 → F-60: node-only reuse ignores surface | PASS: the same Map changes from Popover to World and Person children disappear | Surface participates in the request key. The wider all-inputs guarantee still lacks artifact revision: F-70. |
| F-42 → F-61: three identity equalities | PASS on source inspection and the client identity laws | `PositionIdentity.Comparer` is the shared identity authority. Keep that identity label-independent when fixing presentation invalidation. |

The eager-read reproduction uses a collection larger than the new initial clamp, rather than assuming that the old three-row fixture must take three requests. This tests the original defect under the intentional new paging policy.

F-62 has one server-owned page ceiling, including the generic element read; the client follows `next` without duplicating that ceiling. The existing tests and source check support this specific closure, not a claim that every legacy read meets 27b.

## New findings

### F-67 — Important: an older reveal overwrites a newer reveal

Site: `client/Views/FocusView.razor:51–57`.

With 20 children loaded, click More twice while reads are pending. Both reads use `_series.Current`; the second starts from the same 20-entry `Paging` value but asks for 40 more. Complete the second request first: the view shows 60. Complete the first request afterward: it overwrites `Pages[kind]` with the older 40-entry result, while `Shown[kind]` remains 60. The view shrinks from 60 to 40 without a user action.

The bUnit reproduction below fails `Expected: 60; Actual: 40`. The requests are delivered in a valid network order and the fake answers their requested limits. This is not a fixed-delay timing test.

Failed abstraction: presentation generation protects navigation changes, but does not order asynchronous changes within one frontier group. Proposed closure: a single group paging state machine that serializes/coalesces demand or rejects stale completions, with monotone retained progress and current-only failures. Enumerate overlapping reveal, collapse, retry, navigation and disposal transitions, across every affordance. A global latest-request token alone would incorrectly cancel independent groups.

### F-68 — Important: generic paging still retains and renders the entire revealed prefix

Sites: `client/Exploring/Paging.cs:7–37`, `client/Exploring/Explorable.cs:8,66–69`, `client/Views/FocusView.razor:119–155`.

Starting with 300 children, fourteen More clicks leave all 300 loaded and rendered. `Paging.Then` copies the entire retained prefix into each new value; `Shown` renders that prefix without virtualization. `Explorable._pages` additionally memoizes every `(kind, cursor, limit)` result without eviction. Collapse decreases the visible count but does not discard the retained pages.

This is the **new generic path**, not F-63's known legacy `Whole` calls. F-59 fixed initial demand, but 27e requires a bounded working set and visible-window rendering over long interactions too. The scratch test uses 40 as an illustrative fixed viewport bound, **not an owner-approved product budget**; the concrete evidence is 300 of 300 DOM children and source-level absence of eviction/windowing. At greater cardinality the same path retains the entire revealed collection.

Failed abstraction: a bounded page request has been used as if it implied a bounded client collection. Proposed closure: a shared windowed page store with eviction and a virtualized view over the visible window; laws exercise long navigation/reveal sequences at increasing cardinalities and gate resident entries and DOM nodes independently of total collection size. Keep the server's transport ceiling separate from the client's viewport/cache budget.

### F-69 — Important: element reads derive counts by scanning every adjacent edge

Sites: `server/atlas-graph/src/sqlite/snapshot.rs:357–376`; `server/atlas-contract/src/graph.rs:58,122,142–155`.

Every generic node/edge record calls `edge_summary`, which executes:

```sql
SELECT rel, dir, COUNT(DISTINCT edge_id)
FROM all_edge_index WHERE subject = ?1 GROUP BY rel, dir
```

The subject index finds the adjacency range, but counting distinct edges still visits that entire range. One returned count for a high-degree position is not an index lookup plus returned-size work (27b). The count is determined entirely by the artifact and belongs in compiled data (27).

An isolated SQLite probe with the exact query and an indexed `WITHOUT ROWID` table returned one summary row at both 10,000 and 100,000 edges, while VM work rose from approximately 130,000 to 1,300,000 operations. `EXPLAIN QUERY PLAN` reports the subject lookup plus a temporary B-tree for `count(DISTINCT)`. This is a mechanism/growth reproduction, **not a measurement of production p95**: the production `all_edge_index` is a union view over section indexes, whereas the probe deliberately uses the simpler single indexed table. The query pre-existed this stack; FOCUS-6's new generic element path carries it forward, so this is an unmigrated category under 24a, not a claim that the stack invented the query. It is distinct from F-37's missing budget gate.

Proposed closure: the compiler writes distinct counts per position and edge kind, respecting cross-section duplicate edge identity; served summaries read that index only. The raw counting/scan path is unavailable to served code. An enumerating parity law covers every relation/direction, and a future-size test holds result cardinality fixed while degree grows.

### F-70 — Important: presentation cache identity omits the artifact root

Sites: `client/Exploring/PresentationRequest.cs:3`; `client/Views/FocusView.razor:24–33`; `client/Exploring/Explorable.cs:72`; `client/GraphExplorableClient.cs:16–28`.

Replace a FocusView's element with a newly resolved record having the same kind/id and surface, but `Version = root-b` and label `New map`, after it displayed `root-a` / `Old map`. It still displays `Old map`. The reproduction below fails on the public component parameter transition. `PresentationRequest` equality delegates to entity identity, so the early return ignores changed artifact inputs. `GraphExplorableClient.Elements` also discards `ElementPage.Version`; edge records have no per-record version to recover it from.

This is a demonstrated component/cache contract failure when a newer record is supplied, **not a claim that a live hot-reload route was exercised**. It extends the all-inputs closure of F-60 under new rule 27d; its original surface-only reproduction is green. Keep entity identity stable and separate it from snapshot/presentation identity.

Proposed closure: carry the artifact root through resolved nodes and edges, key cached answers/presentations by root and request, and invalidate old-root pages together. Laws cover same id on a new root, same-root reuse, both node/edge elements, and paged responses crossing a root change.

## D.R.Y., Haskell bar, and category pass

The approved `Explore` composition, closed outcome cases, one `IExplorer` read, and separate `IPresenter` are a substantial simplification. The review does not propose an alternative to the signed-off R19/R20 types. `Resume` batches its saved trail resolution. Labels and edge ends are served with the data, and identity wrappers now share one comparer.

The remaining state in `FocusView.Presented` consists of independently mutable page/count dictionaries; F-67 demonstrates that their consistency is a convention instead of a guarantee. `Paging` centralizes the mechanics, but an ever-growing prefix is not the working-set abstraction required by 27e. Similarly, the common `summary_at` door does not close server derivation while its underlying store still executes COUNT. These are concrete 14b/24a/24b failures rather than requests for more wrappers.

Against 27–27g: inspected the compiler/reader split, generic element and neighbour reads, paging/caches, batching, and the map/presentation changes. F-69 concerns 27/27b, F-70 concerns 27d, F-68 concerns 27e; F-67 is the asynchronous interaction closure. Existing queued architecture debt and deferred map tiling are not certified closed by this review. In particular, existing 27f gates are not evidence of a 10× synthetic budget gate (F-37). Known F-55, F-63, F-65 and F-66 are not re-filed, nor are already recorded label/hash/route findings.

The added-comment scan is not clean: `server/atlas-contract/src/wire/graph.rs` adds `///` descriptions on `ElementPage.version` and `.next`, and edits an existing version doc line. These are further sites in existing **F-12 / A-STRIP**, not a new finding number. Its unresolved field-description question is not a recorded exception to rule 9. Moved pre-existing comments were distinguished from new content with a rename-aware diff.

## Independent verification

At exact head `caddd75`, in isolated worktree `/home/donovan/w/A-F1-stack-review`:

- `dotnet test client.Tests`: **685 passed**, zero failed.
- `dotnet test client.ContractTests`: **55 passed**, zero failed.
- `CARGO_TARGET_DIR=/home/donovan/mut/codex-A-F1-stack-review nice -n 10 cargo test --manifest-path graph-types/Cargo.toml --all-features -j4`: **147 passed**, zero failed.
- Scratch bUnit reproductions: **4 original behavior reproductions passed; 3 new red reproductions**, matching F-67, F-68 and F-70.
- SQLite growth probe: counts correct, degree-linear work for a fixed-size answer (F-69).
- Read the FOCUS-6 close report, prior Codex review, complete-range diff, applicable main-branch principles, and ops gate results/rulings. Traced exploration, presentation, paging, saved navigation, World/geography, generic server reads, and compiler/reader law boundaries.

The author's full Rust, timing, contract-gate and browser results at `caddd75` are recorded on ops; **I did not independently rerun those full gates**. Browser carried failures and flakes are not new findings here. No owner server was touched, no fixture was re-blessed, and no lock remains held by this review.

Logs on this machine: `/tmp/codex-caddd75-{client-tests,contract-tests,graph-types,repros,summary-probe}.log`. Scratch project: `/tmp/codex-caddd75-review/Review.csproj`. The self-contained evidence below preserves the probes beyond those temporary files.

## Reproduction source

Create a net10.0 test project outside the checkout, reference `client/BibleAtlas.Client.csproj`, and link `client.Tests/Explore/ServedGraph.cs` from **caddd75**. Packages: Microsoft.NET.Test.Sdk 17.14.1, bunit 2.11.3, xunit 2.9.3, xunit.runner.visualstudio 3.0.2; implicit usings and nullable enabled; add a global `using Xunit;`. Run after `. ~/.bible-atlas-env`.

```csharp
using BibleAtlas.Client;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Legacy;
using BibleAtlas.Client.Views;
using BibleAtlas.Client.Tests;
using Bunit;
using Microsoft.Extensions.DependencyInjection;

public sealed class ReviewTests : BunitContext
{
    private static readonly NodeRef Map = ServedGraph.Ref(NodeKind.Map, "Map:test", "Map");
    private static readonly NodeRef Person = ServedGraph.Ref(NodeKind.Person, "Person:test", "Person");
    public ReviewTests() => Services.AddSingleton<IPresenter>(new GraphPresenter());

    [Fact]
    public void Original_Back_alias_reproduction_is_fixed()
    {
        var identity = ServedGraph.Ref(NodeKind.Event, "Event:test", "Event");
        var node = Resolved.Node(identity);
        var views = new LegacyPresentations();
        var start = new Exploration(node, []);
        var original = views.Present(start);
        var year = new Year("1406 BC", -1406);
        var dated = start.Follow(new Step(EdgeKind.DatedBy, node));
        views.Arrive(dated, new YearNode(new TimeRange(year, "1406 BC", year), identity));
        Assert.Same(original, views.Present(dated.WalkedBack()));
    }

    [Fact]
    public void Original_failure_reproduction_is_fixed_and_can_retry()
    {
        var graph = new PagedGraph { Fail = true };
        var node = Resolved.At(graph, ServedGraph.At(Map));
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));
        Assert.Single(view.FindAll("[data-testid='could-not-load-retry']"));
        graph.Fail = false;
        view.Find("[data-testid='could-not-load-retry']").Click();
        Assert.Equal(20, view.FindAll(".focus-child").Count);
    }

    [Fact]
    public void Original_eager_paging_reproduction_is_fixed_at_large_cardinality()
    {
        var graph = new PagedGraph();
        var node = Resolved.At(graph, ServedGraph.At(Map));
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));
        Assert.Equal(new (int?, int)[] { (null, 20) }, graph.Reads);
        Assert.Equal(20, view.FindAll(".focus-child").Count);
    }

    [Fact]
    public void Original_surface_reuse_reproduction_is_fixed()
    {
        var graph = new PagedGraph();
        var node = Resolved.At(graph, ServedGraph.At(Map));
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));
        view.Render(p => p.Add(v => v.Surface, Surface.World));
        Assert.Empty(view.FindAll(".focus-child"));
    }

    [Fact]
    public async Task Older_reveal_completion_does_not_replace_a_newer_reveal()
    {
        var graph = new PagedGraph { Delay = true };
        var node = Resolved.At(graph, ServedGraph.At(Map));
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));
        var first = view.Find("[data-testid='popover-children-shows-more']").ClickAsync(new());
        view.WaitForAssertion(() => Assert.Single(graph.Pending));
        var second = view.Find("[data-testid='popover-children-shows-more']").ClickAsync(new());
        view.WaitForAssertion(() => Assert.Equal(2, graph.Pending.Count));
        graph.Pending[1].SetResult(graph.Page(20, 40));
        await second;
        Assert.Equal(60, view.FindAll(".focus-child").Count);
        graph.Pending[0].SetResult(graph.Page(20, 20));
        await first;
        Assert.Equal(60, view.FindAll(".focus-child").Count);
    }

    [Fact]
    public void Same_identity_from_a_new_artifact_refreshes_the_presentation()
    {
        var first = ServedGraph.Card(Map.Kind, Map.Id, "Old map") with { Version = "root-a" };
        var newer = first with { Label = "New map", Version = "root-b" };
        var oldNode = Resolved.Node(new ServedGraph().Serving(first), Map);
        var newNode = Resolved.Node(new ServedGraph().Serving(newer), Map);
        var view = Render<FocusView>(p => p.Add(v => v.Node, oldNode).Add(v => v.Surface, Surface.Popover));
        view.Render(p => p.Add(v => v.Node, newNode));
        Assert.Equal("New map", view.Find(".focus-title").TextContent);
    }

    [Fact]
    public void A_long_reveal_keeps_the_rendered_working_set_bounded()
    {
        var graph = new PagedGraph();
        var node = Resolved.At(graph, ServedGraph.At(Map));
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));
        for (var i = 0; i < 14; i++)
            view.Find("[data-testid='popover-children-shows-more']").Click();
        Assert.True(view.FindAll(".focus-child").Count <= 40, $"Rendered {view.FindAll(".focus-child").Count} of 300 children");
    }

    private sealed class PagedGraph : IExplorableClient
    {
        public bool Fail;
        public bool Delay;
        public List<(int?, int)> Reads = [];
        public List<TaskCompletionSource<EdgePage>> Pending = [];
        public Task<NodeRecord> Card(string id) => Task.FromResult(ServedGraph.Card(Map.Kind, Map.Id, Map.Label, new FrontierGroup(EdgeKind.Shows, 300)) with { Map = ServedGraph.MapWindow(ServedGraph.Range(new Year("2 BC", -2), new Year("1 BC", -1), "2–1 BC")) });
        public async Task<IReadOnlyList<Element>> Elements(IReadOnlyList<string> ids) => [new NodeElement(await Card(Map.Id))];
        public Task<EdgePage> Edges(string id, EdgeKind kind, int? cursor = null, int limit = 20)
        {
            Reads.Add((cursor, limit));
            if (Fail) return Task.FromException<EdgePage>(new HttpRequestException("offline"));
            if (Delay && cursor is not null)
            {
                var pending = new TaskCompletionSource<EdgePage>(TaskCreationOptions.RunContinuationsAsynchronously);
                Pending.Add(pending);
                return pending.Task;
            }
            return Task.FromResult(Page(cursor ?? 0, limit));
        }
        public EdgePage Page(int start, int n) => ServedGraph.Page(EdgeKind.Shows, start+n < 300 ? start+n : null, Enumerable.Range(start,n).Select(i => Person with { Id = $"Person:{i}" }).ToArray());
        public Task<TextWindow> Reading(string r, int n, WindowDir d = WindowDir.Onward, Corpus c = Corpus.Bible) => throw new NotSupportedException();
    }
}

```

SQLite probe (Python standard library):

```python
import sqlite3
query = 'SELECT rel, dir, COUNT(DISTINCT edge_id) FROM all_edge_index WHERE subject = ?1 GROUP BY rel, dir'
for degree in (10_000, 100_000):
    db = sqlite3.connect(':memory:')
    db.execute('CREATE TABLE all_edge_index(subject TEXT, rel INTEGER, dir INTEGER, ord INTEGER, edge_id TEXT, PRIMARY KEY(subject,rel,dir,ord)) WITHOUT ROWID')
    db.executemany('INSERT INTO all_edge_index VALUES (?,1,0,?,?)', [('subject', i, str(i)) for i in range(degree)])
    ticks = [0]
    def tick():
        ticks[0] += 1
        return 0
    db.set_progress_handler(tick, 1000)
    rows = db.execute(query, ('subject',)).fetchall()
    print(f'degree={degree}, returned_rows={len(rows)}, VM_ops_approx={ticks[0]*1000}, result={rows}')
    db.set_progress_handler(None, 0)
    print(db.execute('EXPLAIN QUERY PLAN ' + query, ('subject',)).fetchall())

```
