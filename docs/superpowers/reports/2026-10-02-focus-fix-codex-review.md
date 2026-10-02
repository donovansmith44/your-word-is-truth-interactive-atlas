# Codex re-review: F-67–F-70 fixes and More/Less

Verdict: **changes requested**. F-67 and F-69 pass re-review. F-68 and F-70 have concrete remaining sites in their existing categories; no duplicate F-numbers are assigned.

Reviewed the queued fix range `caddd75..8bf2292` on `lane/claude/F1-int`, in the context of the prior whole-stack review (`678a0d2..caddd75`). Also reviewed the newer, one-commit follow-up `8bf2292..f9789f1` on `lane/claude/PAGES20`, so the verdict includes the owner's latest **20 per page, More appends to 40 then slides, Less goes back 20, 32 cached pages** ruling. The older 60-row setting is superseded, not a new finding. The two remaining defects reproduce on both heads.

## Findings that remain open

### F-70 — Important: person mentions bypass artifact-root enforcement

At `f9789f1`: `client/Exploring/Paging.cs:42–43,67–77`, `client/Legacy/PopoverSectionProviders.cs:1269`, and `client/Components/PersonMentionsList.razor`.

The new generic element path correctly carries the served root, compares presentation identity separately from entity identity, rejects a mismatched neighbour response, and renews the exploration. But the other `Paging.Window` overload receives the raw `IExplorableClient`. Its `Neighbours` adapter immediately discards `EdgePage.Version`; the new `Remembered` cache is keyed only by cursor and limit. `PersonCardAndMentionsSection` uses this overload in the running app.

Deterministic bUnit reproduction: open `PersonMentionsList` with 20 entries served under root A; change the fake graph to root B; click More. The rendered window contains **20 root-A entries followed by 20 root-B entries**, with no refusal or renewal. The rows deliberately identify their source root in the served label, so the test observes the rendered result, not internal fields. This fails at both `8bf2292` and `f9789f1`.

This is an unmigrated site of F-70's root-consistency category under 24a/24b, not F-63's known legacy whole-collection reads. The component was migrated to `PageWindow` by this fix wave, and PAGES20 adds a cache on that same bypass. The direct same-id/new-root FocusView reproduction now passes; that specific fix is credited.

Closure: route the production person-mentions window through the same root-aware read and renewal mechanism as generic windows. Keep the root until validation and caching have occurred; reject or renew the whole window before mixing answers. An enumerating law must cover **every production page-window entry point**, including the raw-client overload, with changes between initial read/More/Less/retry and cached revisits. Do not add a second independent root checker to this component.

### F-68 — Important: cursor history remains unbounded and is copied on every slide

At `f9789f1`: `client/Exploring/PageWindow.cs:22,115,121` (same implementation at `8bf2292`, with the older window width).

The visible rows and cached page payloads are now bounded. However, every evicted block appends its start cursor to `_earlier` using `[.. _earlier, cursor]`, allocating and copying the entire previous history. Moving back similarly copies the prefix with `SkipLast(1).ToList()`.

A complete forward traversal with fixed 20-row pages and a 40-row window retains:

| Collection size | Visible rows at end | Retained earlier cursors |
| ---: | ---: | ---: |
| 300 | 40 | 13 |
| 3,000 | 40 | 148 |
| 30,000 | 40 | 1,498 |

The same probe at `8bf2292` retains 12 / 147 / 1,497 cursors under its 60-row window. Thus metadata space is O(pages visited), a slide copies O(pages visited) metadata, and a full forward walk performs quadratic total cursor-copy work. This is the remaining bounded-working-set/per-page-work defect under 27e; the 32-page LRU does not bound `_earlier`.

The diagnostic probe reflects the private history solely to measure retained state; its field name is **not** a proposed permanent test contract. It asserts independence from collection size, not an invented numerical cursor budget. The current laws successfully check visible rows and cache eviction, but do not account for this retained metadata.

