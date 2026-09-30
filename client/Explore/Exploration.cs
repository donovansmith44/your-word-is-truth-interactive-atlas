using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record Exploration(Explorable Start, IReadOnlyList<Link> Links)
{
    public Explorable Current => Links.Count == 0 ? Start : Links[^1].Target;

    public IReadOnlyList<Link> Breadcrumb => Links.Aggregate(new List<Link>(), Collapse);

    public Exploration Follow(Link link) => this with { Links = [.. Links, link] };

    public Exploration Back()
    {
        var crumbs = Breadcrumb;
        return crumbs.Count == 0
            ? this
            : Follow(new Link(crumbs[^1].Kind.Dual(), SourceOf(crumbs, crumbs.Count - 1)));
    }

    public bool Equals(Exploration? other) =>
        other is not null && Start == other.Start && Links.SequenceEqual(other.Links);

    public override int GetHashCode() => Links.Aggregate(Start.GetHashCode(), HashCode.Combine);

    private List<Link> Collapse(List<Link> crumbs, Link link)
    {
        if (Retraces(crumbs, link))
        {
            crumbs.RemoveAt(crumbs.Count - 1);
        }
        else
        {
            crumbs.Add(link);
        }

        return crumbs;
    }

    private bool Retraces(List<Link> crumbs, Link link) =>
        crumbs.Count > 0
        && link.Kind == crumbs[^1].Kind.Dual()
        && link.Target == SourceOf(crumbs, crumbs.Count - 1);

    private Explorable SourceOf(IReadOnlyList<Link> hops, int index) => index == 0 ? Start : hops[index - 1].Target;
}
