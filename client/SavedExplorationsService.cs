using BibleAtlas.Client.Explore;
using Microsoft.JSInterop;

namespace BibleAtlas.Client;

/// <summary>
/// Batch G2 decision 4: one saved exploration -- an ordered node trail
/// (decision 1), snapshotted by decision 2's save affordance, not a live
/// link (saving again with a longer trail produces a NEW entry, never
/// mutates this one). The exact JSON shape the brief pins: `{id, name,
/// createdUtc, nodes: [descriptor]}`, one document per storage key
/// "explorations-v1" (a JSON array of these).
/// </summary>
public sealed record SavedExploration(string Id, string Name, DateTimeOffset CreatedUtc, List<ExplorationDescriptor> Nodes);

/// <summary>
/// Batch G2 decisions 2/4/5: the app-lifetime singleton backing the
/// hamburger menu's own saved-explorations list -- localStorage-backed via
/// <see cref="LocalStore"/>, restored once at construction, synced (written
/// through) on every mutation, degrading to in-memory-only (never a crash)
/// if storage is unavailable, per that file's own header comment.
///
/// <see cref="Changed"/> is the one small addition <see cref="LocalStore"/>'s
/// own read/write primitive doesn't need on its own: this service is
/// injected into MULTIPLE independent component instances at once (the
/// popover's own save button; the hamburger panel's own list) -- a plain
/// C# event lets a mutation made from ONE (Save, in ExplorerPopover) tell
/// the OTHER (the hamburger panel, if already open) to re-render, the same
/// "one owner mutates, every reader re-renders on notice" shape
/// <c>NavigationManager.LocationChanged</c> already gives MainLayout for
/// route changes.
/// </summary>
public sealed class SavedExplorationsService
{
    private const string StorageKey = "explorations-v1";
    private readonly IJSInProcessRuntime _js;
    private List<SavedExploration> _items;

    public SavedExplorationsService(IJSInProcessRuntime js)
    {
        _js = js;
        Available = LocalStore.Probe(_js);
        _items = Available ? LocalStore.Read(_js, StorageKey, new List<SavedExploration>()) : new List<SavedExploration>();
    }

    public IReadOnlyList<SavedExploration> Items => _items;

    /// LocalStore.Probe's own cached, one-time result -- see that method's
    /// own doc comment. ExplorerPopover's save button and MainLayout's
    /// hamburger button both read this to decide whether to render at all.
    public bool Available { get; }

    public event Action? Changed;

    /// Decision 2: snapshots the CURRENT trail under an auto-generated name
    /// (first->last node titles, e.g. "GEN.1.1 → Jerusalem"; the bare title
    /// alone for a one-node trail; a plain fallback for the -- practically
    /// unreachable, the popover always has >=1 node on its stack -- empty
    /// case). Never mutates a prior save; each call appends a brand-new
    /// entry, per decision 2's own "save = snapshot, not live link."
    public SavedExploration Save(IReadOnlyList<ExplorationDescriptor> trail)
    {
        var nodes = trail.ToList();
        var name = nodes.Count switch
        {
            0 => "Empty exploration",
            1 => nodes[0].Title,
            _ => $"{nodes[0].Title} → {nodes[^1].Title}",
        };
        var item = new SavedExploration(Guid.NewGuid().ToString("n"), name, DateTimeOffset.UtcNow, nodes);
        _items = _items.Append(item).ToList();
        Persist();
        return item;
    }

    /// Decision 5: inline rename (exploration-rename-{id}) -- a no-op if
    /// `id` no longer exists (e.g. deleted from another open tab's own copy
    /// of this service; each tab has its own in-memory List, so this is a
    /// real, if rare, possibility -- never an exception for it).
    public void Rename(string id, string name)
    {
        _items = _items.Select(i => i.Id == id ? i with { Name = name } : i).ToList();
        Persist();
    }

    /// Decision 5: delete (exploration-delete-{id}) -- same no-op-on-missing
    /// stance as Rename above.
    public void Delete(string id)
    {
        _items = _items.Where(i => i.Id != id).ToList();
        Persist();
    }

    private void Persist()
    {
        LocalStore.Write(_js, StorageKey, _items);
        Changed?.Invoke();
    }
}
