using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class SavedExplorationsTests
{
    private const string V3Key = "explorations-v3";
    private const string V2Key = "explorations-v2";
    private const string V1Key = "explorations-v1";
    private const string V1Store = """
        [{"id":"seed1","name":"GEN.1.1 → Moses","createdUtc":"2026-09-01T00:00:00+00:00",
          "nodes":[{"kind":"Verse","key":"GEN.1.1","title":"GEN.1.1","isGeneralKind":false},
                   {"kind":"Author","key":"GEN","title":"Moses","isGeneralKind":false},
                   {"kind":"Book","key":"GEN","title":"GEN","isGeneralKind":false}]}]
        """;

    private static readonly string V2Store = $$$"""
        [{"id":"seed2","name":"GEN.1.1 → Genesis","createdUtc":"2026-09-01T00:00:00+00:00",
          "start":{"id":"text-unit:GEN.1.1","kind":"TextUnit","label":"GEN.1.1"},
          "steps":[{"kind":{{{(int)EdgeKind.MemberOf}}},"target":{"id":"Container:bible-book-GEN","kind":"Container","label":"Genesis"}}]}]
        """;

    private static readonly DateTimeOffset Seeded = DateTimeOffset.Parse("2026-09-01T00:00:00+00:00");
    private static readonly Explorable Genesis1_1 = Resolved.Node(NodeKind.TextUnit, "text-unit:GEN.1.1", "GEN.1.1");
    private static readonly Explorable Genesis = Resolved.Node(NodeKind.Container, "Container:bible-book-GEN", "Genesis");
    private static readonly NodeRef Genesis1_1Ref = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:GEN.1.1", "GEN.1.1");
    private static readonly NodeRef GenesisRef = ServedGraph.Ref(NodeKind.Container, "Container:bible-book-GEN", "Genesis");
    private static readonly NodeRef LegacyGenesisRef = ServedGraph.Ref(NodeKind.Container, "Container:bible-book-GEN", "GEN");
    private static readonly Exploration Genesis1_1ToGenesis = new(Genesis1_1, [new Step(EdgeKind.MemberOf, Genesis)]);
    private static readonly SavedExploration Seed1 = new("seed1", "GEN.1.1 → Moses", Seeded, ServedGraph.At(Genesis1_1Ref), [new Link(EdgeKind.MemberOf, ServedGraph.At(LegacyGenesisRef))]);
    private static readonly SavedExploration Seed2 = new("seed2", "GEN.1.1 → Genesis", Seeded, ServedGraph.At(Genesis1_1Ref), [new Link(EdgeKind.MemberOf, ServedGraph.At(GenesisRef))]);

    [Fact]
    public void Saving_an_exploration_names_it_after_its_start_and_end_and_keeps_every_step()
    {
        // Arrange
        var service = new SavedExplorationsService(new InMemoryLocalStorage());

        // Act
        var saved = service.Save(Genesis1_1ToGenesis);

        // Assert
        Assert.Equal(
            (new SavedExploration(saved.Id, "GEN.1.1 → Genesis", saved.CreatedUtc, ServedGraph.At(Genesis1_1Ref), [new Link(EdgeKind.MemberOf, ServedGraph.At(GenesisRef))]), true),
            (saved, service.Items.SequenceEqual([saved])));
    }

    [Fact]
    public void Saving_an_exploration_that_went_nowhere_names_it_after_its_start()
    {
        // Arrange
        var service = new SavedExplorationsService(new InMemoryLocalStorage());

        // Act
        var saved = service.Save(new Exploration(Genesis1_1, []));

        // Assert
        Assert.Equal(new SavedExploration(saved.Id, "GEN.1.1", saved.CreatedUtc, ServedGraph.At(Genesis1_1Ref), []), saved);
    }

    [Fact]
    public void What_is_saved_is_stored_under_explorations_v3_and_read_back_by_the_next_session()
    {
        // Arrange
        var store = new InMemoryLocalStorage();
        var session = new SavedExplorationsService(store);
        session.Save(Genesis1_1ToGenesis);

        // Act
        var nextSession = new SavedExplorationsService(store);

        // Assert
        Assert.Equal(
            WholeValue.Of(new { session.Items, Keys = new[] { V3Key } }),
            WholeValue.Of(new { nextSession.Items, Keys = store.Items.Keys }));
    }

    [Fact]
    public void A_v1_store_is_translated_once_into_v3_with_its_dropped_steps_counted_and_v1_left_in_place()
    {
        // Arrange
        var store = new InMemoryLocalStorage();
        store.Items[V1Key] = V1Store;

        // Act
        var service = new SavedExplorationsService(store);

        // Assert
        Assert.Equal(
            WholeValue.Of(new { Items = new[] { Seed1 }, Dropped = 1, Keys = new[] { V1Key, V3Key }, NextSession = new[] { Seed1 } }),
            WholeValue.Of(new { service.Items, service.Dropped, Keys = store.Items.Keys, NextSession = new SavedExplorationsService(store).Items }));
    }

    [Fact]
    public void The_session_after_the_translation_reads_v3_and_drops_nothing()
    {
        // Arrange
        var store = new InMemoryLocalStorage();
        store.Items[V1Key] = V1Store;
        _ = new SavedExplorationsService(store);

        // Act
        var nextSession = new SavedExplorationsService(store);

        // Assert
        Assert.Equal(0, nextSession.Dropped);
    }

    [Fact]
    public void An_emptied_v3_store_is_not_refilled_from_v2_or_v1()
    {
        // Arrange
        var store = new InMemoryLocalStorage();
        store.Items[V1Key] = V1Store;
        store.Items[V2Key] = V2Store;
        store.Items[V3Key] = "[]";

        // Act
        var service = new SavedExplorationsService(store);

        // Assert
        Assert.Equal(
            WholeValue.Of(new { Items = Array.Empty<SavedExploration>(), Dropped = 0 }),
            WholeValue.Of(new { service.Items, service.Dropped }));
    }

    [Fact]
    public void A_v2_store_is_translated_whole_into_v3_before_v1_is_read_and_v2_left_in_place()
    {
        // Arrange
        var store = new InMemoryLocalStorage();
        store.Items[V1Key] = V1Store;
        store.Items[V2Key] = V2Store;

        // Act
        var service = new SavedExplorationsService(store);

        // Assert
        Assert.Equal(
            WholeValue.Of(new { Items = new[] { Seed2 }, Dropped = 0, Keys = new[] { V1Key, V2Key, V3Key }, NextSession = new[] { Seed2 } }),
            WholeValue.Of(new { service.Items, service.Dropped, Keys = store.Items.Keys, NextSession = new SavedExplorationsService(store).Items }));
    }

    [Fact]
    public void A_saved_step_onto_an_edge_is_read_back_by_the_next_session()
    {
        // Arrange
        var store = new InMemoryLocalStorage();
        var attests = ServedGraph.EdgeRef(EdgeKind.AttestedIn, "Attests:00aa", "The Red Sea parted · Attested in · GEN.1.1");
        var graph = new ServedGraph()
            .Serving(ServedGraph.Card(NodeKind.TextUnit, Genesis1_1Ref.Id, Genesis1_1Ref.Label))
            .Serving(ServedGraph.EdgeRecordOf(attests, ServedGraph.Ref(NodeKind.Event, "Event:red_sea", "The Red Sea parted"), Genesis1_1Ref));
        var saved = new SavedExplorationsService(store).Save(new Exploration(Resolved.Node(graph, Genesis1_1Ref), [new Step(EdgeKind.Attests, Resolved.At(graph, ServedGraph.AtEdge(attests)))]));

        // Act
        var nextSession = new SavedExplorationsService(store);

        // Assert
        Assert.Equal(
            WholeValue.Of(new[] { new SavedExploration(saved.Id, $"GEN.1.1 → {attests.Label}", saved.CreatedUtc, ServedGraph.At(Genesis1_1Ref), [new Link(EdgeKind.Attests, ServedGraph.AtEdge(attests))]) }),
            WholeValue.Of(nextSession.Items));
    }

    [Fact]
    public void Renaming_changes_that_exploration_only_and_persists()
    {
        // Arrange
        var store = new InMemoryLocalStorage();
        var service = new SavedExplorationsService(store);
        var first = service.Save(Genesis1_1ToGenesis);
        var second = service.Save(Genesis1_1ToGenesis);

        // Act
        service.Rename(second.Id, "Where Genesis begins");

        // Assert
        Assert.Equal(
            new[] { first, second with { Name = "Where Genesis begins" } },
            new SavedExplorationsService(store).Items);
    }

    [Fact]
    public void Deleting_removes_that_exploration_only_and_persists()
    {
        // Arrange
        var store = new InMemoryLocalStorage();
        var service = new SavedExplorationsService(store);
        var first = service.Save(Genesis1_1ToGenesis);
        var second = service.Save(Genesis1_1ToGenesis);

        // Act
        service.Delete(first.Id);

        // Assert
        Assert.Equal(new[] { second }, new SavedExplorationsService(store).Items);
    }

    [Fact]
    public void Every_change_to_the_list_raises_Changed()
    {
        // Arrange
        var service = new SavedExplorationsService(new InMemoryLocalStorage());
        var changes = 0;
        service.Changed += () => changes++;

        // Act
        var saved = service.Save(Genesis1_1ToGenesis);
        service.Rename(saved.Id, "Renamed");
        service.Delete(saved.Id);

        // Assert
        Assert.Equal(3, changes);
    }

    [Fact]
    public void Without_local_storage_nothing_is_available_and_nothing_was_dropped()
    {
        // Arrange
        var noStorage = new ThrowingLocalStorage();

        // Act
        var service = new SavedExplorationsService(noStorage);

        // Assert
        Assert.Equal(
            WholeValue.Of(new { Available = false, Items = Array.Empty<SavedExploration>(), Dropped = 0 }),
            WholeValue.Of(new { service.Available, service.Items, service.Dropped }));
    }

    [Fact]
    public void The_trail_of_a_saved_exploration_is_its_start_then_each_step_s_target()
    {
        // Arrange
        var saved = Seed1;

        // Act
        var trail = saved.Trail;

        // Assert
        Assert.Equal([ServedGraph.At(Genesis1_1Ref), ServedGraph.At(LegacyGenesisRef)], trail);
    }

    [Fact]
    public void A_saved_exploration_cut_at_a_node_keeps_the_steps_before_it()
    {
        // Arrange
        var saved = Seed1;

        // Act
        var cut = (saved.UpTo(0), saved.UpTo(1));

        // Assert
        Assert.Equal((saved with { Steps = [] }, saved), cut);
    }

    private sealed class ThrowingLocalStorage : Microsoft.JSInterop.IJSInProcessRuntime
    {
        public TValue Invoke<TValue>(string identifier, params object?[]? args) => throw new InvalidOperationException(identifier);

        public ValueTask<TValue> InvokeAsync<TValue>(string identifier, object?[]? args) => throw new InvalidOperationException(identifier);

        public ValueTask<TValue> InvokeAsync<TValue>(string identifier, CancellationToken cancellationToken, object?[]? args) =>
            throw new InvalidOperationException(identifier);
    }
}
