using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public delegate Task<Page<T>> PageRead<T>(int? cursor, int limit);

internal sealed record Paging<T>(IReadOnlyList<T> Kept, int Read, int? Previous, int? Next)
{
    public bool Ended => Next is null;

    public static Paging<T> Of(Page<T> page, Func<T, bool> keep) =>
        new(page.Items.Where(keep).ToList(), page.Items.Count, page.Previous, page.Next);

    public async Task<Paging<T>> Reading(PageRead<T> read, int wanted, Func<T, bool> keep)
    {
        var paging = this;
        while (paging.Read < wanted && !paging.Ended)
        {
            paging = paging.Then(await read(paging.Next, wanted - paging.Read), keep);
        }

        return paging;
    }

    public async Task<Paging<T>> ToTheEnd(PageRead<T> read)
    {
        var paging = this;
        while (!paging.Ended)
        {
            paging = paging.Then(await read(paging.Next, int.MaxValue), Paging.Everything);
        }

        return paging;
    }

    private Paging<T> Then(Page<T> page, Func<T, bool> keep) =>
        new([.. Kept, .. page.Items.Where(keep)], Read + page.Items.Count, Previous, page.Next);
}

public static class Paging
{
    public static bool Everything<T>(T _) => true;

    public static Task<PageWindow<Entry>> Window(PresentationRequest request, EdgeKind kind) =>
        PageWindow<Entry>.Opened(
            (cursor, limit) => request.Element.Entries(kind, cursor, limit),
            entry => request.Offers(entry.Neighbour),
            Affordances.Of(kind).InitialClamp,
            () => request.Element.Moved);

    public static async Task<Page<EdgeEntry>> First(IExplorableClient graph, string positionId, EdgeKind kind)
    {
        var read = await From(Neighbours(graph, positionId, kind), null, Affordances.Of(kind).InitialClamp, Everything);
        return new Page<EdgeEntry>(read.Kept, read.Previous, read.Next);
    }

    public static async Task<IReadOnlyList<EdgeEntry>> Whole(IExplorableClient graph, string positionId, EdgeKind kind)
    {
        var read = Neighbours(graph, positionId, kind);
        return (await Paging<EdgeEntry>.Of(await read(null, int.MaxValue), Everything).ToTheEnd(read)).Kept;
    }

    public static async Task<Link?> FirstLink(Explorable element, EdgeKind kind) => (await Links(element, kind, null)).Items.FirstOrDefault();

    public static async Task<Page<Link>> Links(Explorable element, EdgeKind kind, int? cursor)
    {
        var read = await From((next, limit) => element.Entries(kind, next, limit), cursor, Affordances.Of(kind).InitialClamp, Everything);
        return new Page<Link>(read.Kept.Select(entry => entry.Neighbour).ToList(), read.Previous, read.Next);
    }

    internal static async Task<Paging<T>> From<T>(PageRead<T> read, int? start, int wanted, Func<T, bool> keep) =>
        await Paging<T>.Of(await read(start, wanted), keep).Reading(read, wanted, keep);

    private static PageRead<EdgeEntry> Neighbours(IExplorableClient graph, string positionId, EdgeKind kind) =>
        async (cursor, limit) =>
        {
            var page = await graph.Edges(positionId, kind, cursor, limit);
            return new Page<EdgeEntry>(page.Entries, page.Previous, page.Next);
        };
}
