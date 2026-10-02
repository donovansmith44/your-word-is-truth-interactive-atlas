namespace BibleAtlas.Client;

// Not thread-safe -- relies on the single-threaded Blazor WASM runtime this backs.
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
