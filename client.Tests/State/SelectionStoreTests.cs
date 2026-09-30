using BibleAtlas.Client;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.State;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.JSInterop;

namespace BibleAtlas.Client.Tests.State;

public sealed class SelectionStoreTests
{
    private const string V1Key = "selection-v1";
    private const string V2Key = "selection-v2";
    private const string V1Store = """
        [{"kind":"Verse","key":"GEN.1.1","title":"GEN.1.1"},
         {"kind":"Place","key":"jerusalem","title":"Jerusalem"},
         {"kind":"Author","key":"GEN","title":"Moses"}]
        """;
    private const string V2StoreWithADuplicate = """
        [{"id":"text-unit:GEN.1.1","kind":"TextUnit","label":"GEN.1.1"},
         {"id":"Place:jerusalem","kind":"Place","label":"Jerusalem"},
         {"id":"Place:jerusalem","kind":"Place","label":"Jerusalem (Salem)"}]
        """;

    private static readonly NodeRef Genesis1_1 = ServedGraph.Ref(NodeKind.TextUnit, "text-unit:GEN.1.1", "GEN.1.1");
    private static readonly NodeRef Jerusalem = ServedGraph.Ref(NodeKind.Place, "Place:jerusalem", "Jerusalem");

    private static StateAtom<IReadOnlyList<NodeRef>> AtomOver(IJSInProcessRuntime storage)
    {
        var services = new ServiceCollection();
        services.AddSingleton<IJSRuntime>(storage);
        AppServices.AddSelectionAtom(services);
        return services.BuildServiceProvider().GetRequiredService<StateAtom<IReadOnlyList<NodeRef>>>();
    }

    [Fact]
    public void A_v1_selection_is_translated_into_the_atom_once_and_written_as_v2()
    {
        // Arrange
        var store = new InMemoryLocalStorage();
        store.Items[V1Key] = V1Store;

        // Act
        var atom = AtomOver(store);

        // Assert
        Assert.Equal(
            WholeValue.Of(new { Value = new[] { Genesis1_1, Jerusalem }, Keys = new[] { V1Key, V2Key }, NextSession = new[] { Genesis1_1, Jerusalem } }),
            WholeValue.Of(new { atom.Value, Keys = store.Items.Keys, NextSession = AtomOver(store).Value }));
    }

    [Fact]
    public void A_v2_selection_seeds_the_atom_with_each_node_once()
    {
        // Arrange
        var store = new InMemoryLocalStorage();
        store.Items[V2Key] = V2StoreWithADuplicate;

        // Act
        var atom = AtomOver(store);

        // Assert
        Assert.Equal([Genesis1_1, Jerusalem], atom.Value);
    }

    [Fact]
    public void Every_change_to_the_selection_is_written_under_selection_v2()
    {
        // Arrange
        var store = new InMemoryLocalStorage();
        var atom = AtomOver(store);

        // Act
        atom.Dispatch(new ToggleSelection(Jerusalem));

        // Assert
        Assert.Equal([Jerusalem], AtomOver(store).Value);
    }

    [Fact]
    public void Without_local_storage_the_selection_starts_empty()
    {
        // Arrange
        var noStorage = new ThrowingLocalStorage();

        // Act
        var atom = AtomOver(noStorage);

        // Assert
        Assert.Empty(atom.Value);
    }

    private sealed class ThrowingLocalStorage : IJSInProcessRuntime
    {
        public TValue Invoke<TValue>(string identifier, params object?[]? args) => throw new InvalidOperationException(identifier);

        public ValueTask<TValue> InvokeAsync<TValue>(string identifier, object?[]? args) => throw new InvalidOperationException(identifier);

        public ValueTask<TValue> InvokeAsync<TValue>(string identifier, CancellationToken cancellationToken, object?[]? args) =>
            throw new InvalidOperationException(identifier);
    }
}
