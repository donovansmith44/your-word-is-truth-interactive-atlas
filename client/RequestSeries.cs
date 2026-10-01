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

    public async Task<T?> Fetch<T>(Func<Task<T>> ask) where T : class
    {
        var answer = await ask();
        return Superseded ? null : answer;
    }

    public async Task<(A, B)?> Fetch<A, B>(Func<Task<A>> first, Func<Task<B>> second)
    {
        var (a, b) = (first(), second());
        var answers = (await a, await b);
        return Superseded ? null : answers;
    }

    public async Task<(A, B, C)?> Fetch<A, B, C>(Func<Task<A>> first, Func<Task<B>> second, Func<Task<C>> third)
    {
        var (a, b, c) = (first(), second(), third());
        var answers = (await a, await b, await c);
        return Superseded ? null : answers;
    }
}
