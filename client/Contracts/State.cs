namespace BibleAtlas.Client.Contracts;

// ============================================================================
// Spec §4d: STATE CONTRACTS -- semantics + algebra. APPROVED FOR
// IMPLEMENTATION ahead of the rest of the spec (owner ruling 2026-08-26,
// verbatim: "no we're not going to track down the state bugs one by one.
// we're going to structure our front end so that they cannot occur.").
//
// SEMANTICS: all shared state lives in named, typed ATOMS, one canonical
// store each. Components never hold copies -- they render PROJECTIONS and
// emit INTENTS. Desync is impossible by construction.
//
// LAWS (property-tested over generated intent sequences; every
// implementation of these interfaces is bound by all five):
//   1. Single-writer: atoms mutate only via Dispatch.
//   2. Idempotence: same intent twice = once.
//   3. No-echo: link-derived updates carry origin, never re-derive their
//      source (bidirectional links cannot oscillate).
//   4. Confluence: same intent sequence => same final state regardless of
//      render timing.
//   5. Agreement: all projections of one atom are equal ALWAYS.
//
// Seed atoms: Locus, TimeWindow, FocusStack, Selection, ViewArrangement
// (names in AtomNames below). ViewStateService remains the PERSISTENCE
// layer beneath atoms (it persists state; atoms OWN it); URLs/deep links
// are projections under the same agreement law. Follow-text IS
// Link(Locus -> TimeWindow), active in split view; both chapter boxes are
// projections of the ONE Locus atom (SYNC-1 dies here by construction).
// See Views.cs header for the compiled-authority discipline binding this file.
// ============================================================================

/// <summary>
/// SKELETON RESOLUTION (spec §4d names <c>IIntent&lt;T&gt;</c> in
/// <see cref="IStateAtom{T}.Dispatch"/> without declaring its members): an
/// intent is a pure, self-describing transition. <see cref="Apply"/> must be
/// idempotent (law 2: Apply(Apply(v)) == Apply(v)); <see cref="Origin"/> is
/// law 3's echo tag -- null for a user gesture, the deriving link's name for
/// a link-derived update, so a bidirectional link can refuse to re-derive
/// its own source.
/// </summary>
public interface IIntent<T>
{
    /// <summary>Stable intent name ("set-locus", ...) -- test/telemetry
    /// vocabulary, one name per intent shape.</summary>
    string Name { get; }

    /// <summary>Null = user-originated; otherwise the name of the
    /// <see cref="IStateLink{A,B}"/> that derived this intent (law 3).</summary>
    string? Origin { get; }

    T Apply(T current);
}

/// <summary>Spec §4d: one named, typed, canonical store.
/// <see cref="Dispatch"/> is the ONLY write path (law 1).</summary>
public interface IStateAtom<T>
{
    /// <summary>"locus" | "time-window" | "selection" | ... -- see
    /// <see cref="AtomNames"/>.</summary>
    string Name { get; }

    T Value { get; }

    void Dispatch(IIntent<T> intent);

    /// <summary>SKELETON RESOLUTION: the change-notification seam projections
    /// re-render through. Law 5 (agreement) is unachievable without one;
    /// declaring it here keeps every implementation on the same seam. Fires
    /// after <see cref="Value"/> has changed (never on a no-op dispatch --
    /// that silence is what makes law 2 observable).</summary>
    event Action? Changed;
}

/// <summary>
/// Spec §4d: what components consume; pure, read-only. The default-bodied
/// <see cref="Value"/> is DERIVED from the source atom, never stored -- a
/// projection holding its own copy is exactly the desync-by-copy disease
/// this layer retires, so no implementation may override it with storage.
/// </summary>
public interface IProjection<T>
{
    IStateAtom<T> Source { get; }

    T Value => Source.Value;
}

/// <summary>
/// Spec §4d ALGEBRA: atoms compose via LINKS -- declared derivations making
/// modes (follow-text) first-class testable objects. A link observes its
/// source and dispatches derived intents (tagged with the link's name as
/// <see cref="IIntent{T}.Origin"/> -- law 3) into its target while
/// <see cref="Active"/>; links toggle = modes.
/// </summary>
public interface IStateLink<A, B>
{
    IStateAtom<A> Source { get; }
    IStateAtom<B> Target { get; }

    /// <summary>Pure (law 4 depends on it).</summary>
    B Derive(A source, B current);

    bool Active { get; }
}

/// <summary>
/// CONTROLLER ADDITION (2026-08-26, ST-2 review Adjudication A; owner may
/// veto): an EFFECT is the async materialization of an atom's value into
/// the world (a scene fetch, JS interop) -- everything a pure projection
/// cannot be. ST-2 proved the hazard class this contract retires: singleton
/// atoms + Blazor's unordered dispose/init let a stale, still-subscribed
/// component instance run effects against the shared atom (the
/// cross-instance corruption), and law-2 idempotence means an
/// already-converged atom fires no Changed on remount (the blank-pane
/// defect). Semantics, both defects dead by construction:
/// one owner per effect name; CLAIM on mount, release on dispose (the
/// returned IDisposable), LATEST claim wins (the newest live instance is
/// always the owner); claiming RECONCILES -- the effect runs against the
/// atom's CURRENT value at claim time even though no Changed fires; while
/// claimed, the registry runs the effect on every real change of the
/// source atom. Only the registry invokes effects -- a component never
/// subscribes an effect handler to an atom directly.
/// </summary>
public interface IStateEffect<T>
{
    /// <summary>Stable effect name -- the ownership key ("follow-scene", ...).</summary>
    string Name { get; }

    IStateAtom<T> Source { get; }

    /// <summary>Gate: whether this value calls for materialization at all
    /// (e.g. only ScriptureMode windows). Pure.</summary>
    bool AppliesTo(T value);

    /// <summary>Idempotent per value: materializing the same value twice
    /// must be safe and cheap (the reconcile-on-claim path depends on it).</summary>
    Task Materialize(T value);
}

/// <summary>See <see cref="IStateEffect{T}"/>. Claim = subscribe + reconcile;
/// disposing the returned handle = release. Latest claim wins.</summary>
public interface IEffectRegistry
{
    IDisposable Claim<T>(IStateEffect<T> effect);
}

/// <summary>Spec §4d's seed atom names -- the one vocabulary shared by atom
/// implementations, persistence keys, and tests. Extend-only.</summary>
public static class AtomNames
{
    public const string Locus = "locus";
    public const string TimeWindow = "time-window";
    public const string FocusStack = "focus-stack";
    public const string Selection = "selection";
    public const string ViewArrangement = "view-arrangement";
}
