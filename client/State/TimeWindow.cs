using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

public abstract record TimeWindow
{
    public static readonly TimeWindow Default = new TimeMode(-5, 33);
}

public sealed record TimeMode(int From, int To) : TimeWindow;

public sealed record ScriptureMode(string Ref) : TimeWindow;

public sealed record SetTimeWindow(int From, int To, string? Origin = null) : IIntent<TimeWindow>
{
    public string Name => "set-time-window";

    public TimeWindow Apply(TimeWindow current) => new TimeMode(From, To);
}

public sealed record SetScriptureWindow(string Ref, string? Origin = null) : IIntent<TimeWindow>
{
    public string Name => "set-scripture-window";

    public TimeWindow Apply(TimeWindow current) => new ScriptureMode(Ref);
}

// Derive must stay pure and synchronous -- it cannot perform I/O. The actual scene-fetch effect
// lives elsewhere, subscribed to the Target atom's own Changed, not to Source directly.
public sealed class FollowTextLink : IStateLink<Locus, TimeWindow>
{
    private readonly Func<bool> _active;

    public FollowTextLink(IStateAtom<Locus> source, IStateAtom<TimeWindow> target, Func<bool> active)
    {
        Source = source;
        Target = target;
        _active = active;
    }

    public string Name => "follow-text";

    public IStateAtom<Locus> Source { get; }
    public IStateAtom<TimeWindow> Target { get; }

    public TimeWindow Derive(Locus source, TimeWindow current) => new ScriptureMode(source.Ref);

    public bool Active => _active();
}
