import { test, expect } from '@playwright/test';
import fc from 'fast-check';
// Batch M requirement 2 ("a mathematically proven way to convert between
// the surfaces and the vectors using linear algebra laws," user direction
// 2026-08-20, verbatim): THE LAWS ARE THE TESTS. This file imports
// client/wwwroot/js/geo.js DIRECTLY, node-side -- no page.goto, no browser,
// no webServer dependency for the assertions themselves (a pure-node
// fast-check spec, per the batch brief's own context note: "a pure-node
// fast-check spec that never opens a browser is fine and fast"). It still
// lives in tests/ux/ as an ordinary *.spec.ts file, discovered and run the
// same way as every other spec in this suite, so a future edit that breaks
// the algebra fails THIS suite, by name, exactly as the brief requires.
import {
  toVec, fromVec, toVecDeg, fromVecDeg, dot, cross, norm, normalize, angleBetween,
  slerp, rotateAboutAxis, resampleRing, buildCorrespondence, windingSign, slerpRing, RESAMPLE_N,
} from '../../client/wwwroot/js/geo.js';

// Pure, synchronous, in-process math -- each property evaluates in
// microseconds, so "thousands of generated inputs" (the brief's own
// wording) costs nothing close to what a network/browser-bound property
// elsewhere in this suite would (see lib/fc.ts's own RUNS_API/RUNS_UI,
// tuned much lower for exactly that reason). A dedicated, much higher
// constant for this file alone.
const RUNS_LAW = 3000;

const EPS_POLE = 1e-6; // stays clear of the poles, where longitude is genuinely unrecoverable (see geo.js's own fromVec doc comment)
const TOL = 1e-9; // the brief's own stated round-trip/law tolerance

function arbPhi() {
  return fc.double({ min: -Math.PI / 2 + EPS_POLE, max: Math.PI / 2 - EPS_POLE, noNaN: true });
}
function arbLambda() {
  // Canonical range atan2 itself always returns into -- restricting
  // generation to it makes the round-trip law a literal value match rather
  // than needing a mod-2*pi equivalence check (the same POINT, e.g.
  // lambda=3*pi, would otherwise legitimately canonicalize to a different
  // literal number without the underlying geometry having changed at all).
  return fc.double({ min: -Math.PI, max: Math.PI, noNaN: true });
}
function arbT() {
  return fc.double({ min: 0, max: 1, noNaN: true });
}
function arbAngle() {
  return fc.double({ min: -Math.PI, max: Math.PI, noNaN: true });
}
/** A unit vector, built from a random (phi, lambda) pair -- not statistically area-uniform on the sphere, but varied/representative, which is all a property test needs. */
function arbUnitVec() {
  return fc.tuple(arbPhi(), arbLambda()).map(([phi, lambda]) => toVec(phi, lambda));
}
function closeTo(a: number, b: number, tol: number, msg: string) {
  expect(Math.abs(a - b), `${msg}: |${a} - ${b}| = ${Math.abs(a - b)}, expected <= ${tol}`).toBeLessThanOrEqual(tol);
}
function vecCloseTo(a: number[], b: number[], tol: number, msg: string) {
  for (let i = 0; i < 3; i++) {
    closeTo(a[i], b[i], tol, `${msg} [component ${i}]`);
  }
}

// --- (a) round-trip identity, BOTH directions, within 1e-9, away from poles ---

test('GEO-1a: fromVec(toVec(phi, lambda)) round-trips within 1e-9, phi in the open interval (-90deg, 90deg)', () => {
  fc.assert(
    fc.property(arbPhi(), arbLambda(), (phi, lambda) => {
      const [phi2, lambda2] = fromVec(toVec(phi, lambda));
      closeTo(phi2, phi, TOL, 'phi round-trip');
      closeTo(lambda2, lambda, TOL, 'lambda round-trip');
    }),
    { numRuns: RUNS_LAW }
  );
});

test('GEO-1b: toVec(fromVec(v)) round-trips within 1e-9, for v built from a non-pole (phi, lambda)', () => {
  fc.assert(
    fc.property(arbPhi(), arbLambda(), (phi, lambda) => {
      const v = toVec(phi, lambda);
      const v2 = toVec(...fromVec(v));
      vecCloseTo(v2, v, TOL, 'vector round-trip');
    }),
    { numRuns: RUNS_LAW }
  );
});

