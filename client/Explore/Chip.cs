namespace BibleAtlas.Client.Explore;

public sealed record Chip(string Label, string ChipTestId, ChipTarget Target);

public abstract record ChipTarget
{
    public sealed record Push(IExplorable Next) : ChipTarget;
    public sealed record NavigateWorld(string Query) : ChipTarget;
    public sealed record NavigateReader(string Book, int Chapter, int? Verse) : ChipTarget;
}
