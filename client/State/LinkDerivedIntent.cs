using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-1: the ONE generic vehicle <see cref="StateLinkRunner{A,B}"/>
/// uses to dispatch a link's <see cref="IStateLink{A,B}.Derive"/> result into
/// its Target -- <see cref="IStateLink{A,B}.Derive"/> returns a plain VALUE
/// (B), not an intent (the compiled contract's own shape), so something has
/// to wrap it before it can reach <see cref="IStateAtom{T}.Dispatch"/>. Using
/// one generic wrapper here (rather than requiring every atom's own intent
/// vocabulary to define an "external override" shape) keeps the link runner
/// completely agnostic to what B's real intent records look like.
///
/// Idempotent by construction (<see cref="Apply"/> ignores <paramref
/// name="current"/> entirely and always returns the same
/// <see cref="NewValue"/>) -- law 2 holds trivially. <see cref="Origin"/> is
/// always the deriving link's own <see cref="IStateLink{A,B}.Name"/> -- never
/// null -- which is exactly law 3's echo tag: <see cref="StateLinkRunner{A,B}"/>
/// reads it back off the target atom's own <c>LastOrigin</c> to recognize
/// "this atom's last change was link-derived" and refuse to re-derive
/// through it.
/// </summary>
public sealed record LinkDerivedIntent<T>(string Origin, T NewValue) : IIntent<T>
{
    public string Name => $"link:{Origin}";

    public T Apply(T current) => NewValue;
}
