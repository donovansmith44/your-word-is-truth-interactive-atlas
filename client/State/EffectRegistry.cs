using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

public sealed class EffectRegistry : IEffectRegistry
{
    private sealed class Slot
    {
        public object? Owner;
        public Action? Unsubscribe;
    }

    private readonly Dictionary<string, Slot> _slots = new();

    public EffectClaim Claim<T>(IStateEffect<T> effect)
    {
        var slot = GetOrAddSlot(effect.Name);

        slot.Unsubscribe?.Invoke();
        slot.Unsubscribe = null;

        var token = new object();
        slot.Owner = token;

        void OnSourceChanged()
        {
            if (!ReferenceEquals(slot.Owner, token))
            {
                return;
            }

            var value = effect.Source.Value;
            if (effect.AppliesTo(value))
            {
                _ = effect.Materialize(value);
            }
        }

        effect.Source.Changed += OnSourceChanged;
        slot.Unsubscribe = () => effect.Source.Changed -= OnSourceChanged;

        // A converged atom raises no Changed for a brand-new claim, so materialize once against the current value here.
        Task? reconcileTask = null;
        if (effect.AppliesTo(effect.Source.Value))
        {
            reconcileTask = effect.Materialize(effect.Source.Value);
        }

        return new EffectClaim(reconcileTask, () =>
        {
            effect.Source.Changed -= OnSourceChanged;
            if (ReferenceEquals(slot.Owner, token))
            {
                slot.Owner = null;
                slot.Unsubscribe = null;
            }
        });
    }

    IDisposable IEffectRegistry.Claim<T>(IStateEffect<T> effect) => Claim(effect);

    public void Release(string name)
    {
        if (!_slots.TryGetValue(name, out var slot))
        {
            return;
        }

        slot.Unsubscribe?.Invoke();
        slot.Unsubscribe = null;
        slot.Owner = null;
    }

    private Slot GetOrAddSlot(string name)
    {
        if (!_slots.TryGetValue(name, out var slot))
        {
            slot = new Slot();
            _slots[name] = slot;
        }

        return slot;
    }
}

public sealed class EffectClaim : IDisposable
{
    private Action? _release;

    internal EffectClaim(Task? reconcileTask, Action release)
    {
        ReconcileTask = reconcileTask;
        _release = release;
    }

    // Only valid immediately after Claim returns; later Changed-triggered materializations are fire-and-forget and never update it.
    public Task? ReconcileTask { get; }

    public void Dispose()
    {
        _release?.Invoke();
        _release = null;
    }
}

public sealed class DelegateEffect<T> : IStateEffect<T> where T : notnull
{
    private readonly Func<T, bool> _appliesTo;
    private readonly Func<T, Task> _materialize;

    public DelegateEffect(string name, IStateAtom<T> source, Func<T, bool> appliesTo, Func<T, Task> materialize)
    {
        Name = name;
        Source = source;
        _appliesTo = appliesTo;
        _materialize = materialize;
    }

    public string Name { get; }
    public IStateAtom<T> Source { get; }
    public bool AppliesTo(T value) => _appliesTo(value);
    public Task Materialize(T value) => _materialize(value);
}
