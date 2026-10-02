using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.Views;
using Bunit;
using Microsoft.Extensions.DependencyInjection;

namespace BibleAtlas.Client.Tests;

public sealed class FocusViewTests : BunitContext
{
    private const string Genesis2Id = "Container:bible-chapter-GEN-2";
    private const string Genesis2Label = "Genesis 2";
    private const int CitesServed = 5;
    private const int CitesFirstPage = 3;
    private const int CitesSecondPageCursor = 3;
    private const int SecondOfTwo = 1;

    private static readonly NodeRef Genesis = ServedGraph.Ref(NodeKind.Container, "Container:bible-book-GEN", "Genesis");
    private static readonly NodeRef Genesis1 = ServedGraph.Ref(NodeKind.Container, "Container:bible-chapter-GEN-1", "Genesis 1");
    private static readonly NodeRef Genesis3 = ServedGraph.Ref(NodeKind.Container, "Container:bible-chapter-GEN-3", "Genesis 3");
    private static readonly NodeRef Verse1 = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:GEN.2.1", "GEN.2.1");
    private static readonly NodeRef Verse2 = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:GEN.2.2", "GEN.2.2");
    private static readonly NodeRef Adam = ServedGraph.Ref(NodeKind.Person, "Person:adam", "Adam");
    private static readonly NodeRef Eden = ServedGraph.Ref(NodeKind.Place, "Place:eden", "Eden");
    private static readonly NodeRef EdenMap = ServedGraph.Ref(NodeKind.Map, "Map:era-eden", "The world of Eden");
    private static readonly NodeRef[] Citations = Enumerable.Range(1, CitesServed)
        .Select(n => ServedGraph.Ref(NodeKind.TextUnit, $"text-unit:JHN.1.{n}", $"JHN.1.{n}"))
        .ToArray();

    private static readonly string Genesis2Card = $$"""
        <button type="button" class="focus-up explorable" data-testid="popover-up-member-of-Container:bible-book-GEN">Genesis</button>
        {{EdgeStep(EdgeKind.MemberOf, Genesis)}}
        <div class="popover-section" data-testid="popover-section-card">
            <p class="focus-title" data-testid="popover-card-title">Genesis 2</p>
            <dl class="focus-fields">
                <div class="focus-field" data-testid="popover-field-Provenance"><dt>Provenance</dt><dd>kjv</dd></div>
            </dl>
        </div>
        <button type="button" class="focus-arrow explorable" data-testid="popover-prev">‹ Genesis 1</button>
        {{EdgeStep(EdgeKind.PrecedesIn, Genesis1)}}
        <button type="button" class="focus-arrow explorable" data-testid="popover-next">Genesis 3 ›</button>
        {{EdgeStep(EdgeKind.FollowsIn, Genesis3)}}
        <div class="popover-section" data-testid="popover-children-contains">
            <button type="button" class="focus-child explorable" data-testid="popover-child-contains-text-unit:GEN.2.1">GEN.2.1</button>
            {{EdgeStep(EdgeKind.Contains, Verse1)}}
            <button type="button" class="focus-child explorable" data-testid="popover-child-contains-text-unit:GEN.2.2">GEN.2.2</button>
            {{EdgeStep(EdgeKind.Contains, Verse2)}}
        </div>
        <div class="popover-section" data-testid="popover-section-mentions">
            <p class="catechism-section-heading" data-testid="popover-section-mentions-heading">Mentions (2)</p>
            <button type="button" class="focus-link explorable" data-testid="popover-link-mentions-Person:adam">Adam</button>
            {{EdgeStep(EdgeKind.Mentions, Adam)}}
            <button type="button" class="focus-link explorable" data-testid="popover-link-mentions-Place:eden">Eden</button>
            {{EdgeStep(EdgeKind.Mentions, Eden)}}
        </div>
        """;

    private static readonly string CitesClampedToThree = $$"""
        <div class="popover-section" data-testid="popover-section-cites">
            <p class="catechism-section-heading" data-testid="popover-section-cites-heading">Cites (5)</p>
            {{Cited(Citations[..CitesFirstPage])}}
            <div class="popover-reveal-controls">
                <button type="button" class="popover-reveal-link explorable-quiet" data-testid="popover-section-cites-more" aria-label="Show 2 more entries" title="Show 2 more entries">more (2)</button>
            </div>
        </div>
        """;

