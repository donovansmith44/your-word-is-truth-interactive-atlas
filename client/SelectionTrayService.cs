using BibleAtlas.Client.Explore;
using BibleAtlas.Client.State;
using Microsoft.JSInterop;

namespace BibleAtlas.Client;

/// <summary>
/// Batch G2 decision 6 (multi-select v1, RULED per batch-r-report.md §7):
/// PERSISTENCE beneath the Selection atom (Batch ST-3, R2) -- Ctrl/Cmd-click
/// on an explorable element or map marker/label now dispatches
/// <see cref="ToggleSelection"/> directly onto the shared
/// <c>StateAtom&lt;IReadOnlyList&lt;ExplorationDescriptor&gt;&gt;</c>
/// (<see cref="AtomNames.Selection"/>); this class's OWN role narrows to
/// exactly what R2 asks ("SelectionTrayService becomes persistence BENEATH
/// the atom") -- write "selection-v1" to localStorage on every atom
/// <see cref="IStateAtom{T}.Changed"/>, and expose <see cref="Available"/>
/// (LocalStore.Probe's own cached, one-time round-trip result) for the
/// SAVE-affordance-hiding UI that already reads it. The atom itself is now
/// what "persists across pages" (a DI singleton, same reasoning this file's
/// own header used to give for itself); the READ (cold-start restore) side
/// moved to the atom's own construction (Program.cs) -- StateAtom{T}'s own
/// constructor `initial` parameter, seeded from the SAME LocalStore.Read/
/// Probe idiom this service used to perform itself, so "selection-v1"'s own
/// on-disk JSON SHAPE (a bare array of {kind,key,title} descriptors) is
/// UNCHANGED end to end -- cold-start compatibility is a test
/// (client.Tests/State/SelectionTests.cs).
///
/// <see cref="Items"/>/<see cref="Toggle"/>/<see cref="Remove"/>/
/// <see cref="Clear"/>/<see cref="Contains"/>/the old <see cref="Changed"/>
/// (atom-Changed-shaped, now redundant with the atom's own event) are
/// RETIRED -- every former caller (SelectionTray.razor, Reader.razor,
/// World.razor) now injects the Selection atom directly; see the batch
/// report's own retirement table.
/// </summary>
public sealed class SelectionTrayService
{
    public const string StorageKey = "selection-v1";

    public SelectionTrayService(IJSInProcessRuntime js, StateAtom<IReadOnlyList<ExplorationDescriptor>> atom)
    {
        Available = LocalStore.Probe(js);
        if (Available)
        {
            atom.Changed += () => LocalStore.Write(js, StorageKey, atom.Value);
        }
    }

    /// LocalStore.Probe's own cached, one-time result -- see that method's
    /// own doc comment. Even when false, the Selection atom still works IN
    /// MEMORY for the rest of the current session (a plain DI singleton) --
    /// only cross-reload durability is lost; the tray itself is never
    /// hidden by this flag (only the SAVE affordance, decision 2, is --
    /// persistence loss for a live selection set is a much smaller cost
    /// than for a deliberately-saved trail).
    public bool Available { get; }
}
