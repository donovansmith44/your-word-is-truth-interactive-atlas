using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record Step(EdgeKind Kind, Explorable Target);

public sealed record Exploration(Explorable Start, IReadOnlyList<Step> Steps)
{
    public Explorable Current => Steps.Count == 0 ? Start : Steps[^1].Target;

    public IReadOnlyList<Step> Breadcrumb => Steps.Aggregate(new List<Step>(), Collapse);

    public IReadOnlyList<Explorable> Path => [Start, .. Breadcrumb.Select(step => step.Target)];

    public Exploration Follow(Step step) => this with { Steps = [.. Steps, step] };

    public Exploration Back()
    {
        var crumbs = Breadcrumb;
        return crumbs.Count == 0
            ? this
            : Follow(new Step(crumbs[^1].Kind.Dual(), SourceOf(crumbs, crumbs.Count - 1)));
    }

    public bool Equals(Exploration? other) =>
        other is not null && Start == other.Start && Steps.SequenceEqual(other.Steps);

    public override int GetHashCode() => Steps.Aggregate(Start.GetHashCode(), HashCode.Combine);

    private List<Step> Collapse(List<Step> crumbs, Step step)
    {
        if (Retraces(crumbs, step))
        {
            crumbs.RemoveAt(crumbs.Count - 1);
        }
        else
        {
            crumbs.Add(step);
        }

        return crumbs;
    }

    private bool Retraces(List<Step> crumbs, Step step) =>
        crumbs.Count > 0
        && step.Kind == crumbs[^1].Kind.Dual()
        && step.Target == SourceOf(crumbs, crumbs.Count - 1);

    private Explorable SourceOf(IReadOnlyList<Step> hops, int index) => index == 0 ? Start : hops[index - 1].Target;
}
