import { toVecDeg, fromVecDeg, slerpRing, buildCorrespondence } from './geo.js';

/**
 * Mirrors the Rust-side TimeRange::intersects filter and from-sort exactly (inclusive-
 * inclusive), so drag-frame animation avoids a network round trip; keep both
 * implementations in sync.
 */
export function lookup(roster, from, to) {
    const grouped = new Map();
    for (const entry of roster || []) {
        if (entry.from > to || entry.to < from) {
            continue;
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

export function overlay(lines) {
    const tierCount = lines.length;
    return lines.map((entry, idx) => {
        const age = tierCount === 1 ? 'newest' : idx === 0 ? 'oldest' : idx === tierCount - 1 ? 'newest' : 'middle';
        return { ...entry, age, tierIndex: idx, tierCount };
    });
}

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

export function clearCorrespondenceCache() {
    correspondenceCache.clear();
}

/**
 * Uses the midpoint of each line's [from, to] span as its timeline knot, not the raw
 * boundary years: adjacent curated eras touch at adjacent years (e.g. -931/-930), so
 * using the raw edges would squeeze the whole morph into a single simulated year.
 */
function knotYear(line) {
    return (line.from + line.to) / 2;
}

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
        // Different ring counts have no well-defined per-vertex correspondence; snap instead of fabricating a shape.
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
