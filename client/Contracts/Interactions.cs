namespace BibleAtlas.Client.Contracts;

// Spec §4c (owner amendment 2026-08-26): interaction contracts on ALL
// components. A component MAY NOT respond to a gesture it has not declared;
// two components declaring the same (Gesture, Semantic) must behave
// identically -- uniformity as a testable guarantee, not a convention.
// See Views.cs header for the compiled-authority discipline binding this file.

/// <summary>Spec §4c: the closed gesture vocabulary. Every entry is an
/// already-litigated house law; adding one is a controller/owner decision,
/// never an implementer convenience.</summary>
public enum Gesture
{
    Hover,
    Click,
    CtrlClick,
    ShiftClick,
    Highlight,
}

/// <summary>Spec §4c: a component that declares its interactions. Composes
/// onto <see cref="IViewComponent"/>s; undeclared gestures are contract
/// violations (testable).</summary>
public interface IInteractive
{
    IReadOnlyList<IInteractionContract> Interactions { get; }
}

/// <summary>
/// Spec §4c. Seed vocabulary = the litigated gesture laws: Hover="peek"
/// (grace-timed), Click="explore"/"pin" (ONE-RULE/PIN-1), CtrlClick="select"
/// (tray ruling), ShiftClick="range-extend", Highlight="emphasize".
/// </summary>
public interface IInteractionContract
{
    Gesture Gesture { get; }

    /// <summary>"peek" | "explore" | "pin" | "select" | "range-extend" |
    /// "emphasize".</summary>
    string Semantic { get; }

    TimingDiscipline Timing { get; }
}

/// <summary>
/// Spec §4c: "grace/debounce constants as contract data." SKELETON
/// RESOLUTION (the spec names the type, not its members): the two timing
/// constants the app's hover-safety discipline already litigates -- a grace
/// delay before a gesture takes effect and a debounce floor between
/// repeats. 0 = not applicable to this contract.
/// </summary>
public sealed record TimingDiscipline(int GraceMs, int DebounceMs)
{
    /// <summary>For gestures with no timing discipline (plain click).</summary>
    public static readonly TimingDiscipline Immediate = new(0, 0);
}
