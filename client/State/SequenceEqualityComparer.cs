namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-3: <see cref="StateAtom{T}"/>'s law 2 (idempotence) compares a
/// dispatch's result against the CURRENT value via
/// <see cref="EqualityComparer{T}.Default"/> unless a caller supplies its
/// own comparer (<see cref="StateAtom{T}"/>'s own constructor parameter).
/// The Selection atom's value is an ORDERED <c>IReadOnlyList{ExplorationDescriptor}</c>
/// (R2) -- <see cref="List{T}"/>/arrays do NOT get structural equality for
/// free in C# (no <c>Equals</c> override), so two logically-identical lists
/// built as two separate <see cref="List{T}"/> instances (the ordinary case:
/// every intent's own <c>Apply</c> below returns a FRESH list) would compare
/// UNEQUAL under the default comparer, breaking law 2 (every dispatch would
/// look like a "real" change even when nothing moved) and law 5 (agreement)
/// alike. This is the fix: element-wise, ORDER-SENSITIVE equality (Selection
/// is explicitly an ORDERED list per R2 -- insertion order is part of its own
/// observable shape, e.g. the tray's own chip order), reusable for any future
/// list-valued atom, not just this one.
/// </summary>
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
