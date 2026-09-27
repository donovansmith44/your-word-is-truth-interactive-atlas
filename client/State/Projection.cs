using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

public sealed class Projection<T> : IProjection<T>
{
    public Projection(IStateAtom<T> source) => Source = source;

    public IStateAtom<T> Source { get; }

    // Redeclared because C# does not surface a default interface implementation on a reference
    // typed to the concrete class, only through an IProjection<T>-typed one; call sites here hold
    // Projection<T>, so without this `.Value` would be a compile error.
    public T Value => Source.Value;
}
