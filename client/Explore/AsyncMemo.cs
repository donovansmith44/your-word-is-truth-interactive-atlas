namespace BibleAtlas.Client.Explore;

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
        // Must be assigned before ObserveFault is attached: a second concurrent
        // caller needs to see this field already set and await the same task.
        _task = task;
        ObserveFault(task);
        return task;
    }

    // Clears the cache on fault so a transient failure self-heals on the next
    // call, instead of permanently poisoning this instance. ReferenceEquals
    // guards against clobbering a newer task a retried Get() may have installed.
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
