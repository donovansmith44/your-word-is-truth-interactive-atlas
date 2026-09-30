using BibleAtlas.Client.Components;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.State;
using Bunit;
using Microsoft.Extensions.DependencyInjection;

namespace BibleAtlas.Client.Tests;

public sealed class ExplorerPopoverTests : BunitContext
{
    private static readonly NodeRef Exodus = ServedGraph.Ref(NodeKind.Narrative, "Narrative:exodus", "The Exodus");
    private static readonly NodeRef Wilderness = ServedGraph.Ref(NodeKind.Narrative, "Narrative:wilderness", "The Wilderness");
    private static readonly SavedExploration AtExodus = new("seed", "Seed", DateTimeOffset.UnixEpoch, Exodus, []);

    private const string ExodusPresented = """
        <div class="popover-body" data-testid="popover-body">
            <div class="popover-section" data-testid="popover-section-card">
                <p class="focus-title" data-testid="popover-card-title">The Exodus</p>
                <dl class="focus-fields">
                    <div class="focus-field" data-testid="popover-field-Provenance"><dt>Provenance</dt><dd>kjv</dd></div>
                </dl>
            </div>
            <button type="button" class="focus-arrow explorable" data-testid="popover-next">The Wilderness ›</button>
        </div>
        """;

    [Fact]
    public void A_node_no_legacy_provider_serves_is_presented_as_its_card_and_frontier()
    {
        // Arrange
        Hosting(Narratives());

        // Act
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Saved, AtExodus));

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
        var popover = Render<ExplorerPopover>(p => p.Add(v => v.Saved, AtExodus));

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

    private static ServedGraph Narratives() =>
        new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.Narrative, Exodus.Id, Exodus.Label, new FrontierGroup(EdgeKind.FollowsIn, 1)))
            .Serving(Exodus.Id, EdgeKind.FollowsIn, null, ServedGraph.Page(EdgeKind.FollowsIn, null, Wilderness))
            .Serving(ServedGraph.Card(NodeKind.Narrative, Wilderness.Id, Wilderness.Label));

    private StateAtom<ExplorationState> Hosting(ServedGraph graph)
    {
        JSInterop.Mode = JSRuntimeMode.Loose;
        Services.AddSingleton(new AtlasClient(new HttpClient { BaseAddress = new Uri("http://unserved.invalid") }));
        Services.AddSingleton<IExplorableClient>(graph);
        Services.AddSingleton<IExplorer>(new GraphExplorer(graph));
        Services.AddSingleton(new SavedExplorationsService(new InMemoryLocalStorage()));
        Services.AddSingleton<OwnershipRegistry>();
        Services.AddSingleton(new StateAtom<IReadOnlyList<NodeRef>>(AtomNames.Selection, Selection.Empty, SequenceEqualityComparer<NodeRef>.Instance));
        AppServices.AddStateAtoms(Services);
        return Services.GetRequiredService<StateAtom<ExplorationState>>();
    }
}