test('GEO-1c: toVecDeg/fromVecDeg are faithful degree wrappers over the pure radian core', () => {
  fc.assert(
    fc.property(
      fc.double({ min: -90 + EPS_POLE, max: 90 - EPS_POLE, noNaN: true }),
      fc.double({ min: -179, max: 180, noNaN: true }),
      (latDeg, lonDeg) => {
        const viaDeg = toVecDeg(latDeg, lonDeg);
        const viaRad = toVec((latDeg * Math.PI) / 180, (lonDeg * Math.PI) / 180);
        vecCloseTo(viaDeg, viaRad, TOL, 'toVecDeg vs toVec');
        const backDeg = fromVecDeg(viaDeg);
        closeTo(backDeg[0], latDeg, TOL * (180 / Math.PI) + 1e-7, 'fromVecDeg lat');
      }
    ),
    { numRuns: RUNS_LAW }
  );
});

// --- (b) closure: every module operation returns unit norm ---

test('GEO-2a: toVec always returns a unit vector, everywhere including at the poles', () => {
  fc.assert(
    fc.property(fc.double({ min: -Math.PI / 2, max: Math.PI / 2, noNaN: true }), arbLambda(), (phi, lambda) => {
      closeTo(norm(toVec(phi, lambda)), 1, TOL, 'toVec norm');
    }),
    { numRuns: RUNS_LAW }
  );
});

test('GEO-2b: slerp(a, b, t) always returns a unit vector', () => {
  fc.assert(
    fc.property(arbUnitVec(), arbUnitVec(), arbT(), (a, b, t) => {
      closeTo(norm(slerp(a, b, t)), 1, TOL, 'slerp norm');
    }),
    { numRuns: RUNS_LAW }
  );
});

test('GEO-2c: rotateAboutAxis(v, axis, angle) always returns a unit vector', () => {
  fc.assert(
    fc.property(arbUnitVec(), arbUnitVec(), arbAngle(), (v, axis, angle) => {
      closeTo(norm(rotateAboutAxis(v, axis, angle)), 1, TOL, 'rotate norm');
    }),
    { numRuns: RUNS_LAW }
  );
});

// --- (c) slerp identities ---

test('GEO-3a: slerp endpoint identities -- t=0 returns a, t=1 returns b', () => {
  fc.assert(
    fc.property(arbUnitVec(), arbUnitVec(), (a, b) => {
      vecCloseTo(slerp(a, b, 0), a, TOL, 'slerp t=0');
      vecCloseTo(slerp(a, b, 1), b, TOL, 'slerp t=1');
    }),
    { numRuns: RUNS_LAW }
  );
});

test('GEO-3b: slerp symmetry -- slerp(a, b, t) === slerp(b, a, 1 - t)', () => {
  fc.assert(
    fc.property(arbUnitVec(), arbUnitVec(), arbT(), (a, b, t) => {
      // Same near-antipodal exclusion as GEO-3c/GEO-3d (see geo.js's own
      // slerp doc comment): the renormalize fix guarantees CLOSURE (unit
      // norm, GEO-2b) unconditionally, but symmetry is a DIRECTIONAL claim
      // -- acos's own derivative diverges as dot(a,b) -> -1, so a
      // sub-ULP-level difference in how theta is computed can still shift
      // the result's own AIM measurably right at that ill-conditioned
      // edge, which normalize (a pure rescale) cannot correct. Filtered,
      // not silently passed vacuously -- same fc.pre reasoning as GEO-3c.
      const theta = angleBetween(a, b);
      fc.pre(theta < Math.PI - 1e-3);
      vecCloseTo(slerp(a, b, t), slerp(b, a, 1 - t), TOL, 'slerp symmetry');
    }),
    { numRuns: RUNS_LAW }
  );
});

