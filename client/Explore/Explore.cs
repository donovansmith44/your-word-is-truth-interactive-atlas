using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed class Explore<T>
{
    private readonly Func<IExplorer, Exploration, Task<Outcome<(T Value, Exploration Trail)>>> _walk;

    internal Explore(Func<IExplorer, Exploration, Task<Outcome<(T Value, Exploration Trail)>>> walk) => _walk = walk;

    public Task<Outcome<(T Value, Exploration Trail)>> Run(IExplorer explorer, Exploration from) => _walk(explorer, from);
}

public static class Explore
{
    private static readonly Request Unsuperseded = new(CancellationToken.None);

    public static Explore<T> Return<T>(T value) => new((_, trail) => Arrived(value, trail));

    public static Explore<Explorable> Here { get; } = new((_, trail) => Arrived(trail));

    public static Explore<Page<Link>> Links(EdgeKind kind, int? cursor = null) =>
        Asking((_, trail) => Paging.Links(trail.Current, kind, cursor), (_, trail) => trail);

    public static Explore<Explorable> Follow(Link link) =>
        Asking((explorer, _) => explorer.Follow(link), (target, trail) => trail.Follow(new Step(link.Kind, target)));

    public static Explore<Explorable> Back { get; } = new((_, trail) => Arrived(Retraced(trail)));

    public static Explore<Unit> Replay(IReadOnlyList<Link> links) =>
        links.Aggregate(Return(new Unit()), (walk, link) => walk.SelectMany(_ => Follow(link)).Select(_ => new Unit()));

    public static async Task<Outcome<Exploration>> Begin(IExplorer explorer, PositionRef start) =>
        (await Unsuperseded.Fetch(() => explorer.Resolve(start))).Select(node => new Exploration(node, []));

    public static Explore<U> Select<T, U>(this Explore<T> m, Func<T, U> f) => m.SelectMany(value => Return(f(value)));

    public static Explore<U> SelectMany<T, U>(this Explore<T> m, Func<T, Explore<U>> f) =>
        new(async (explorer, from) => await (await m.Run(explorer, from)).Then(walked => f(walked.Value).Run(explorer, walked.Trail)));

    public static Explore<V> SelectMany<T, U, V>(this Explore<T> m, Func<T, Explore<U>> f, Func<T, U, V> project) =>
        m.SelectMany(t => f(t).Select(u => project(t, u)));

    private static Explore<T> Asking<T>(Func<IExplorer, Exploration, Task<T>> ask, Func<T, Exploration, Exploration> record) =>
        new(async (explorer, trail) => (await Unsuperseded.Fetch(() => ask(explorer, trail))).Select(answer => (answer, record(answer, trail))));

    private static Task<Outcome<(Explorable Value, Exploration Trail)>> Arrived(Exploration trail) => Arrived(trail.Current, trail);

    private static Task<Outcome<(T Value, Exploration Trail)>> Arrived<T>(T value, Exploration trail) =>
        Task.FromResult<Outcome<(T Value, Exploration Trail)>>(new Outcome<(T Value, Exploration Trail)>.Arrived((value, trail)));

    private static Exploration Retraced(Exploration trail) =>
        trail is { Breadcrumb: [.., var last], Path: [.., var source, _] } ? trail.Follow(new Step(last.Kind.Dual(), source)) : trail;
}
