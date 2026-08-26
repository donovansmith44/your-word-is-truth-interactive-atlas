// Batch C3 (dense-marker disambiguation + clustering): map.js's own
// `marker-cluster-{n}` glyph (decision 3) is deliberately namespaced under
// the SAME `marker-` prefix every other marker kind on this plate already
// uses (see CONTRACT.md's own `marker-cluster-{n}` note) -- which means a
// bare `/^marker-/` locator, this suite's own long-standing "one element
// per LIT place" idiom (predates C3), now ALSO matches cluster glyphs
// whenever a scene's own far/mid-tier fit produces any. This regex is the
// shared fix: `marker-` NOT immediately followed by `cluster-`, so every
// existing "count of lit place markers" assertion keeps meaning exactly
// what it always did, regardless of whether any of a window's own places
// happen to be clustered together this pass (every place's own
// `marker-{placeId}` element stays ATTACHED, never removed, even while
// hidden inside a cluster -- see applyMarkerClusters' own comment -- so a
// `toBeAttached()` check on one specific id is unaffected either way, only
// a bare COUNT across the whole `marker-` namespace needs this).
export const LIT_MARKER_TESTID = /^marker-(?!cluster-)/;

// Same exclusion, as a raw CSS selector (Playwright's own `:visible`
// pseudo-class appended) -- for a caller that wants "any ONE lit marker
// that's actually visible right now" (e.g. `.first()`), not just a count.
// A `marker-{placeId}` element absorbed into a cluster this pass stays
// ATTACHED but hidden (applyMarkerClusters' own comment) -- LIT_MARKER_TESTID
// alone would still happily match it and hand a caller a "visible" element
// that never becomes visible, so this selector filters that out too.
export const VISIBLE_LIT_MARKER_SELECTOR = '[data-testid^="marker-"]:not([data-testid^="marker-cluster-"]):visible';
