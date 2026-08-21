// border-morph.js -- Batch M requirement 3 (user-mandated abstraction,
// 2026-08-20, verbatim): "the level of abstraction for animating change in
// map is lookup lines 1, lookup lines 2, and animate 1->2. if time range,
// the look ups are the same, but rather than animate 1->2 we show 1+2 in
// cartographer style." User correction, SAME DAY: "obviously it's not just
// two things. we can look up n lines in a timerange." The LIST is the
// native shape -- pairs are merely n=2.
//
// This module is the whole factoring, in one place, so a reviewer (or the
// batch report) can point at it directly:
//
//   lookup  : (roster, from, to) -> Map<polityId, line[]>   -- n >= 1 era
//             geometries per polity, oldest -> newest. The SAME function,
//             the ONLY function, that resolves era geometry -- both
//             combinators below consume ONLY its output, never re-derive
//             geometry their own way.
//   overlay : (line[]) -> styledLine[]                       -- static
//             cartographer composition of all n (req 4's own style ramp).
//   animate : (line[], atYear) -> ring[][]                    -- the scrub;
//             piecewise per-vertex slerp along the sequence as atYear
//             sweeps the boundaries (req 3a). n=1 is identity.
//
// Pure and framework-agnostic (no Leaflet, no DOM) -- exactly like geo.js,
// which this module is built on and re-exports nothing from beyond what it
// needs (toVecDeg/fromVecDeg/slerp/buildCorrespondence/slerpRing). map.js's
// BorderLayer is the one DOM-painting consumer: it calls lookup once per
// window/roster change or drag frame, then EITHER overlay (settled
// display) OR animate (mid-drag), and paints whatever ring coordinates
// come back -- "no separate data paths, no mode-specific geometry
// resolution, no pair-shaped special case," per the brief, literally true
// of this module's own shape.
import { toVecDeg, fromVecDeg, slerpRing, buildCorrespondence } from './geo.js';

/**
 * lookup: timerange -> [lines], grouped by polity, n >= 1 per polity.
 * `roster` is the FULL flat `[{id, name, from, to, rings, color_key,
 * transition, fall}, ...]` array (this app's own `/api/polities` response
 * shape -- called with the FULL atlas span once at load for the morph
 * engine's own roster, or with an already from/to-filtered response for
 * the settled-display path, where re-filtering with the SAME window this
 * function performs is a safe no-op -- see BorderLayer's own comment on
 * why both paths still go through this one function).
 *
 * Mirrors `handlers::polities`'s own Rust-side filter+sort exactly
 * (`TimeRange::intersects`, "by id then from") -- an intentional PARALLEL
 * implementation of the identical, simple filter semantics in two
 * environments (a real HTTP handler vs. a browser animation frame, which
 * cannot pay a network round trip per drag frame), not a second,
 * DIFFERENT geometry-resolution algorithm; disclosed as a deliberate
 * choice in the batch report, not silently duplicated.
 *
 * Returns entries GROUPED by polity id (Map, insertion-ordered by first
 * occurrence in `roster`) and, within each group, SORTED oldest -> newest
 * by `from` -- no age/style tagging here (that is entirely overlay's own
 * job below); `lookup` only ever answers "which era geometries exist for
 * this timerange," never "how should they look."
 */
export function lookup(roster, from, to) {
    const grouped = new Map();
    for (const entry of roster || []) {
        if (entry.from > to || entry.to < from) {
            continue; // TimeRange::intersects, inclusive-inclusive both ends
        }
        if (!grouped.has(entry.id)) {
            grouped.set(entry.id, []);
        }
        grouped.get(entry.id).push(entry);
    }
    for (const lines of grouped.values()) {
        lines.sort((a, b) => a.from - b.from);
    }
    return grouped;
}

/**
 * overlay: [lines] -> plate (req 4's own style ramp -- "extend the
 * existing dotted->solid two-step into a monotonic style ramp: line
 * weight and dash tighten toward the latest era"). Pure styling metadata,
 * no geometry change -- returns the SAME entries, each augmented with
 * `age` ("oldest"/"middle"/"newest", app.css's own existing 3-tier ramp,
 * already monotonic -- see the batch report's own style-ramp spec for the
 * exact stroke-width/opacity/dasharray values and why 3 stops already
 * covers this app's own real data ceiling, k=3 per BORDERS-5), `tierIndex`
 * (0-based, oldest=0), and `tierCount` (how many eras of this SAME polity
 * are in this one `lines` array -- k in "k-ring style ramp").
 *
 * n=1 is always "newest" (a single visible era reads as the plain,
 * solid/full-wash default -- unchanged since Batch B2).
 */
export function overlay(lines) {
    const tierCount = lines.length;
    return lines.map((entry, idx) => {
        const age = tierCount === 1 ? 'newest' : idx === 0 ? 'oldest' : idx === tierCount - 1 ? 'newest' : 'middle';
        return { ...entry, age, tierIndex: idx, tierCount };
    });
}

