# FOCUS-1 (the types) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce the FOCUS types — `Explorable`, `Link`, `IFrontier`, `IExplorer`, `Presentation`, `Affordance`, `Focus`, `Exploration`/`ExplorationState`, v2 saves — replace `FocusStack`, and retarget `ExplorerPopover` onto them, while every existing node kind keeps rendering through its legacy providers behind a bridge that FOCUS-2…9 dismantle kind by kind.

**Architecture:** New types live in `client/Explore/`. The popover's trail becomes an `Exploration` of `Link`s (F1/F2/F8); its frontier for any node comes from one `GraphExplorer` over `IExplorableClient` (F4); how each link kind is offered is one table, `Affordances.Of` (§3.4). Legacy `IExplorable` nodes stay as the *rendering* path for kinds not yet migrated: each gains an `Identity`, `Reconstruct` becomes `LegacyNodes.For`, and legacy pushes name their edge kind so even the old path produces true links. Kinds with no legacy providers are presented as the generic `Card` + frontier from this batch on.

**Tech Stack:** .NET 10 Blazor WebAssembly; xUnit; Reqnroll; Playwright; Stryker.NET; generated `NodeKind`/`EdgeKind`/`NodeCard`/`EdgePage`/`NodeRef` from CONTRACT-1.

**Spec:** `docs/superpowers/specs/2026-09-26-focus-exploration-design.md` (§3 types, §4 factory, §9 deletions, §10 rulings R1–R9). **Prerequisites:** CONTRACT-1a/1b complete; FOCUS-0 complete (Anchor nodes for place dates with ids `Anchor:place-{placeId}-{established|destroyed}`; `authored-by`; BoC root + succession); CONTRACT-2 complete (labels, loci, anchors on the wire).

## Global Constraints

- `docs/PRINCIPLES.md` binds. Tests: whole-body assertions, one behaviour per test named as a sentence, `// Arrange` `// Act` `// Assert` only, no magic numbers, newspaper order in every file.
- Names are the spec's: `Explorable`, `Link`, `FrontierGroup`, `IFrontier`, `IExplorer`, `GraphExplorer`, `Presentation.{Sequence,Text,Card,Field}`, `Anchor`, `Locus`, `Affordance.{Arrows,InlineChildren,UpCrumb,MapHatch,SectionList}`, `Affordances.Of`, `Focus`, `Exploration`, `ExplorationState.{Closed,Open}`, intents `Open`, `Follow`, `Back`, `Reset`, `Reseed`; `AtomNames.Exploration = "exploration"`; `SavedExploration`, `SavedNode`, `SavedLink`, storage key `explorations-v2`; `LegacyNodes.For`; `Chip`, `ChipTarget`.
- The Playwright suite is the behaviour gate, with two deliberate exceptions re-expressed under F8 (Task 9): `state-focus.spec.ts` "a Back landing is recorded in the trail" and `saved-explorations.spec.ts` "consecutive-duplicate collapse". Every popover test id in the FOCUS spec §6 survives.
- The popover's ownership mechanics (`OwnershipRegistry`, claim-and-`Reseed`, `Dispose` order) are preserved exactly; only the value type changes.
- Commit per task; push at the end (owner authorization on record).
- All `dotnet` commands from the repo root; Playwright needs API :8000 and client :5000 running.

---

### Task 1: `Explorable`, `Link`, `FrontierGroup`, `Page<T>`, `Locus`, `Anchor`

**Files:**
- Create: `client/Explore/Explorable.cs`, `client/Explore/Link.cs`, `client/Explore/Frontier.cs` (FrontierGroup, Page, IFrontier), `client/Explore/Locus.cs` (Locus, Anchor)
- Test: `client.Tests/Explore/ExplorableTests.cs`

**Interfaces:**
- Consumes: generated `NodeKind`, `EdgeKind`, `NodeRef`, `NodeCard`.
- Produces: `Explorable(NodeKind Kind, string Id, string Label)` with `From(NodeRef)`, `From(NodeCard)`; `Link(EdgeKind Kind, Explorable Target)`; `FrontierGroup(EdgeKind Kind, int Count)`; `Page<T>(IReadOnlyList<T> Items, int? Next)`; `IFrontier { Explorable Node; IReadOnlyList<FrontierGroup> Groups; Task<Page<Link>> Links(EdgeKind kind, int? cursor = null, int limit = DefaultPageSize); }`; `Locus(string Ref, int? Start = null, int? End = null)`; `Anchor(Locus Locus, Link Link)`.

- [ ] **Step 1: Write the failing tests**

`client.Tests/Explore/ExplorableTests.cs`:
```csharp
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests.Explore;

public sealed class ExplorableTests
{
    private const string Genesis1 = "Container:bible-chapter-GEN-1";

    [Fact]
    public void An_explorable_made_from_a_node_ref_carries_kind_id_and_label()
    {
        // Arrange
        var node = new NodeRef { Id = Genesis1, Kind = NodeKind.Container, Label = "Genesis 1" };
        // Act
        var explorable = Explorable.From(node);
        // Assert
        Assert.Equal(new Explorable(NodeKind.Container, Genesis1, "Genesis 1"), explorable);
    }

    [Fact]
    public void An_explorable_made_from_a_node_card_carries_kind_id_and_label()
    {
        // Arrange
        var card = new NodeCard { Id = Genesis1, Kind = NodeKind.Container, Label = "Genesis 1", Provenance = "kjv", Version = "v" };
        // Act
        var explorable = Explorable.From(card);
        // Assert
        Assert.Equal(new Explorable(NodeKind.Container, Genesis1, "Genesis 1"), explorable);
    }

    [Fact]
    public void Two_links_to_the_same_node_under_the_same_kind_are_equal()
    {
        // Arrange
        var genesis2 = new Explorable(NodeKind.Container, "Container:bible-chapter-GEN-2", "Genesis 2");
        // Act
        var (a, b) = (new Link(EdgeKind.FollowsIn, genesis2), new Link(EdgeKind.FollowsIn, genesis2 with { }));
        // Assert
        Assert.Equal(a, b);
    }
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `dotnet test client.Tests --filter ExplorableTests`
Expected: compile errors — `Explorable`, `Link` not found.

- [ ] **Step 3: Implement**

`client/Explore/Explorable.cs`:
```csharp
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record Explorable(NodeKind Kind, string Id, string Label)
{
    public static Explorable From(NodeRef node) => new(node.Kind, node.Id, node.Label);

    public static Explorable From(NodeCard card) => new(card.Kind, card.Id, card.Label);
}
```
`client/Explore/Link.cs`:
```csharp
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record Link(EdgeKind Kind, Explorable Target);
```
`client/Explore/Frontier.cs`:
```csharp
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public interface IFrontier
{
    const int DefaultPageSize = 20;

    Explorable Node { get; }

    IReadOnlyList<FrontierGroup> Groups { get; }

    Task<Page<Link>> Links(EdgeKind kind, int? cursor = null, int limit = DefaultPageSize);
}

public sealed record FrontierGroup(EdgeKind Kind, int Count);

public sealed record Page<T>(IReadOnlyList<T> Items, int? Next);
```
`client/Explore/Locus.cs`:
```csharp
namespace BibleAtlas.Client.Explore;

public sealed record Locus(string Ref, int? Start = null, int? End = null);

public sealed record Anchor(Locus Locus, Link Link);
```

- [ ] **Step 4: Run**

Run: `dotnet test client.Tests --filter ExplorableTests`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add client/Explore/Explorable.cs client/Explore/Link.cs client/Explore/Frontier.cs client/Explore/Locus.cs client.Tests/Explore/ExplorableTests.cs
git commit -m "focus: Explorable, Link, IFrontier, Locus, Anchor -- the graph as the client holds it (spec §3.1)"
```

---

### Task 2: `Exploration`, `ExplorationState`, and the intents

**Files:**
- Create: `client/Explore/Exploration.cs`, `client/State/ExplorationState.cs`
- Modify: `client/Contracts/State.cs` (`AtomNames.Exploration`), `client/AppServices.cs` (register the atom)
- Test: `client.Tests/Explore/ExplorationTests.cs`, `client.Tests/State/ExplorationStateTests.cs`

