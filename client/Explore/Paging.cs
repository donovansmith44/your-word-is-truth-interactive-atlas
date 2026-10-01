using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record Paging(IReadOnlyList<Entry> Offered, int Read, int? Next)
{
    public static async Task<Paging> Opened(PresentationRequest request, FrontierGroup group)
    {
        var wanted = Affordances.Of(group.Kind).InitialClamp;
        return await Empty.Then(request, await request.Element.Entries(group.Kind, cursor: null, limit: wanted)).Reading(request, group.Kind, wanted);
    }

    public async Task<Paging> Reading(PresentationRequest request, EdgeKind kind, int wanted)
    {
        var paging = this;
        while (paging.Read < wanted && paging.Next is int cursor)
        {
            paging = paging.Then(request, await request.Element.Entries(kind, cursor, wanted - paging.Read));
        }

        return paging;
    }

    private static readonly Paging Empty = new([], 0, null);

    private Paging Then(PresentationRequest request, Page<Entry> page) =>
        new([.. Offered, .. page.Items.Where(entry => request.Offers(entry.Neighbour))], Read + page.Items.Count, page.Next);
}
