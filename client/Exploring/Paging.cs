using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public delegate Task<Page<T>> PageRead<T>(int? cursor, int limit);

internal sealed record Paging<T>(IReadOnlyList<T> Kept, int Read, int? Next, bool Ended)
{
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
        new([.. Kept, .. page.Items.Where(keep)], Read + page.Items.Count, page.Next, page.Next is null);
}

public static class Paging
{
    public static bool Everything<T>(T _) => true;

    public static Task<PageWindow<Entry>> Window(PresentationRequest request, FrontierGroup group) =>
        PageWindow<Entry>.Opened((cursor, limit) => request.Element.Entries(group.Kind, cursor, limit), entry => request.Offers(entry.Neighbour), Affordances.Of(group.Kind).InitialClamp);

    public static Task<PageWindow<EdgeEntry>> Window(IExplorableClient graph, string positionId, EdgeKind kind) =>
        PageWindow<EdgeEntry>.Opened(Remembered(Neighbours(graph, positionId, kind)), Everything, Affordances.Of(kind).InitialClamp);

    public static async Task<Page<EdgeEntry>> First(IExplorableClient graph, string positionId, EdgeKind kind)
    {
        var read = await From(Neighbours(graph, positionId, kind), null, Affordances.Of(kind).InitialClamp, Everything);
        return new Page<EdgeEntry>(read.Kept, read.Next);
    }

    public static async Task<IReadOnlyList<EdgeEntry>> Whole(IExplorableClient graph, string positionId, EdgeKind kind) =>
        (await Unread<EdgeEntry>(null).ToTheEnd(Neighbours(graph, positionId, kind))).Kept;

    public static async Task<Link?> FirstLink(Explorable element, EdgeKind kind) => (await Links(element, kind, null)).Items.FirstOrDefault();

    public static async Task<Page<Link>> Links(Explorable element, EdgeKind kind, int? cursor)
    {
        var read = await From((next, limit) => element.Entries(kind, next, limit), cursor, Affordances.Of(kind).InitialClamp, Everything);
        return new Page<Link>(read.Kept.Select(entry => entry.Neighbour).ToList(), read.Next);
    }

    internal static Task<Paging<T>> From<T>(PageRead<T> read, int? start, int wanted, Func<T, bool> keep) =>
        Unread<T>(start).Reading(read, wanted, keep);

    private static Paging<T> Unread<T>(int? start) => new([], 0, start, false);

    private static PageRead<T> Remembered<T>(PageRead<T> read)
    {
        var pages = new PageStore<(int? Cursor, int Limit), Page<T>>(ServedPages.Resident);
        return (cursor, limit) => pages.Read((cursor, limit), () => read(cursor, limit));
    }

    private static PageRead<EdgeEntry> Neighbours(IExplorableClient graph, string positionId, EdgeKind kind) =>
        async (cursor, limit) =>
        {
            var page = await graph.Edges(positionId, kind, cursor, limit);
            return new Page<EdgeEntry>(page.Entries, page.Next);
        };
}