**Interfaces:**
- Produces: `Exploration(Explorable Start, IReadOnlyList<Link> Links)` with `Current`, `Follow(Link)`, `Back()`, `Breadcrumb`; `ExplorationState = Closed | Open(Exploration)`; intents `Open(Explorable, string? Origin = null)`, `Follow(Link, …)`, `Back(…)`, `Reset(…)`, `Reseed(Exploration, …)` all `IIntent<ExplorationState>`; `AtomNames.Exploration = "exploration"`; `StateAtom<ExplorationState>` registered with initial `new ExplorationState.Closed()`.

- [ ] **Step 1: Write the failing tests**

`client.Tests/Explore/ExplorationTests.cs`:
```csharp
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests.Explore;

public sealed class ExplorationTests
{
    private static readonly Explorable Genesis1 = new(NodeKind.Container, "Container:bible-chapter-GEN-1", "Genesis 1");
    private static readonly Explorable Genesis2 = new(NodeKind.Container, "Container:bible-chapter-GEN-2", "Genesis 2");
    private static readonly Explorable Genesis  = new(NodeKind.Container, "Container:bible-book-GEN", "Genesis");
    private static readonly Link ToGenesis2 = new(EdgeKind.FollowsIn, Genesis2);
    private static readonly Link UpToGenesis = new(EdgeKind.MemberOf, Genesis);

    [Fact]
    public void An_exploration_that_has_gone_nowhere_is_at_its_start()
    {
        // Arrange
        var exploration = new Exploration(Genesis1, []);
        // Act
        var current = exploration.Current;
        // Assert
        Assert.Equal(Genesis1, current);
    }

    [Fact]
    public void Following_a_link_appends_it_and_moves_the_current_node()
    {
        // Arrange
        var exploration = new Exploration(Genesis1, []);
        // Act
        var next = exploration.Follow(ToGenesis2);
        // Assert
        Assert.Equal(new Exploration(Genesis1, [ToGenesis2]), next);
        Assert.Equal(Genesis2, next.Current);
    }

    [Fact]
    public void Going_back_follows_the_dual_of_the_last_link()
    {
        // Arrange
        var exploration = new Exploration(Genesis1, [ToGenesis2]);
        // Act
        var back = exploration.Back();
        // Assert
        Assert.Equal(new Exploration(Genesis1, [ToGenesis2, new Link(EdgeKind.PrecedesIn, Genesis1)]), back);
        Assert.Equal(Genesis1, back.Current);
    }

    [Fact]
    public void Going_back_from_the_start_changes_nothing()
    {
        // Arrange
        var exploration = new Exploration(Genesis1, []);
        // Act
        var back = exploration.Back();
        // Assert
        Assert.Equal(exploration, back);
    }

    [Fact]
    public void The_breadcrumb_collapses_a_link_followed_by_its_dual()
    {
        // Arrange
        var exploration = new Exploration(Genesis1, [ToGenesis2, new Link(EdgeKind.PrecedesIn, Genesis1), UpToGenesis]);
        // Act
        var breadcrumb = exploration.Breadcrumb;
        // Assert
        Assert.Equal(new[] { UpToGenesis }, breadcrumb);
    }

    [Fact]
    public void The_breadcrumb_keeps_a_revisit_that_is_not_an_immediate_return()
    {
        // Arrange
        var toGenesis1 = new Link(EdgeKind.Contains, Genesis1);
        var exploration = new Exploration(Genesis1, [ToGenesis2, UpToGenesis, toGenesis1]);
        // Act
        var breadcrumb = exploration.Breadcrumb;
        // Assert
        Assert.Equal(new[] { ToGenesis2, UpToGenesis, toGenesis1 }, breadcrumb);
    }
}
```

`client.Tests/State/ExplorationStateTests.cs`:
```csharp
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

public sealed class ExplorationStateTests
{
    private static readonly Explorable Genesis1 = new(NodeKind.Container, "Container:bible-chapter-GEN-1", "Genesis 1");
    private static readonly Explorable Genesis2 = new(NodeKind.Container, "Container:bible-chapter-GEN-2", "Genesis 2");
    private static readonly Link ToGenesis2 = new(EdgeKind.FollowsIn, Genesis2);

    [Fact]
    public void Open_on_a_closed_state_starts_an_exploration_at_that_node()
    {
        // Arrange
        ExplorationState closed = new ExplorationState.Closed();
        // Act
        var opened = new Open(Genesis1).Apply(closed);
        // Assert
        Assert.Equal(new ExplorationState.Open(new Exploration(Genesis1, [])), opened);
    }

    [Fact]
    public void Open_on_the_node_already_current_is_a_no_op()
    {
        // Arrange
        ExplorationState open = new ExplorationState.Open(new Exploration(Genesis1, [ToGenesis2]));
        // Act
        var again = new Open(Genesis2).Apply(open);
        // Assert
        Assert.Equal(open, again);
    }

    [Fact]
    public void Open_on_a_different_node_starts_afresh()
    {
        // Arrange
        ExplorationState open = new ExplorationState.Open(new Exploration(Genesis1, [ToGenesis2]));
        // Act
        var fresh = new Open(Genesis1).Apply(open);
        // Assert
        Assert.Equal(new ExplorationState.Open(new Exploration(Genesis1, [])), fresh);
    }

    [Fact]
    public void Follow_appends_and_Back_appends_the_dual()
    {
        // Arrange
        ExplorationState open = new ExplorationState.Open(new Exploration(Genesis1, []));
        // Act
        var followed = new Follow(ToGenesis2).Apply(open);
        var back = new Back().Apply(followed);
        // Assert
        Assert.Equal(new ExplorationState.Open(new Exploration(Genesis1, [ToGenesis2])), followed);
        Assert.Equal(new ExplorationState.Open(new Exploration(Genesis1, [ToGenesis2, new Link(EdgeKind.PrecedesIn, Genesis1)])), back);
    }

    [Fact]
    public void Follow_and_Back_on_a_closed_state_change_nothing()
    {
        // Arrange
        ExplorationState closed = new ExplorationState.Closed();
        // Act
        var results = (new Follow(ToGenesis2).Apply(closed), new Back().Apply(closed));
        // Assert
        Assert.Equal((closed, closed), results);
    }

    [Fact]
    public void Reset_closes_and_Reseed_replaces_verbatim()
    {
        // Arrange
        var snapshot = new Exploration(Genesis2, [new Link(EdgeKind.PrecedesIn, Genesis1)]);
        ExplorationState open = new ExplorationState.Open(new Exploration(Genesis1, [ToGenesis2]));
        // Act
        var results = (new Reset().Apply(open), new Reseed(snapshot).Apply(open));
        // Assert
        Assert.Equal((new ExplorationState.Closed(), new ExplorationState.Open(snapshot)), results);
    }
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `dotnet test client.Tests --filter "ExplorationTests|ExplorationStateTests"`
Expected: compile errors — types not found.

- [ ] **Step 3: Implement**

`client/Explore/Exploration.cs`:
```csharp
namespace BibleAtlas.Client.Explore;

public sealed record Exploration(Explorable Start, IReadOnlyList<Link> Links)
{
    public Explorable Current => Links.Count == 0 ? Start : Links[^1].Target;

    public Exploration Follow(Link link) => this with { Links = [.. Links, link] };

    public Exploration Back() =>
        Links.Count == 0 ? this : Follow(new Link(Links[^1].Kind.Dual(), NodeBefore(Links.Count - 1)));

    public IReadOnlyList<Link> Breadcrumb => Links.Aggregate(new List<Link>(), Collapse);

    private Explorable NodeBefore(int index) => index == 0 ? Start : Links[index - 1].Target;

    private static List<Link> Collapse(List<Link> crumbs, Link link)
    {
        if (crumbs.Count > 0 && crumbs[^1].Kind.Dual() == link.Kind && Returns(crumbs, link))
        {
            crumbs.RemoveAt(crumbs.Count - 1);
        }
        else
        {
            crumbs.Add(link);
        }
        return crumbs;
    }

    private static bool Returns(List<Link> crumbs, Link link) =>
        link.Target == (crumbs.Count >= 2 ? crumbs[^2].Target : null) || crumbs.Count == 1;

    public bool Equals(Exploration? other) =>
        other is not null && Start == other.Start && Links.SequenceEqual(other.Links);