test('GEO-3c: slerp angle linearity -- angleBetween(a, slerp(a,b,t)) === t * angleBetween(a,b)', () => {
  fc.assert(
    fc.property(
      arbUnitVec(),
      arbUnitVec(),
      arbT(),
      (a, b, t) => {
        const theta = angleBetween(a, b);
        // Stays clear of BOTH of slerp's own documented ill-conditioned
        // zones (see geo.js's own slerp doc comment): theta near 0 (the
        // lerp+normalize fallback boundary, |1-a.b| < 1e-9) and theta near
        // pi (near-antipodal a/b, where sin(theta) -- the formula's own
        // denominator -- is ALSO near zero and no separate fallback curve
        // is defined). Angle linearity is a property of the sin-weighted
        // geometric path specifically; filtered, not silently passed
        // vacuously -- fast-check's own `fc.pre` re-draws until the
        // predicate holds, so this still exercises thousands of genuinely
        // varied (a, b, t) triples across the well-conditioned domain.
        fc.pre(theta > 1e-3 && theta < Math.PI - 1e-3);
        const measured = angleBetween(a, slerp(a, b, t));
        closeTo(measured, t * theta, 1e-6, 'angle linearity');
      }
    ),
    { numRuns: RUNS_LAW }
  );
});

test('GEO-3d: slerp planarity -- slerp(a,b,t) is orthogonal to a x b', () => {
  fc.assert(
    fc.property(arbUnitVec(), arbUnitVec(), arbT(), (a, b, t) => {
      const n = cross(a, b);
      // a x b is only a meaningful plane normal when a and b are not
      // (anti)parallel -- filtered the same way GEO-3c filters near-zero
      // theta, plus the mirror case near theta=pi.
      const theta = angleBetween(a, b);
      fc.pre(theta > 1e-3 && theta < Math.PI - 1e-3);
      closeTo(dot(slerp(a, b, t), n), 0, 1e-6, 'slerp planarity');
    }),
    { numRuns: RUNS_LAW }
  );
});

// --- (d) rotation isometry ---

test('GEO-4: rotation about any axis preserves dot products (orthogonal transformation)', () => {
  fc.assert(
    fc.property(arbUnitVec(), arbUnitVec(), arbUnitVec(), arbAngle(), (u, v, axis, angle) => {
      const before = dot(u, v);
      const after = dot(rotateAboutAxis(u, axis, angle), rotateAboutAxis(v, axis, angle));
      closeTo(after, before, TOL, 'rotation isometry');
    }),
    { numRuns: RUNS_LAW }
  );
});

// --- (e) resample: point count, closure, winding, endpoints on the source ring ---

/**
 * A closed lat/lon ring shaped like a mildly-perturbed regular polygon --
 * 3-10 vertices, first repeated as last per this app's own curated
 * convention. Deliberately NOT fully-independent random points: an early
 * version generated those, and fast-check's own shrinker readily walked
 * them to degenerate slivers (every point within float-epsilon of one
 * spot, or a thin spike out to a single outlier) for which "winding" has
 * no real meaning -- see GEO-5c's own comment. A perturbed polygon is both
 * a guaranteed-non-degenerate fix AND a closer analog of this app's own
 * real curated rings (a recognizable, roughly-polygonal shape, 30-80
 * hand-placed points per design-direction.md) than scattered points ever
 * were. `cw` covers BOTH winding directions -- real curated rings are
 * drawn in whichever order was natural to the curator, never constrained.
 */
function arbRing() {
  return fc
    .tuple(
      fc.integer({ min: 3, max: 10 }),
      fc.double({ min: -70, max: 70, noNaN: true }),
      fc.double({ min: -170, max: 170, noNaN: true }),
      fc.double({ min: 2, max: 15, noNaN: true }),
      fc.boolean()
    )
    .map(([count, centerLat, centerLon, radius, cw]) => {
      const pts: [number, number][] = [];
      for (let i = 0; i < count; i++) {
        const angle = (cw ? -1 : 1) * (i / count) * 2 * Math.PI;
        // Deterministic (function of i only, no extra fast-check
        // randomness) per-vertex radius jitter so the ring isn't a
        // perfect polygon either, while staying well clear of degenerate.
        const r = radius * (0.7 + 0.3 * Math.abs(Math.sin(i * 2.399963229728653)));
        pts.push([centerLat + r * Math.sin(angle), centerLon + r * Math.cos(angle)]);
      }
      pts.push(pts[0]);
      return pts;
    });
}

test('GEO-5a: resampleRing always returns exactly n points', () => {
  fc.assert(
    fc.property(arbRing(), fc.integer({ min: 4, max: 200 }), (ring, n) => {
      expect(resampleRing(ring, n).length).toBe(n);
    }),
    { numRuns: RUNS_LAW }
  );
});