Closure: a bounded navigation representation whose backward paging preserves the owner's More/Less behavior without retaining/copying every visited cursor. Choose the client/server boundary for that capability explicitly rather than silently shortening backward navigation. Gate all resident paging metadata and per-turn work over long traversals, alongside the existing DOM/payload bounds. Replacing the list with an append-efficient but unbounded collection would only fix the copying cost.

## Fixes that pass

- **F-67:** `PageWindow` serializes reads per group and coalesces demand. The original two-click scenario now keeps one read in flight; after both requested pages arrive, PAGES20 shows the expected rows 21–60 without a stale completion replacing them. The existing interleaving model tests cover More/Fewer/Answer/Fail, with separate stop and retry laws. The independent component reproduction passes.
- **F-69:** compiled `edge_count` replaces per-request `COUNT(DISTINCT edge_id)`. The real-artifact parity law passes across inhabited relation kinds and positions. The actual SQLite reader growth law passes at degree 10 versus 10,000 with equal VM work for the summary. The served-source counting gate passes. Already-filed F-73's optional-section duplicate-edge limitation remains on the queue; this review does not re-file or waive it.
- **F-68, completed portion:** More/Less now renders 20 then 40 rows and slides at 40; the independent full 300-entry reveal ends at rows 261–300, then Less shows 241–280. `ServedPages` evicts payloads through the shared LRU. The failure is retained cursor history, not the original full DOM prefix.
- **F-70, completed portion:** same entity/root reuses presentation; the same entity on a different root refreshes it. Node and edge resolutions retain the envelope root, and multi-page element reads restart across a changed root. The independent new-label component reproduction and the author's artifact-move laws pass. The remaining bypass is identified above.

## D.R.Y., Haskell bar and closure

The direction is sound: one group paging state machine replaces independently mutable shown/page dictionaries, and `PageStore` factors memoization/eviction rather than copying it between caches. Entity identity and served identity are deliberately separate. Compiled count rows remove the degree-dependent server derivation.

The two residual findings are abstraction-boundary failures. A rootless adapter can still feed the same view abstraction as a validated root-aware reader, so the type/API boundary permits the mixed-root offender. Meanwhile a bounded payload abstraction has an unbounded navigation-history member, so the resource guarantee is narrower than the public claim. These are existing F-70/F-68 categories under 14b/24a/24b, not requests for unrelated features.

The incremental rename-aware added-comment scan over `client/`, `server/`, and `graph-types/` found no new non-AAA comment lines. `LruCache` moved unchanged, including its old comment. Existing F-12/A-STRIP and the previously filed field-comment sites are not declared closed. Known F-55, F-63, F-65, F-66, F-72 and F-73 are not new reports here.

## Independent verification and limits

| Check | Exact head | Result |
| --- | --- | --- |
| `dotnet test client.Tests` | `8bf2292` | 715 passed |
| `dotnet test client.ContractTests` | `8bf2292` | 55 passed |
| `dotnet test client.Tests` | `f9789f1` | 721 passed |
| `dotnet test client.ContractTests` | `f9789f1` | 55 passed |
| `atlas-graph --test sqlite_laws an_edge_summary_costs_the_same_work_however_many_edges_its_position_holds` | `8bf2292` | 1 passed |
| `atlas-graph --test edge_counts_real_data` | `8bf2292` | 1 passed; all artifact positions compared |
| `atlas-contract --test no_served_edge_counting` | `8bf2292` | 2 passed |
| Independent bUnit/resource probes below | `f9789f1` | 3 fixed-behavior probes passed, 2 residual-category probes failed |
| The two residual probes retargeted to integration head | `8bf2292` | Both failed, same mechanisms |

Rust checks used `. ~/.bible-atlas-env`, `CARGO_TARGET_DIR=/home/donovan/mut/codex-A-F1-fix-review`, `nice -n 10`, and `-j4`; no concurrent Codex Rust builds. The first integration-head client build collided with my simultaneous contract build's shared client output, so I reran it sequentially and obtained the 715-pass result above; that setup failure is not a product finding. The first scratch asynchronous test incorrectly waited for a render while the state machine was awaiting another response; corrected it to wait on the fake transport's explicit read signal, and the serialization reproduction passes.

