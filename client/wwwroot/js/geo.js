const DEG2RAD = Math.PI / 180;
const RAD2DEG = 180 / Math.PI;

export const RESAMPLE_N = 128;

export function toVec(phi, lambda) {
    const cosPhi = Math.cos(phi);
    return [cosPhi * Math.cos(lambda), cosPhi * Math.sin(lambda), Math.sin(phi)];
}

/**
 * Uses atan2(z, hypot(x,y)) rather than asin(z) for latitude: asin's derivative diverges
 * as |z| -> 1 near the poles, amplifying float error; atan2 has no such blowup.
 * Renormalizes the input first so drift accumulated through slerp/rotate is corrected
 * in one place before the inverse trig runs.
 *
 * At a pole (x=y=0) longitude is genuinely unrecoverable (toVec collapses every
 * longitude to the same point there); this canonicalizes lambda=0 rather than
 * returning an arbitrary answer.
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

export function toVecDeg(latDeg, lonDeg) {
    return toVec(latDeg * DEG2RAD, lonDeg * DEG2RAD);
}

export function fromVecDeg(v) {
    const [phi, lambda] = fromVec(v);
    return [phi * RAD2DEG, lambda * RAD2DEG];
}

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

export function angleBetween(a, b) {
    return Math.acos(clamp(dot(a, b), -1, 1));
}

/**
 * Falls back to normalized lerp when a and b are numerically coincident (theta≈0),
 * since sin(theta) in the slerp denominator is otherwise too close to zero to divide by.
 * Antipodal inputs (theta≈pi) get no equivalent fallback -- this app's data never
 * produces antipodal correspondence pairs, and there is no single well-defined
 * shortest path between exactly-antipodal points anyway. The final renormalize is
 * unconditional because wa*a + wb*b can drift measurably off unit norm exactly
 * where sin(theta) is ill-conditioned.
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
 * Resamples a closed ring to exactly n points, evenly spaced by arc length (great-circle
 * angle) rather than by original vertex density. Point 0 always lands exactly on the
 * ring's first vertex.
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
        return Array.from({ length: n }, () => verts[0].slice());
    }

    const step = total / n;
    const out = new Array(n);
    let edgeIndex = 0;
    let consumed = 0;
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

/** Coarse winding sign, only meaningful for comparing two rings against each other, never as an absolute orientation/area. */
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
 * Reverses target before alignment if the two rings wind opposite ways, so per-index
 * slerp pairs points that are actually near each other rather than producing a twisted,
 * self-crossing morph.
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

export function slerpRing(source, target, t) {
    const n = source.length;
    const out = new Array(n);
    for (let i = 0; i < n; i++) {
        out[i] = slerp(source[i], target[i], t);
    }
    return out;
}