test('GEO-5b: every resampled point is a unit vector (closure)', () => {
  fc.assert(
    fc.property(arbRing(), (ring) => {
      for (const v of resampleRing(ring, RESAMPLE_N)) {
        closeTo(norm(v), 1, TOL, 'resampled point norm');
      }
    }),
    { numRuns: 300 } // resample itself does O(RESAMPLE_N) work per call; 300 rings * 128 points is already 38k checks
  );
});

test('GEO-5d: resampleRing point 0 lands exactly on the source ring\'s own first vertex', () => {
  fc.assert(
    fc.property(arbRing(), fc.integer({ min: 4, max: 200 }), (ring, n) => {
      const resampled = resampleRing(ring, n);
      const expected = toVecDeg(ring[0][0], ring[0][1]);
      vecCloseTo(resampled[0], expected, 1e-9, 'resample endpoint on source ring');
    }),
    { numRuns: RUNS_LAW }
  );
});

test('GEO-5c: buildCorrespondence always aligns source and target to the SAME winding, regardless of the target ring\'s own original point order', () => {
  fc.assert(
    fc.property(arbRing(), arbRing(), fc.boolean(), (ringA, ringB, reverseB) => {
      const target = reverseB ? ringB.slice().reverse() : ringB;
      const { source, target: aligned } = buildCorrespondence(ringA, target, 32);
      expect(windingSign(source)).toBe(windingSign(aligned));
    }),
    { numRuns: 500 }
  );
});

test('GEO-5: n=1 (a single visible era) is the animate combinator\'s own identity case -- slerpRing at any t returns the same ring unchanged', () => {
  fc.assert(
    fc.property(arbRing(), arbT(), (ring, t) => {
      const line = resampleRing(ring, 32);
      const still = slerpRing(line, line, t);
      for (let i = 0; i < line.length; i++) {
        vecCloseTo(still[i], line[i], TOL, `slerpRing identity at index ${i}`);
      }
    }),
    { numRuns: 500 }
  );
});

// --- Fixed-value spot checks against independently computed great-circle
// values (the brief's own explicit requirement) ---------------------------
//
// Independent method: the standard flat great-circle "intermediate point"
// formula (Ed Williams' Aviation Formulary / the well-known Movable Type
// Scripts geodesy reference), computed here directly against the raw
// lat/lon numbers -- a SEPARATE code path from geo.js's own toVec/slerp/
// fromVec composition (no call into this module's own functions), even
// though both implement the same well-established great-circle
// mathematics. The angular-distance cross-check uses the HAVERSINE
// formula specifically, a genuinely differently-derived formula for the
// same quantity (historically motivated by better conditioning at small
// angles than a raw arccos), not merely the same arithmetic written twice.
function haversineAngleRad(lat1: number, lon1: number, lat2: number, lon2: number): number {
  const r = Math.PI / 180;
  const [p1, l1, p2, l2] = [lat1 * r, lon1 * r, lat2 * r, lon2 * r];
  const dLat = p2 - p1, dLon = l2 - l1;
  const a = Math.sin(dLat / 2) ** 2 + Math.cos(p1) * Math.cos(p2) * Math.sin(dLon / 2) ** 2;
  return 2 * Math.asin(Math.min(1, Math.sqrt(a)));
}
function greatCircleMidpointDeg(lat1: number, lon1: number, lat2: number, lon2: number): [number, number] {
  const r = Math.PI / 180;
  const [p1, l1, p2, l2] = [lat1 * r, lon1 * r, lat2 * r, lon2 * r];
  const d = haversineAngleRad(lat1, lon1, lat2, lon2);
  const a = Math.sin(d / 2) / Math.sin(d);
  const b = Math.sin(d / 2) / Math.sin(d);
  const x = a * Math.cos(p1) * Math.cos(l1) + b * Math.cos(p2) * Math.cos(l2);
  const y = a * Math.cos(p1) * Math.sin(l1) + b * Math.cos(p2) * Math.sin(l2);
  const z = a * Math.sin(p1) + b * Math.sin(p2);
  return [(Math.atan2(z, Math.sqrt(x * x + y * y)) * 180) / Math.PI, (Math.atan2(y, x) * 180) / Math.PI];
}