    private static readonly string CitesRevealedWhole = $$"""
        <div class="popover-section" data-testid="popover-section-cites">
            <p class="catechism-section-heading" data-testid="popover-section-cites-heading">Cites (5)</p>
            {{Cited(Citations)}}
            <div class="popover-reveal-controls">
                <button type="button" class="popover-reveal-link explorable-quiet" data-testid="popover-section-cites-collapse" aria-label="Show fewer entries" title="Show fewer entries">less</button>
            </div>
        </div>
        """;

    private static readonly string AdamCard = $$"""
        <div class="popover-section" data-testid="popover-section-card">
            <p class="focus-title" data-testid="popover-card-title">Adam</p>
            <dl class="focus-fields">
                <div class="focus-field" data-testid="popover-field-Provenance"><dt>Provenance</dt><dd>kjv</dd></div>
            </dl>
        </div>
        <div class="popover-section" data-testid="popover-section-mentioned-in">
            <p class="catechism-section-heading" data-testid="popover-section-mentioned-in-heading">Mentioned in (1)</p>
            <button type="button" class="focus-link explorable" data-testid="popover-link-mentioned-in-text-unit:GEN.2.1">GEN.2.1</button>
            {{EdgeStep(EdgeKind.MentionedIn, Verse1)}}
        </div>
        """;

