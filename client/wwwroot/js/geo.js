// geo.js -- Batch M requirement 2 ("a mathematically proven way to convert
// between the surfaces and the vectors using linear algebra laws," user
// direction 2026-08-20, verbatim). A small, pure (no Leaflet, no DOM, no
// map.js import), framework-agnostic module: the internal representation
// of a point on the biblical world's sphere is a 3D UNIT VECTOR, converted
// to/from the surface (lat/lon) coordinates every OTHER file in this app
// already uses. Every function here is a plain array-in, array-out pure
// function -- deliberately importable both from the browser (map.js/
// border-morph.js, as an ES module) and from plain Node (this module's own
// law-test spec, tests/ux/geo.spec.ts, which imports it directly with zero
// browser/Playwright page needed -- "a pure-node fast-check spec that never
// opens a browser," per the batch brief's own context note).
//
// THE LAWS ARE THE TESTS, per the brief: this file states the formulas and
// their proofs/domains; tests/ux/geo.spec.ts is where thousands of
// fast-check-generated inputs actually PROVE closure, round-trip identity,
// slerp's own geometric identities, rotation isometry, and resample's own
// properties -- see that file's own header for the full law list and
// tolerances. A future edit that breaks the algebra fails THAT suite by
// name, per the brief's own explicit requirement.
//
// Units: `toVec`/`fromVec` (the pure mathematical core the brief's own
// formula is stated against) take/return RADIANS (phi = latitude, lambda =
// longitude, the standard spherical-coordinate names) -- exactly the
// symbols the brief's own formula uses. `toVecDeg`/`fromVecDeg` are thin
// degree-based wrappers for this app's own [lat, lon]-in-degrees
// convention (every curated ring, every Leaflet call, the /api/polities
// wire) -- kept as a SEPARATE, thin layer so the law tests exercise the
// pure formula directly, in the units it's stated in, with no unit-
// conversion noise folded into what's being proven.

const DEG2RAD = Math.PI / 180;
const RAD2DEG = 180 / Math.PI;

/** Resample target point count for era-to-era ring correspondence (req 2). */
export const RESAMPLE_N = 128;

// --- toVec / fromVec: the proven conversion ---------------------------

/**
 * Forward map, EXACTLY per the brief: toVec(phi, lambda) = (cos(phi)*cos(lambda),
 * cos(phi)*sin(lambda), sin(phi)). phi/lambda in RADIANS.
 *
 * PROOF this is a unit vector (the two-line proof the brief asks for, cited
 * here in the doc comment): |toVec(phi,lambda)|^2
 *   = (cos(phi)cos(lambda))^2 + (cos(phi)sin(lambda))^2 + sin(phi)^2
 *   = cos^2(phi)*(cos^2(lambda) + sin^2(lambda)) + sin^2(phi)      [factor cos^2(phi)]
 *   = cos^2(phi)*1 + sin^2(phi)                                    [Pythagorean identity, inner]
 *   = cos^2(phi) + sin^2(phi) = 1.                                 [Pythagorean identity, outer]
 * QED -- tested directly as the CLOSURE law (tests/ux/geo.spec.ts) over
 * thousands of generated (phi, lambda) pairs, not just asserted here.
 */
export function toVec(phi, lambda) {
    const cosPhi = Math.cos(phi);
    return [cosPhi * Math.cos(lambda), cosPhi * Math.sin(lambda), Math.sin(phi)];
}

/**
 * Inverse map, EXACTLY per the brief: fromVec(x,y,z) = (atan2(z, hypot(x,y)),
 * atan2(y,x)) -- NOT asin(z) for latitude, deliberately: asin's own
 * derivative diverges as |z| -> 1 (near the poles), amplifying any float
 * error already present in z into a much larger latitude error; atan2(z,
 * hypot(x,y)) has no such blowup anywhere on the sphere (it's the same
 * numerically-stable form every serious geodesy library uses for exactly
 * this reason).
 *
 * "Clamp any float drift before inverse trig" (the brief, verbatim): this
 * function re-NORMALIZES its input first (divides by the vector's own
 * actual norm, which may have drifted from exactly 1 after a chain of
 * slerp/rotate calls) before ever calling atan2 -- so accumulated float
 * error is corrected back onto the true unit sphere in ONE place, rather
 * than silently compounding through the inverse trig.
 *
 * Degenerate domain, stated honestly (the brief's own requirement): at a
 * pole (cos(phi)=0, i.e. x=y=0) toVec collapses EVERY longitude to the same
 * point, (0,0,+-1) -- the forward map is not injective there, so longitude
 * is genuinely unrecoverable from the vector alone. This function
 * canonicalizes lambda=0 at that one domain point (explicitly, not by
 * accident of atan2(0,0)'s own IEEE754 signed-zero behavior, which differs
 * from atan2(0,-0) -- see the `horizontal < POLE_EPS` branch below) rather
 * than returning an arbitrary answer. The round-trip law
 * (tests/ux/geo.spec.ts) is therefore stated on phi in the OPEN interval
 * (-90deg, 90deg) -- irrelevant to this app's own curated data (no polity
 * ring vertex sits at a literal pole) but mandatory for the claim to be
 * true, not merely convenient.
 */
