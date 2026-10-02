namespace BibleAtlas.Client.Contracts;

public interface IIntent<T>
{
    string Name { get; }

    string? Origin { get; }

    // Must be idempotent: Apply(Apply(v)) == Apply(v).
    T Apply(T current);
}

public interface IStateAtom<T>
{
    string Name { get; }

    T Value { get; }

    void Dispatch(IIntent<T> intent);

    // Fires only after Value has actually changed -- never on a no-op dispatch.
    event Action? Changed;
}

public interface IProjection<T>
{
    IStateAtom<T> Source { get; }

    // Must stay derived from Source -- overriding with a stored copy reintroduces desync.
    T Value => Source.Value;
}

public interface IStateLink<A, B>
{
    IStateAtom<A> Source { get; }
    IStateAtom<B> Target { get; }

    // Must be pure.
    B Derive(A source, B current);

    bool Active { get; }
}

public interface IStateEffect<T>
{
    string Name { get; }

    IStateAtom<T> Source { get; }

    bool AppliesTo(T value);

    // Must be idempotent per value: materializing the same value twice must be safe and cheap.
    Task Materialize(T value);
}

public interface IEffectRegistry
{
    IDisposable Claim<T>(IStateEffect<T> effect);
}

public static class AtomNames
{
    public const string Locus = "locus";
    public const string TimeWindow = "time-window";
    public const string Exploration = "exploration";
    public const string Selection = "selection";
    public const string ViewArrangement = "view-arrangement";
}
