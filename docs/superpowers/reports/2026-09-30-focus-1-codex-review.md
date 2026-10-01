# Codex review: FOCUS-1

Reviewed `678a0d2..3baaeb6`, using the current main-branch PRINCIPLES (including 24a/24b and 27). Verdict: **changes requested**. This is an independent review, not a landing or a declaration that the batch is done.

## Findings

### F-38 — Important: Back cannot restore distinct legacy views of the same node

`client/Explore/RenderedLegacyNodes.cs:23` overwrites the only renderer retained for an Explorable identity. The live event-date link at `PopoverSectionProviders.cs:1057` pushes a `YearNode` with the event's identity. `ExplorerPopover.PushAsync` remembers that YearNode under the event key. Back returns to the event identity, but `Focus` obtains the overwritten YearNode again: the date body remains and Back disappears. F1-4 authorizes the shared identity; it does not make losing the previous presentation correct. The same category includes PassageNode/VerseNode and TimeAndPlaceNode/EventNode aliases.

Reproduction: remember an EventNode, remember its YearNode under the same resolved event, follow DatedBy, Back, then resolve the renderer. Expected original EventNode; actual YearNode. Existing tests only repeat one renderer per identity.

Failed abstraction: graph identity is being used as interaction/presentation identity. Side: client. Proposed closure: the legacy bridge retains the presentation associated with each navigation entry, or alias views become local presentations without impersonating graph hops; enumerate all legacy aliases and test forward/back restoration for each. Do not change the graph's identity to encode a view.

### F-39 — Important: generic frontier failures escape the popover's error handling

`client/Views/FocusView.razor:95` awaits initial requests and `:118` awaits reveal requests without an error boundary or an error result. `Request.Fetch` suppresses stale successful results but propagates exceptions. The parent popover's Arriving/LoadLegacy catches do not enclose child rendering. An offline/500 edge request therefore becomes an unhandled component exception rather than the existing recoverable load message. A stale failed request has the same hole.

Reproduction: resolve a Map card successfully and fail its Shows page with HttpRequestException. Rendering FocusView throws `offline` through the renderer. Existing stale-response tests release successful pages only.

Failed abstraction: asynchronous presentation outcomes are not represented by the request/presentation abstraction. Side: client. Proposed closure: one state/outcome abstraction for initial loads, page fills and reveals, covering current failure, superseded failure, disposal and retry. Both generic and legacy renderers consume that abstraction.

### F-40 — Important: opening inline groups drains the entire collection

`client/Views/FocusView.razor:112` uses `group.Count` as the initial demand for every non-SectionList affordance; `:123` follows all cursors until that count is met, then renders every child. Contains/Shows groups therefore have no bound or reveal control. The whole card waits on Task.WhenAll over the fills. A map or non-legacy container with a large collection incurs collection-sized network, memory and DOM work before first presentation, contrary to 27c/27e. Filtering unsupported links can likewise drain a group in search of enough offered entries (related to existing F-28, but this is the loading cost).

Reproduction: a three-page Shows group requests cursors `[null, 1, 2]` during initial rendering, without a user reveal. Expected only the initial page. Scaling the group scales the opening cost.

Failed abstraction: an affordance does not carry a bounded paging policy. Side: client (with server support if filtering requires it). Proposed closure: all affordances consume bounded pages through one paging abstraction; reveal/virtualization requests more on demand. A law enumerates every affordance and asserts a bounded initial read/render count at large cardinalities.

### F-41 — Important: presentation reuse ignores the surface

`client/Views/FocusView.razor:87` compares only Node. Render a Map on Popover, then change Surface to World: the previously admitted Person link stays visible, although Presentation.Offers rejects it on World. The markup handles change to world while the cached presentation/pages still represent Popover. Current production integration always mounts this view on Popover, so this is an exposed component-contract defect rather than a demonstrated current route transition.

Reproduction: render Map → Shows Person on Popover, set Surface.World with the same node, assert no child link. One disallowed Person button remains.

Failed abstraction: the identity of a presentation omits one of its inputs. Side: client. Proposed closure: one presentation request key includes the surface and resolved node/artifact identity; loading, filtering and reuse all use that key. Test every permitted surface transition for the same node.

### F-42 — Important (14b D.R.Y.): graph identity equality is declared three times

`client/Contract/NodeIdentity.cs:11`, `client/Explore/Explorable.cs:39`, and `client/Explore/Link.cs:8` separately define node equality as Kind+Id and separately hash those fields. Selection uses the comparer, while exploration collapse and link equality carry copies. The current results agree, but a change to identity can split these behaviors. Proposed closure: one typed identity/comparer owns equality and hashing; all wrappers compose over it, with label-insensitivity laws covering selection, links and exploration.

## Verification and review passes

