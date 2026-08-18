namespace BibleAtlas.Client.Tests;

public class LruCacheTests
{
    [Fact]
    public void Eviction_RemovesLeastRecentlyUsedEntry_WhenCapacityExceeded()
    {
        var cache = new LruCache<int, string>(48);
        for (var i = 0; i < 49; i++)
        {
            cache.Put(i, $"value-{i}");
        }

        Assert.False(cache.TryGet(0, out _), "the first-inserted entry should have been evicted");
        for (var i = 1; i < 49; i++)
        {
            Assert.True(cache.TryGet(i, out var value), $"entry {i} should still be cached");
            Assert.Equal($"value-{i}", value);
        }
    }

    [Fact]
    public void TryGet_RefreshesRecency_SoRecentlyTouchedEntrySurvivesEviction()
    {
        var cache = new LruCache<int, string>(48);
        for (var i = 0; i < 48; i++)
        {
            cache.Put(i, $"value-{i}");
        }

        // Touch key 0 so it becomes the most-recently-used entry; without this
        // refresh, key 0 (the oldest insert) would be the next eviction victim.
        Assert.True(cache.TryGet(0, out var refreshed));
        Assert.Equal("value-0", refreshed);

        // One more insert exceeds capacity (48 -> 49 distinct keys): the
        // least-recently-used entry is evicted. Because key 0 was just
        // refreshed, key 1 -- now the oldest untouched entry -- is evicted
        // instead of key 0.
        cache.Put(48, "value-48");

        Assert.True(cache.TryGet(0, out var stillThere), "recently-refreshed entry must survive eviction");
        Assert.Equal("value-0", stillThere);
        Assert.False(cache.TryGet(1, out _), "the least-recently-used entry (never refreshed) should be evicted");
        Assert.True(cache.TryGet(48, out var newest));
        Assert.Equal("value-48", newest);
    }
}
