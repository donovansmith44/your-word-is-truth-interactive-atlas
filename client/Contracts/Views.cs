namespace BibleAtlas.Client.Contracts;

// ============================================================================
// THE COMPILED CONTRACT AUTHORITY -- frontend type system.
//
// Owner order 2026-08-26: "write a compilable code skeleton first and work
// directly from that so that there is no drift from our agreed upon types."
// These files ARE the agreed types of
// docs/superpowers/specs/2026-08-26-frontend-backend-contract-design.md §4
// (owner-reviewed on 2026-08-26 across five phone-review rounds), compiled so
// the compiler -- not prose -- holds implementers to them. Discipline mirrors
// the atlas-graph-types crate exactly: EXTEND-ONLY for batch implementers;
// any change to an existing member routes through the controller with an
// applied diff, per the standing types-first veto.
//
// §0 TOTAL-CAPTURE LAW (owner verbatim): "there shall be nothing implemented
// which is not captured by an interface and contract."
// ============================================================================

/// <summary>
/// Spec §4: a View composes components AND declares its own escape hatches
/// (owner amendment 2026-08-26): Reader declares "open-world"/"enter-split",
/// World the mirror, Split its collapses.
/// </summary>
public interface IView
{
    string Name { get; }
    IReadOnlyList<IViewComponent> Components { get; }
    IReadOnlyList<IEscapeHatch> EscapeHatches { get; }
}

/// <summary>Spec §4: marker + lifecycle seam. Capabilities compose onto it
/// (<see cref="IFocusComponent"/>, <see cref="IInteractive"/>) -- no
/// inheritance hierarchies.</summary>
public interface IViewComponent { }

/// <summary>
/// Spec §4b: a composition IS a view (so compositions nest, and get
/// hatches/components like any other view). Today's split view becomes the
/// first <see cref="IViewComposition"/>: Members=[Reader, World],
/// Layout="split-h" (bible left, atlas right).
/// </summary>
public interface IViewComposition : IView
{
    IReadOnlyList<IView> Members { get; }
    ICompositionLayout Layout { get; }
}

/// <summary>
/// Spec §4b: layout kind is contract DATA ("split-h" | "overlay" |
/// "pinned-panel" | ...), as are its constraints (min widths, focus rules)
/// when a layout kind grows them -- extend this interface then, never encode
/// constraints in component internals.
/// </summary>
public interface ICompositionLayout
{
    string Kind { get; }
}
