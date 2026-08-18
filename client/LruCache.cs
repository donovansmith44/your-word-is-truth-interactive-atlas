namespace BibleAtlas.Client;

/// <summary>
/// A small fixed-capacity least-recently-used cache. <see cref="Put"/> evicts
/// the least recently touched entry once <c>capacity</c> distinct keys are
/// exceeded; <see cref="TryGet"/> refreshes an entry's recency on every hit,
/// so a hot entry is never evicted just because it was inserted first.
/// Not thread-safe (the Blazor WASM runtime this backs is single-threaded).
/// </summary>
public sealed class LruCache<TKey, TValue> where TKey : notnull
{
    private readonly int _capacity;
    private readonly Dictionary<TKey, LinkedListNode<(TKey Key, TValue Payload)>> _index;
    private readonly LinkedList<(TKey Key, TValue Payload)> _recency = new();

    public LruCache(int capacity)
    {
        if (capacity <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(capacity), capacity, "capacity must be positive");
        }

        _capacity = capacity;
        _index = new Dictionary<TKey, LinkedListNode<(TKey Key, TValue Payload)>>(capacity);
    }

    /// <summary>Looks up <paramref name="key"/> and, on a hit, marks it as the most recently used entry.</summary>
    public bool TryGet(TKey key, out TValue value)
    {
        if (_index.TryGetValue(key, out var node))
        {
            _recency.Remove(node);
            _recency.AddFirst(node);
            value = node.Value.Payload;
            return true;
        }

        value = default!;
        return false;
    }

    /// <summary>Inserts or refreshes <paramref name="key"/> as the most recently used entry, evicting the least recently used entry if this is a new key that would exceed capacity.</summary>
    public void Put(TKey key, TValue value)
    {
        if (_index.TryGetValue(key, out var existing))
        {
            _recency.Remove(existing);
            _index.Remove(key);
        }
        else if (_index.Count >= _capacity)
        {
            var lru = _recency.Last;
            if (lru is not null)
            {
                _recency.RemoveLast();
                _index.Remove(lru.Value.Key);
            }
        }

        var node = new LinkedListNode<(TKey Key, TValue Payload)>((key, value));
        _recency.AddFirst(node);
        _index[key] = node;
    }
}
