using BibleAtlas.Client.Components;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Legacy;
using BibleAtlas.Client.State;
using Bunit;
using Microsoft.AspNetCore.Components;
using Microsoft.Extensions.DependencyInjection;

namespace BibleAtlas.Client.Tests;

public sealed class ExplorerPopoverTests : BunitContext
{
    private static readonly NodeRef Exodus = ServedGraph.Ref(NodeKind.Narrative, "Narrative:exodus", "The Exodus");
    private static readonly NodeRef Wilderness = ServedGraph.Ref(NodeKind.Narrative, "Narrative:wilderness", "The Wilderness");
    private static readonly NodeRef Bethel = ServedGraph.Ref(NodeKind.Place, "Place:bethel-1", "Bethel");
    private const string TheWorld = "http://localhost/world";
    private static readonly SavedExploration AtExodus = new("seed", "Seed", DateTimeOffset.UnixEpoch, ServedGraph.At(Exodus), []);

    private const string ExodusPresented = """
        <div class="popover-body" data-testid="popover-body">
            <div class="popover-section" data-testid="popover-section-card">
                <p class="focus-title" data-testid="popover-card-title">The Exodus</p>
                <dl class="focus-fields">
                    <div class="focus-field" data-testid="popover-field-Provenance"><dt>Provenance</dt><dd>kjv</dd></div>
                </dl>
            </div>
            <span class="focus-entry"><button type="button" class="focus-arrow explorable" data-testid="popover-next">The Wilderness ›</button><button type="button" class="focus-entry-edge explorable-quiet" data-testid="popover-entry-edge-follows-in-e:Narrative:wilderness" aria-label="e The Wilderness" title="e The Wilderness">&#8942;</button></span>
        </div>
        """;