I read the author's full workspace/contract/timing/browser evidence recorded on ops; those complete gates were **not independently rerun**. No owner ports were used, no application files/compiled artifacts/fixtures were changed, and no shared locks are held. Worktrees are `/home/donovan/w/A-F1-fix-review` and `/home/donovan/w/A-F1-pages-review`. Logs are `/tmp/codex-8bf2292-*` and `/tmp/codex-f9789f1-*`.

Handoff: keep A-F1/A-F6 in review; close the two named residual categories or record an explicit owner disposition, then re-review the new integrated head including PAGES20. F-67 and the original F-69 scan need not be rediscovered on the next pass.

## Reproduction source

Create an external net10.0 test project referencing `client/BibleAtlas.Client.csproj` and linking `client.Tests/Explore/ServedGraph.cs` from **f9789f1**. Packages: Microsoft.NET.Test.Sdk 17.14.1, bunit 2.11.3, xunit 2.9.3, xunit.runner.visualstudio 3.0.2; nullable/implicit usings enabled and global `using Xunit;`. The exact scratch project is `/tmp/codex-f9789f1-review/Review.csproj`.

```csharp
using BibleAtlas.Client;
using BibleAtlas.Client.Components;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Views;
using BibleAtlas.Client.Tests;
using Bunit;
using Microsoft.Extensions.DependencyInjection;
using System.Reflection;

public sealed class ReviewTests : BunitContext
{
    private static readonly NodeRef Map = ServedGraph.Ref(NodeKind.Map, "Map:test", "Map");
    public ReviewTests() => Services.AddSingleton<IPresenter>(new GraphPresenter());

    [Fact]
    public async Task Two_more_clicks_are_serialized_and_settle_on_the_latest_bounded_window()
    {
        var graph = new PagedGraph { Delay = true };
        var view = Render<FocusView>(p => p.Add(v => v.Node, Resolved.At(graph, ServedGraph.At(Map))).Add(v => v.Surface, Surface.Popover));
        var first = view.Find("[data-testid='popover-children-shows-more']").ClickAsync(new());
        view.WaitForAssertion(() => Assert.Single(graph.Pending));
        await view.Find("[data-testid='popover-children-shows-more']").ClickAsync(new());
        Assert.Single(graph.Pending);
        graph.Pending[0].SetResult(graph.Page(20, 20, EdgeKind.Shows));
        await graph.SecondRead.Task.WaitAsync(TimeSpan.FromSeconds(5));
        Assert.Equal(2, graph.Pending.Count);
        graph.Pending[1].SetResult(graph.Page(40, 20, EdgeKind.Shows));
        await first;
        Assert.Equal(Enumerable.Range(20,40).Select(i=>$"root-a:{i}"), view.FindAll(".focus-child").Select(e=>e.TextContent));
    }

    [Fact]
    public void A_long_reveal_shows_only_the_owner_approved_forty_rows()
    {
        var graph = new PagedGraph();
        var view = Render<FocusView>(p => p.Add(v => v.Node, Resolved.At(graph, ServedGraph.At(Map))).Add(v => v.Surface, Surface.Popover));
        for (var i = 0; i < 14; i++) view.Find("[data-testid='popover-children-shows-more']").Click();
        Assert.Equal(Enumerable.Range(260,40).Select(i=>$"root-a:{i}"), view.FindAll(".focus-child").Select(e=>e.TextContent));
        Assert.Equal("261–300 of 300", view.Find("[data-testid='popover-children-shows-position']").TextContent);
        view.Find("[data-testid='popover-children-shows-collapse']").Click();
        Assert.Equal(Enumerable.Range(240,40).Select(i=>$"root-a:{i}"), view.FindAll(".focus-child").Select(e=>e.TextContent));
    }

    [Fact]
    public void Same_identity_from_a_new_artifact_refreshes_the_presentation()
    {
        var old = ServedGraph.Card(Map.Kind, Map.Id, "Old map");
        var fresh = old with { Label = "New map" };
        var first = Resolved.Node(new ServedGraph().AtRoot("root-a").Serving(old), Map);
        var second = Resolved.Node(new ServedGraph().AtRoot("root-b").Serving(fresh), Map);
        var view = Render<FocusView>(p => p.Add(v => v.Node, first).Add(v => v.Surface, Surface.Popover));
        view.Render(p => p.Add(v => v.Node, second));
        Assert.Equal("New map",view.Find(".focus-title").TextContent);
    }

    [Fact]
    public async Task Person_mentions_do_not_mix_artifact_roots_in_one_window()
    {
        var graph = new PagedGraph();
        var mentions = await Paging.Window(graph, "Person:test", EdgeKind.MentionedIn);
        var view = Render<PersonMentionsList>(p => p.Add(v => v.Mentions, mentions).Add(v=>v.TotalCount,300));
        graph.Root = "root-b";
        await view.Find("[data-testid='person-mentions-more']").ClickAsync(new());
        var labels = view.FindAll(".popover-event-row").Select(e=>e.TextContent).ToList();
        Assert.True(labels.All(s=>s.StartsWith("root-a:")) || labels.All(s=>s.StartsWith("root-b:")), string.Join(",",labels));
    }

    [Fact]
    public async Task Paging_metadata_does_not_grow_with_collection_size()
    {
        var retained = new List<int>();
        foreach (var size in new[]{300,3000,30000})
        {
            Task<Page<int>> Read(int? cursor,int limit)
            {
                var start=cursor??0;
                var end=Math.Min(start+limit,size);
                return Task.FromResult(new Page<int>(Enumerable.Range(start,end-start).ToArray(),end<size?end:null));
            }
            var window=await PageWindow<int>.Opened(Read,Paging.Everything,20);
            for(var i=1;i<size/20;i++) await window.More();
            var history=(IReadOnlyList<int?>)typeof(PageWindow<int>).GetField("_earlier",BindingFlags.Instance|BindingFlags.NonPublic)!.GetValue(window)!;
            retained.Add(history.Count);
        }
        Assert.Single(retained.Distinct());
    }

    private sealed class PagedGraph : IExplorableClient
    {
        public string Root="root-a";
        public bool Delay;
        public List<TaskCompletionSource<EdgePage>> Pending=[];
        public TaskCompletionSource SecondRead = new(TaskCreationOptions.RunContinuationsAsynchronously);
        public Task<NodeRecord> Card(string id) => Task.FromResult(ServedGraph.Card(Map.Kind, Map.Id, Map.Label, new FrontierGroup(EdgeKind.Shows,300)));
        public async Task<ElementPage> Elements(IReadOnlyList<string> ids) => new(elements:[new NodeElement(await Card(Map.Id))],next:null,version:Root);
        public Task<EdgePage> Edges(string id,EdgeKind kind,int? cursor=null,int limit=20)
        {
            if(Delay && cursor is not null)
            {
                var pending=new TaskCompletionSource<EdgePage>(TaskCreationOptions.RunContinuationsAsynchronously);
                Pending.Add(pending);
                if(Pending.Count==2) SecondRead.SetResult();
                return pending.Task;
            }
            return Task.FromResult(Page(cursor??0,limit,kind));
        }
        public EdgePage Page(int start,int n,EdgeKind kind) => ServedGraph.Page(kind,start+n<300?start+n:null,Enumerable.Range(start,n).Select(i=>ServedGraph.Ref(kind==EdgeKind.MentionedIn?NodeKind.TextUnit:NodeKind.Person,$"item:{Root}:{i}",$"{Root}:{i}")).ToArray()) with { Version=Root };
        public Task<TextWindow> Reading(string r,int n,WindowDir d=WindowDir.Onward,Corpus c=Corpus.Bible)=>throw new NotSupportedException();
    }
}

```