export function fromVec(v) {
    const n = norm(v);
    const x = v[0] / n, y = v[1] / n, z = v[2] / n;
    const horizontal = Math.hypot(x, y);
    const phi = Math.atan2(z, horizontal);
    const POLE_EPS = 1e-12;
    const lambda = horizontal < POLE_EPS ? 0 : Math.atan2(y, x);
    return [phi, lambda];
}

/** Degree-based wrapper (this app's own [lat, lon] convention) over the pure {@link toVec}. */
export function toVecDeg(latDeg, lonDeg) {
    return toVec(latDeg * DEG2RAD, lonDeg * DEG2RAD);
}

/** Degree-based wrapper over the pure {@link fromVec}. */
export function fromVecDeg(v) {
    const [phi, lambda] = fromVec(v);
    return [phi * RAD2DEG, lambda * RAD2DEG];
}

// --- Plain linear algebra -----------------------------------------------

export function dot(a, b) {
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

export function cross(a, b) {
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

export function norm(a) {
    return Math.sqrt(dot(a, a));
}

export function normalize(a) {
    const n = norm(a);
    return [a[0] / n, a[1] / n, a[2] / n];
}

function lerp3(a, b, t) {
    return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
}

function clamp(x, lo, hi) {
    return Math.max(lo, Math.min(hi, x));
}

/** Great-circle angular distance (radians) between two UNIT vectors. */
export function angleBetween(a, b) {
    return Math.acos(clamp(dot(a, b), -1, 1));
}

// --- slerp: the animate combinator's own per-vertex primitive -----------

/**
 * Spherical linear interpolation, standard sin-weighted form:
 * slerp(a,b,t) = (sin((1-t)*theta)*a + sin(t*theta)*b) / sin(theta), theta =
 * arccos(a.b) -- the exact form the brief specifies. `a`/`b` MUST be unit
 * vectors (every caller in this module produces them via toVec/resampleRing,
 * never hand-typed).
 *
 * Fallback, EXACTLY per the brief: when |1 - a.b| < 1e-9 (a and b are
 * numerically the same point, theta≈0), sin(theta) -- the formula's own
 * denominator -- is too close to zero to divide by safely; lerp+normalize
 * (linear interpolation, then re-projected onto the unit sphere) is used
 * instead, exact at both endpoints and a smooth, correct-in-the-limit
 * approximation in between, since a and b are themselves nearly coincident
 * there.
 *
 * Disclosed, not silently assumed: the OTHER degenerate configuration
 * (a.b ~ -1, antipodal points, theta~pi, sin(theta) ALSO ~0) gets no
 * SEPARATE fallback curve -- the brief's own fallback condition is scoped
 * to a.b~+1 only, and this app's real data never produces antipodal
 * correspondence pairs (adjacent-era border vertices of one polity never
 * move to the opposite side of the globe; there is also no single
 * well-defined shortest path between exactly-antipodal points in the first
 * place -- infinitely many great circles pass through both). What IS
 * guaranteed unconditionally, everywhere, including near (but not at)
 * antipodal inputs: the final re-normalize below. `wa*a + wb*b`'s own raw
 * sum can drift measurably off unit norm exactly where sin(theta) is
 * ill-conditioned (theta near 0 OR pi) -- caught live by this module's own
 * property tests (tests/ux/geo.spec.ts's GEO-2b/GEO-3b, which fast-check's
 * shrinker walked to a=[1,0,0], b nearly [-1,0,0], landing norm ~1e-9 off).
 * Renormalizing costs one extra sqrt and three divides, is a no-op to float
 * precision on an already-good result (t=0/t=1 endpoints included -- see
 * the endpoint-identity law), and turns the CLOSURE guarantee (every
 * output is a genuine unit vector) into something true by construction
 * rather than merely usually-true.
 */
export function slerp(a, b, t) {
    const d = clamp(dot(a, b), -1, 1);
    if (Math.abs(1 - d) < 1e-9) {
        return normalize(lerp3(a, b, t));
    }
    const theta = Math.acos(d);
    const sinTheta = Math.sin(theta);
    const wa = Math.sin((1 - t) * theta) / sinTheta;
    const wb = Math.sin(t * theta) / sinTheta;
    return normalize([wa * a[0] + wb * b[0], wa * a[1] + wb * b[1], wa * a[2] + wb * b[2]]);
}

// --- Rotation (isometry law + the closure law's own "rotations" case) ---

/**
 * Rotates unit vector `v` by `angleRad` radians about the unit `axis`,
 * Rodrigues' rotation formula (a standard, textbook result -- Rodrigues
 * 1840; see any linear algebra/computer graphics reference -- "no fancy
 * math you can't verify," a reuse of established mathematics, not an
 * invention, exactly the same standing this app's own `ring_is_simple`
 * segment-intersection test already has for planar geometry):
 *
 *   v_rot = v*cos(angle) + (axis x v)*sin(angle) + axis*(axis.v)*(1-cos(angle))
 *
 * A rotation about a fixed axis is an ORTHOGONAL transformation -- it
 * preserves dot products between any two vectors it's applied to (the
 * isometry law, tests/ux/geo.spec.ts) and therefore preserves norm (the
 * closure law's own "rotations" case: rotating a unit vector yields
 * another unit vector).
 */
export function rotateAboutAxis(v, axis, angleRad) {
    const k = normalize(axis);
    const cosA = Math.cos(angleRad);
    const sinA = Math.sin(angleRad);
    const kCrossV = cross(k, v);
    const kDotV = dot(k, v);
    const oneMinusCos = 1 - cosA;
    return [
        v[0] * cosA + kCrossV[0] * sinA + k[0] * kDotV * oneMinusCos,
        v[1] * cosA + kCrossV[1] * sinA + k[1] * kDotV * oneMinusCos,
        v[2] * cosA + kCrossV[2] * sinA + k[2] * kDotV * oneMinusCos,
    ];
}

// --- Ring resampling + correspondence (req 2's own "the real engineering
// problem," per the persisted design idea) -------------------------------

/** Strips this app's own curated closed-ring convention (first point repeats as the last), if present. */
function stripClosingRepeat(ring) {
    if (ring.length >= 2) {
        const [a0, a1] = ring[0];
        const [b0, b1] = ring[ring.length - 1];
        if (a0 === b0 && a1 === b1) {
            return ring.slice(0, -1);
        }
    }
    return ring;
}

/**
 * Resamples a closed ring (an array of `[latDeg, lonDeg]` pairs, this app's
 * own curated convention -- OPEN or CLOSED, the trailing repeat is stripped
 * first) to exactly `n` points, evenly spaced by ARC LENGTH (great-circle
 * angle) walking the whole closed loop -- not by original vertex density,
 * which the curated rings vary a lot (30-80 hand-placed points, per
 * design-direction.md's own recognizability band). Returns `n` UNIT
 * VECTORS (never lat/lon -- callers project back only after slerp/animate
 * has run, "projection happens LAST, after interpolation").
 *
 * Algorithm: walk the ring's own m edges once to get each edge's own
 * great-circle length (angleBetween of its two endpoints) and the total
 * perimeter; then walk n equally-spaced target arc-length positions
 * (0, step, 2*step, ...) with a single forward-only two-pointer scan (both
 * the target index and the current edge index only ever advance -- O(n+m)
 * total, not O(n*m)), slerping within whichever edge each target position
 * currently falls inside. Point 0 always lands exactly on the ring's own
 * first vertex (arc-length position 0 is edge 0's own start, local t=0) --
 * the "endpoints on the source ring" law (tests/ux/geo.spec.ts) is this
 * property, checked directly, not assumed.
 */
export function resampleRing(latLngRing, n = RESAMPLE_N) {
    const open = stripClosingRepeat(latLngRing);
    const verts = open.map(([lat, lon]) => toVecDeg(lat, lon));
    const m = verts.length;
    if (m === 0) {
        return [];
    }
    if (m === 1) {
        return Array.from({ length: n }, () => verts[0].slice());
    }

    const edgeAngle = new Array(m);
    let total = 0;
    for (let i = 0; i < m; i++) {
        edgeAngle[i] = angleBetween(verts[i], verts[(i + 1) % m]);
        total += edgeAngle[i];
    }
    if (total < 1e-12) {
        // Degenerate (every vertex coincides) -- nothing to resample.
        return Array.from({ length: n }, () => verts[0].slice());
    }

    const step = total / n;
    const out = new Array(n);
    let edgeIndex = 0;
    let consumed = 0; // arc length already walked, up to the START of edgeIndex
    for (let k = 0; k < n; k++) {
        const target = k * step;
        while (edgeIndex < m - 1 && consumed + edgeAngle[edgeIndex] < target - 1e-12) {
            consumed += edgeAngle[edgeIndex];
            edgeIndex++;
        }
        const a = verts[edgeIndex];
        const b = verts[(edgeIndex + 1) % m];
        const localLen = edgeAngle[edgeIndex];
        const localT = localLen < 1e-12 ? 0 : clamp((target - consumed) / localLen, 0, 1);
        out[k] = slerp(a, b, localT);
    }
    return out;
}

function meanVector(vecs) {
    let sx = 0, sy = 0, sz = 0;
    for (const v of vecs) {
        sx += v[0]; sy += v[1]; sz += v[2];
    }
    return [sx / vecs.length, sy / vecs.length, sz / vecs.length];
}

/**
 * A ring's own winding SIGN (+1/-1), used only to compare two rings'
 * windings against each other, never as an absolute area/orientation
 * claim. sum_i (v[i] x v[i+1]) is the discrete/spherical analog of the
 * planar shoelace formula -- its direction approximates the ring's own
 * normal; its dot with the ring's own mean point (a rough "up" from the
 * ring's own center) gives the sign two same-shape, oppositely-drawn rings
 * will always disagree on. Coarse and index-only, deliberately -- "no
 * fancy math you can't verify" -- this only ever needs to answer "do these
 * two rings wind the same way," never compute a real area.
 */
export function windingSign(vecs) {
    const n = vecs.length;
    let sx = 0, sy = 0, sz = 0;
    for (let i = 0; i < n; i++) {
        const c = cross(vecs[i], vecs[(i + 1) % n]);
        sx += c[0]; sy += c[1]; sz += c[2];
    }
    const mean = meanVector(vecs);
    return dot([sx, sy, sz], mean) >= 0 ? 1 : -1;
}

/**
 * The cyclic start-offset `k` (0..n-1) of `target` that MINIMIZES the
 * summed pairwise angle sum_i angleBetween(source[i], target[(i+k)%n]) --
 * "start-vertex alignment (minimize summed pairwise angle)," per the
 * brief, verbatim. Brute force over all n candidate offsets (O(n^2) angle
 * computations, ~16k for n=128) -- a ONE-TIME cost per adjacent-era-pair at
 * load, not a per-frame cost, so the simplest-to-verify approach is the
 * right one ("no fancy math you can't verify").
 */
function bestStartOffset(source, target) {
    const n = source.length;
    let bestK = 0;
    let bestSum = Infinity;
    for (let k = 0; k < n; k++) {
        let sum = 0;
        for (let i = 0; i < n; i++) {
            sum += angleBetween(source[i], target[(i + k) % n]);
        }
        if (sum < bestSum) {
            bestSum = sum;
            bestK = k;
        }
    }
    return bestK;
}

/**
 * Builds an era-to-era CORRESPONDENCE: two arrays of `n` unit vectors,
 * index-paired (source[i] <-> target[i] is one morph vertex pair), from two
 * curated lat/lon rings. "Correspondence pairs are DERIVED at load per
 * adjacent era pair -- never written back to TOML" (the brief, verbatim):
 * this function is pure and idempotent, safe to call every time a
 * (source-era, target-era) pair is first needed and cache client-side
 * (border-morph.js's own job) -- at-rest curated data never changes.
 *
 * Winding normalization: if the two resampled rings wind opposite ways
 * (drawn in opposite point order -- both directions are equally valid
 * curator input, this app's ring-simplicity validation never constrains
 * winding), `target` is reversed BEFORE alignment, so per-index slerp pairs
 * points that are actually near each other rather than producing a
 * twisted, self-crossing morph.
 *
 * Start-vertex alignment: see {@link bestStartOffset}. Applied to the
 * (possibly winding-reversed) target only -- `source`'s own point order is
 * always the untouched result of {@link resampleRing}.
 */
export function buildCorrespondence(sourceLatLngRing, targetLatLngRing, n = RESAMPLE_N) {
    const source = resampleRing(sourceLatLngRing, n);
    let target = resampleRing(targetLatLngRing, n);

    if (source.length === 0 || target.length === 0) {
        return { source, target };
    }

    if (windingSign(source) !== windingSign(target)) {
        target = target.slice().reverse();
    }

    const offset = bestStartOffset(source, target);
    const aligned = new Array(n);
    for (let i = 0; i < n; i++) {
        aligned[i] = target[(i + offset) % n];
    }
    return { source, target: aligned };
}

/**
 * Per-vertex slerp across a whole correspondence at parameter `t` -- the
 * CPU evaluator's own inner loop (border-morph.js's `animate` combinator
 * calls this once per polity per frame while dragging). Returns `n` unit
 * vectors; projection (toVec -> Leaflet layer point) happens in the
 * caller, AFTER this, per the brief's "project AFTER interpolation each
 * frame."
 */
export function slerpRing(source, target, t) {
    const n = source.length;
    const out = new Array(n);
    for (let i = 0; i < n; i++) {
        out[i] = slerp(source[i], target[i], t);
    }
    return out;
}
