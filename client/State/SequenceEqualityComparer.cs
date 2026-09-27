namespace BibleAtlas.Client.State;

// List<T>/arrays don't get structural equality for free in C#, so two logically-identical lists
// built as separate instances (the normal case: intents return a fresh list) would compare
// unequal under the default comparer, breaking a StateAtom's idempotence check even when nothing
// changed. This provides order-sensitive structural equality instead.
public sealed class SequenceEqualityComparer<T> : IEqualityComparer<IReadOnlyList<T>>
{
    public static readonly SequenceEqualityComparer<T> Instance = new();

    public bool Equals(IReadOnlyList<T>? x, IReadOnlyList<T>? y)
    {
        if (ReferenceEquals(x, y))
        {
            return true;
        }

        if (x is null || y is null)
        {
            return false;
        }

        return x.SequenceEqual(y);
    }

    public int GetHashCode(IReadOnlyList<T> obj)
    {
        var hash = new HashCode();
        hash.Add(obj.Count);
        foreach (var item in obj)
        {
            hash.Add(item);
        }

        return hash.ToHashCode();
    }
}