    public override int GetHashCode() => Links.Aggregate(Start.GetHashCode(), HashCode.Combine);
}
```
`Returns` needs the start to be visible for the `crumbs.Count == 1` case; write it as an instance method (`private bool Returns(...)`) comparing against `Start` when the collapse would empty the crumbs, and make `Collapse` an instance method too (`Links.Aggregate(new List<Link>(), Collapse)` works with an instance method group). The tests are the specification; make them pass with the smallest code that keeps newspaper order (public members first).

`client/State/ExplorationState.cs`:
```csharp
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.State;

public abstract record ExplorationState
{
    public sealed record Closed : ExplorationState;

    public sealed record Open(Exploration Exploration) : ExplorationState;
}

public sealed record Open(Explorable Node, string? Origin = null) : IIntent<ExplorationState>
{
    public string Name => "exploration-open";

    public ExplorationState Apply(ExplorationState current) =>
        current is ExplorationState.Open { Exploration.Current: var here } && here == Node
            ? current
            : new ExplorationState.Open(new Exploration(Node, []));
}

public sealed record Follow(Link Link, string? Origin = null) : IIntent<ExplorationState>
{
    public string Name => "exploration-follow";

    public ExplorationState Apply(ExplorationState current) =>
        current is ExplorationState.Open open ? new ExplorationState.Open(open.Exploration.Follow(Link)) : current;
}

public sealed record Back(string? Origin = null) : IIntent<ExplorationState>
{
    public string Name => "exploration-back";

    public ExplorationState Apply(ExplorationState current) =>
        current is ExplorationState.Open open ? new ExplorationState.Open(open.Exploration.Back()) : current;
}

public sealed record Reset(string? Origin = null) : IIntent<ExplorationState>
{
    public string Name => "exploration-reset";

    public ExplorationState Apply(ExplorationState current) => new ExplorationState.Closed();
}

public sealed record Reseed(Exploration Snapshot, string? Origin = null) : IIntent<ExplorationState>
{
    public string Name => "exploration-reseed";

    public ExplorationState Apply(ExplorationState current) => new ExplorationState.Open(Snapshot);
}
```
`IIntent<T>` requires `Apply` to be idempotent (`Contracts/State.cs:9`): `Follow` is not — following the same link twice appends twice. That is the law `ToggleSelection` already breaks deliberately; `Follow` is the same class of intent (a user gesture), and `ConformanceTests` must list it beside `ToggleSelection` in Task 8.

`client/Contracts/State.cs`: add `public const string Exploration = "exploration";` to `AtomNames` and remove `FocusStack` (Task 8 finishes the removal). `client/AppServices.cs:16`: replace the `FocusStack` registration with
```csharp
services.AddSingleton(_ => new StateAtom<ExplorationState>(AtomNames.Exploration, new ExplorationState.Closed()));
```
(keep the `FocusStack` registration until Task 5 retargets the popover; both may coexist for the length of this plan).

- [ ] **Step 4: Run**

Run: `dotnet test client.Tests --filter "ExplorationTests|ExplorationStateTests"`
Expected: 12 passed.

- [ ] **Step 5: Commit**

```bash
git add client/Explore/Exploration.cs client/State/ExplorationState.cs client/Contracts/State.cs client/AppServices.cs client.Tests/Explore/ExplorationTests.cs client.Tests/State/ExplorationStateTests.cs
git commit -m "focus: Exploration is a start plus links; Back follows the dual (F1, F8); ExplorationState atom and intents"
```

---

### Task 3: `Affordances` — the one table; `EdgeSectionRegistry` dies

**Files:**
- Create: `client/Explore/Affordance.cs`
- Delete: `client/Explore/EdgeSectionRegistry.cs`, `client.Tests/Contract/EdgeSectionRegistryTests.cs`
- Modify: `client/Components/PersonMentionsList.razor:78`, `client/Explore/PopoverSectionProviders.cs:350-351,368,1661,1732`
- Test: `client.Tests/Explore/AffordancesTests.cs`

**Interfaces:**
- Produces: `Affordance` (abstract record; `Arrows`, `InlineChildren`, `UpCrumb`, `MapHatch`, `SectionList(SectionStyle Style, int InitialClamp, SectionOrder Order)`), `SectionStyle`/`SectionOrder` enums (moved from the registry, unchanged), `Affordances.Of(EdgeKind) -> Affordance`, `Affordances.DefaultList`.

- [ ] **Step 1: Write the failing tests**

`client.Tests/Explore/AffordancesTests.cs`:
```csharp
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests.Explore;

public sealed class AffordancesTests
{
    [Fact]
    public void Every_declared_kind_has_an_affordance()
    {
        // Arrange
        var kinds = Enum.GetValues<EdgeKind>();
        // Act
        var affordances = kinds.Select(k => Affordances.Of(k)).ToArray();
        // Assert
        Assert.Equal(kinds.Length, affordances.Length);
        Assert.All(affordances, Assert.NotNull);
    }

