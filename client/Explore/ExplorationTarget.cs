namespace BibleAtlas.Client.Explore;

public sealed record Exploration(string Label, string ChipTestId, ExplorationTarget Target);

public abstract record ExplorationTarget
{
    public sealed record Push(IExplorable Next) : ExplorationTarget;
    public sealed record NavigateWorld(string Query) : ExplorationTarget;
    public sealed record NavigateReader(string Book, int Chapter, int? Verse) : ExplorationTarget;
}
