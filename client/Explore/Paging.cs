using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public delegate Task<Page<T>> PageRead<T>(int? cursor, int limit);

public sealed record Paging<T>(IReadOnlyList<T> Kept, int Read, int? Next, bool Ended)
{
    public async Task<Paging<T>> Reading(PageRead<T> read, int wanted, Func<T, bool> keep)
    {
        var paging = this;
        while (paging.Read < wanted && !paging.Ended)
        {
            paging = paging.Then(await read(paging.Next, Math.Min(wanted - paging.Read, Paging.LargestPage)), keep);
        }

        return paging;
    }

    public async Task<Paging<T>> ToTheEnd(PageRead<T> read)
    {
        var paging = this;
        while (!paging.Ended)
        {
            paging = paging.Then(await read(paging.Next, Paging.LargestPage), Paging.Everything);
        }

        return paging;
    }

    public bool Equals(Paging<T>? other) =>
        other is not null && (Read, Next, Ended) == (other.Read, other.Next, other.Ended) && Kept.SequenceEqual(other.Kept);

    public override int GetHashCode() => Kept.Aggregate(HashCode.Combine(Read, Next, Ended), HashCode.Combine);

    private Paging<T> Then(Page<T> page, Func<T, bool> keep) =>
        new([.. Kept, .. page.Items.Where(keep)], Read + page.Items.Count, page.Next, page.Next is null);
}

public static class Paging
{
    public const int LargestPage = 200;

    public static Paging<T> Unread<T>() => new([], 0, null, false);

    public static bool Everything<T>(T _) => true;

    public static Task<Paging<Entry>> Opened(PresentationRequest request, FrontierGroup group) =>
        Unread<Entry>().Revealed(request, group.Kind, Affordances.Of(group.Kind).InitialClamp);

    public static Task<Paging<Entry>> Revealed(this Paging<Entry> paging, PresentationRequest request, EdgeKind kind, int wanted) =>
        paging.Reading((cursor, limit) => request.Element.Entries(kind, cursor, limit), wanted, entry => request.Offers(entry.Neighbour));

    public static Task<Paging<EdgeEntry>> First(IExplorableClient graph, string positionId, EdgeKind kind) =>
        Unread<EdgeEntry>().Reading(Neighbours(graph, positionId, kind), Affordances.Of(kind).InitialClamp, Everything);

    public static Task<Paging<EdgeEntry>> Next(this Paging<EdgeEntry> paging, IExplorableClient graph, string positionId, EdgeKind kind) =>
        paging.Reading(Neighbours(graph, positionId, kind), paging.Read + Affordances.Of(kind).InitialClamp, Everything);

    public static async Task<IReadOnlyList<EdgeEntry>> Whole(IExplorableClient graph, string positionId, EdgeKind kind) =>
        (await Unread<EdgeEntry>().ToTheEnd(Neighbours(graph, positionId, kind))).Kept;

    public static async Task<Link?> FirstLink(Explorable element, EdgeKind kind) => (await Links(element, kind, null)).Items.FirstOrDefault();

    public static async Task<Page<Link>> Links(Explorable element, EdgeKind kind, int? cursor)
    {
        var read = await (Unread<Entry>() with { Next = cursor }).Reading((next, limit) => element.Entries(kind, next, limit), Affordances.Of(kind).InitialClamp, Everything);
        return new Page<Link>(read.Kept.Select(entry => entry.Neighbour).ToList(), read.Next);
    }

    private static PageRead<EdgeEntry> Neighbours(IExplorableClient graph, string positionId, EdgeKind kind) =>
        async (cursor, limit) =>
        {
            var page = await graph.Edges(positionId, kind, cursor, limit);
            return new Page<EdgeEntry>(page.Entries, page.Next);
        };
}
