using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Views;
using Bunit;
using Microsoft.Extensions.DependencyInjection;

namespace BibleAtlas.Client.Tests;

public sealed class FocusViewTests : BunitContext
{
    private const string Genesis2Id = "Container:bible-chapter-GEN-2";
    private const string Genesis2Label = "Genesis 2";
    private const int CitesBeyondOnePage = 5;
    private static readonly int CitesFirstPage = Affordances.PageSize;
    private static readonly int CitesServed = CitesFirstPage + CitesBeyondOnePage;
    private static readonly int CitesSecondPageCursor = CitesFirstPage;
    private const int SecondOfTwo = 1;

    private static readonly NodeRef Genesis = ServedGraph.Ref(NodeKind.Container, "Container:bible-book-GEN", "Genesis");
    private static readonly NodeRef Genesis1 = ServedGraph.Ref(NodeKind.Container, "Container:bible-chapter-GEN-1", "Genesis 1");
    private static readonly NodeRef Genesis3 = ServedGraph.Ref(NodeKind.Container, "Container:bible-chapter-GEN-3", "Genesis 3");
    private static readonly NodeRef Verse1 = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:GEN.2.1", "GEN.2.1");
    private static readonly NodeRef Verse2 = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:GEN.2.2", "GEN.2.2");
    private static readonly NodeRef Adam = ServedGraph.Ref(NodeKind.Person, "Person:adam", "Adam");
    private static readonly NodeRef Eden = ServedGraph.Ref(NodeKind.Place, "Place:eden", "Eden");
    private static readonly NodeRef EdenMap = ServedGraph.Ref(NodeKind.Map, "Map:era-eden", "The world of Eden");
    private static readonly TimeRange EdenWindow = ServedGraph.Range(new Year(label: "4004 BC", value: -4003), new Year(label: "2348 BC", value: -2347), "4004 BC – 2348 BC");
    private static readonly NodeRef[] Citations = Enumerable.Range(1, CitesServed)
        .Select(n => ServedGraph.Ref(NodeKind.TextUnit, $"text-unit:JHN.1.{n}", $"JHN.1.{n}"))
        .ToArray();

    private const string John316Words = "For God so loved the world, that he gave his only begotten Son";
    private const int WorldStart = 21;
    private const int WorldEnd = 26;
    private const int RedLetterStart = 28;
    private const string ConfessionWords = "Our churches teach that God so loved the world.";
    private const int CitationStart = 24;
    private const int CitationEnd = 47;
    private const int FirstAnchor = 0;

