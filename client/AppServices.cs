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
            var initial = available
                ? (IReadOnlyList<ExplorationDescriptor>)LocalStore.Read(js, Selection.StorageKey, new List<ExplorationDescriptor>())
                    .DistinctBy(d => (d.Kind, d.Key)).ToList()
                : Selection.Empty;
            var atom = new StateAtom<IReadOnlyList<ExplorationDescriptor>>(AtomNames.Selection, initial, SequenceEqualityComparer<ExplorationDescriptor>.Instance);
            if (available)
            {
                atom.Changed += () => LocalStore.Write(js, Selection.StorageKey, atom.Value);
            }

            return atom;
        });
    }
}