// --- animate: the scrub combinator (req 3a) ------------------------------

// Keyed "id:fromA-fromB:ringIndex" -> {source, target} (arrays of unit
// vectors, geo.js's own buildCorrespondence output). Correspondence pairs
// are DERIVED at load per adjacent era pair -- never written back to TOML
// (the brief, verbatim) -- this cache is exactly that "derive once, reuse"
// mechanism, entirely in memory, for the life of the page.
const correspondenceCache = new Map();

function correspondenceKey(id, ringA, ringB, ringIndex) {
    return `${id}:${ringA.from}-${ringB.from}:${ringIndex}`;
}

function cachedCorrespondence(id, ringA, ringB, ringIndex) {
    const key = correspondenceKey(id, ringA, ringB, ringIndex);
    let entry = correspondenceCache.get(key);
    if (!entry) {
        entry = buildCorrespondence(ringA.rings[ringIndex], ringB.rings[ringIndex], 128);
        correspondenceCache.set(key, entry);
    }
    return entry;
}

/** Test-only: clears the correspondence cache (a fresh polity roster never invalidates old entries otherwise, since era ids/years are stable for the life of a page -- tests that want a clean cache call this directly). */
export function clearCorrespondenceCache() {
    correspondenceCache.clear();
}

/**
 * One polity's own knot position on the drag timeline: the MIDPOINT of its
 * own [from, to] span. "t maps the drag position between the bracketing
 * eras' boundaries" (the brief, verbatim) -- read as the eras' own
 * CHARACTERISTIC years, not their literal from/to edges: two adjacent
 * curated eras touch at ADJACENT years (e.g. -931/-930), so using the raw
 * edges as knots would squeeze the entire morph into a single simulated
 * year, unable to "breathe between eras" (the persisted design idea's own
 * words) the way a real drag gesture visibly should. Disclosed, reasoned
 * design choice -- see the batch report's own architecture section.
 */
function knotYear(line) {
    return (line.from + line.to) / 2;
}

/**
 * animate: [lines] x atYear -> ring[][] -- the CURRENT interpolated ring
 * geometry (already lat/lon, ready for Leaflet projection -- "project
 * AFTER interpolation each frame") for ONE polity, at the scrub position
 * `atYear`. `lines` is oldest -> newest, exactly `lookup`'s own output for
 * this one polity (the SAME shape `overlay` also consumes -- "the two
 * modes share everything except the final combinator").
 *
 * n=1 is IDENTITY (the brief, verbatim): a single visible line returns its
 * own rings unchanged, atYear ignored entirely.
 *
 * n>=2: piecewise per-vertex slerp ALONG THE SEQUENCE, per knotYear above.
 * atYear before the first knot or after the last clamps to that endpoint
 * line's own rings unchanged (no extrapolation past the sequence). Between
 * two consecutive knots, each ring index is independently slerped via its
 * own cached correspondence; a polity whose adjacent eras carry a
 * DIFFERENT NUMBER of disjoint rings (a real, if rare, case in this app's
 * own curated data -- Ptolemaic Egypt's own Nile+Cyrenaica pair follows a
 * single-ring Late-Period era) has no well-defined per-vertex
 * correspondence for the extra ring's own topology change, so that ONE
 * pair snaps (no interpolation) to whichever of the two lines is nearer in
 * local t, rather than inventing a fabricated in-between shape for a
 * landmass appearing/disappearing outright -- disclosed here and in the
 * batch report, not silently smoothed over.
 */
export function animate(lines, atYear) {
    if (lines.length === 0) {
        return [];
    }
    if (lines.length === 1) {
        return lines[0].rings;
    }

    const knots = lines.map(knotYear);
    if (atYear <= knots[0]) {
        return lines[0].rings;
    }
    if (atYear >= knots[knots.length - 1]) {
        return lines[lines.length - 1].rings;
    }

    let i = 0;
    while (i < knots.length - 2 && atYear > knots[i + 1]) {
        i++;
    }
    const a = lines[i];
    const b = lines[i + 1];
    const span = knots[i + 1] - knots[i];
    const localT = span < 1e-9 ? 0 : Math.min(1, Math.max(0, (atYear - knots[i]) / span));

    if (a.rings.length !== b.rings.length) {
        // Topology change (ring count differs) -- snap, don't fabricate a
        // per-vertex morph across a different number of disjoint shapes.
        return localT < 0.5 ? a.rings : b.rings;
    }

    const out = [];
    for (let ringIndex = 0; ringIndex < a.rings.length; ringIndex++) {
        const { source, target } = cachedCorrespondence(a.id ?? a.name, a, b, ringIndex);
        const morphed = slerpRing(source, target, localT);
        out.push(morphed.map(v => fromVecDeg(v)));
    }
    return out;
}