// Jerusalem (31.78N, 35.22E -- the same LAND_POINT world-land-mask.spec.ts
// already uses) and Babylon (32.54N, 44.42E, the ruins near Hillah, Iraq).
test('GEO-6a: slerp midpoint (Jerusalem -> Babylon) matches an independently-computed great-circle midpoint', () => {
  const jer = toVecDeg(31.78, 35.22);
  const bab = toVecDeg(32.54, 44.42);
  const mid = fromVecDeg(slerp(jer, bab, 0.5));
  const expected = greatCircleMidpointDeg(31.78, 35.22, 32.54, 44.42);
  closeTo(mid[0], expected[0], 1e-6, 'Jerusalem-Babylon midpoint lat');
  closeTo(mid[1], expected[1], 1e-6, 'Jerusalem-Babylon midpoint lon');
  // And the literal, independently-verified numeric value (computed once,
  // offline, via the same reference formula -- pinned so a future
  // regression in EITHER the module or this test's own formula shows up as
  // a concrete, readable failure, not just an internal cross-check).
  closeTo(mid[0], 32.24335476366412, 1e-6, 'Jerusalem-Babylon midpoint lat, pinned value');
  closeTo(mid[1], 39.80077599306642, 1e-6, 'Jerusalem-Babylon midpoint lon, pinned value');
});

test('GEO-6b: angleBetween (Jerusalem -> Babylon) matches an independently-computed haversine distance', () => {
  const jer = toVecDeg(31.78, 35.22);
  const bab = toVecDeg(32.54, 44.42);
  const measured = angleBetween(jer, bab);
  const expected = haversineAngleRad(31.78, 35.22, 32.54, 44.42);
  closeTo(measured, expected, 1e-9, 'Jerusalem-Babylon angular distance');
  closeTo(measured, 0.13653541261614513, 1e-9, 'Jerusalem-Babylon angular distance, pinned value');
});

// A second, longer-distance pair (Jerusalem -> Rome, ~2300km/20.7deg) so
// the spot check isn't tuned to one short, possibly-lucky distance.
test('GEO-6c: slerp midpoint and angular distance (Jerusalem -> Rome) match independently-computed great-circle values', () => {
  const jer = toVecDeg(31.78, 35.22);
  const rome = toVecDeg(41.9, 12.5);
  const mid = fromVecDeg(slerp(jer, rome, 0.5));
  const expectedMid = greatCircleMidpointDeg(31.78, 35.22, 41.9, 12.5);
  closeTo(mid[0], expectedMid[0], 1e-6, 'Jerusalem-Rome midpoint lat');
  closeTo(mid[1], expectedMid[1], 1e-6, 'Jerusalem-Rome midpoint lon');
  closeTo(mid[0], 37.38300243149092, 1e-6, 'Jerusalem-Rome midpoint lat, pinned value');
  closeTo(mid[1], 24.623561974420877, 1e-6, 'Jerusalem-Rome midpoint lon, pinned value');

  const measured = angleBetween(jer, rome);
  const expectedAngle = haversineAngleRad(31.78, 35.22, 41.9, 12.5);
  closeTo(measured, expectedAngle, 1e-9, 'Jerusalem-Rome angular distance');
  closeTo(measured, 0.36156378798520494, 1e-9, 'Jerusalem-Rome angular distance, pinned value');
});

// --- dot/cross/norm/normalize sanity (the plain linear algebra primitives) ---

test('GEO-7: normalize always returns a unit vector for any non-zero input', () => {
  fc.assert(
    fc.property(
      fc.array(fc.double({ min: -100, max: 100, noNaN: true }), { minLength: 3, maxLength: 3 }).filter(v => norm(v) > 1e-6),
      (arr) => {
        closeTo(norm(normalize(arr)), 1, TOL, 'normalize norm');
      }
    ),
    { numRuns: RUNS_LAW }
  );
});

test('GEO-8: cross product is anti-commutative and orthogonal to both inputs', () => {
  fc.assert(
    fc.property(arbUnitVec(), arbUnitVec(), (a, b) => {
      const c1 = cross(a, b);
      const c2 = cross(b, a);
      vecCloseTo(c1, [-c2[0], -c2[1], -c2[2]], TOL, 'cross anti-commutativity');
      closeTo(dot(c1, a), 0, TOL, 'cross orthogonal to a');
      closeTo(dot(c1, b), 0, TOL, 'cross orthogonal to b');
    }),
    { numRuns: RUNS_LAW }
  );
});
