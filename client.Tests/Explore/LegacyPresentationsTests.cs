using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class LegacyPresentationsTests
{
    private static readonly NodeRef Exodus = ServedGraph.Ref(NodeKind.Narrative, "Narrative:exodus", "The Exodus");
    private static readonly NodeRef Wilderness = ServedGraph.Ref(NodeKind.Narrative, "Narrative:wilderness", "The Wilderness");

    [Fact]
    public void Back_from_an_events_time_restores_the_event_body()
    {
        // Arrange
        var terah = Resolved.Node(LegacyViews.TerahLeavesUr);
        var views = new LegacyPresentations();
        var trail = new Exploration(terah, []);
        var eventBody = views.Present(trail);
        var dated = trail.Follow(new Step(EdgeKind.DatedBy, terah));
        views.Arrive(dated, new YearNode(LegacyViews.TwoThousandBc, LegacyViews.TerahLeavesUr));

        // Act
        var restored = views.Present(dated.WalkedBack());

        // Assert
        Assert.Same(eventBody, restored);
    }

    [Fact]
    public void The_laws_below_enumerate_every_legacy_view_there_is()
    {
        // Arrange
        var views = typeof(IExplorable).Assembly.GetTypes().Where(type => type is { IsClass: true, IsAbstract: false } && typeof(IExplorable).IsAssignableFrom(type));

        // Act
        var enumerated = LegacyViews.Every().Select(view => view.GetType());

        // Assert
        Assert.Equal(views.OrderBy(type => type.Name), enumerated.OrderBy(type => type.Name));
    }

    [Fact]
    public void Back_from_any_view_pushed_onto_its_own_node_restores_the_view_that_node_had()
    {
        // Arrange
        var laws = LegacyViews.Every().Select(view =>
        {
            var node = Resolved.Node(view.Identity);
            var views = new LegacyPresentations();
            var trail = new Exploration(node, []);
            var before = views.Present(trail);
            var pushed = trail.Follow(new Step(EdgeKind.DatedBy, node));
            views.Arrive(pushed, view);
            return (View: view, Before: before, Views: views, Pushed: pushed);
        }).ToList();

        // Act
        var outcomes = laws.Select(law => (law.View.GetType().Name, Forward: law.Views.Present(law.Pushed), Back: law.Views.Present(law.Pushed.WalkedBack()))).ToList();

        // Assert
        Assert.Equal(
            laws.Select(law => (law.View.GetType().Name, true, true)),
            outcomes.Zip(laws, (outcome, law) => (outcome.Name, ReferenceEquals(outcome.Forward, law.View), ReferenceEquals(outcome.Back, law.Before))));
    }

    [Fact]
    public void Back_onto_any_pushed_view_restores_that_view()
    {
        // Arrange
        var graph = Narratives();
        var wilderness = Resolved.Node(graph, Wilderness);
        var laws = LegacyViews.Every().Select(view =>
        {
            var views = new LegacyPresentations();
            var start = new Exploration(Resolved.Node(graph, Exodus), []);
            var pushed = start.Follow(new Step(EdgeKind.Mentions, Resolved.Node(view.Identity)));
            views.Present(start);
            views.Arrive(pushed, view);
            var onward = pushed.Follow(new Step(EdgeKind.FollowsIn, wilderness));
            views.Present(onward);
            return (View: view, Views: views, Back: onward.WalkedBack());
        }).ToList();

        // Act
        var restored = laws.Select(law => law.Views.Present(law.Back)).ToList();

        // Assert
        Assert.Equal(laws.Select(law => (law.View.GetType().Name, true)), laws.Zip(restored, (law, view) => (law.View.GetType().Name, ReferenceEquals(view, law.View))));
    }

    [Fact]
    public void A_node_followed_to_afresh_after_backing_off_a_view_pushed_onto_it_presents_its_own_view()
    {
        // Arrange
        var graph = Narratives();
        var laws = LegacyViews.Every().Select(view =>
        {
            var node = Resolved.Node(view.Identity);
            var views = new LegacyPresentations();
            var start = new Exploration(Resolved.Node(graph, Exodus), []);
            var pushed = start.Follow(new Step(EdgeKind.Mentions, node));
            views.Present(start);
            views.Arrive(pushed, view);
            views.Present(pushed.WalkedBack());
            return (View: view, Node: node, Views: views, Again: pushed.WalkedBack().Follow(new Step(EdgeKind.Mentions, node)));
        }).ToList();

        // Act
        var presented = laws.Select(law => law.Views.Present(law.Again)).ToList();

        // Assert
        Assert.Equal(
            laws.Select(law => (law.View.GetType().Name, LegacyNodes.For(law.Node)?.GetType())),
            laws.Zip(presented, (law, view) => (law.View.GetType().Name, ReferenceEquals(view, law.View) ? typeof(void) : view?.GetType())));
    }

    [Fact]
    public void A_node_presented_again_presents_the_view_it_presented_before()
    {
        // Arrange
        var views = new LegacyPresentations();
        var trail = new Exploration(Resolved.Node(LegacyViews.TerahLeavesUr), []);
        var first = views.Present(trail);

        // Act
        var again = views.Present(trail);

        // Assert
        Assert.Same(first, again);
    }

    [Fact]
    public void A_kind_with_no_legacy_body_presents_nothing()
    {
        // Arrange
        var views = new LegacyPresentations();

        // Act
        var presented = views.Present(new Exploration(Resolved.Node(Narratives(), Exodus), []));

        // Assert
        Assert.Null(presented);
    }

    private static ServedGraph Narratives() =>
        new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Narrative, Exodus.Id, Exodus.Label))
            .Serving(ServedGraph.Card(NodeKind.Narrative, Wilderness.Id, Wilderness.Label));
}