    [Fact]
    public void The_named_kinds_get_their_named_affordances_and_every_other_kind_is_a_default_list()
    {
        // Arrange
        var expected = new Dictionary<EdgeKind, Affordance>
        {
            [EdgeKind.FollowsIn]     = new Affordance.Arrows(),
            [EdgeKind.PrecedesIn]    = new Affordance.Arrows(),
            [EdgeKind.Contains]      = new Affordance.InlineChildren(),
            [EdgeKind.Shows]         = new Affordance.InlineChildren(),
            [EdgeKind.MemberOf]      = new Affordance.UpCrumb(),
            [EdgeKind.ShownOn]       = new Affordance.UpCrumb(),
            [EdgeKind.Cites]         = new Affordance.SectionList(SectionStyle.Quiet,    InitialClamp: 3,  SectionOrder.VotesRanked),
            [EdgeKind.Mentions]      = new Affordance.SectionList(SectionStyle.Standard, InitialClamp: 50, SectionOrder.Canonical),
            [EdgeKind.MentionedIn]   = new Affordance.SectionList(SectionStyle.Standard, InitialClamp: 12, SectionOrder.Canonical),
            [EdgeKind.CommentedOnBy] = new Affordance.SectionList(SectionStyle.Standard, InitialClamp: 20, SectionOrder.Canonical),
        };
        var others = Enum.GetValues<EdgeKind>().Except(expected.Keys);
        // Act
        var named = expected.Keys.ToDictionary(k => k, Affordances.Of);
        var rest = others.Select(Affordances.Of).Distinct().ToArray();
        // Assert
        Assert.Equal(expected, named);
        Assert.Equal(new[] { Affordances.DefaultList }, rest);
    }
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `dotnet test client.Tests --filter AffordancesTests`
Expected: compile error — `Affordance` not found.

- [ ] **Step 3: Implement; move the two enums; retarget consumers; delete the registry**

`client/Explore/Affordance.cs`:
```csharp
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public abstract record Affordance
{
    public sealed record Arrows : Affordance;

    public sealed record InlineChildren : Affordance;

    public sealed record UpCrumb : Affordance;

    public sealed record SectionList(SectionStyle Style, int InitialClamp, SectionOrder Order) : Affordance;
}

public static class Affordances
{
    public static readonly Affordance DefaultList = new Affordance.SectionList(SectionStyle.Standard, InitialClamp: 20, SectionOrder.Canonical);

    public static Affordance Of(EdgeKind kind) => Named.GetValueOrDefault(kind, DefaultList);

    private static readonly IReadOnlyDictionary<EdgeKind, Affordance> Named = new Dictionary<EdgeKind, Affordance>
    {
        [EdgeKind.FollowsIn]     = new Affordance.Arrows(),
        [EdgeKind.PrecedesIn]    = new Affordance.Arrows(),
        [EdgeKind.Contains]      = new Affordance.InlineChildren(),
        [EdgeKind.Shows]         = new Affordance.InlineChildren(),
        [EdgeKind.MemberOf]      = new Affordance.UpCrumb(),
        [EdgeKind.ShownOn]       = new Affordance.UpCrumb(),
        [EdgeKind.Cites]         = new Affordance.SectionList(SectionStyle.Quiet,    InitialClamp: 3,  SectionOrder.VotesRanked),
        [EdgeKind.Mentions]      = new Affordance.SectionList(SectionStyle.Standard, InitialClamp: 50, SectionOrder.Canonical),
        [EdgeKind.MentionedIn]   = new Affordance.SectionList(SectionStyle.Standard, InitialClamp: 12, SectionOrder.Canonical),
        [EdgeKind.CommentedOnBy] = new Affordance.SectionList(SectionStyle.Standard, InitialClamp: 20, SectionOrder.Canonical),
    };
}

public enum SectionStyle { Standard, Quiet }

public enum SectionOrder { VotesRanked, Canonical }
```
(The two enums move here verbatim from `EdgeSectionRegistry.cs`, including their `why` comments.) Then, at each consumer, replace the registry read with the affordance's `SectionList`:
- `PersonMentionsList.razor:78`: `EdgeSectionRegistry.MentionedIn.EdgeKind` → `EdgeKind.MentionedIn`; its clamp → `((Affordance.SectionList)Affordances.Of(EdgeKind.MentionedIn)).InitialClamp`.
- `PopoverSectionProviders.cs:350-351,368` (`Cites.InitialClamp`), `:1661` (`Mentions`), `:1732` (`MentionedIn`): the same pattern.
`git rm client/Explore/EdgeSectionRegistry.cs client.Tests/Contract/EdgeSectionRegistryTests.cs`; add `Affordances` to `client.Tests/stryker-config.json`'s `mutate` list.

- [ ] **Step 4: Run**

Run: `dotnet build client && dotnet test client.Tests --filter AffordancesTests && npx playwright test tests/ux --grep "person|xrefs|popover-sections"`
Expected: green.

- [ ] **Step 5: Commit**

```bash
git add -A client client.Tests
git commit -m "focus: Affordances.Of -- how each kind of link is offered, one table; EdgeSectionRegistry absorbed"
```

---

### Task 4: `GraphExplorer` — one frontier, one presentation for every kind

**Files:**
- Create: `client/Explore/Presentation.cs`, `client/Explore/Explorer.cs` (IExplorer, GraphExplorer, GraphFrontier)
- Modify: `client/Program.cs` (register `IExplorer`)
- Test: `client.Tests/Explore/GraphExplorerTests.cs`

**Interfaces:**
- Produces: `Presentation` (`Sequence(string Title, IReadOnlyList<Link> Children)`, `Text(Locus Locus, string Body, IReadOnlyList<Anchor> Anchors)`, `Card(string Title, IReadOnlyList<Field> Fields)`, `Field(string Name, string Value, Link? Link = null)`); `IExplorer { Task<Explorable> Resolve(NodeKind kind, string id); Task<IFrontier> Explore(Explorable node); Task<Presentation> Present(Explorable node); }`; `GraphExplorer(IExplorableClient graph)`.
- In this batch `Present` returns the generic `Card(label, [("Kind", kind), ("Provenance", provenance)])` for **every** kind; FOCUS-2…7 replace it kind by kind.

- [ ] **Step 1: Write the failing tests**

`client.Tests/Explore/GraphExplorerTests.cs` — a fake `IExplorableClient` returning fixed generated records (no network):
```csharp
using BibleAtlas.Client;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests.Explore;

public sealed class GraphExplorerTests
{
    private const string Genesis1 = "Container:bible-chapter-GEN-1";
    private const string Genesis2 = "Container:bible-chapter-GEN-2";
    private const int VersesInGenesis1 = 31;

    [Fact]
    public async Task Exploring_a_node_lists_its_frontier_groups_from_the_card()
    {
        // Arrange
        var explorer = new GraphExplorer(new FakeGraph());
        // Act
        var frontier = await explorer.Explore(new Explorable(NodeKind.Container, Genesis1, "Genesis 1"));
        // Assert
        Assert.Equal(new[]
        {
            new FrontierGroup(EdgeKind.Contains, VersesInGenesis1),
            new FrontierGroup(EdgeKind.MemberOf, 1),
            new FrontierGroup(EdgeKind.FollowsIn, 1),
        }, frontier.Groups);
    }

    [Fact]
    public async Task A_frontier_group_pages_to_links_in_server_order()
    {
        // Arrange
        var frontier = await new GraphExplorer(new FakeGraph()).Explore(new Explorable(NodeKind.Container, Genesis1, "Genesis 1"));
        // Act
        var page = await frontier.Links(EdgeKind.FollowsIn);
        // Assert
        Assert.Equal(new Page<Link>([new Link(EdgeKind.FollowsIn, new Explorable(NodeKind.Container, Genesis2, "Genesis 2"))], Next: null), page);
    }

    [Fact]
    public async Task Presenting_any_node_yields_the_generic_card_until_its_kind_is_migrated()
    {
        // Arrange
        var explorer = new GraphExplorer(new FakeGraph());
        // Act
        var presentation = await explorer.Present(new Explorable(NodeKind.Container, Genesis1, "Genesis 1"));
        // Assert
        Assert.Equal(new Presentation.Card("Genesis 1", [new Presentation.Field("Kind", "Container"), new Presentation.Field("Provenance", "kjv")]), presentation);
    }

    private sealed class FakeGraph : IExplorableClient
    {
        public Task<NodeCard> Card(string id) => Task.FromResult(new NodeCard
        {
            Id = id, Kind = NodeKind.Container, Label = "Genesis 1", Provenance = "kjv", Version = "v",
            EdgeSummary = [ new() { Kind = EdgeKind.Contains, Count = VersesInGenesis1 }, new() { Kind = EdgeKind.MemberOf, Count = 1 }, new() { Kind = EdgeKind.FollowsIn, Count = 1 } ],
        });

        public Task<EdgePage> Edges(string id, EdgeKind kind, int? cursor = null, int limit = 20) => Task.FromResult(new EdgePage
        {
            Kind = kind, Version = "v", Next = null,
            Entries = [ new() { Edge = "e", Node = new NodeRef { Id = Genesis2, Kind = NodeKind.Container, Label = "Genesis 2" } } ],
        });

        public Task<TextWindow> Reading(string fromRef, int n, string dir = "onward", string corpus = "bible") => throw new NotSupportedException();
    }
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `dotnet test client.Tests --filter GraphExplorerTests`
Expected: compile errors.

- [ ] **Step 3: Implement**

`client/Explore/Presentation.cs`:
```csharp
namespace BibleAtlas.Client.Explore;

public abstract record Presentation
{
    public sealed record Sequence(string Title, IReadOnlyList<Link> Children) : Presentation;

    public sealed record Text(Locus Locus, string Body, IReadOnlyList<Anchor> Anchors) : Presentation;

    public sealed record Card(string Title, IReadOnlyList<Field> Fields) : Presentation
    {
        public bool Equals(Card? other) => other is not null && Title == other.Title && Fields.SequenceEqual(other.Fields);

        public override int GetHashCode() => Fields.Aggregate(Title.GetHashCode(), HashCode.Combine);
    }

    public sealed record Field(string Name, string Value, Link? Link = null);
}
```
(`Sequence` and `Text` get the same list-aware equality the moment a test needs it — FOCUS-2/3.)

`client/Explore/Explorer.cs`:
```csharp
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public interface IExplorer
{
    Task<Explorable> Resolve(NodeKind kind, string id);

    Task<IFrontier> Explore(Explorable node);

    Task<Presentation> Present(Explorable node);
}

public sealed class GraphExplorer(IExplorableClient graph) : IExplorer
{
    public async Task<Explorable> Resolve(NodeKind kind, string id) => Explorable.From(await graph.Card(id));

    public async Task<IFrontier> Explore(Explorable node)
    {
        var card = await graph.Card(node.Id);
        return new GraphFrontier(node, card.EdgeSummary.Select(e => new FrontierGroup(e.Kind, e.Count)).ToArray(), graph);
    }

    public async Task<Presentation> Present(Explorable node)
    {
        var card = await graph.Card(node.Id);
        return new Presentation.Card(card.Label, [new Presentation.Field("Kind", card.Kind.ToString()), new Presentation.Field("Provenance", card.Provenance)]);
    }
}

public sealed class GraphFrontier(Explorable node, IReadOnlyList<FrontierGroup> groups, IExplorableClient graph) : IFrontier
{
    public Explorable Node => node;

    public IReadOnlyList<FrontierGroup> Groups => groups;

