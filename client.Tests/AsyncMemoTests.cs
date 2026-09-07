namespace BibleAtlas.Client.Tests;

/// <summary>
/// PERF-3 fix round 1 (review Q-2, Major -- "faulted-task poisoning"):
/// direct, isolated proof of <see cref="Explore.AsyncMemo{T}"/>'s own two
/// load-bearing properties -- (1) concurrent callers before the first
/// fetch resolves share ONE in-flight task (the original PERF-3
/// conviction: VerseNode.DetailAsync raced under
/// ExplorerPopover.LoadCurrent's Task.WhenAll, firing duplicate requests),
/// and (2) a FAULTED fetch resets the backing field so the very next
/// access is a genuine retry, never permanently poisoned (the regression
/// the review caught: a plain task-cache alone lost the OLD value-memoizing
/// idiom's "a transient failure self-heals on the next call" property --
/// real impact on FocusStack.Back, which pops the literal same node
/// instance rather than reconstructing one). Same "inject a scripted
/// sequence of outcomes, assert the state machine's own recovery"
/// discipline ViewStateServiceTests.cs already established for this
/// project's own async/stateful invariants -- here the injected sequence
/// is fetch outcomes (fail, then succeed) rather than mount/unmount calls.
/// </summary>
public class AsyncMemoTests
{
    [Fact]
    public async Task ConcurrentCallersBeforeTheFirstResolveShareOneInFlightTask()
    {
        var memo = new Explore.AsyncMemo<int>();
        var callCount = 0;
        var gate = new TaskCompletionSource<int>();

        Task<int> Fetch()
        {
            callCount++;
            return gate.Task;
        }

        // Two "concurrent" callers, exactly the ExplorerPopover.LoadCurrent
        // shape (Task.WhenAll's own synchronous dispatch: both calls happen
        // before either fetch has resolved).
        var first = memo.Get(Fetch);
        var second = memo.Get(Fetch);

        Assert.Equal(1, callCount); // the second Get() never called Fetch again
        Assert.Same(first, second); // both callers are awaiting the SAME task instance

        gate.SetResult(42);
        Assert.Equal(42, await first);
        Assert.Equal(42, await second);
    }

    [Fact]
    public async Task AFaultedFetchResetsTheCache_TheNextGetIsAGenuineRetry()
    {
        var memo = new Explore.AsyncMemo<int>();
        var callCount = 0;

        Task<int> Fetch()
        {
            callCount++;
            // First call fails (a transient network blip); every call after
            // that succeeds -- the exact "inject a failing fetch, then a
            // succeeding one" sequence the controller ruling named.
            return callCount == 1
                ? Task.FromException<int>(new InvalidOperationException("transient failure"))
                : Task.FromResult(99);
        }

        await Assert.ThrowsAsync<InvalidOperationException>(() => memo.Get(Fetch));
        Assert.Equal(1, callCount);

        // The REAL regression this test guards: a plain task-cache (no
        // reset-on-fault) would return the SAME faulted task forever here,
        // re-throwing on every future Get() for this node instance's
        // lifetime -- exactly FocusStack.Back's own "pop the literal same
        // node object" path, permanently broken after one transient blip.
        var recovered = await memo.Get(Fetch);
        Assert.Equal(99, recovered);
        Assert.Equal(2, callCount); // the retry actually re-fetched, not a cached fault
    }

    [Fact]
    public async Task ASuccessfulFetchStaysCached_RepeatedGetNeverRefetches()
    {
        var memo = new Explore.AsyncMemo<int>();
        var callCount = 0;

        Task<int> Fetch()
        {
            callCount++;
            return Task.FromResult(7);
        }

        Assert.Equal(7, await memo.Get(Fetch));
        Assert.Equal(7, await memo.Get(Fetch));
        Assert.Equal(7, await memo.Get(Fetch));
        Assert.Equal(1, callCount); // idempotent -- exactly one real fetch across three calls
    }

    [Fact]
    public async Task CompletedValueOrDefault_NullBeforeResolve_ThenTheRealValueOnceResolved()
    {
        var memo = new Explore.AsyncMemo<int>();
        var gate = new TaskCompletionSource<int>();

        Assert.Equal(0, memo.CompletedValueOrDefault); // int's own default -- never started

        var inFlight = memo.Get(() => gate.Task);
        Assert.Equal(0, memo.CompletedValueOrDefault); // still in flight, not yet resolved

        gate.SetResult(5);
        await inFlight;
        Assert.Equal(5, memo.CompletedValueOrDefault); // resolved -- EventNode.CachedKind's own read shape
    }

    [Fact]
    public async Task CompletedValueOrDefault_StaysDefaultAfterAFault_NeverAStaleValue()
    {
        var memo = new Explore.AsyncMemo<int>();
        var refetch = memo.Get(() => Task.FromException<int>(new InvalidOperationException()));
        await Assert.ThrowsAsync<InvalidOperationException>(() => refetch);
        Assert.Equal(0, memo.CompletedValueOrDefault); // faulted, never completed successfully -- default, not stale
    }
}
