namespace BibleAtlas.Client.Exploring;

public sealed class LegacyPresentations
{
    private IReadOnlyList<Retained> _trail = [];

    public IExplorable? Present(Exploration exploration) => Retain(exploration.Path, arriving: null);

    public void Arrive(Exploration exploration, IExplorable view) => Retain(exploration.Path, view);

    private IExplorable? Retain(IReadOnlyList<Explorable> path, IExplorable? arriving)
    {
        var unchanged = _trail.Zip(path).TakeWhile(pair => pair.First.Entry == pair.Second).Count();
        var kept = Math.Min(unchanged, arriving is null ? path.Count : path.Count - 1);
        _trail = [.. _trail.Take(kept), .. path.Skip(kept).Select((entry, at) => new Retained(entry, (kept + at == path.Count - 1 ? arriving : null) ?? LegacyNodes.For(entry)))];
        return _trail[^1].View;
    }

    private sealed record Retained(Explorable Entry, IExplorable? View);
}
