using BibleAtlas.Client.Explore;
using Microsoft.JSInterop;

namespace BibleAtlas.Client;

/// <summary>
/// Batch G2 decision 6 (multi-select v1, RULED per batch-r-report.md §7):
/// the app-lifetime singleton backing the Selection Tray -- Ctrl/Cmd-click
/// on an explorable element or map marker/label toggles that node's own
/// <see cref="ExplorationDescriptor"/> in/out of this set. Persisted via
/// <see cref="LocalStore"/> under "selection-v1" -- the SAME descriptor
/// vocabulary decision 4's saved explorations use (one JSON array of
/// descriptors here, vs. one array of {id,name,createdUtc,nodes} there),
/// same restore-once/sync-on-mutation/degrade-gracefully idiom, same
/// <see cref="Changed"/> multi-reader-notification shape as
/// <see cref="SavedExplorationsService"/> -- see that file's own header
/// comment for why. Being a plain DI singleton (app-lifetime, not
/// per-page) is ALSO the entire mechanism behind "persists across pages"
/// (decision 6's own requirement): a selection made in the reader is still
/// sitting in this SAME instance's own <see cref="Items"/> when World
/// mounts afterward, localStorage or not -- localStorage only adds survival
/// across a hard reload, which in-memory state alone could never give.
///
/// Identity is Kind+Key (never Title, which is cached display text and
/// could theoretically collide or go stale) -- <see cref="Toggle"/>/
/// <see cref="Remove"/>/<see cref="Contains"/> all match on that pair.
///
/// NO ACTION VERBS live here, per decision 6's own explicit scoping ("the
/// tray exists so the owner can judge the gesture; the verb set is a later
/// brief") -- this class exposes exactly add/remove/clear/list, nothing
/// that DOES anything with a selected set once gathered.
/// </summary>
public sealed class SelectionTrayService
{
    private const string StorageKey = "selection-v1";
    private readonly IJSInProcessRuntime _js;
    private List<ExplorationDescriptor> _items;

    public SelectionTrayService(IJSInProcessRuntime js)
    {
        _js = js;
        Available = LocalStore.Probe(_js);
        _items = Available ? LocalStore.Read(_js, StorageKey, new List<ExplorationDescriptor>()) : new List<ExplorationDescriptor>();
    }

    public IReadOnlyList<ExplorationDescriptor> Items => _items;

    /// LocalStore.Probe's own cached, one-time result -- see that method's
    /// own doc comment. Even when false, this service still works IN
    /// MEMORY for the rest of the current session (a plain DI singleton, per
    /// this class's own header comment) -- only cross-reload durability is
    /// lost; the tray itself is never hidden by this flag (only the SAVE
    /// affordance, decision 2, is -- persistence loss for a live selection
    /// set is a much smaller cost than for a deliberately-saved trail).
    public bool Available { get; }

    public event Action? Changed;

    /// The Ctrl/Cmd-click gesture itself: not currently in the tray -> add
    /// it; already in the tray -> remove it. Returns true when the node
    /// ended up SELECTED after this call (added), false when deselected
    /// (removed) -- purely informational for a caller that wants to reflect
    /// its own just-toggled state locally; nothing about whether to call
    /// this in the first place depends on it.
    public bool Toggle(ExplorationDescriptor descriptor)
    {
        var existing = _items.FirstOrDefault(i => i.Kind == descriptor.Kind && i.Key == descriptor.Key);
        bool selected;
        if (existing is not null)
        {
            _items = _items.Where(i => i != existing).ToList();
            selected = false;
        }
        else
        {
            _items = _items.Append(descriptor).ToList();
            selected = true;
        }

        Persist();
        return selected;
    }

    /// selection-chip-{i}'s own remove control.
    public void Remove(ExplorationDescriptor descriptor)
    {
        _items = _items.Where(i => !(i.Kind == descriptor.Kind && i.Key == descriptor.Key)).ToList();
        Persist();
    }

    /// selection-clear.
    public void Clear()
    {
        _items = new List<ExplorationDescriptor>();
        Persist();
    }

    public bool Contains(string kind, string key) => _items.Any(i => i.Kind == kind && i.Key == key);

    private void Persist()
    {
        LocalStore.Write(_js, StorageKey, _items);
        Changed?.Invoke();
    }
}
