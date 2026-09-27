using BibleAtlas.Client.Explore;
using Microsoft.JSInterop;

namespace BibleAtlas.Client;

public sealed record SavedExploration(string Id, string Name, DateTimeOffset CreatedUtc, List<ExplorationDescriptor> Nodes);

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

    public bool Available { get; }

    public event Action? Changed;

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

    // A no-op if id no longer exists (e.g. deleted from another open tab's own copy of this
    // service -- each tab has its own in-memory list) -- never an exception for it.
    public void Rename(string id, string name)
    {
        _items = _items.Select(i => i.Id == id ? i with { Name = name } : i).ToList();
        Persist();
    }

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
