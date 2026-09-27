using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

// Law 1 (single writer) only holds if T is an immutable value: Value hands out the stored
// reference directly, so a mutable T would let any holder mutate shared state outside Dispatch,
// undetected by the reflection test that only checks for a public setter.
public sealed class StateAtom<T> : IStateAtom<T> where T : notnull
{
    private readonly IEqualityComparer<T> _comparer;

    public StateAtom(string name, T initial, IEqualityComparer<T>? comparer = null)
    {
        Name = name;
        Value = initial;
        _comparer = comparer ?? EqualityComparer<T>.Default;
    }

    public string Name { get; }

    public T Value { get; private set; }

    // Sticky across a no-op Dispatch: reflects the last EFFECTIVE change's origin, not
    // necessarily the most recent dispatch attempted.
    public string? LastOrigin { get; private set; }

    public event Action? Changed;

    public void Dispatch(IIntent<T> intent)
    {
        var next = intent.Apply(Value);
        if (_comparer.Equals(Value, next))
        {
            return;
        }

        Value = next;
        LastOrigin = intent.Origin;
        Changed?.Invoke();
    }
}