    public async Task<Page<Link>> Links(EdgeKind kind, int? cursor = null, int limit = IFrontier.DefaultPageSize)
    {
        var page = await graph.Edges(node.Id, kind, cursor, limit);
        return new Page<Link>(page.Entries.Select(e => new Link(kind, Explorable.From(e.Node))).ToArray(), page.Next);
    }
}
```
`Page<T>` needs list-aware equality for the test: add `Equals`/`GetHashCode` over `Items` and `Next` in `Frontier.cs` (as `Card` does). `client/Program.cs`: `builder.Services.AddSingleton<IExplorer>(sp => new GraphExplorer(sp.GetRequiredService<IExplorableClient>()));`.

- [ ] **Step 4: Run**

Run: `dotnet test client.Tests --filter GraphExplorerTests`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add client/Explore/Presentation.cs client/Explore/Explorer.cs client/Explore/Frontier.cs client/Program.cs client.Tests/Explore/GraphExplorerTests.cs
git commit -m "focus: GraphExplorer -- one Explore and one Present for every kind; the generic Card until a kind is migrated"
```

---

### Task 5: The strangler bridge — `IExplorable.Identity`, `LegacyNodes.For`, pushes that name their kind, `Chip`

**Files:**
- Modify: `client/Explore/IExplorable.cs`, the 14 `client/Explore/*Node.cs`, `client/Explore/ExplorationDescriptor.cs` → rename to `client/Explore/LegacyNodes.cs`, `client/Explore/ExplorationTarget.cs` → `client/Explore/Chip.cs`, `client/Explore/PopoverSections.cs` (`PushAsync(node, via)`), `client/Explore/PopoverSectionProviders.cs` (every `PushAsync` call)
- Test: `client.Tests/Explore/IdentityTests.cs`

**Interfaces:**
- Produces: `IExplorable.Identity : Explorable`; `LegacyNodes.For(Explorable, AtlasClient, IExplorableClient) -> Task<IExplorable>`; `Chip(string Label, string TestId, ChipTarget Target)`, `ChipTarget.{Push, NavigateWorld, NavigateReader}`; `IPopoverSectionContext.PushAsync(IExplorable node, EdgeKind via)`.

- [ ] **Step 1: Write the failing test**

`client.Tests/Explore/IdentityTests.cs`:
```csharp
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests.Explore;

public sealed class IdentityTests
{
    [Fact]
    public void Every_legacy_node_names_the_graph_node_it_stands_for()
    {
        // Arrange
        var nodes = new (IExplorable Node, Explorable Expected)[]
        {
            (new VerseNode("GEN.1.1"),                                         new(NodeKind.TextUnit,  "text-unit:GEN.1.1",              "GEN.1.1")),
            (new ConcordUnitNode("BoC 7.2.1", "text"),                         new(NodeKind.TextUnit,  "text-unit:BoC 7.2.1",            "BoC 7.2.1")),
            (new ChapterNode("GEN", 1, 50),                                    new(NodeKind.Container, "Container:bible-chapter-GEN-1",  "Genesis 1")),
            (new BookNode("GEN"),                                              new(NodeKind.Container, "Container:bible-book-GEN",       "Genesis")),
            (new PassageNode("GEN.1.1-5", "text"),                             new(NodeKind.Container, "Container:bible-passage-GEN.1.1-5", "GEN.1.1-5")),
            (new PlaceNode("hazor_1", "Hazor"),                                new(NodeKind.Place,     "hazor_1",                        "Hazor")),
            (new PersonNode("Person:moses", "Moses"),                          new(NodeKind.Person,    "Person:moses",                   "Moses")),
            (new EventNode("ab_ur", "Abram in Ur"),                            new(NodeKind.Event,     "ab_ur",                          "Abram in Ur")),
            (new CatechismNode("commandment-1", "The First Commandment"),      new(NodeKind.CatechismItem, "commandment-1",              "The First Commandment")),
            (new CommentaryItemNode("kretzmann/0.1.0", "Genesis 1:1"),         new(NodeKind.CommentaryItem, "kretzmann/0.1.0",           "Genesis 1:1")),
            (new AuthorNode("GEN"),                                            new(NodeKind.Container, "Container:bible-book-GEN",       "Genesis")),
            (new TimeAndPlaceNode("hazor_1", "Hazor", "ab_ur", when, "Abram in Ur", []), new(NodeKind.Event, "ab_ur",                  "Abram in Ur")),
            (new YearNode("samaria_1022", "Established", when, [], null, eventId: "theo-176"), new(NodeKind.Event, "theo-176",   "Established")),
            (new PolityDeltaNode("egypt", "Egypt", "fall", -1500, -1400, null, [], null), new(NodeKind.Polity, "egypt",                 "Egypt")),
        };
        // Act
        var identities = nodes.Select(n => n.Node.Identity).ToArray();
        // Assert
        Assert.Equal(nodes.Select(n => n.Expected).ToArray(), identities);
    }

    private static readonly TimeRange when = new() { From = new Year { Value = -2000, Label = "2000 BC" }, To = new Year { Value = -2000, Label = "2000 BC" }, Label = "2000 BC" };
}
```
The exact id strings for chapter/book/passage containers and text units are the server's (`graph_wire::encode_node_id`). Confirm each against the live API (`/api/contents/bible`, a `/api/node/{id}` call) and correct the expected literals *before* writing the implementation — the test documents the truth, it does not invent it. Constructor signatures are those in the FOCUS grounding table (§B), plus `YearNode`'s new `eventId` (FOCUS-0 §7 item 4 puts `event_id` on the claim and CONTRACT-2's `PlaceDetail.established/destroyed` carries it); adjust to the current signatures if CONTRACT-2 changed a parameter type. A `YearNode` whose claim has no event has no identity and is not opened as a popover root (it renders as a dated field on the place's card).

- [ ] **Step 2: Run to verify it fails**

Run: `dotnet test client.Tests --filter IdentityTests`
Expected: compile error — `Identity` not a member of `IExplorable`.

- [ ] **Step 3: Implement**

`IExplorable.cs`: add `Explorable Identity { get; }`; rename `ExploreAsync`'s return to `Task<IReadOnlyList<Chip>>`. Each Node class implements `Identity` as one expression (the table above). `git mv client/Explore/ExplorationTarget.cs client/Explore/Chip.cs` and rename `Exploration` → `Chip`, `ChipTestId` → `TestId`, `ExplorationTarget` → `ChipTarget` across the 14 nodes and `ExplorerPopover.razor` (compiler-driven; `PopoverChromeConformanceTests`' string scan looks for `new Exploration(` — update its pattern to `new Chip(`).

`git mv client/Explore/ExplorationDescriptor.cs client/Explore/LegacyNodes.cs`:
```csharp
public static class LegacyNodes
{
    public static Task<IExplorable> For(Explorable node, AtlasClient api, IExplorableClient graph) => node.Kind switch
    {
        NodeKind.TextUnit when node.Id.StartsWith("text-unit:BoC ") => ConcordUnit(node, graph),
        NodeKind.TextUnit => Task.FromResult<IExplorable>(new VerseNode(node.Id["text-unit:".Length..])),
        NodeKind.Container when node.Id.StartsWith("Container:bible-chapter-") => Task.FromResult<IExplorable>(Chapter(node)),
        NodeKind.Container when node.Id.StartsWith("Container:bible-book-") => Task.FromResult<IExplorable>(new BookNode(node.Id["Container:bible-book-".Length..])),
        NodeKind.Container when node.Id.StartsWith("Container:bible-passage-") => Passage(node, api),
        NodeKind.Place => Task.FromResult<IExplorable>(new PlaceNode(node.Id, node.Label)),
        NodeKind.Person => Task.FromResult<IExplorable>(new PersonNode(node.Id, node.Label)),
        NodeKind.Event => Task.FromResult<IExplorable>(new EventNode(node.Id, node.Label)),
        NodeKind.CatechismItem => Task.FromResult<IExplorable>(new CatechismNode(node.Id, node.Label)),
        NodeKind.CommentaryItem => Task.FromResult<IExplorable>(new CommentaryItemNode(node.Id, node.Label)),
        NodeKind.Anchor => Anchor(node, api),
        _ => throw new NotSupportedException($"{node.Kind} has no legacy node; it is presented by GraphExplorer"),
    };
    // Chapter/Passage/ConcordUnit/Anchor bodies are the corresponding `Reconstruct` cases moved verbatim, keyed by the id instead of the old Key.
}
```
`ExplorationDescriptor` itself (the record) is deleted; `Capture` is gone (`Identity` replaces it). Any caller of `Capture` becomes `.Identity`; callers of `Reconstruct` become `LegacyNodes.For`. `IPopoverSectionContext.PushAsync(IExplorable node)` → `PushAsync(IExplorable node, EdgeKind via)`; every provider call site passes its kind from the grounding table (§A): `CrossRefsSection` → `EdgeKind.Cites`; `CatechismSeamSection`/`CatechismInConcordSection`/`ConcordSmallCatechismSection` → `EdgeKind.CatechismLink`; `PlaceDatesSection` → `EdgeKind.DatedBy`; `PlaceEventsSection` → `EdgeKind.SiteOf`; `VerseEventMembershipSection` → `EdgeKind.Attests`; `VersePassageMembershipSection` → `EdgeKind.MemberOf`; `EventDateAndPlacesSection` → `EdgeKind.LocatedAt`; `EventWitnessesSection` → `EdgeKind.AttestedIn`; `EventMentionsSection` → `EdgeKind.MentionedIn`; `EventAnaloguesSection` → `EdgeKind.AnalogousTo`; `VerseParallelsSection` → `EdgeKind.Parallel`; `EventChronologySection` → `EdgeKind.FollowsIn`/`PrecedesIn`; `PolityDeltaEventSection` → `EdgeKind.DatedBy`; `VersePersonsSection` → `EdgeKind.Mentions`; `PersonCardAndMentionsSection` → `EdgeKind.MentionedIn`; `PersonEventsSection` → `EdgeKind.ParticipatesIn`; `PersonFamilySection` → `ParentOf`/`ChildOf`/`PartnerOf` per row; `CatechismScripturesSection`/`PolityDeltaScripturesSection` → `EdgeKind.Cites`/`JustifiedBy`; `YearFrontierSection` → `EdgeKind.Dates`; `ChapterCardSection` (verses) → `EdgeKind.Contains`. Chips: `ChipTarget.Push` gains a `Via` (`BookNode` push from a verse → `EdgeKind.MemberOf`).