    [Fact]
    public void A_node_no_legacy_provider_serves_is_presented_as_its_card_and_frontier()
    {
        // Arrange
        Hosting(Narratives());

        // Act
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Resume(AtExodus)));

        // Assert
        popover.WaitForAssertion(() => popover.Find("[data-testid='popover-body']").MarkupMatches(ExodusPresented));
    }

    [Fact]
    public void Following_a_link_of_the_card_is_a_hop_the_trail_records_and_back_is_its_dual()
    {
        // Arrange
        var graph = Narratives();
        var atom = Hosting(graph);
        var exodus = Resolved.Node(graph, Exodus);
        var wilderness = Resolved.Node(graph, Wilderness);
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Resume(AtExodus)));

        // Act
        popover.WaitForElement("[data-testid='popover-next']").Click();
        var followed = popover.WaitForElement("[data-testid='popover-breadcrumb-back']");
        var afterFollow = atom.Value;
        followed.Click();

        // Assert
        popover.WaitForAssertion(() => Assert.Equal(
            (
                new ExplorationState.Open(new Exploration(exodus, [new Step(EdgeKind.FollowsIn, wilderness)])),
                new ExplorationState.Open(new Exploration(exodus, [new Step(EdgeKind.FollowsIn, wilderness), new Step(EdgeKind.PrecedesIn, exodus)]))
            ),
            (afterFollow, atom.Value)));
    }

    [Fact]
    public void A_commentary_item_is_presented_as_the_prose_its_served_card_carries()
    {
        // Arrange
        const string prose = "In the beginning, cp. John 1, 1, that is, when time first began.";
        Hosting(new ServedGraph().Serving(new NodeRecord(
            book: null, catechism: null, description: prose, edgeSummary: [], @event: null,
            id: Wire.Node("CommentaryItem:kretzmann/0.1.0"), kind: NodeKind.CommentaryItem, label: "The Creation of Chaos and Light",
            person: null, place: null, era: null, map: null, polity: null, provenance: ServedGraph.ServedProvenance, text: null, version: Wire.Root("v"))));

        // Act
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Legacy(new CommentaryItemNode("kretzmann/0.1.0", "The Creation of Chaos and Light"))));

        // Assert
        popover.WaitForAssertion(() => Assert.Equal(prose, popover.Find(".popover-commentary-text").TextContent));
    }

    [Fact]
    public void Opening_on_a_position_resolves_it_and_opens_there()
    {
        // Arrange
        var graph = Narratives();
        var atom = Hosting(graph);

        // Act
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Explore(ServedGraph.At(Exodus))));

        // Assert
        popover.WaitForAssertion(() => Assert.Equal(new ExplorationState.Open(new Exploration(Resolved.Node(graph, Exodus), [])), atom.Value));
    }

    [Fact]
    public void Opening_on_a_save_begins_at_its_start_and_replays_its_steps()
    {
        // Arrange
        var graph = Narratives();
        var atom = Hosting(graph);
        var saved = AtExodus with { Steps = [new Link(EdgeKind.FollowsIn, ServedGraph.At(Wilderness))] };

        // Act
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Resume(saved)));

        // Assert
        var replayed = new Exploration(Resolved.Node(graph, Exodus), [new Step(EdgeKind.FollowsIn, Resolved.Node(graph, Wilderness))]);
        popover.WaitForAssertion(() => Assert.Equal(new ExplorationState.Open(replayed), atom.Value));
    }

    [Fact]
    public void Opening_on_a_save_reads_its_start_and_every_step_in_one_read()
    {
        // Arrange
        const int StartAndSteps = 1;
        var graph = Narratives();
        var atom = Hosting(graph);
        var saved = AtExodus with { Steps = [new Link(EdgeKind.FollowsIn, ServedGraph.At(Wilderness)), new Link(EdgeKind.PrecedesIn, ServedGraph.At(Exodus))] };

        // Act
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Resume(saved)));

        // Assert
        popover.WaitForAssertion(() => Assert.Equal(StartAndSteps, graph.ElementReads));
    }

    [Fact]
    public void Opening_on_a_legacy_node_resolves_its_identity_and_remembers_its_rendering()
    {
        // Arrange
        var graph = Narratives();
        var atom = Hosting(graph);
        var legacy = new CommentaryItemNode("kretzmann/0.1.0", "The Creation of Chaos and Light");
        graph.Serving(ServedGraph.Card(NodeKind.CommentaryItem, legacy.Identity.Id, legacy.Identity.Label));

        // Act
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Legacy(legacy)));

        // Assert
        popover.WaitForAssertion(() => Assert.Equal(legacy.Title, popover.Find("[data-testid='popover-title']").TextContent));
        Assert.Equal(legacy.Identity.Id, ((ExplorationState.Open)atom.Value).Exploration.Current.Id);
    }

    [Fact]
    public void Showing_a_place_on_the_map_hands_its_exploration_to_the_world_open_and_unheld()
    {
        // Arrange
        var graph = Narratives().Serving(ServedGraph.Card(NodeKind.Place, Bethel.Id, Bethel.Label));
        var atom = Hosting(graph);
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Explore(ServedGraph.At(Bethel))));

        // Act
        popover.WaitForElement("[data-testid='popover-chip-map']").Click();

        // Assert
        Assert.Equal(
            (new ExplorationState.Open(new Exploration(Resolved.Node(graph, Bethel), [])), false, TheWorld),
            (atom.Value, Services.GetRequiredService<OwnershipRegistry>().IsHeld(AtomNames.Exploration), Services.GetRequiredService<NavigationManager>().Uri));
    }

    [Fact]
    public void A_popover_the_world_hosts_offers_no_way_to_the_map()
    {
        // Arrange
        var graph = Narratives().Serving(ServedGraph.Card(NodeKind.Place, Bethel.Id, Bethel.Label));
        Hosting(graph);

        // Act
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Explore(ServedGraph.At(Bethel))).Add(v => v.HostedByWorld, true));

        // Assert
        popover.WaitForElement("[data-testid='popover-card-title']");
        Assert.Empty(popover.FindAll("[data-testid='popover-chip-map']"));
    }

    [Fact]
    public void An_arrival_that_fails_offers_to_try_again_and_trying_again_arrives()
    {
        // Arrange
        var graph = Narratives();
        var atom = Hosting(new FlakyGraph(graph, failedReads: 1));
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Explore(ServedGraph.At(Exodus))));

        // Act
        popover.WaitForElement("[data-testid='could-not-load-retry']").Click();

        // Assert
        popover.WaitForAssertion(() => popover.Find("[data-testid='popover-body']").MarkupMatches(ExodusPresented));
        Assert.Equal(new ExplorationState.Open(new Exploration(Resolved.Node(graph, Exodus), [])), atom.Value);
    }

    [Fact]
    public async Task A_push_before_the_popover_has_arrived_anywhere_changes_nothing()
    {
        // Arrange
        var atom = Hosting(new FlakyGraph(Narratives(), failedReads: 1));
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Explore(ServedGraph.At(Exodus))));
        popover.WaitForElement("[data-testid='could-not-load-retry']");

        // Act
        await popover.InvokeAsync(() => popover.Instance.PushAsync(new PopoverOpening.Explore(ServedGraph.At(Wilderness)), EdgeKind.FollowsIn));

        // Assert
        Assert.Equal(new ExplorationState.Closed(), atom.Value);
    }

    [Fact]
    public void A_legacy_body_that_fails_to_load_offers_to_try_again_and_trying_again_loads_it()
    {
        // Arrange
        var graph = Narratives();
        Hosting(graph);
        var legacy = new FlakyLegacy(Exodus, failures: 1);
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Legacy(legacy)));

        // Act
        popover.WaitForElement("[data-testid='could-not-load-retry']").Click();

        // Assert
        popover.WaitForAssertion(() => popover.Find("[data-testid='popover-body']").MarkupMatches($"""<div class="popover-body" data-testid="popover-body"><p>{FlakyLegacy.Loaded}</p></div>"""));
    }

    [Fact]
    public void A_legacy_section_whose_read_fails_offers_to_try_again_and_trying_again_loads_it()
    {
        // Arrange
        const string prose = "In the beginning, cp. John 1, 1, that is, when time first began.";
        var legacy = new CommentaryItemNode("kretzmann/0.1.0", "The Creation of Chaos and Light");
        Hosting(new FlakyGraph(new ServedGraph().Serving(ServedGraph.Card(NodeKind.CommentaryItem, legacy.Identity.Id, legacy.Identity.Label) with { Description = prose }), failedReads: 0, failedCards: 1));
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Opening, new PopoverOpening.Legacy(legacy)));

        // Act
        popover.WaitForElement("[data-testid='could-not-load-retry']").Click();

        // Assert
        popover.WaitForAssertion(() => Assert.Equal(prose, popover.Find(".popover-commentary-text").TextContent));
    }

    private sealed class FlakyLegacy(NodeRef identity, int failures) : IExplorable
    {
        public const string Loaded = "loaded at last";

        private int _failed;

        public string Title => identity.Label;

        public string Kind => nameof(FlakyLegacy);

        public NodeRef Identity => identity;

        public Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api) => Task.FromResult<IReadOnlyList<Chip>>([]);

        public Task<RenderFragment> BodyAsync(AtlasClient api) =>
            _failed++ < failures
                ? Task.FromException<RenderFragment>(new HttpRequestException(Offline))
                : Task.FromResult<RenderFragment>(builder =>
                {
                    builder.OpenElement(0, "p");
                    builder.AddContent(1, Loaded);
                    builder.CloseElement();
                });
    }

    private sealed class FlakyGraph(ServedGraph served, int failedReads, int failedCards = 0) : IExplorableClient
    {
        private int _reads;
        private int _cards;

        public Task<NodeRecord> Card(NodeId id) =>
            _cards++ < failedCards ? Task.FromException<NodeRecord>(new HttpRequestException(Offline)) : served.Card(id);

        public Task<ElementPage> Elements(IReadOnlyList<ElementId> ids) =>
            _reads++ < failedReads ? Task.FromException<ElementPage>(new HttpRequestException(Offline)) : served.Elements(ids);

        public Task<EdgePage> Edges(ElementId positionId, EdgeKind kind, EdgePageCursor? cursor = null, int limit = BibleAtlas.Client.Exploring.Affordances.PageSize) =>
            served.Edges(positionId, kind, cursor, limit);

        public Task<TextWindow> Reading(TextWindowReference fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
            served.Reading(fromRef, n, dir, corpus);
    }

    private const string Offline = "offline";

    private static ServedGraph Narratives() =>
        new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Narrative, Exodus.Id, Exodus.Label, new FrontierGroup(EdgeKind.FollowsIn, 1)))
            .Serving(Exodus.Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, Wilderness))
            .Serving(ServedGraph.Card(NodeKind.Narrative, Wilderness.Id, Wilderness.Label));

    private StateAtom<ExplorationState> Hosting(IExplorableClient graph)
    {
        JSInterop.Mode = JSRuntimeMode.Loose;
        Services.AddSingleton(new AtlasClient(new HttpClient { BaseAddress = new Uri("http://unserved.invalid") }));
        Services.AddSingleton<IExplorableClient>(graph);
        Services.AddSingleton<IExplorer>(new GraphExplorer(graph));
        Services.AddSingleton<IPresenter>(new GraphPresenter());
        Services.AddSingleton(new SavedExplorationsService(new InMemoryLocalStorage()));
        Services.AddSingleton<OwnershipRegistry>();
        Services.AddSingleton(new StateAtom<IReadOnlyList<NodeRef>>(AtomNames.Selection, Selection.Empty, SequenceEqualityComparer<NodeRef>.Instance));
        AppServices.AddStateAtoms(Services);
        return Services.GetRequiredService<StateAtom<ExplorationState>>();
    }
}
