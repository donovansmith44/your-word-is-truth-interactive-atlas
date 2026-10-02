namespace BibleAtlas.Client.Exploring;

public sealed class PageStore<TKey, TPage>(int resident) where TKey : notnull
{
    private readonly LruCache<TKey, AsyncMemo<TPage>> _pages = new(resident);

    public Task<TPage> Read(TKey key, Func<Task<TPage>> fetch)
    {
        if (!_pages.TryGet(key, out var memo))
        {
            memo = new AsyncMemo<TPage>();
            _pages.Put(key, memo);
        }

        return memo.Get(fetch);
    }
}
