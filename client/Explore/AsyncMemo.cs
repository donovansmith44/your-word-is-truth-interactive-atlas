namespace BibleAtlas.Client.Explore;

/// <summary>
/// PERF-3 fix round 1 (review Q-2, Major -- "faulted-task poisoning"): the
/// ONE reset-on-fault async-memoization idiom every "one fetch, shared by
/// every section provider that reads this node instance" class in this
/// file now uses (<see cref="VerseNode.DetailAsync"/>,
/// <see cref="PassageNode.XrefsAsync"/>/<see cref="PassageNode.CatechismAsync"/>,
/// <see cref="EventNode.DetailAsync"/>/<see cref="EventNode.NarrativePositionsAsync"/>,
/// <see cref="PlaceNode.DetailAsync"/>, <see cref="CatechismNode.DetailAsync"/>) --
/// extracted so the fix lives in ONE place, not six-plus near-identical
/// copies that could drift.
///
/// Caches the in-flight/completed TASK itself, not the resolved value --
/// this is what makes concurrent callers under
/// <c>ExplorerPopover.LoadCurrent</c>'s own <c>Task.WhenAll</c> dispatch
/// share the SAME pending request instead of each firing their own (the
/// original PERF-3 fix, this batch's dominant conviction: <see cref="Get"/>
/// assigns the field SYNCHRONOUSLY, before returning control to the
/// caller, so a second concurrent caller sees a non-null field and awaits
/// the SAME task rather than starting a new request).
///
/// On FAULT (<c>fetch</c> throwing, e.g. a transient network failure), the
/// backing field is cleared -- the real fix this round (review Q-2): a
/// value-memoizing `_cached ??= await ...` only ever assigned on SUCCESS,
/// so a transient failure self-healed on the very next call; a naive
/// task-memoizing fix loses that self-healing (once faulted, the field
/// stays permanently faulted for that instance's lifetime -- a real
/// regression on `FocusStack.Back`'s own "pop the literal same node
/// object, never reconstruct" path: one transient failure would
/// permanently break that popover entry for the rest of the session).
///
/// AsyncMemoTests.cs's own <c>AFaultedFetchResetsTheCache</c> test caught
/// a genuine ordering bug in this class's own FIRST draft: a naive
/// `try { return await fetch(); } catch { _task = null; throw; }` inside
/// an `async` helper resets the field BEFORE the caller has even assigned
/// it, whenever `fetch()` faults SYNCHRONOUSLY (an already-completed
/// faulted `Task` -- `await` on one never actually suspends, so the whole
/// try/catch runs to completion before the async method call even
/// returns to `Get`, making the reset a no-op that a SUBSEQUENT,
/// unconditional field assignment then immediately overwrites right back
/// to the faulted task). Fixed by assigning the field FIRST (a plain
/// synchronous statement, never inside an async continuation), then
/// observing the SAME task instance for a fault via a fire-and-forget
/// `async void` watcher -- <c>ReferenceEquals</c> guards against clobbering
/// a NEWER task a later, already-retried <see cref="Get"/> call may have
/// installed by the time this watcher's own continuation runs.
/// </summary>
public sealed class AsyncMemo<T>
{
    private Task<T>? _task;

    public Task<T> Get(Func<Task<T>> fetch)
    {
        if (_task is { } current)
        {
            return current;
        }

        var task = fetch();
        _task = task; // assigned FIRST, synchronously -- see this class's own doc comment for why ordering matters
        ObserveFault(task);
        return task;
    }

    /// The currently-resolved value, if the fetch has already completed
    /// successfully -- null otherwise (never started, still in flight, or
    /// faulted). Never blocks (`.Result` is only ever read once
    /// `IsCompletedSuccessfully` is already true). For the rare synchronous
    /// read a node needs before/without awaiting (e.g.
    /// <see cref="EventNode.CachedKind"/>'s own "read whatever's already
    /// resolved, fall back otherwise" contract).
    public T? CompletedValueOrDefault => _task is { IsCompletedSuccessfully: true } ? _task.Result : default;

    // Fire-and-forget by design (async void -- the one legitimate use of
    // it: a background side effect nothing awaits or needs the result of;
    // any exception `task` itself carries is already observed correctly
    // by every real caller of Get, which awaits `task` directly, never
    // this method). `ReferenceEquals` guards the reset: only clear `_task`
    // if IT STILL holds the exact instance that just faulted -- if a
    // caller already retried (a fresh Get() after a PRIOR fault reset the
    // field) and installed a newer task by the time this fires, that newer
    // task must never be clobbered.
    private async void ObserveFault(Task<T> task)
    {
        try
        {
            await task;
        }
        catch
        {
            if (ReferenceEquals(_task, task))
            {
                _task = null;
            }
        }
    }
}
