using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

public sealed record Chip(string Label, string ChipTestId, ChipTarget Target);

public abstract record ChipTarget
{
    public sealed record Push(IExplorable Next, EdgeKind Via) : ChipTarget;
    public sealed record NavigateWorld(string Query) : ChipTarget;
    public sealed record NavigateReader(string Book, int Chapter, int? Verse) : ChipTarget;
}