    private static readonly NodeRef John316 = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:JHN.3.16", "JHN.3.16");
    private static readonly NodeRef John315 = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:JHN.3.15", "JHN.3.15");
    private static readonly NodeRef John317 = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:JHN.3.17", "JHN.3.17");
    private static readonly NodeRef John3 = ServedGraph.Ref(NodeKind.Container, "Container:bible-chapter-JHN-3", "John 3");
    private static readonly NodeRef Nicodemus = ServedGraph.Ref(NodeKind.Event, "Event:nicodemus", "Jesus teaches Nicodemus");
    private static readonly NodeRef Romans58 = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:ROM.5.8", "ROM.5.8");
    private static readonly NodeRef SecondArticle = ServedGraph.Ref(NodeKind.CatechismItem, "CatechismItem:creed-2", "The Second Article");
    private static readonly NodeRef World = ServedGraph.Ref(NodeKind.Place, "Place:world", "The world");
    private const string Romans58Words = "But God commendeth his love toward us";
    private static readonly UnitText Romans58Text = ServedGraph.UnitTextOf(new BibleRef(book: BookId.ROM, chapter: 5, verse: 8), Romans58Words, [], []);
    private static readonly NodeRef AugsburgIv = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:BoC 7.4.1", "BoC 7.4.1");
    private static readonly UnitText John316Text = ServedGraph.UnitTextOf(
        new BibleRef(book: BookId.JHN, chapter: 3, verse: 16), John316Words, [ServedGraph.AnchorOf(EdgeKind.Mentions, World, WorldStart, WorldEnd)], [new WordsOfChristSpan(end: John316Words.Length, start: RedLetterStart)]);
    private static readonly UnitText AugsburgIvText = ServedGraph.UnitTextOf(
        new ConcordRef(article: 4, paragraph: 1, part: 7), ConfessionWords, [ServedGraph.AnchorOf(EdgeKind.Cites, John316, CitationStart, CitationEnd)], []);

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
            {{Worded($"popover-words-contains-{Verse1.Id}", Verse1)}}
            <button type="button" class="focus-child explorable" data-testid="popover-child-contains-text-unit:GEN.2.2">GEN.2.2</button>
            {{EdgeStep(EdgeKind.Contains, Verse2)}}
            {{Worded($"popover-words-contains-{Verse2.Id}", Verse2)}}
        </div>
        <div class="popover-section" data-testid="popover-section-mentions">
            <p class="catechism-section-heading" data-testid="popover-section-mentions-heading">Mentions (2)</p>
            <button type="button" class="focus-link explorable" data-testid="popover-link-mentions-Person:adam">Adam</button>
            {{EdgeStep(EdgeKind.Mentions, Adam)}}
            <button type="button" class="focus-link explorable" data-testid="popover-link-mentions-Place:eden">Eden</button>
            {{EdgeStep(EdgeKind.Mentions, Eden)}}
        </div>
        """;

    private static readonly string CitesFirstPageShown = $$"""
        <div class="popover-section" data-testid="popover-section-cites">
            <p class="catechism-section-heading" data-testid="popover-section-cites-heading">Cites ({{CitesServed}})</p>
            {{Cited(Citations[..CitesFirstPage])}}
            <div class="popover-reveal-controls">
                <span class="popover-page-position" data-testid="popover-section-cites-position">1–{{CitesFirstPage}} of {{CitesServed}}</span>
                <button type="button" class="popover-reveal-link explorable-quiet" data-testid="popover-section-cites-more">More</button>
            </div>
        </div>
        """;

    private static readonly string CitesRevealedWhole = $$"""
        <div class="popover-section" data-testid="popover-section-cites">
            <p class="catechism-section-heading" data-testid="popover-section-cites-heading">Cites ({{CitesServed}})</p>
            {{Cited(Citations)}}
            <div class="popover-reveal-controls">
                <span class="popover-page-position" data-testid="popover-section-cites-position">1–{{CitesServed}} of {{CitesServed}}</span>
                <button type="button" class="popover-reveal-link explorable-quiet" data-testid="popover-section-cites-collapse">Less</button>
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
            {{Worded($"popover-words-mentioned-in-{Verse1.Id}", Verse1)}}
        </div>
        """;

    private const string EdenCard = """
        <div class="popover-section" data-testid="popover-section-card">
            <p class="focus-title" data-testid="popover-card-title">Eden</p>
            <dl class="focus-fields">
                <div class="focus-field" data-testid="popover-field-Provenance"><dt>Provenance</dt><dd>kjv</dd></div>
            </dl>
        </div>
        """;

    private const string WayToTheMap = """
        <button type="button" class="popover-head-action" data-testid="popover-chip-map" aria-label="Show on the map" title="Show on the map">&#8982;</button>
        """;

    private const string WayToTheReader = """
        <button type="button" class="popover-head-action" data-testid="popover-chip-context" aria-label="Read in context" title="Read in context">&#182;</button>
        """;

    public FocusViewTests() => Services.AddSingleton<IPresenter>(new GraphPresenter());

    [Fact]
    public void A_node_whose_home_is_the_world_offers_its_host_the_way_to_the_map()
    {
        // Arrange
        var eden = Resolved.Node(new ServedGraph().Serving(ServedGraph.Card(NodeKind.Place, Eden.Id, Eden.Label)), Eden);

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, eden).Add(v => v.Surface, Surface.Popover).Add(v => v.OnShowOnWorld, () => { }));

        // Assert
        view.MarkupMatches(WayToTheMap + EdenCard);
    }

    [Fact]
    public void Choosing_the_way_to_the_map_asks_the_host_to_show_the_node_there()
    {
        // Arrange
        var asked = 0;
        var eden = Resolved.Node(new ServedGraph().Serving(ServedGraph.Card(NodeKind.Place, Eden.Id, Eden.Label)), Eden);
        var view = Render<FocusView>(p => p.Add(v => v.Node, eden).Add(v => v.Surface, Surface.Popover).Add(v => v.OnShowOnWorld, () => asked++));

        // Act
        view.Find("[data-testid='popover-chip-map']").Click();

        // Assert
        Assert.Equal(1, asked);
    }

    [Fact]
    public void A_node_whose_home_is_not_the_world_offers_no_way_to_the_map()
    {
        // Arrange
        var adam = Resolved.Node(WithAdam(new ServedGraph()), Adam);

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, adam).Add(v => v.Surface, Surface.Popover).Add(v => v.OnShowOnWorld, () => { }));

        // Assert
        view.MarkupMatches(AdamCard);
    }

    [Fact]
    public void A_host_that_offers_no_way_to_the_map_shows_none()
    {
        // Arrange
        var eden = Resolved.Node(new ServedGraph().Serving(ServedGraph.Card(NodeKind.Place, Eden.Id, Eden.Label)), Eden);

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, eden).Add(v => v.Surface, Surface.Popover));

        // Assert
        view.MarkupMatches(EdenCard);
    }

    [Fact]
    public void A_focus_is_its_card_then_every_frontier_group_offered_by_its_kinds_affordance()
    {
        // Arrange
        var genesis2 = Genesis2(Genesis2Graph());

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, genesis2).Add(v => v.Surface, Surface.Popover));

        // Assert
        view.MarkupMatches(Genesis2Card + CitesFirstPageShown);
    }

    [Fact]
    public void Revealing_more_of_a_listed_group_pages_its_links_from_the_server()
    {
        // Arrange
        var view = Render<FocusView>(p => p.Add(v => v.Node, Genesis2(Genesis2Graph())).Add(v => v.Surface, Surface.Popover));

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
            .Add(v => v.Node, Genesis2(Genesis2Graph()))
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
            .Add(v => v.Node, Genesis2(Genesis2Graph()))
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
                (EdgeKind.Mentions, Adam), (EdgeKind.Mentions, Eden),
            }.Concat(Citations[..CitesFirstPage].Select(citation => (EdgeKind.Cites, citation))).Select(entry => new Link(entry.Item1, ServedGraph.AtEdge(ServedGraph.EdgeTo(entry.Item1, ServedGraph.At(entry.Item2)))))),
            WholeValue.Of(followed));
    }

    [Fact]
    public void An_edge_shows_its_two_ends_as_crumbs()
    {
        // Arrange
        var mention = ServedGraph.EdgeRef(EdgeKind.Mentions, "Mentions:00ee", "GEN.2.1 · Mentions · Eden");
        var graph = new ServedGraph().Serving(ServedGraph.EdgeRecordOf(mention, Verse1, Eden));
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
        var graph = WithAdam(new ServedGraph());

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, Resolved.Node(graph, Adam)).Add(v => v.Surface, Surface.Popover));

        // Assert
        Assert.Equal($"{EdgeKind.MentionedIn.DisplayLabel()} (1)", view.Find("[data-testid='popover-section-mentioned-in-heading']").TextContent);
    }

    [Fact]
    public void A_heading_counts_only_on_the_popover()
    {
        // Arrange
        var graph = WithAdam(new ServedGraph());
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
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Container, Genesis2Id, Genesis2Label, new FrontierGroup(EdgeKind.PrecedesIn, 1), new FrontierGroup(EdgeKind.FollowsIn, 1)))
            .Serving(Genesis2Id, EdgeKind.PrecedesIn, null, ServedGraph.Page(EdgeKind.PrecedesIn, null, Genesis1))
            .Serving(Genesis2Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, Genesis3));
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
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Map, EdenMap.Id, EdenMap.Label, new FrontierGroup(EdgeKind.Shows, 2), new FrontierGroup(EdgeKind.MentionedIn, 1)) with { Map = ServedGraph.MapWindow(EdenWindow) })
            .Serving(EdenMap.Id, EdgeKind.Shows, null, ServedGraph.Page(EdgeKind.Shows, null, Eden, Adam))
            .Serving(EdenMap.Id, EdgeKind.MentionedIn, null, ServedGraph.Page(EdgeKind.MentionedIn, null, Verse1));

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, Resolved.Node(graph, EdenMap)).Add(v => v.Surface, Surface.World));

        // Assert
        view.MarkupMatches("""
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
        var graph = Genesis2Graph();
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
        var graph = WithAdam(Genesis2Graph());
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
        var genesis2 = await explorer.BeginAt(ServedGraph.At(Genesis2Ref));
        var adam = await explorer.BeginAt(ServedGraph.At(Adam));
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
        var genesis2 = await explorer.BeginAt(ServedGraph.At(Genesis2Ref));
        var adam = await explorer.BeginAt(ServedGraph.At(Adam));
        var view = Render<FocusView>(p => p.Add(v => v.Node, genesis2).Add(v => v.Surface, Surface.Popover));
        view.Render(p => p.Add(v => v.Node, adam));

