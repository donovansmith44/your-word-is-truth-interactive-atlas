using System.Text.Json.Serialization;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Legacy;
using Microsoft.JSInterop;

namespace BibleAtlas.Client;

public sealed record SavedExploration(string Id, string Name, DateTimeOffset CreatedUtc, PositionRef Start, IReadOnlyList<Link> Steps)
{
    [JsonIgnore]
    public IReadOnlyList<PositionRef> Trail => [Start, .. Steps.Select(step => step.Target)];

    public SavedExploration UpTo(int node) => this with { Steps = Steps.Take(node).ToList() };

    public bool Equals(SavedExploration? other) =>
        other is not null
        && (Id, Name, CreatedUtc) == (other.Id, other.Name, other.CreatedUtc)
        && PositionIdentity.Comparer.Equals(Start, other.Start)
        && Steps.SequenceEqual(other.Steps);

    public override int GetHashCode() => Steps.Aggregate(HashCode.Combine(Id, Name, CreatedUtc, PositionIdentity.Comparer.GetHashCode(Start)), HashCode.Combine);
}

public sealed class SavedExplorationsService
{
    public const string StorageKey = "explorations-v3";

    private readonly IJSInProcessRuntime _js;
    private List<SavedExploration> _items;

    public SavedExplorationsService(IJSInProcessRuntime js)
    {
        _js = js;
        Available = LocalStore.Probe(_js);
        if (!Available)
        {
            _items = [];
            return;
        }

        if (LocalStore.Read<List<SavedExploration>?>(_js, StorageKey, null) is { } stored)
        {
            _items = stored;
            return;
        }

        if (LocalStore.Read<List<V2Exploration>?>(_js, LegacySaves.V2ExplorationsKey, null) is { } v2)
        {
            _items = LegacySaves.Explorations(v2).ToList();
        }
        else
        {
            var translated = LegacySaves.Explorations(LocalStore.Read(_js, LegacySaves.ExplorationsKey, new List<V1Exploration>()));
            _items = translated.Kept.ToList();
            Dropped = translated.Dropped;
        }

        LocalStore.Write(_js, StorageKey, _items);
    }

    public IReadOnlyList<SavedExploration> Items => _items;

    public bool Available { get; }

    public int Dropped { get; }

    public event Action? Changed;

    public SavedExploration Save(Exploration exploration)
    {
        var name = exploration.Steps.Count == 0
            ? exploration.Start.Label
            : $"{exploration.Start.Label} → {exploration.Current.Label}";
        var item = new SavedExploration(
            Guid.NewGuid().ToString("n"), name, DateTimeOffset.UtcNow,
            exploration.Start.Identity, exploration.Steps.Select(step => new Link(step.Kind, step.Target.Identity)).ToList());
        _items = _items.Append(item).ToList();
        Persist();
        return item;
    }

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
