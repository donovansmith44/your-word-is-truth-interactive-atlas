using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-1: the ONE concrete <see cref="IProjection{T}"/> implementation.
/// Holds nothing but a reference to its <see cref="Source"/> atom -- NO value
/// storage of its own, per the contract's own doc comment ("a projection
/// holding its own copy is exactly the desync-by-copy disease this layer
/// retires"); <see cref="Value"/> is the interface's default-derived
/// <c>Source.Value</c>, never overridden here.
///
/// LAW 5 (agreement): trivially true by construction -- every
/// <see cref="Projection{T}"/> over the SAME atom reads the SAME
/// <see cref="IStateAtom{T}.Value"/> property at read time, so N
/// projections of one atom can never disagree at any instant (there is
/// nothing per-projection that could drift). client.Tests/State/
/// ConfluenceAgreementLawTests.cs still exercises this over generated intent
/// sequences -- not because the invariant is in doubt, but because the LAW
/// is a binding test obligation per State.cs's header, and a future
/// implementation swap must not be able to violate it silently.
/// </summary>
public sealed class Projection<T> : IProjection<T>
{
    public Projection(IStateAtom<T> source) => Source = source;

    public IStateAtom<T> Source { get; }

    // Redeclares the interface's own default-bodied member: C# does NOT
    // surface a default interface implementation on a reference typed to
    // the CONCRETE class (only through an IProjection<T>-typed reference) --
    // every real call site in this app holds a Projection<T> field/variable,
    // not an IProjection<T> one, so without this redeclaration `.Value`
    // would be a compile error there. Still a pure forwarding read, NOT
    // storage -- the interface's own doc comment permits overriding, just
    // never with a stored copy; this is the identical expression the
    // default body already uses.
    public T Value => Source.Value;
}
