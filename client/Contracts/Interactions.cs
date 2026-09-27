namespace BibleAtlas.Client.Contracts;

public enum Gesture
{
    Hover,
    Click,
    CtrlClick,
    ShiftClick,
    Highlight,
}

public interface IInteractive
{
    IReadOnlyList<IInteractionContract> Interactions { get; }
}

public interface IInteractionContract
{
    Gesture Gesture { get; }

    string Semantic { get; }

    TimingDiscipline Timing { get; }
}

public sealed record TimingDiscipline(int GraceMs, int DebounceMs)
{
    public static readonly TimingDiscipline Immediate = new(0, 0);
}