        // Act
        held.Release();
        await view.InvokeAsync(() => { });

        // Assert
        view.MarkupMatches(AdamCard);
    }

    private const string CouldNotLoad = """
        <p class="popover-meta" data-testid="could-not-load">Couldn't load this — check your connection and try again.</p>
        <button type="button" class="popover-reveal-link explorable-quiet" data-testid="could-not-load-retry">Try again</button>
        """;

    private const int Multitude = 10_000;
    private const int Revealed = 3;
    private const int LongReveal = 100;
    private const string RootA = "root-a";
    private const string RootB = "root-b";
    private const string OldLabel = "Old title";
    private const string NewLabel = "New title";

    [Fact]
    public async Task A_frontier_that_fails_to_arrive_is_a_failure_the_view_offers_to_try_again()
    {
        // Arrange
        var graph = new FailingGraph(Genesis2Graph(), failures: int.MaxValue);
        var explorer = new GraphExplorer(graph);
        var genesis2 = await explorer.BeginAt(ServedGraph.At(Genesis2Ref));

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, genesis2).Add(v => v.Surface, Surface.Popover));

        // Assert
        view.MarkupMatches(CouldNotLoad);
    }

    [Fact]
    public async Task Trying_again_after_a_failure_asks_again_and_presents_what_arrives()
    {
        // Arrange
        var graph = new FailingGraph(Genesis2Graph(), failures: 1);
        var explorer = new GraphExplorer(graph);
        var node = await explorer.BeginAt(ServedGraph.At(Genesis2Ref));
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));

        // Act
        await view.Find("[data-testid='could-not-load-retry']").ClickAsync(new());

        // Assert
        view.MarkupMatches(Genesis2Card + CitesFirstPageShown);
    }

    [Fact]
    public async Task A_failure_arriving_after_another_node_was_presented_is_dropped()
    {
        // Arrange
        var held = new HeldGraph(WithAdam(Genesis2Graph()), EdgeKind.Mentions, null);
        var explorer = new GraphExplorer(held);
        var genesis2 = await explorer.BeginAt(ServedGraph.At(Genesis2Ref));
        var adam = await explorer.BeginAt(ServedGraph.At(Adam));
        var view = Render<FocusView>(p => p.Add(v => v.Node, genesis2).Add(v => v.Surface, Surface.Popover));
        view.Render(p => p.Add(v => v.Node, adam));

        // Act
        held.Fail();
        await view.InvokeAsync(() => { });

        // Assert
        view.MarkupMatches(AdamCard);
    }

    [Fact]
    public async Task A_failure_arriving_after_the_view_is_gone_is_dropped()
    {
        // Arrange
        var held = new HeldGraph(Genesis2Graph(), EdgeKind.Mentions, null);
        var explorer = new GraphExplorer(held);
        var genesis2 = await explorer.BeginAt(ServedGraph.At(Genesis2Ref));
        Render<FocusView>(p => p.Add(v => v.Node, genesis2).Add(v => v.Surface, Surface.Popover));
        await DisposeComponentsAsync();

        // Act
        held.Fail();
        var failure = await Record.ExceptionAsync(() => Renderer.Dispatcher.InvokeAsync(() => { }));

        // Assert
        Assert.Null(failure);
    }

    [Fact]
    public async Task A_reveal_that_fails_to_arrive_is_a_failure_the_view_offers_to_try_again()
    {
        // Arrange
        var held = new HeldGraph(Genesis2Graph(), EdgeKind.Cites, CitesSecondPageCursor);
        var explorer = new GraphExplorer(held);
        var node = await explorer.BeginAt(ServedGraph.At(Genesis2Ref));
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));
        view.Find("[data-testid='popover-section-cites-more']").Click();

        // Act
        held.Fail();
        await view.InvokeAsync(() => { });

        // Assert
        view.WaitForAssertion(() => view.MarkupMatches(CouldNotLoad));
    }

    [Fact]
    public async Task Every_affordance_opens_on_one_bounded_page_however_large_its_group()
    {
        // Arrange
        var graph = new MultitudeGraph(Enum.GetValues<EdgeKind>());
        var explorer = new GraphExplorer(graph);
        var node = await explorer.BeginAt(ServedGraph.At(Genesis2Ref));

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));

        // Assert
        Assert.Equal(
            Enum.GetValues<EdgeKind>().Select(kind => (kind, Asked: 1, Read: Affordances.Of(kind).InitialClamp, Shown: Affordances.Of(kind).InitialClamp)),
            Enum.GetValues<EdgeKind>().Select(kind => (kind, Asked: graph.Asked(kind), Read: graph.Read(kind), Shown: view.FindAll($"[data-testid^='popover-entry-edge-{kind.WireName()}-{ServedGraph.EdgeId}:']").Count)));
    }

    [Fact]
    public async Task A_group_whose_links_the_surface_does_not_offer_is_not_drained_looking_for_one_it_does()
    {
        // Arrange
        var graph = new MultitudeGraph([EdgeKind.Shows]);
        var explorer = new GraphExplorer(graph);
        var node = await explorer.BeginAt(ServedGraph.At(Genesis2Ref));

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.World));

        // Assert
        Assert.Equal((1, Affordances.PageSize, 0), (graph.Asked(EdgeKind.Shows), graph.Read(EdgeKind.Shows), view.FindAll(".focus-child").Count));
    }

    [Fact]
    public async Task No_group_offers_to_reveal_all_of_a_collection_at_once()
    {
        // Arrange
        var graph = new MultitudeGraph(Enum.GetValues<EdgeKind>());
        var explorer = new GraphExplorer(graph);
        var node = await explorer.BeginAt(ServedGraph.At(Genesis2Ref));

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));

        // Assert
        Assert.Equal((true, 0), (view.FindAll("[data-testid$='-more']").Count > 0, view.FindAll("[data-testid$='-more-all']").Count));
    }

    [Fact]
    public async Task Revealing_more_of_inline_children_reads_one_more_bounded_page()
    {
        // Arrange
        var graph = new MultitudeGraph([EdgeKind.Contains]);
        var explorer = new GraphExplorer(graph);
        var node = await explorer.BeginAt(ServedGraph.At(Genesis2Ref));
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));

        // Act
        await view.Find("[data-testid='popover-children-contains-more']").ClickAsync(new());

        // Assert
        Assert.Equal((2, 2 * Affordances.PageSize, 2 * Affordances.PageSize), (graph.Asked(EdgeKind.Contains), graph.Read(EdgeKind.Contains), view.FindAll(".focus-child").Count));
    }

    [Fact]
    public async Task Every_affordance_asked_for_more_while_more_is_arriving_reads_one_page_at_a_time_and_shows_both()
    {
        // Arrange
        var graph = new MultitudeGraph(Enum.GetValues<EdgeKind>(), held: true);
        var explorer = new GraphExplorer(graph);
        var node = await explorer.BeginAt(ServedGraph.At(Genesis2Ref));
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));
        var mores = view.FindAll("[data-testid$='-more']").Select(more => more.GetAttribute("data-testid")).ToList();

        // Act
        foreach (var more in mores)
        {
            _ = view.Find($"[data-testid='{more}']").ClickAsync(new());
            _ = view.Find($"[data-testid='{more}']").ClickAsync(new());
        }

        while (await view.InvokeAsync(graph.Answer))
        {
        }

        // Assert
        Assert.Equal(
            Enum.GetValues<EdgeKind>().Select(kind => (kind, InFlight: 1, Shown: Math.Min(Revealed, PageWindow.BlocksShown(Affordances.Of(kind).InitialClamp)) * Affordances.Of(kind).InitialClamp)),
            Enum.GetValues<EdgeKind>().Select(kind => (kind, InFlight: graph.MostInFlight(kind), Shown: view.FindAll($"[data-testid^='popover-entry-edge-{kind.WireName()}-{ServedGraph.EdgeId}:']").Count)));
    }

    [Fact]
    public async Task Every_list_affordance_shows_a_bounded_window_at_every_position_of_a_long_reveal_and_back()
    {
        // Arrange
        var cardinalities = new[] { 300, 3_000, 30_000 };

        // Act
        var mostShown = new List<(int Cardinality, EdgeKind Kind, int Rows)>();
        foreach (var cardinality in cardinalities)
        {
            var graph = new MultitudeGraph(Listed, cardinality);
            var node = await new GraphExplorer(graph).BeginAt(ServedGraph.At(Genesis2Ref));
            var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));
            foreach (var kind in Listed)
            {
                var rows = new List<int> { Rows(view, kind) };
                foreach (var turn in new[] { "more", "collapse" })
                {
                    for (var step = 0; step < LongReveal && view.FindAll($"[data-testid='{Handle(kind)}-{turn}']") is [var offered]; step++)
                    {
                        await offered.ClickAsync(new());
                        rows.Add(Rows(view, kind));
                    }
                }

                mostShown.Add((cardinality, kind, rows.Max()));
            }
        }

        // Assert
        Assert.Equal(
            cardinalities.SelectMany(cardinality => Listed.Select(kind => (cardinality, kind, MostRowsShown))),
            mostShown);
    }

    [Fact]
    public async Task The_controls_state_the_rows_shown_and_offer_only_the_ways_that_lead_somewhere()
    {
        // Arrange
        var graph = new MultitudeGraph([EdgeKind.Contains], Cardinality);
        var node = await new GraphExplorer(graph).BeginAt(ServedGraph.At(Genesis2Ref));
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));
        var section = Handle(EdgeKind.Contains);
        var opened = Controls(view, section);

        // Act
        while (view.FindAll($"[data-testid='{section}-more']") is [var more])
        {
            await more.ClickAsync(new());
        }

        var atTheEnd = Controls(view, section);
        while (view.FindAll($"[data-testid='{section}-collapse']") is [var less])
        {
            await less.ClickAsync(new());
        }

        // Assert
        Assert.Equal(
            [
                ($"1–{Affordances.PageSize} of {Cardinality}", false, true),
                ($"{Cardinality - MostRowsShown + 1}–{Cardinality} of {Cardinality}", true, false),
                ($"1–{Affordances.PageSize} of {Cardinality}", false, true),
            ],
            new[] { opened, atTheEnd, Controls(view, section) });
    }

    [Fact]
    public async Task Less_after_more_reads_back_from_the_page_store_without_asking_again()
    {
        // Arrange
        var graph = new MultitudeGraph([EdgeKind.Contains]);
        var node = await new GraphExplorer(graph).BeginAt(ServedGraph.At(Genesis2Ref));
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));
        for (var more = 0; more < PageWindow.BlocksShown(Affordances.PageSize) + 1; more++)
        {
            await view.Find("[data-testid='popover-children-contains-more']").ClickAsync(new());
        }

        var asked = graph.Asked(EdgeKind.Contains);

        // Act
        for (var less = 0; less < PageWindow.BlocksShown(Affordances.PageSize) + 1; less++)
        {
            await view.Find("[data-testid='popover-children-contains-collapse']").ClickAsync(new());
        }

        // Assert
        Assert.Equal(
            (asked, WholeValue.Of(Enumerable.Range(0, Affordances.PageSize).Select(n => $"popover-child-contains-Person:{n}"))),
            (graph.Asked(EdgeKind.Contains), WholeValue.Of(view.FindAll(".focus-child").Select(child => child.GetAttribute("data-testid")))));
    }

    private const int Cardinality = 300;

    private const int MostRowsShown = 40;

    private static readonly EdgeKind[] Listed = [EdgeKind.Contains, EdgeKind.Cites, EdgeKind.MentionedIn];

    private static string Handle(EdgeKind kind) =>
        Affordances.Of(kind) is Affordance.InlineChildren ? $"popover-children-{kind.WireName()}" : $"popover-section-{kind.WireName()}";

    private static int Rows(IRenderedComponent<FocusView> view, EdgeKind kind) =>
        view.FindAll($"[data-testid^='popover-entry-edge-{kind.WireName()}-{ServedGraph.EdgeId}:']").Count;

    private static (string Position, bool Less, bool More) Controls(IRenderedComponent<FocusView> view, string section) =>
        (view.Find($"[data-testid='{section}-position']").TextContent,
            view.FindAll($"[data-testid='{section}-collapse']").Count == 1,
            view.FindAll($"[data-testid='{section}-more']").Count == 1);

    [Fact]
    public async Task Fewer_after_a_long_reveal_slides_the_window_back_over_what_it_let_go()
    {
        // Arrange
        var graph = new MultitudeGraph([EdgeKind.Contains]);
        var node = await new GraphExplorer(graph).BeginAt(ServedGraph.At(Genesis2Ref));
        var view = Render<FocusView>(p => p.Add(v => v.Node, node).Add(v => v.Surface, Surface.Popover));
        for (var more = 0; more < PageWindow.BlocksShown(Affordances.PageSize); more++)
        {
            await view.Find("[data-testid='popover-children-contains-more']").ClickAsync(new());
        }

        // Act
        await view.Find("[data-testid='popover-children-contains-collapse']").ClickAsync(new());

        // Assert
        Assert.Equal(
            Enumerable.Range(0, PageWindow.ShownEntries).Select(n => $"popover-child-contains-Person:{n}"),
            view.FindAll(".focus-child").Select(child => child.GetAttribute("data-testid")));
    }

    [Fact]
    public void Every_element_served_again_from_a_new_artifact_is_presented_afresh()
    {
        // Arrange
        var mention = ServedGraph.EdgeRef(EdgeKind.Mentions, "Mentions:00ee", "Old mention");
        var elements = new (PositionRef Target, Func<string, ServedGraph> Served)[]
        {
            (ServedGraph.At(EdenMap), label => new ServedGraph().Serving(ServedGraph.Card(NodeKind.Map, EdenMap.Id, label) with { Map = ServedGraph.MapWindow(EdenWindow) })),
            (ServedGraph.AtEdge(mention), label => new ServedGraph().Serving(ServedGraph.EdgeRecordOf(ServedGraph.EdgeRef(EdgeKind.Mentions, mention.Id, label), Verse1, Eden))),
        };

        // Act
        var titles = elements.Select(element =>
        {
            var view = Render<FocusView>(p => p.Add(v => v.Node, Resolved.At(element.Served(OldLabel).AtRoot(RootA), element.Target)).Add(v => v.Surface, Surface.Popover));
            view.Render(p => p.Add(v => v.Node, Resolved.At(element.Served(NewLabel).AtRoot(RootB), element.Target)));
            return view.Find(".focus-title").TextContent;
        }).ToList();

        // Assert
        Assert.Equal([NewLabel, NewLabel], titles);
    }

    [Fact]
    public async Task Changing_the_surface_recomputes_the_links_offered()
    {
        // Arrange
        var graph = EdenMapGraph();
        var explorer = new GraphExplorer(graph);
        var map = await explorer.BeginAt(ServedGraph.At(EdenMap));
        var view = Render<FocusView>(p => p.Add(v => v.Node, map).Add(v => v.Surface, Surface.Popover));

        // Act
        view.Render(p => p.Add(v => v.Surface, Surface.World));

        // Assert
        Assert.Equal(["world-child-shows-Place:eden"], view.FindAll(".focus-child").Select(child => child.GetAttribute("data-testid")));
    }

    [Fact]
    public void Every_surface_transition_presents_what_a_fresh_presentation_on_the_new_surface_does()
    {
        // Arrange
        var graph = EdenMapGraph();
        var map = Resolved.Node(graph, EdenMap);
        var transitions = Enum.GetValues<Surface>().SelectMany(from => Enum.GetValues<Surface>(), (from, to) => (From: from, To: to)).ToList();

        // Act
        var differing = transitions.Where(transition =>
        {
            var view = Render<FocusView>(p => p.Add(v => v.Node, map).Add(v => v.Surface, transition.From));
            view.Render(p => p.Add(v => v.Surface, transition.To));
            var fresh = Render<FocusView>(p => p.Add(v => v.Node, map).Add(v => v.Surface, transition.To));
            return Record.Exception(() => view.MarkupMatches(fresh.Markup)) is not null;
        }).ToList();

        // Assert
        Assert.Empty(differing);
    }

    [Fact]
    public void A_text_shows_its_served_words_with_each_anchor_as_a_link_of_its_served_kind()
    {
        // Arrange
        var verse = Resolved.Node(new ServedGraph().Serving(ServedGraph.TextCard(John316, John316Text)), John316);

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, verse).Add(v => v.Surface, Surface.Popover));

        // Assert
        view.Find("[data-testid='popover-text']").MarkupMatches("""
            <p class="focus-text" data-testid="popover-text">For God so loved the <button type="button" class="focus-anchor explorable" data-testid="popover-anchor-mentions-Place:world-0">world</button>, <span class="words-of-christ">that he gave his only begotten Son</span></p>
            """);
    }

    [Fact]
    public void Following_an_anchor_follows_a_link_of_the_anchors_kind_to_its_node()
    {
        // Arrange
        var followed = new List<Link>();
        var paragraph = Resolved.Node(new ServedGraph().Serving(ServedGraph.TextCard(AugsburgIv, AugsburgIvText)), AugsburgIv);
        var view = Render<FocusView>(p => p.Add(v => v.Node, paragraph).Add(v => v.Surface, Surface.Popover).Add(v => v.OnFollow, link => followed.Add(link)));

        // Act
        view.Find($"[data-testid='popover-anchor-cites-{John316.Id}-{FirstAnchor}']").Click();

        // Assert
        Assert.Equal([new Link(EdgeKind.Cites, ServedGraph.At(John316))], followed);
    }

    [Fact]
    public void The_words_of_Christ_are_marked_as_served()
    {
        // Arrange
        var verse = Resolved.Node(new ServedGraph().Serving(ServedGraph.TextCard(John316, John316Text)), John316);

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, verse).Add(v => v.Surface, Surface.Popover));

        // Assert
        Assert.Equal([John316Words[RedLetterStart..]], view.FindAll("[data-testid='popover-text'] .words-of-christ").Select(red => red.TextContent).ToList());
    }

    [Fact]
    public void A_verse_offers_its_neighbouring_verses_as_arrows_its_chapter_as_a_crumb_and_its_other_neighbours_as_sections()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.TextCard(
                John316,
                John316Text,
                new FrontierGroup(EdgeKind.FollowsIn, 1),
                new FrontierGroup(EdgeKind.PrecedesIn, 1),
                new FrontierGroup(EdgeKind.MemberOf, 1),
                new FrontierGroup(EdgeKind.Cites, 1),
                new FrontierGroup(EdgeKind.Attests, 1),
                new FrontierGroup(EdgeKind.CatechismLink, 1)))
            .Serving(John316.Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, John317))
            .Serving(John316.Id, EdgeKind.PrecedesIn, null, ServedGraph.Page(EdgeKind.PrecedesIn, null, John315))
            .Serving(John316.Id, EdgeKind.MemberOf, null, ServedGraph.Page(EdgeKind.MemberOf, null, John3))
            .Serving(John316.Id, EdgeKind.Cites, null, ServedGraph.Page(EdgeKind.Cites, null, Romans58))
            .Serving(John316.Id, EdgeKind.Attests, null, ServedGraph.Page(EdgeKind.Attests, null, Nicodemus))
            .Serving(John316.Id, EdgeKind.CatechismLink, null, ServedGraph.Page(EdgeKind.CatechismLink, null, SecondArticle));

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, Resolved.Node(graph, John316)).Add(v => v.Surface, Surface.Popover));

        // Assert
        Assert.Equal(
            [
                $"popover-up-member-of-{John3.Id}", EdgeStepHandle(EdgeKind.MemberOf, John3),
                "popover-section-text", "popover-text", $"popover-anchor-mentions-{World.Id}-{FirstAnchor}", "popover-field-Provenance",
                "popover-next", EdgeStepHandle(EdgeKind.FollowsIn, John317),
                "popover-prev", EdgeStepHandle(EdgeKind.PrecedesIn, John315),
                "popover-section-cites", "popover-section-cites-heading", $"popover-link-cites-{Romans58.Id}", EdgeStepHandle(EdgeKind.Cites, Romans58), $"popover-words-cites-{Romans58.Id}-text",
                "popover-section-attests", "popover-section-attests-heading", $"popover-link-attests-{Nicodemus.Id}", EdgeStepHandle(EdgeKind.Attests, Nicodemus),
                "popover-section-catechism-link", "popover-section-catechism-link-heading", $"popover-link-catechism-link-{SecondArticle.Id}", EdgeStepHandle(EdgeKind.CatechismLink, SecondArticle),
            ],
            view.FindAll("[data-testid]").Select(element => element.GetAttribute("data-testid")).ToList());
    }

    [Fact]
    public void A_list_whose_neighbours_are_text_units_shows_each_ones_served_words_under_its_link()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.TextCard(John316, John316Text, new FrontierGroup(EdgeKind.Cites, 1)))
            .Serving(ServedGraph.TextCard(Romans58, Romans58Text))
            .Serving(John316.Id, EdgeKind.Cites, null, ServedGraph.Page(EdgeKind.Cites, null, Romans58));

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, Resolved.Node(graph, John316)).Add(v => v.Surface, Surface.Popover));

        // Assert
        view.Find("[data-testid='popover-section-cites']").MarkupMatches($$"""
            <div class="popover-section" data-testid="popover-section-cites">
              <p class="catechism-section-heading" data-testid="popover-section-cites-heading">Cites (1)</p>
              <button type="button" class="focus-link explorable-quiet" data-testid="popover-link-cites-{{Romans58.Id}}">{{Romans58.Label}}</button>
              <button type="button" class="focus-entry-edge explorable-quiet" data-testid="{{EdgeStepHandle(EdgeKind.Cites, Romans58)}}" aria-label:ignore title:ignore>&#8942;</button>
              <p class="focus-text" data-testid="popover-words-cites-{{Romans58.Id}}-text">{{Romans58Words}}</p>
            </div>
            """);
    }

    [Fact]
    public void The_words_of_a_page_of_text_units_are_read_in_one_element_read()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.TextCard(John316, John316Text, new FrontierGroup(EdgeKind.Cites, Citations.Length)))
            .Serving(John316.Id, EdgeKind.Cites, null, ServedGraph.Page(EdgeKind.Cites, null, Citations));
        foreach (var citation in Citations)
        {
            graph.Serving(ServedGraph.TextCard(citation, Romans58Text));
        }

        var verse = Resolved.Node(graph, John316);
        var resolving = graph.ElementReads;

        // Act
        Render<FocusView>(p => p.Add(v => v.Node, verse).Add(v => v.Surface, Surface.Popover));

        // Assert
        Assert.Equal(1, graph.ElementReads - resolving);
    }

    [Fact]
    public void A_list_whose_neighbours_are_not_text_units_reads_no_words()
    {
        // Arrange
        var graph = new ServedGraph()
            .Serving(ServedGraph.TextCard(John316, John316Text, new FrontierGroup(EdgeKind.Attests, 1)))
            .Serving(John316.Id, EdgeKind.Attests, null, ServedGraph.Page(EdgeKind.Attests, null, Nicodemus));
        var verse = Resolved.Node(graph, John316);
        var resolving = graph.ElementReads;

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, verse).Add(v => v.Surface, Surface.Popover));

        // Assert
        Assert.Equal((0, 0), (graph.ElementReads - resolving, view.FindAll("[data-testid='popover-section-attests'] .focus-text").Count));
    }

    [Fact]
    public void A_text_shows_its_fields_as_a_card_does()
    {
        // Arrange
        var verse = Resolved.Node(new ServedGraph().Serving(ServedGraph.TextCard(John316, John316Text)), John316);

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, verse).Add(v => v.Surface, Surface.Popover));

        // Assert
        view.Find(".focus-fields").MarkupMatches("""
            <dl class="focus-fields">
                <div class="focus-field" data-testid="popover-field-Provenance"><dt>Provenance</dt><dd>kjv</dd></div>
            </dl>
            """);
    }

    [Fact]
    public void A_verse_offers_its_host_the_way_to_read_it_in_context()
    {
        // Arrange
        var verse = Resolved.Node(new ServedGraph().Serving(ServedGraph.TextCard(John316, John316Text)), John316);

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, verse).Add(v => v.Surface, Surface.Popover).Add(v => v.OnReadInContext, (BibleRef _) => { }));

        // Assert
        view.Find("[data-testid='popover-chip-context']").MarkupMatches(WayToTheReader);
    }

    [Fact]
    public void Choosing_the_way_to_read_in_context_asks_the_host_to_read_at_the_verses_served_locus()
    {
        // Arrange
        var asked = new List<BibleRef>();
        var verse = Resolved.Node(new ServedGraph().Serving(ServedGraph.TextCard(John316, John316Text)), John316);
        var view = Render<FocusView>(p => p.Add(v => v.Node, verse).Add(v => v.Surface, Surface.Popover).Add(v => v.OnReadInContext, (BibleRef locus) => asked.Add(locus)));

        // Act
        view.Find("[data-testid='popover-chip-context']").Click();

        // Assert
        Assert.Equal([John316Text.Locus], asked);
    }

    [Fact]
    public void A_paragraph_offers_no_way_to_read_a_verse_in_context()
    {
        // Arrange
        var paragraph = Resolved.Node(new ServedGraph().Serving(ServedGraph.TextCard(AugsburgIv, AugsburgIvText)), AugsburgIv);

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, paragraph).Add(v => v.Surface, Surface.Popover).Add(v => v.OnReadInContext, (BibleRef _) => { }));

        // Assert
        Assert.Empty(view.FindAll("[data-testid='popover-chip-context']"));
    }

    [Fact]
    public void A_host_that_offers_no_way_to_the_reader_shows_none()
    {
        // Arrange
        var verse = Resolved.Node(new ServedGraph().Serving(ServedGraph.TextCard(John316, John316Text)), John316);

        // Act
        var view = Render<FocusView>(p => p.Add(v => v.Node, verse).Add(v => v.Surface, Surface.Popover));

        // Assert
        Assert.Empty(view.FindAll("[data-testid='popover-chip-context']"));
    }

    private static readonly NodeRef Genesis2Ref = ServedGraph.Ref(NodeKind.Container, Genesis2Id, Genesis2Label);

    private static Explorable Genesis2(ServedGraph graph) => Resolved.Node(graph, Genesis2Ref);

    private static ServedGraph WithAdam(ServedGraph graph) =>
        graph
            .Serving(ServedGraph.Card(NodeKind.Person, Adam.Id, Adam.Label, new FrontierGroup(EdgeKind.MentionedIn, 1)))
            .Serving(Adam.Id, EdgeKind.MentionedIn, null, ServedGraph.Page(EdgeKind.MentionedIn, null, Verse1));

    private static ServedGraph EdenMapGraph() =>
        new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Map, EdenMap.Id, EdenMap.Label, new FrontierGroup(EdgeKind.Shows, 2), new FrontierGroup(EdgeKind.MentionedIn, 1)) with { Map = ServedGraph.MapWindow(EdenWindow) })
            .Serving(EdenMap.Id, EdgeKind.Shows, null, ServedGraph.Page(EdgeKind.Shows, null, Eden, Adam))
            .Serving(EdenMap.Id, EdgeKind.MentionedIn, null, ServedGraph.Page(EdgeKind.MentionedIn, null, Verse1));

    private sealed class FailingGraph(ServedGraph served, int failures) : IExplorableClient
    {
        private int _failed;

        public Task<NodeRecord> Card(string id) => served.Card(id);

        public Task<ElementPage> Elements(IReadOnlyList<string> ids) => served.Elements(ids);

        public Task<EdgePage> Edges(string positionId, EdgeKind kind, int? cursor = null, int limit = BibleAtlas.Client.Exploring.Affordances.PageSize) =>
            _failed++ < failures ? Task.FromException<EdgePage>(new HttpRequestException(Offline)) : served.Edges(positionId, kind, cursor, limit);

        public Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
            served.Reading(fromRef, n, dir, corpus);
    }

    private sealed class MultitudeGraph(IReadOnlyList<EdgeKind> kinds, int size = Multitude, bool held = false) : IExplorableClient
    {
        private readonly ServedGraph _cards = new ServedGraph().Serving(ServedGraph.Card(NodeKind.Container, Genesis2Id, Genesis2Label, kinds.Select(kind => new FrontierGroup(kind, size)).ToArray()));
        private readonly Dictionary<EdgeKind, (int Asked, int Read)> _reads = [];
        private readonly List<(EdgeKind Kind, EdgePage Page, TaskCompletionSource<EdgePage> Answer)> _pending = [];
        private readonly Dictionary<EdgeKind, int> _mostInFlight = [];

        public int Asked(EdgeKind kind) => _reads.GetValueOrDefault(kind).Asked;

        public int Read(EdgeKind kind) => _reads.GetValueOrDefault(kind).Read;

        public int MostInFlight(EdgeKind kind) => _mostInFlight.GetValueOrDefault(kind);

        public bool Answer()
        {
            if (_pending is not [var pending, ..])
            {
                return false;
            }

            _pending.RemoveAt(0);
            pending.Answer.SetResult(pending.Page);
            return true;
        }

        public Task<NodeRecord> Card(string id) => _cards.Card(id);

        public Task<ElementPage> Elements(IReadOnlyList<string> ids) => _cards.Elements(ids);

        public Task<EdgePage> Edges(string positionId, EdgeKind kind, int? cursor = null, int limit = BibleAtlas.Client.Exploring.Affordances.PageSize)
        {
            var from = cursor ?? 0;
            var to = Math.Min(from + limit, size);
            var (asked, read) = _reads.GetValueOrDefault(kind);
            _reads[kind] = (asked + 1, read + to - from);
            var people = Enumerable.Range(from, to - from).Select(n => ServedGraph.Ref(NodeKind.Person, $"Person:{n}", $"Person {n}")).ToArray();
            var page = ServedGraph.Page(kind, to < size ? to : null, people) with { Previous = ServedGraph.PageBefore(from, limit) };
            if (!held || cursor is null)
            {
                return Task.FromResult(page);
            }

            var answer = new TaskCompletionSource<EdgePage>();
            _pending.Add((kind, page, answer));
            _mostInFlight[kind] = Math.Max(MostInFlight(kind), _pending.Count(pending => pending.Kind == kind));
            return answer.Task;
        }

        public Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
            throw new NotSupportedException();
    }

    private const string Offline = "offline";

    private sealed class HeldGraph(ServedGraph served, EdgeKind heldKind, int? heldCursor) : IExplorableClient
    {
        private readonly TaskCompletionSource _release = new();

        public void Release() => _release.SetResult();

        public void Fail() => _release.SetException(new HttpRequestException(Offline));

        public Task<NodeRecord> Card(string id) => served.Card(id);

        public Task<ElementPage> Elements(IReadOnlyList<string> ids) => served.Elements(ids);

        public async Task<EdgePage> Edges(string positionId, EdgeKind kind, int? cursor = null, int limit = BibleAtlas.Client.Exploring.Affordances.PageSize)
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
            $"""<button type="button" class="focus-link explorable-quiet" data-testid="popover-link-cites-{verse.Id}">{verse.Label}</button>{EdgeStep(EdgeKind.Cites, verse)}{Worded($"popover-words-cites-{verse.Id}", verse)}"""));

    private static string Worded(string handle, NodeRef unit) =>
        $"""<p class="focus-text" data-testid="{handle}-text">{unit.Id}</p>""";
}
