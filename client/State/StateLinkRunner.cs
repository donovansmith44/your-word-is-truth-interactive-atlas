using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

public sealed class StateLinkRunner<A, B> : IDisposable
    where A : notnull
    where B : notnull
{
    private readonly string _name;
    private readonly IStateLink<A, B> _link;
    private readonly StateAtom<A> _source;
    private readonly StateAtom<B> _target;
    private bool _disposed;

    public StateLinkRunner(string name, IStateLink<A, B> link, StateAtom<A> source, StateAtom<B> target)
    {
        if (!ReferenceEquals(link.Source, source))
        {
            throw new ArgumentException("link.Source must be the same instance as the supplied source atom", nameof(source));
        }

        if (!ReferenceEquals(link.Target, target))
        {
            throw new ArgumentException("link.Target must be the same instance as the supplied target atom", nameof(target));
        }

        _name = name;
        _link = link;
        _source = source;
        _target = target;
        _source.Changed += OnSourceChanged;
    }

    private void OnSourceChanged()
    {
        if (_disposed || !_link.Active)
        {
            return;
        }

        // No-echo: if Source's last effective change was itself link-derived, refuse to
        // re-derive -- otherwise a bidirectional link pair (A<->B, both Active) would oscillate
        // forever on a single user gesture. LastOrigin is sticky (holds the last real change's
        // origin indefinitely, not just for this instant), so this guard would misfire if an atom
        // is ever both a link's target and an independently-linked source -- not a case that
        // exists in this app today.
        if (_source.LastOrigin is not null)
        {
            return;
        }

        var derived = _link.Derive(_source.Value, _target.Value);
        _target.Dispatch(new LinkDerivedIntent<B>(_name, derived));
    }

    // Deliberately skips the no-echo guard: a caller-invoked sync is not a reaction to a Source
    // echo, and this covers the case a Changed subscription alone can't -- Active flipping
    // false->true (e.g. the follow toggle) needs to resync Target to Source's current value
    // immediately, not wait for the next unrelated Source mutation.
    public void SyncNow()
    {
        if (_disposed || !_link.Active)
        {
            return;
        }

        var derived = _link.Derive(_source.Value, _target.Value);
        _target.Dispatch(new LinkDerivedIntent<B>(_name, derived));
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _source.Changed -= OnSourceChanged;
    }
}
