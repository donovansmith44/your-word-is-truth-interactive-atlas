using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public sealed record Step(EdgeKind Kind, Explorable Target)
{
    public bool Equals(Step? other) => other is not null && Kind == other.Kind && Explorable.Served.Equals(Target, other.Target);

    public override int GetHashCode() => HashCode.Combine(Kind, Explorable.Served.GetHashCode(Target));
}

public sealed record Exploration(Explorable Start, IReadOnlyList<Step> Steps)
{
    public Explorable Current => Steps.Count == 0 ? Start : Steps[^1].Target;

    public IReadOnlyList<Step> Breadcrumb => Steps.Aggregate(new List<Step>(), Collapse);

    public IReadOnlyList<Explorable> Path => [Start, .. Breadcrumb.Select(step => step.Target)];

    public IReadOnlyList<Explorable> Walked => [Start, .. Steps.Select(step => step.Target)];

    public bool OnOneRoot => Walked.All(element => element.Root == Current.Root);

    public Exploration Follow(Step step) => this with { Steps = [.. Steps, step] };

    public static Exploration Along(IReadOnlyList<Explorable> walked, IEnumerable<EdgeKind> kinds) =>
        new(walked[0], kinds.Zip(walked.Skip(1), (kind, target) => new Step(kind, target)).ToList());

    public bool Equals(Exploration? other) =>
        other is not null && Explorable.Served.Equals(Start, other.Start) && Steps.SequenceEqual(other.Steps);

    public override int GetHashCode() => Steps.Aggregate(Explorable.Served.GetHashCode(Start), HashCode.Combine);

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