- [ ] **Step 4: Run**

Run: `dotnet build client && dotnet test client.Tests --filter "IdentityTests|PopoverChromeConformanceTests"`
Expected: green (the popover still compiles against `Chip`; Task 6 retargets it).

- [ ] **Step 5: Commit**

```bash
git add -A client client.Tests
git commit -m "focus: every legacy node knows the graph node it stands for; Reconstruct becomes LegacyNodes.For; pushes name their edge kind; Exploration(chip) renamed Chip"
```

---

### Task 6: Retarget `ExplorerPopover` onto `ExplorationState`

**Files:**
- Modify: `client/Components/ExplorerPopover.razor`; its hosts `client/Pages/{Reader,World,Concord,Kretzmann}.razor`, `client/Layout/MainLayout.razor` (add `HostView`)
- Test: `client.Tests/State/FocusStackOwnershipHandoffTests.cs` → renamed `ExplorationOwnershipHandoffTests.cs` (retyped, same five laws)

**Interfaces:**
- Consumes: Tasks 2–5.
- Produces: `ExplorerPopover` parameters `Root: IExplorable` (unchanged for now), `SeedExploration: Exploration?` (replaces `SeedStack`), `HostView: string`; internal `Follow(Link)`; breadcrumb from `Exploration.Breadcrumb`; hatches from `ViewRegistry.Get(HostView).EscapeHatches`.

- [ ] **Step 1: Retype the ownership tests first (they are the failing tests)**