    [Fact]
    public void A_focus_is_its_card_then_every_frontier_group_offered_by_its_kinds_affordance()
    {
        // Arrange
        var genesis2 = Genesis2(Explored(Genesis2Graph()));

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, genesis2).Add(v => v.Surface, Surface.Popover));

        // Assert
        view.MarkupMatches(Genesis2Card + CitesClampedToThree);
    }

    [Fact]
    public void Revealing_more_of_a_listed_group_pages_its_links_from_the_server()
    {
        // Arrange
        var view = Render<FocusView>(p => p.Add(v => v.Node, Genesis2(Explored(Genesis2Graph()))).Add(v => v.Surface, Surface.Popover));

        // Act
        view.Find("[data-testid='popover-section-cites-more']").Click();

        // Assert
        view.MarkupMatches(Genesis2Card + CitesRevealedWhole);
    }

    [Fact]
    public void Every_link_of_the_frontier_follows_its_edge_when_chosen()
    {
        // Arrange
        var followed = new List<Link>();
        var view = Render<FocusView>(p => p
            .Add(v => v.Node, Genesis2(Explored(Genesis2Graph())))
            .Add(v => v.Surface, Surface.Popover)
            .Add(v => v.OnFollow, link => followed.Add(link)));

        // Act
        view.Find("[data-testid='popover-up-member-of-Container:bible-book-GEN']").Click();
        view.Find("[data-testid='popover-prev']").Click();
        view.Find("[data-testid='popover-next']").Click();
        view.Find("[data-testid='popover-child-contains-text-unit:GEN.2.2']").Click();
        view.Find("[data-testid='popover-link-mentions-Place:eden']").Click();
        view.Find("[data-testid='popover-link-cites-text-unit:JHN.1.3']").Click();

        // Assert
        Assert.Equal(
            [
                new Link(EdgeKind.MemberOf, ServedGraph.At(Genesis)),
                new Link(EdgeKind.PrecedesIn, ServedGraph.At(Genesis1)),
                new Link(EdgeKind.FollowsIn, ServedGraph.At(Genesis3)),
                new Link(EdgeKind.Contains, ServedGraph.At(Verse2)),
                new Link(EdgeKind.Mentions, ServedGraph.At(Eden)),
                new Link(EdgeKind.Cites, ServedGraph.At(Citations[2])),
            ],
            followed);
    }

    [Fact]
    public void Every_entry_offers_a_step_onto_its_edge()
    {
        // Arrange
        var followed = new List<Link>();
        var view = Render<FocusView>(p => p
            .Add(v => v.Node, Genesis2(Explored(Genesis2Graph())))
            .Add(v => v.Surface, Surface.Popover)
            .Add(v => v.OnFollow, link => followed.Add(link)));
        var everyStep = view.FindAll("[data-testid^='popover-entry-edge-']").Select(step => step.GetAttribute("data-testid")).ToList();

        // Act
        foreach (var step in everyStep)
        {
            view.Find($"[data-testid='{step}']").Click();
        }

        // Assert
        Assert.Equal(
            WholeValue.Of(new[]
            {
                (EdgeKind.MemberOf, Genesis), (EdgeKind.PrecedesIn, Genesis1), (EdgeKind.FollowsIn, Genesis3), (EdgeKind.Contains, Verse1), (EdgeKind.Contains, Verse2),
                (EdgeKind.Mentions, Adam), (EdgeKind.Mentions, Eden), (EdgeKind.Cites, Citations[0]), (EdgeKind.Cites, Citations[1]), (EdgeKind.Cites, Citations[2]),
            }.Select(entry => new Link(entry.Item1, ServedGraph.AtEdge(ServedGraph.EdgeTo(entry.Item1, ServedGraph.At(entry.Item2)))))),
            WholeValue.Of(followed));
    }

    [Fact]
    public void An_edge_shows_its_two_ends_as_crumbs()
    {
        // Arrange
        var mention = ServedGraph.EdgeRef(EdgeKind.Mentions, "Mentions:00ee", "GEN.2.1 · Mentions · Eden");
        var graph = Explored(new ServedGraph().Serving(ServedGraph.EdgeRecordOf(mention, Verse1, Eden)));
        var followed = new List<Link>();
        var view = Render<FocusView>(p => p
            .Add(v => v.Node, Resolved.At(graph, ServedGraph.AtEdge(mention)))
            .Add(v => v.Surface, Surface.Popover)
            .Add(v => v.OnFollow, link => followed.Add(link)));

        // Act
        view.Find("[data-testid='popover-end-text-unit:GEN.2.1']").Click();
        view.Find("[data-testid='popover-end-Place:eden']").Click();

        // Assert
        view.MarkupMatches("""
            <button type="button" class="focus-up explorable" data-testid="popover-end-text-unit:GEN.2.1">GEN.2.1</button>
            <button type="button" class="focus-up explorable" data-testid="popover-end-Place:eden">Eden</button>
            <div class="popover-section" data-testid="popover-section-card">
                <p class="focus-title" data-testid="popover-card-title">GEN.2.1 · Mentions · Eden</p>
                <dl class="focus-fields">
                    <div class="focus-field" data-testid="popover-field-Provenance"><dt>Provenance</dt><dd>kjv</dd></div>
                </dl>
            </div>
            """);
        Assert.Equal([new Link(EdgeKind.MentionedIn, ServedGraph.At(Verse1)), new Link(EdgeKind.Mentions, ServedGraph.At(Eden))], followed);
    }

    [Fact]
    public void A_heading_reads_the_served_display_label()
    {
        // Arrange
        var graph = Explored(WithAdam(new ServedGraph()));

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, Resolved.Node(graph, Adam)).Add(v => v.Surface, Surface.Popover));

        // Assert
        Assert.Equal($"{EdgeKind.MentionedIn.DisplayLabel()} (1)", view.Find("[data-testid='popover-section-mentioned-in-heading']").TextContent);
    }

    [Fact]
    public void A_heading_counts_only_on_the_popover()
    {
        // Arrange
        var graph = Explored(WithAdam(new ServedGraph()));
        var adam = Resolved.Node(graph, Adam);

        // Act
        var headings = Enum.GetValues<Surface>()
            .Select(surface => (surface, Render<FocusView>(p => p.Add(v => v.Node, adam).Add(v => v.Surface, surface))
                .FindAll(".catechism-section-heading").Single().TextContent))
            .ToList();

        // Assert
        Assert.Equal(
            [(Surface.World, "Mentioned in"), (Surface.Reader, "Mentioned in"), (Surface.Popover, "Mentioned in (1)")],
            headings);
    }

    [Fact]
    public void Every_surface_names_its_handles_and_draws_a_card_only_where_the_kind_has_a_form()
    {
        // Arrange
        var graph = Explored(new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Container, Genesis2Id, Genesis2Label, new FrontierGroup(EdgeKind.PrecedesIn, 1), new FrontierGroup(EdgeKind.FollowsIn, 1)))
            .Serving(Genesis2Id, EdgeKind.PrecedesIn, null, ServedGraph.Page(EdgeKind.PrecedesIn, null, Genesis1))
            .Serving(Genesis2Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, Genesis3)));
        var genesis2 = Genesis2(graph);

        // Act
        var handles = Enum.GetValues<Surface>()
            .Select(surface => (surface, string.Join(" ", Render<FocusView>(p => p.Add(v => v.Node, genesis2).Add(v => v.Surface, surface))
                .FindAll("[data-testid]").Select(element => element.GetAttribute("data-testid")))))
            .ToList();

        // Assert
        Assert.Equal(
            [
                (Surface.World, ""),
                (Surface.Reader, "reader-section-card reader-card-title reader-field-Provenance reader-prev reader-next"),
                (Surface.Popover, $"popover-section-card popover-card-title popover-field-Provenance popover-prev {EdgeStepHandle(EdgeKind.PrecedesIn, Genesis1)} popover-next {EdgeStepHandle(EdgeKind.FollowsIn, Genesis3)}"),
            ],
            handles);
    }

    [Fact]
    public void A_link_is_offered_only_where_its_target_kind_has_a_form_on_the_surface_it_opens_on()
    {
        // Arrange
        var graph = Explored(new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Map, EdenMap.Id, EdenMap.Label, new FrontierGroup(EdgeKind.Shows, 2), new FrontierGroup(EdgeKind.MentionedIn, 1)))
            .Serving(EdenMap.Id, EdgeKind.Shows, null, ServedGraph.Page(EdgeKind.Shows, null, Eden, Adam))
            .Serving(EdenMap.Id, EdgeKind.MentionedIn, null, ServedGraph.Page(EdgeKind.MentionedIn, null, Verse1)));

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, Resolved.Node(graph, EdenMap)).Add(v => v.Surface, Surface.World));

        // Assert
        view.MarkupMatches("""
            <div class="popover-section" data-testid="world-section-card">
                <p class="focus-title" data-testid="world-card-title">The world of Eden</p>
                <dl class="focus-fields">
                    <div class="focus-field" data-testid="world-field-Provenance"><dt>Provenance</dt><dd>kjv</dd></div>
                </dl>
            </div>
            <div class="popover-section" data-testid="world-children-shows">
                <button type="button" class="focus-child explorable" data-testid="world-child-shows-Place:eden">Eden</button>
            </div>
            <div class="popover-section" data-testid="world-section-mentioned-in">
                <p class="catechism-section-heading" data-testid="world-section-mentioned-in-heading">Mentioned in</p>
            </div>
            """);
    }

    [Fact]
    public void Rendering_the_same_node_again_keeps_what_was_revealed()
    {
        // Arrange
        var graph = Explored(Genesis2Graph());
        var view = Render<FocusView>(p => p.Add(v => v.Node, Genesis2(graph)).Add(v => v.Surface, Surface.Popover));
        view.Find("[data-testid='popover-section-cites-more']").Click();

        // Act
        view.Render(p => p.Add(v => v.Node, Genesis2(graph)));

        // Assert
        view.MarkupMatches(Genesis2Card + CitesRevealedWhole);
    }

    [Fact]
    public void Presenting_another_node_replaces_the_card_and_the_frontier()
    {
        // Arrange
        var graph = Explored(WithAdam(Genesis2Graph()));
        var view = Render<FocusView>(p => p.Add(v => v.Node, Genesis2(graph)).Add(v => v.Surface, Surface.Popover));

        // Act
        view.Render(p => p.Add(v => v.Node, Resolved.Node(graph, Adam)));

        // Assert
        view.MarkupMatches(AdamCard);
    }

    [Fact]
    public async Task A_node_presented_while_anothers_first_pages_are_still_arriving_is_not_overwritten_when_they_arrive()
    {
        // Arrange
        var held = new HeldGraph(WithAdam(Genesis2Graph()), EdgeKind.Mentions, null);
        var explorer = new GraphExplorer(held);
        Services.AddSingleton<IExplorer>(explorer);
        var genesis2 = await explorer.Resolve(ServedGraph.At(Genesis2Ref));
        var adam = await explorer.Resolve(ServedGraph.At(Adam));
        var view = Render<FocusView>(p => p.Add(v => v.Node, genesis2).Add(v => v.Surface, Surface.Popover));
        view.Render(p => p.Add(v => v.Node, adam));

        // Act
        held.Release();
        await view.InvokeAsync(() => { });

        // Assert
        view.MarkupMatches(AdamCard);
    }

    [Fact]
    public async Task A_node_presented_while_another_is_still_filling_a_group_is_not_overwritten_when_the_fill_completes()
    {
        // Arrange
        var held = new HeldGraph(WithAdam(Genesis2Graph()), EdgeKind.Mentions, SecondOfTwo);
        var explorer = new GraphExplorer(held);
        Services.AddSingleton<IExplorer>(explorer);
        var genesis2 = await explorer.Resolve(ServedGraph.At(Genesis2Ref));
        var adam = await explorer.Resolve(ServedGraph.At(Adam));
        var view = Render<FocusView>(p => p.Add(v => v.Node, genesis2).Add(v => v.Surface, Surface.Popover));
        view.Render(p => p.Add(v => v.Node, adam));

        // Act
        held.Release();
        await view.InvokeAsync(() => { });

        // Assert
        view.MarkupMatches(AdamCard);
    }

    private ServedGraph Explored(ServedGraph graph)
    {
        Services.AddSingleton<IExplorer>(new GraphExplorer(graph));
        return graph;
    }

    private static readonly NodeRef Genesis2Ref = ServedGraph.Ref(NodeKind.Container, Genesis2Id, Genesis2Label);

    private static Explorable Genesis2(ServedGraph graph) => Resolved.Node(graph, Genesis2Ref);

    private static ServedGraph WithAdam(ServedGraph graph) =>
        graph
            .Serving(ServedGraph.Card(NodeKind.Person, Adam.Id, Adam.Label, new FrontierGroup(EdgeKind.MentionedIn, 1)))
            .Serving(Adam.Id, EdgeKind.MentionedIn, null, ServedGraph.Page(EdgeKind.MentionedIn, null, Verse1));

    private sealed class HeldGraph(ServedGraph served, EdgeKind heldKind, int? heldCursor) : IExplorableClient
    {
        private readonly TaskCompletionSource _release = new();

        public void Release() => _release.SetResult();

        public Task<NodeRecord> Card(string id) => served.Card(id);

        public Task<IReadOnlyList<Element>> Elements(IReadOnlyList<string> ids) => served.Elements(ids);

        public async Task<EdgePage> Edges(string positionId, EdgeKind kind, int? cursor = null, int limit = IExplorableClient.DefaultPageSize)
        {
            if (kind == heldKind && cursor == heldCursor)
            {
                await _release.Task;
            }

            return await served.Edges(positionId, kind, cursor, limit);
        }

        public Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
            served.Reading(fromRef, n, dir, corpus);
    }

    private static ServedGraph Genesis2Graph() =>
        new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Container, Genesis2Id, Genesis2Label,
                new FrontierGroup(EdgeKind.MemberOf, 1),
                new FrontierGroup(EdgeKind.PrecedesIn, 1),
                new FrontierGroup(EdgeKind.FollowsIn, 1),
                new FrontierGroup(EdgeKind.Contains, 2),
                new FrontierGroup(EdgeKind.Mentions, 2),
                new FrontierGroup(EdgeKind.Cites, CitesServed)))
            .Serving(Genesis2Id, EdgeKind.MemberOf, null, ServedGraph.Page(EdgeKind.MemberOf, null, Genesis))
            .Serving(Genesis2Id, EdgeKind.PrecedesIn, null, ServedGraph.Page(EdgeKind.PrecedesIn, null, Genesis1))
            .Serving(Genesis2Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, Genesis3))
            .Serving(Genesis2Id, EdgeKind.Contains, null, ServedGraph.Page(EdgeKind.Contains, SecondOfTwo, Verse1))
            .Serving(Genesis2Id, EdgeKind.Contains, SecondOfTwo, ServedGraph.Page(EdgeKind.Contains, null, Verse2))
            .Serving(Genesis2Id, EdgeKind.Mentions, null, ServedGraph.Page(EdgeKind.Mentions, SecondOfTwo, Adam))
            .Serving(Genesis2Id, EdgeKind.Mentions, SecondOfTwo, ServedGraph.Page(EdgeKind.Mentions, null, Eden))
            .Serving(Genesis2Id, EdgeKind.Cites, null, ServedGraph.Page(EdgeKind.Cites, CitesSecondPageCursor, Citations[..CitesFirstPage]))
            .Serving(Genesis2Id, EdgeKind.Cites, CitesSecondPageCursor, ServedGraph.Page(EdgeKind.Cites, null, Citations[CitesFirstPage..]));

    private static string EdgeStepHandle(EdgeKind kind, NodeRef neighbour) =>
        $"popover-entry-edge-{kind.WireName()}-{ServedGraph.EdgeTo(kind, ServedGraph.At(neighbour)).Id}";

    private static string EdgeStep(EdgeKind kind, NodeRef neighbour)
    {
        var edge = ServedGraph.EdgeTo(kind, ServedGraph.At(neighbour));
        return $"""<button type="button" class="focus-entry-edge explorable-quiet" data-testid="{EdgeStepHandle(kind, neighbour)}" aria-label="{edge.Label}" title="{edge.Label}">&#8942;</button>""";
    }

    private static string Cited(IEnumerable<NodeRef> verses) =>
        string.Concat(verses.Select(verse =>
            $"""<button type="button" class="focus-link explorable-quiet" data-testid="popover-link-cites-{verse.Id}">{verse.Label}</button>{EdgeStep(EdgeKind.Cites, verse)}"""));
}
