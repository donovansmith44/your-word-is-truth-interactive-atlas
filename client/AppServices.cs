using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.State;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.JSInterop;

namespace BibleAtlas.Client;

public static class AppServices
{
    public static void AddStateAtoms(IServiceCollection services)
    {
        services.AddSingleton(_ => new StateAtom<Locus>(AtomNames.Locus, Locus.Default));
        services.AddSingleton(_ => new StateAtom<TimeWindow>(AtomNames.TimeWindow, TimeWindow.Default));
        services.AddSingleton(_ => new StateAtom<ViewArrangement>(AtomNames.ViewArrangement, ViewArrangement.Default));
        services.AddSingleton(_ => new StateAtom<ExplorationState>(AtomNames.Exploration, new ExplorationState.Closed()));
        services.AddSingleton(sp => new PaneScopes(
            sp.GetRequiredService<StateAtom<ViewArrangement>>(),
            sp.GetRequiredService<StateAtom<Locus>>(),
            sp.GetRequiredService<StateAtom<TimeWindow>>()));
    }

    public static void AddSelectionAtom(IServiceCollection services)
    {
        services.AddSingleton(sp =>
        {
            var js = (IJSInProcessRuntime)sp.GetRequiredService<IJSRuntime>();
            var available = LocalStore.Probe(js);
            var initial = available ? StoredSelection(js) : Selection.Empty;
            var atom = new StateAtom<IReadOnlyList<NodeRef>>(AtomNames.Selection, initial, SequenceEqualityComparer<NodeRef>.Instance);
            if (available)
            {
                atom.Changed += () => LocalStore.Write(js, Selection.StorageKey, atom.Value);
            }

            return atom;
        });
    }

    private static IReadOnlyList<NodeRef> StoredSelection(IJSInProcessRuntime js)
    {
        if (LocalStore.Read<List<NodeRef>?>(js, Selection.StorageKey, null) is { } stored)
        {
            return stored.Distinct(NodeIdentity.Comparer).ToList();
        }

        var translated = LegacySaves.Nodes(LocalStore.Read(js, LegacySaves.SelectionKey, new List<V1Node>())).Kept.Distinct(NodeIdentity.Comparer).ToList();
        LocalStore.Write(js, Selection.StorageKey, translated);
        return translated;
    }
}
