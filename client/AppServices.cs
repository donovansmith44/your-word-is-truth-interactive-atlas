using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.State;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.JSInterop;

namespace BibleAtlas.Client;

/// <summary>
/// Fix round 1 (ruling 6.i -- controller, binding, per S-10's own finding
/// that a Program.cs TEXT scan "proves textual construction, not DI
/// registration" and never checks lifetime): the state-atom service
/// registrations, extracted here so <c>client.Tests</c> can exercise them
/// through a REAL <see cref="IServiceCollection"/>/<see cref="ServiceProvider"/>
/// -- resolve twice, assert the SAME instance, for every migrated
/// <see cref="AtomNames"/> entry -- rather than scanning source text.
/// Program.cs calls these SAME two methods directly; this is not a parallel
/// copy a future edit could silently drift from.
///
/// Split in two because <see cref="AddSelectionAtom"/> genuinely needs a
/// live <see cref="IJSInProcessRuntime"/> (to seed from/persist to
/// "selection-v1") while <see cref="AddStateAtoms"/> does not -- a test that
/// only cares about registration/lifetime can register a minimal JS-runtime
/// test double (see ConformanceTests.cs's own <c>ThrowingJsRuntime</c>) and
/// still exercise the REAL factory delegate, since <c>LocalStore.Probe</c>'s
/// own try/catch degrades any JS failure to "storage unavailable" rather
/// than throwing out of the factory.
/// </summary>
public static class AppServices
{
    /// The four JS-independent migrated atoms (Locus/TimeWindow/
    /// ViewArrangement/FocusStack) -- each a singleton, named via
    /// <see cref="AtomNames"/>, exactly as Program.cs's own pre-refactor
    /// inline registrations read.
    public static void AddStateAtoms(IServiceCollection services)
    {
        services.AddSingleton(_ => new StateAtom<Locus>(AtomNames.Locus, Locus.Default));
        services.AddSingleton(_ => new StateAtom<TimeWindow>(AtomNames.TimeWindow, TimeWindow.Default));
        services.AddSingleton(_ => new StateAtom<ViewArrangement>(AtomNames.ViewArrangement, ViewArrangement.Default));
        services.AddSingleton(_ => new StateAtom<FocusStack>(AtomNames.FocusStack, FocusStack.Empty));
    }

    /// The Selection atom -- seeded from (and, once constructed, persisted
    /// to) "selection-v1" via the SAME <see cref="IJSInProcessRuntime"/> cast
    /// every other localStorage-backed registration in Program.cs performs.
    /// See <see cref="Selection"/>'s own header (fix round 1, Q-2) for why
    /// the persistence write is wired directly in this factory rather than
    /// in a separate, force-resolved service.
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
