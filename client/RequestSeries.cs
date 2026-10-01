namespace BibleAtlas.Client;

public sealed class RequestSeries
{
    private CancellationTokenSource? _latest;

    public Request Next()
    {
        _latest?.Cancel();
        _latest = new CancellationTokenSource();
        return new Request(_latest.Token);
    }

    public Request Current => new(_latest?.Token ?? CancellationToken.None);

    public void Stop() => _latest?.Cancel();
}

public readonly record struct Request(CancellationToken Token)
{
    public bool Superseded => Token.IsCancellationRequested;

    public async Task<Outcome<T>> Fetch<T>(Func<Task<T>> ask)
    {
        try
        {
            var answer = await ask();
            return Superseded ? new Outcome<T>.Superseded() : new Outcome<T>.Arrived(answer);
        }
        catch (Exception)
        {
            return Superseded ? new Outcome<T>.Superseded() : new Outcome<T>.Failed();
        }
    }

    public async Task<Outcome<T>> Walk<T>(Func<Task<Outcome<T>>> walk) =>
        await (await Fetch(walk)).Then(Task.FromResult);

    public Task<Outcome<(A, B)>> Fetch<A, B>(Func<Task<A>> first, Func<Task<B>> second) =>
        Fetch(async () =>
        {
            var (a, b) = (first(), second());
            return (await a, await b);
        });

    public Task<Outcome<(A, B, C)>> Fetch<A, B, C>(Func<Task<A>> first, Func<Task<B>> second, Func<Task<C>> third) =>
        Fetch(async () =>
        {
            var (a, b, c) = (first(), second(), third());
            return (await a, await b, await c);
        });
}

public abstract record Outcome<T>
{
    private Outcome()
    {
    }

    public abstract R Match<R>(Func<T, R> arrived, Func<R> failed, Func<R> superseded);

    public Outcome<U> Select<U>(Func<T, U> map) =>
        Match<Outcome<U>>(
            arrived: value => new Outcome<U>.Arrived(map(value)),
            failed: () => new Outcome<U>.Failed(),
            superseded: () => new Outcome<U>.Superseded());

    public Task<Outcome<U>> Then<U>(Func<T, Task<Outcome<U>>> next) =>
        Match(
            arrived: next,
            failed: () => Task.FromResult<Outcome<U>>(new Outcome<U>.Failed()),
            superseded: () => Task.FromResult<Outcome<U>>(new Outcome<U>.Superseded()));

    public void Match(Action<T> arrived, Action failed, Action superseded) =>
        Match(
            arrived: value => Done(() => arrived(value)),
            failed: () => Done(failed),
            superseded: () => Done(superseded));

    public Task Match(Func<T, Task> arrived, Action failed, Action superseded) =>
        Match(
            arrived: arrived,
            failed: () => Done(failed),
            superseded: () => Done(superseded));

    private static Task Done(Action act)
    {
        act();
        return Task.CompletedTask;
    }

    public sealed record Arrived(T Value) : Outcome<T>
    {
        public override R Match<R>(Func<T, R> arrived, Func<R> failed, Func<R> superseded) => arrived(Value);
    }

    public sealed record Failed : Outcome<T>
    {
        public override R Match<R>(Func<T, R> arrived, Func<R> failed, Func<R> superseded) => failed();
    }

    public sealed record Superseded : Outcome<T>
    {
        public override R Match<R>(Func<T, R> arrived, Func<R> failed, Func<R> superseded) => superseded();
    }
}