- Independently ran `dotnet test client.Tests`: **572 passed**, 0 failed. Only the two existing xUnit1031 warnings.
- Independently ran `dotnet test client.ContractTests`: **54 passed**, 0 failed.
- Ran four isolated reproduction tests outside the repository: **four expected failures**, one per behavioral finding F-38..F-41. Project `/tmp/codex-A-F1-review/Review.csproj`, output `/tmp/codex-A-F1-repros.log`. Source reproduced below for handoff.
- Application diff comment scan: no added application comments; the added // Arrange/Act/Assert lines are in Rust tests. Existing comments belong to F-12/A-STRIP.
- Read the Rust wire/CLI changes, generated-contract changes, generic exploration and legacy-bridge paths, persistence/state changes, relevant tests, plan and close report. PositionRef separates nodes/edges; shared union schema construction and CLI serialization remove duplicated wire shapes.
- Type-safety pass: closed exploration intents and enum-exhaustiveness enforcement improve the boundary. Existing F-29 (two optional popover opening parameters), F-22 (arrow direction), F-31 (wire/local ids), and F-21 (edge following) remain recorded; this review does not count them again as new findings.
- Category/closure pass: the Back renderer cache fails its claimed restoration category (F-38); the request-series migration covers stale successes but not generic errors (F-39); the paging and reuse abstractions remain open (F-40/F-41). The identity equality duplication is F-42.
- No Rust workspace/timing/full Playwright rerun during this review; the close report records Rust 1,577 and timing 10/10, while the current queue records the later Playwright run. The tracked close report still says Playwright was not run and predates `3baaeb6`; the controller should reconcile its validation record at the final close. Mutation remains deferred by the owner.
- No application files, fixtures, compiled artifacts, golden views or owner processes changed. No locks taken.

## Reproduction source

The scratch project targets net10.0, references this review worktree's `client/BibleAtlas.Client.csproj`, includes `client.Tests/Explore/ServedGraph.cs`, and uses the same bunit/xunit package versions as client.Tests. Run after sourcing `~/.bible-atlas-env`.

```csharp
using BibleAtlas.Client;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.Views;
using BibleAtlas.Client.Tests;
using Bunit;
using Microsoft.Extensions.DependencyInjection;

public sealed class ReviewTests : BunitContext
{
    private static readonly NodeRef Map = ServedGraph.Ref(NodeKind.Map, "Map:test", "Map");
    private static readonly NodeRef Person = ServedGraph.Ref(NodeKind.Person, "Person:test", "Person");

    [Fact]
    public async Task Changing_surface_recomputes_offered_links()
    {
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(Map.Kind, Map.Id, Map.Label, new FrontierGroup(EdgeKind.Shows, 1)))
            .Serving(Map.Id, EdgeKind.Shows, null, ServedGraph.Page(EdgeKind.Shows, null, Person));
        var explorer = new GraphExplorer(graph);
        Services.AddSingleton<IExplorer>(explorer);
        var node = await explorer.Resolve(Map);
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));
        view.Render(p => p.Add(v => v.Surface, Surface.World));
        Assert.Empty(view.FindAll(".focus-child"));
    }

    [Fact]
    public async Task Opening_inline_children_does_not_load_the_entire_collection()
    {
        var graph = new PagedGraph();
        var explorer = new GraphExplorer(graph);
        Services.AddSingleton<IExplorer>(explorer);
        var node = await explorer.Resolve(Map);
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));
        Assert.Equal(new int?[] { null }, graph.Cursors);
    }

    [Fact]
    public async Task A_failed_frontier_request_does_not_escape_rendering()
    {
        var graph = new FailingGraph();
        var explorer = new GraphExplorer(graph);
        Services.AddSingleton<IExplorer>(explorer);
        var node = await explorer.Resolve(Map);
        var failure = Record.Exception(() => Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover)));
        Assert.Null(failure);
    }

    [Fact]
    public void Back_from_an_event_time_restores_the_event_body()
    {
        var identity = ServedGraph.Ref(NodeKind.Event, "Event:test", "Event");
        var node = Resolved.Node(identity);
        var rendered = new RenderedLegacyNodes();
        var original = rendered.For(node);
        var year = new Year("1406 BC", -1406);
        rendered.Remember(node, new YearNode(new TimeRange(year, "1406 BC", year), identity));
        var trail = new Exploration(node, []).Follow(new Step(EdgeKind.DatedBy, node)).Back();
        Assert.Same(original, rendered.For(trail.Current));
    }

    private sealed class PagedGraph : IExplorableClient
    {
        public List<int?> Cursors { get; } = [];
        public Task<NodeCard> Card(string id) => Task.FromResult(ServedGraph.Card(Map.Kind, Map.Id, Map.Label, new FrontierGroup(EdgeKind.Shows, 3)));
        public Task<EdgePage> Edges(string id, EdgeKind kind, int? cursor = null, int limit = IExplorableClient.DefaultPageSize)
        {
            Cursors.Add(cursor);
            var offset = cursor ?? 0;
            return Task.FromResult(ServedGraph.Page(kind, offset == 2 ? null : offset + 1, Person));
        }
        public Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) => throw new NotSupportedException();
    }

    private sealed class FailingGraph : IExplorableClient
    {
        public Task<NodeCard> Card(string id) => Task.FromResult(ServedGraph.Card(Map.Kind, Map.Id, Map.Label, new FrontierGroup(EdgeKind.Shows, 1)));
        public Task<EdgePage> Edges(string id, EdgeKind kind, int? cursor = null, int limit = IExplorableClient.DefaultPageSize) => Task.FromException<EdgePage>(new HttpRequestException("offline"));
        public Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) => throw new NotSupportedException();
    }
}
```