In `FocusStackOwnershipHandoffTests.cs` → `ExplorationOwnershipHandoffTests.cs`: `StateAtom<FocusStack>` → `StateAtom<ExplorationState>`; `FocusStack.Empty` → `new ExplorationState.Closed()`; `FakePopover.Open(IExplorable)` → `Open(Explorable)` dispatching `new Open(node)`; `Reseed(snapshot)` with an `Exploration` snapshot; assertions compare whole `ExplorationState` values. Run: `dotnet test client.Tests --filter ExplorationOwnershipHandoffTests` → compile errors until the popover is retargeted (the fake mirrors the popover's plumbing and is updated in lockstep).

- [ ] **Step 2: Retarget the popover**

In `ExplorerPopover.razor`:
- injections: `StateAtom<FocusStack> FocusStackAtom` → `StateAtom<ExplorationState> ExplorationAtom`; add `IExplorer Explorer`, `ViewRegistry Registry`.
- fields: `_frozenSnapshot: ExplorationState = new ExplorationState.Closed()`; `Current` → `_currentNode: IExplorable` (the legacy rendering node) alongside `CurrentIdentity => (FocusValue as ExplorationState.Open)!.Exploration.Current`.
- `ApplyLocally(IIntent<ExplorationState>)` unchanged in shape; `OnFocusStackChanged` → `OnExplorationChanged` with the same claim-and-`Reseed` logic (`Value is ExplorationState.Closed && _frozenSnapshot is ExplorationState.Open`).
- `OnInitializedAsync`: `ApplyLocally(SeedExploration is { } seed ? new Reseed(seed) : new Open(Root.Identity)); _currentNode = Root; await LoadCurrent();`.
- `LoadCurrent()`: `_currentNode = await LegacyNodes.For(CurrentIdentity, Atlas, GraphClient)` **only when** `PopoverSectionRegistry.Providers.Any(p => p.AppliesTo(...))` needs a legacy node — i.e. resolve the legacy node first (catching `NotSupportedException`); if there is none, render the new path: `_focus = new Focus(CurrentIdentity, await Explorer.Explore(CurrentIdentity), await Explorer.Present(CurrentIdentity), Affordances.Of, Registry.Get(HostView).EscapeHatches)` and a `FocusView` component (Task 7) renders it. The legacy provider path is otherwise unchanged.
- `PushAsync(node, via)` → `ApplyLocally(new Follow(new Link(via, node.Identity))); await LoadCurrent();`; chip `Push` → the same with the chip's `Via`; `Back()` → `ApplyLocally(new Back())` when `Links.Count > 0`; breadcrumb button shown when `Exploration.Breadcrumb.Count > 0`; `SaveExploration()` → `SavedExplorations.Save(exploration)` (Task 8 retypes the service).
- `Dispose()` order unchanged.
- Hosts: Reader passes `HostView="@ViewNames.Reader"`, World `ViewNames.World`, Kretzmann `ViewNames.Kretzmann`, Concord `ViewNames.Concord`, MainLayout the current view (from `NavigationManager` → `ViewRegistry` route map, or `ViewNames.Reader` as the layout's default) and `SeedExploration` instead of `SeedStack`.

- [ ] **Step 3: Verify**

Run: `dotnet build client && dotnet test client.Tests && npx playwright test tests/ux --grep "popover|state-focus|saved-explorations|split-view|person-card|event"`
Expected: `client.Tests` green; Playwright green **except** the two F8 specs, which fail on the trail assertions Task 9 re-expresses — record their names in the ledger.

- [ ] **Step 4: Commit**

```bash
git add -A client client.Tests
git commit -m "focus: ExplorerPopover explores an Exploration -- Follow/Back over links, breadcrumb collapses returns, hatches from the host view (F1, F8)"
```

---

### Task 7: `FocusView` — rendering a `Focus` through its affordances

**Files:**
- Create: `client/Components/FocusView.razor`, `client/Components/FrontierGroupList.razor`
- Test: `client.Tests/Components/FocusViewTests.cs` (bUnit is not in the repo; test the pure parts — the grouping and ordering — as a static function in `FocusView.razor.cs`)

**Interfaces:**
- Produces: `FocusView` parameters `Focus Focus`, `EventCallback<Link> OnFollow`, `EventCallback<Link> OnToggleSelect`; static `FocusView.Sections(Focus) -> IReadOnlyList<(FrontierGroup Group, Affordance Affordance)>` in the order: `Arrows`, `UpCrumb`, `InlineChildren`, `MapHatch`, then `SectionList`s in `Groups` order.

- [ ] **Step 1: Write the failing test**

```csharp
using BibleAtlas.Client.Components;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests.Components;

public sealed class FocusViewTests
{
    [Fact]
    public void Sections_put_navigation_affordances_first_and_lists_after_in_frontier_order()
    {
        // Arrange
        var groups = new[] { new FrontierGroup(EdgeKind.Mentions, 4), new FrontierGroup(EdgeKind.FollowsIn, 1), new FrontierGroup(EdgeKind.Cites, 2), new FrontierGroup(EdgeKind.MemberOf, 1) };
        var focus = new Focus(new Explorable(NodeKind.TextUnit, "text-unit:GEN.1.1", "GEN.1.1"), new StaticFrontier(groups), new Presentation.Card("GEN.1.1", []), Affordances.Of, []);
        // Act
        var sections = FocusView.Sections(focus);
        // Assert
        Assert.Equal(new[]
        {
            (groups[1], Affordances.Of(EdgeKind.FollowsIn)),
            (groups[3], Affordances.Of(EdgeKind.MemberOf)),
            (groups[0], Affordances.Of(EdgeKind.Mentions)),
            (groups[2], Affordances.Of(EdgeKind.Cites)),
        }, sections);
    }

    private sealed class StaticFrontier(IReadOnlyList<FrontierGroup> groups) : IFrontier
    {
        public Explorable Node => throw new NotSupportedException();
        public IReadOnlyList<FrontierGroup> Groups => groups;
        public Task<Page<Link>> Links(EdgeKind kind, int? cursor = null, int limit = IFrontier.DefaultPageSize) => throw new NotSupportedException();
    }
}
```
`Focus` is declared in this task: `client/Explore/Focus.cs`: `public sealed record Focus(Explorable Node, IFrontier Frontier, Presentation Presentation, Func<EdgeKind, Affordance> Affordances, IReadOnlyList<IEscapeHatch> Hatches);`.

- [ ] **Step 2: Run to verify it fails**

Run: `dotnet test client.Tests --filter FocusViewTests` → compile errors.

- [ ] **Step 3: Implement**

`client/Components/FocusView.razor.cs`:
```csharp
public partial class FocusView
{
    private static readonly IReadOnlyList<Type> NavigationFirst = [typeof(Affordance.Arrows), typeof(Affordance.UpCrumb), typeof(Affordance.InlineChildren), typeof(Affordance.MapHatch)];

    public static IReadOnlyList<(FrontierGroup Group, Affordance Affordance)> Sections(Focus focus) =>
        focus.Frontier.Groups
            .Select(g => (Group: g, Affordance: focus.Affordances(g.Kind)))
            .OrderBy(s => Rank(s.Affordance))
            .ToArray();

    private static int Rank(Affordance affordance)
    {
        var index = NavigationFirst.IndexOf(affordance.GetType());
        return index < 0 ? NavigationFirst.Count : index;
    }
}
```
(`OrderBy` is stable, so lists keep frontier order.) `FocusView.razor` renders: the `Presentation` (`Card` → a `<dl>` of fields with `data-testid="popover-section-card"`; `Geography` → the same card summary plus the window label in the popover — the World view is its real renderer from FOCUS-6; `Sequence`/`Text` arrive in FOCUS-2/3), then each section by affordance: `Arrows` → two buttons `data-testid="popover-prev"`/`popover-next"` that page `Links(kind)` and `OnFollow`; `UpCrumb` → `data-testid="popover-up"`; `SectionList` → `<FrontierGroupList>` (`data-testid="popover-section-{kind.Label()}"`) paging `Links(kind)` with the list's `InitialClamp`, each entry a button `OnFollow(link)`. The first `located-at`/`site-of` link in a list additionally carries `data-testid="popover-chip-map"` and the first `member-of` link `popover-chip-context`, so the existing specs keep their handles. `ExplorerPopover` renders `<FocusView Focus="_focus" OnFollow="FollowAsync" OnToggleSelect="ToggleSelectAsync" />` on the new path from Task 6.

**Home surfaces (spec §3.3):** `client/Explore/HomeSurface.cs` — `public static class HomeSurfaces { public static string Of(Explorable node); }` — total over `NodeKind`: `TextUnit`/`Container` → by corpus from the id (`text-unit:BoC …`/`Container:concord-…` → `ViewNames.Concord`; `kretzmann/…` → `ViewNames.Kretzmann`; else `ViewNames.Reader`), `Map`/`Place`/`Polity`/`Era` → `ViewNames.World`, everything else → the current surface (popover only). Test `HomeSurfacesTests.Every_kind_has_a_home_surface` (whole table). `ExplorerPopover.FollowAsync(link)`: `ApplyLocally(new Follow(link))`; if `HomeSurfaces.Of(link.Target) != HostView` and it is not "popover only", `Nav.NavigateTo(RouteFor(link.Target))` — the routes the old `NavigateReader`/`NavigateWorld` chips built (`/read/{book}/{ch}#v{n}`, `/concord?ref=`, `/world?place=…&from=&to=`), with the exploration carried in the atom; otherwise `await LoadCurrent()`. The `ChipTarget.NavigateWorld`/`NavigateReader` cases and `MapFocusHatch` are deleted in this task; every remaining chip is a `Push` with a `Via`.

- [ ] **Step 4: Verify with a kind that has no legacy providers**

Run: `dotnet test client.Tests --filter FocusViewTests`, then open the app and `Open` a `LexiconEntry` (reach one via a verse's `words` group once CONTRACT-2 serves anchors; before that, via the browser console: navigate to `/read/GEN/1` and dispatch through the popover on `Person:` → `participates-in` → an Event → its frontier lists every group, including kinds like `Narrative` via `follows-in`'s narrative). Expected: the generic card and its frontier groups render with the ids above.

- [ ] **Step 5: Commit**

```bash
git add -A client client.Tests
git commit -m "focus: FocusView renders a Focus through its affordances; every kind without legacy providers is explorable"
```

---

### Task 8: Saves v2, selection v2, and the v1 translations

**Files:**
- Modify: `client/SavedExplorationsService.cs`, `client/Components/ExplorationListItem.razor`, `client/Layout/MainLayout.razor` (`Continue`), `client/State/Selection.cs`, `client/AppServices.cs` (selection atom), `client/Components/SelectionTray.razor` and other selection consumers (compiler-driven)
- Create: `client/Explore/LegacySaves.cs` (v1 → v2 translation)
- Test: `client.Tests/SavedExplorationsTests.cs`, `client.Tests/Explore/LegacySavesTests.cs`

**Interfaces:**
- Produces: `SavedExploration(Guid Id, string Name, DateTimeOffset CreatedUtc, SavedNode Start, IReadOnlyList<SavedLink> Links)`, `SavedNode(NodeKind Kind, string Id, string Label)`, `SavedLink(EdgeKind Kind, SavedNode Target)`; `SavedExplorationsService.Save(Exploration)`, `.Items`, `.Rename`, `.Delete`, `.Dropped` (count of untranslatable v1 steps, reported once); storage `explorations-v2`; `LegacySaves.Translate(JsonElement v1Item) -> (SavedExploration? Saved, int Dropped)`; selection atom `StateAtom<IReadOnlyList<Explorable>>` under `selection-v2`.

- [ ] **Step 1: Write the failing tests**

`client.Tests/Explore/LegacySavesTests.cs`:
```csharp
using System.Text.Json;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests.Explore;

public sealed class LegacySavesTests
{
    private const string V1 = """
        {"id":"seed1","name":"GEN.1.1 → Genesis","createdUtc":"2026-09-01T00:00:00+00:00",
         "nodes":[{"kind":"Verse","key":"GEN.1.1","title":"GEN.1.1","isGeneralKind":false},
                  {"kind":"Book","key":"GEN","title":"Genesis","isGeneralKind":false},
                  {"kind":"TimeAndPlace","key":"hazor_1|ab_ur","title":"Abram in Ur","isGeneralKind":false},
                  {"kind":"Author","key":"GEN","title":"Moses","isGeneralKind":false}]}
        """;

    [Fact]
    public void A_v1_save_becomes_a_start_and_links_of_unknown_kind_dropping_what_has_no_node()
    {
        // Arrange
        var v1 = JsonDocument.Parse(V1).RootElement;
        // Act
        var (saved, dropped) = LegacySaves.Translate(v1);
        // Assert
        Assert.Equal(
            (new SavedExploration(Guid.Empty, "GEN.1.1 → Genesis", DateTimeOffset.Parse("2026-09-01T00:00:00+00:00"),
                 new SavedNode(NodeKind.TextUnit, "text-unit:GEN.1.1", "GEN.1.1"),
                 [ new SavedLink(EdgeKind.MemberOf, new SavedNode(NodeKind.Container, "Container:bible-book-GEN", "Genesis")),
                   new SavedLink(EdgeKind.SiteOf,   new SavedNode(NodeKind.Event, "ab_ur", "Abram in Ur")) ]),
             1),
            (saved! with { Id = Guid.Empty }, dropped));
    }
}
```
The v1 → v2 mapping (spec §3.6, R2): Verse → `TextUnit text-unit:{key}`; ConcordUnit → `TextUnit text-unit:{key}`; Chapter `{BOOK}.{n}` → `Container:bible-chapter-{BOOK}-{n}`; Book → `Container:bible-book-{key}`; Passage → `Container:bible-passage-{key}`; Place/Person/Event/Catechism/CommentaryItem → same id with the new kind; TimeAndPlace `{placeId}|{eventId}` → `Event {eventId}`; Year, Author, PolityDelta → dropped (a Year step's event id is not recoverable from its v1 key). A v1 trail has no kinds between cells; the translated link kind is `MemberOf` when the target is a Container the previous node is inside, `DatedBy` for an Anchor, `SiteOf` for an Event from a Place, and `Contains`/`Mentions`/`Cites` cannot be known — use `EdgeKind.Contains` for TextUnit targets from Containers and `EdgeKind.Mentions` for Place/Person targets from TextUnits; anything else `EdgeKind.MemberOf`. The rule is documented by this test's expected value and by `LegacySaves` itself; it is deleted in FOCUS-9.

`client.Tests/SavedExplorationsTests.cs`: `Save_names_the_exploration_after_its_start_and_end`, `Save_stores_under_explorations_v2`, `Items_include_translated_v1_saves_and_report_dropped_steps_once` — over a fake `IJSInProcessRuntime` capturing `localStorage.setItem` (write the fake in the test file; the service takes `IJSInProcessRuntime` today).

- [ ] **Step 2: Run to verify they fail** — compile errors.

- [ ] **Step 3: Implement**

`SavedExplorationsService`: read `explorations-v2` (if absent, read `explorations-v1`, translate each item with `LegacySaves.Translate`, sum `Dropped`, write v2, leave v1 in place); `Save(Exploration e)` → `new SavedExploration(Guid.NewGuid(), Name(e), DateTimeOffset.UtcNow, ToSaved(e.Start), e.Links.Select(ToSaved).ToArray())` with `Name` = `e.Links.Count == 0 ? e.Start.Label : $"{e.Start.Label} → {e.Current.Label}"`. `MainLayout.Continue(SavedExploration saved, int upTo)` → `_hamburgerSeed = new Exploration(ToExplorable(saved.Start), saved.Links.Take(upTo).Select(ToLink).ToArray())` and `Root = await LegacyNodes.For(seed.Current, …)` (or, for kinds without legacy nodes, a `GraphExplorer`-only open — the popover handles both via Task 6). `ExplorationListItem` renders `Start` then each `Links[i].Target` as `exploration-node-{i}` (start is node 0) and passes `upTo = i`. `Dropped > 0` → one toast `"{n} steps from an older version could not be restored"` (`data-testid="toast"`), shown once per session.

Selection: `Selection.cs` intents take `Explorable`; `AppServices.AddSelectionAtom` becomes `StateAtom<IReadOnlyList<Explorable>>` under `Selection.StorageKey = "selection-v2"`, seeded by translating `selection-v1` descriptors through the same node mapping (`LegacySaves.ToSavedNode`); `ExplorerPopover.ToggleSelectAsync(node)` → `ToggleSelection(node.Identity)`; `SelectionTray.razor` renders `Explorable.Label`/`Kind`.

- [ ] **Step 4: Verify**

Run: `dotnet test client.Tests && npx playwright test tests/ux --grep "saved-explorations|selection|state-focus"`
Expected: unit tests green; Playwright green except the F8 trail specs (Task 9).

- [ ] **Step 5: Commit**

```bash
git add -A client client.Tests
git commit -m "focus: saves are a start and links (explorations-v2), selection is by node identity (selection-v2); v1 translated once, dropped steps reported once"
```

---

### Task 9: Delete the dead contracts; conformance scans; re-express the two F8 specs; gates; push

**Files:**
- Delete: `client/Contracts/Focus.cs` (all of it — `Focus` now lives in `client/Explore/Focus.cs`; `IEscapeHatch` moves to `client/Views/IEscapeHatch.cs`; `IPresentation`, `IFocusComponent`, `IFrontierAbstraction`, `ITraversal`, the old `Focus` record die), `client/State/FocusStack.cs`, `client.Tests/State/FocusStackTests.cs`
- Modify: `client.Tests/State/ConformanceTests.cs` (`FocusStack` → `ExplorationState` at :107, :212, :329, :367, :409-431; the idempotence allowlist gains `Follow` beside `ToggleSelection`; `AtomNames` uniqueness test sees `exploration`), `client/AppServices.cs` (remove the `FocusStack` atom), `tests/ux/state-focus.spec.ts`, `tests/ux/saved-explorations.spec.ts`, `client.Tests/stryker-config.json`

- [ ] **Step 1: Deletions and scans**

```bash
git rm client/Contracts/Focus.cs client/State/FocusStack.cs client.Tests/State/FocusStackTests.cs
```
Create `client/Views/IEscapeHatch.cs` with the interface verbatim; fix `using`s. In `ConformanceTests.cs` replace every `FocusStack` token per the facts (§7); add `("client\\State\\ExplorationState.cs", "Follow", "a user gesture: following the same link twice is two hops, like ToggleSelection")` to the non-idempotent-intent allowlist. Run: `dotnet build client && dotnet test client.Tests` → green.

- [ ] **Step 2: Re-express the two F8 specs**

In `tests/ux/state-focus.spec.ts`, the scenario "a Back landing is recorded in the trail" becomes "going back follows the dual link and the breadcrumb collapses the return": open a verse, follow to its book (`popover-chip-book`), press `popover-breadcrumb-back`, assert `popover-breadcrumb-back` is gone (breadcrumb empty) and `popover-title` is the verse again; then save and assert `exploration-node-0` is the verse, `exploration-node-1` the book, `exploration-node-2` the verse (the full trail is what is saved — Links, not Breadcrumb). In `saved-explorations.spec.ts`, "consecutive-duplicate collapse (GEN.1.1 twice saves once)" becomes "focusing the node already current is not a hop": open GEN.1.1, click GEN.1.1 again, save, assert exactly one `exploration-node-*`.

- [ ] **Step 3: Gates**

`client.Tests/stryker-config.json` `mutate` gains `**/Explore/Explorable.cs`, `**/Explore/Exploration.cs`, `**/Explore/Affordance.cs`, `**/Explore/Explorer.cs`, `**/Explore/Presentation.cs`, `**/Explore/LegacySaves.cs`, `**/State/ExplorationState.cs`, `**/Components/FocusView.razor.cs`. Run: `cd client.Tests && dotnet stryker` → 100% or equivalents recorded with reasons. Then `dotnet test client.Tests && dotnet test client.ContractTests && npx playwright test tests/ux` → green.

- [ ] **Step 4: Commit and push**

```bash
git add -A client client.Tests tests/ux
git commit -m "focus: the dead Focus contracts and FocusStack are gone; conformance scans follow ExplorationState; the two trail specs say what F8 says"
git push origin worktree-bible-atlas-m1
```

---

## Self-review against the spec

- **Coverage:** §3.1 (Task 1), §3.5 Exploration/state/intents (Task 2), §3.4 (Task 3), §3.2–3.3 (Task 4; `Present` is the generic card for every kind — FOCUS-2…7 enrich), strangler bridge incl. `Identity`, `LegacyNodes.For`, kind-naming pushes, `Chip` rename (Task 5), popover retarget + host view + hatches (Task 6), rendering through affordances with preserved ids (Task 7), §3.6 saves + selection + R4 translation (Task 8), §9 FOCUS-1 rows and the F8 spec changes (Task 9). FOCUS-8's substance is delivered by Tasks 4 and 7; FOCUS-8 becomes verification + deleting the legacy fallback.
- **Placeholders:** the id literals in Task 5's identity test are to be confirmed against the live server before implementation — stated as a step, with the source of truth named.
- **Type consistency:** `Explorable`/`Link`/`FrontierGroup`/`Page<T>` (Task 1) are what Tasks 2, 4, 7, 8 build on; `ExplorationState`/intents (Task 2) are what Task 6's popover and Task 9's scans name; `Affordances.Of` (Task 3) is the `Func<EdgeKind, Affordance>` Task 7's `Focus` carries; `LegacyNodes.For` and `Identity` (Task 5) are what Tasks 6 and 8 call.
