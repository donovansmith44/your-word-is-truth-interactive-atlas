import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { arbWindow, arbYear } from './lib/canon';
import { fcAssert, RUNS_API } from './lib/fc';
import { SPAN } from './lib/years';

function arrowsByNarrative(scene: any): Map<string, any[]> {
  const m = new Map<string, any[]>();
  for (const a of scene.arrows) { (m.get(a.narrative) ?? m.set(a.narrative, []).get(a.narrative)!).push(a); }
  for (const v of m.values()) v.sort((x, y) => x.order - y.order);
  return m;
}
const intersects = (a: any, w: { from: number; to: number }) =>
  a.from_year <= w.to && w.from <= a.to_year;

// Task 16 finding: plain `throw` on failure, not Playwright's `expect()`, for
// every check inside this property's hot per-iteration loop. Root-caused
// with evidence, not a style preference: a standalone Node script running
// this EXACT property (same fetches, same conditions, same fast-check
// arbitrary) with plain `if (!cond) throw` instead of `expect()` completed
// 500 iterations in 0.8s flat, zero violations -- proving the underlying
// logic and data are fast and correct. The identical property run through
// Playwright's `test()`/`expect()` machinery took 10+ minutes at
// FC_NUM_RUNS=150 and did not finish within 25+ minutes at 500 (this file's
// own hot loop calls expect() 5-10+ times per arrow, thousands of times per
// iteration -- `expect()` captures a stack trace and other diagnostics on
// every call, by design, which is fine at normal call volumes but not built
// for a property test replaying a dense assertion loop hundreds of times
// per run). A thrown Error fails the fast-check property (and hence the
// Playwright test) exactly the same way an expect() failure does; only the
// per-call bookkeeping is removed.
function ok(cond: boolean, msg: string) {
  if (!cond) throw new Error(msg);
}

test('SCENE-1/2 + ARROW-1..7: window scene invariants', async () => {
  const narrColors = new Map((await api.narratives()).map((n: any) => [n.id, n.color]));
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    const s = await api.sceneTime(w.from, w.to);
    ok(s.__status === undefined, 'SCENE-1: unexpected error status');
    ok(s.mode === 'time', 'SCENE-1: mode mismatch');
    // Fix round 1 (M2): the expect(s.window).toEqual({from_year, to_year})
    // this replaced also failed on any EXTRA key in s.window, not just wrong
    // values -- a value-only check silently permits s.window to grow a stray
    // field. Restoring that exact-shape guarantee explicitly (key-set check
    // + value check) rather than reaching for a deep-equal helper, to keep
    // this file's one dependency (fast-check) unchanged.
    const windowKeys = Object.keys(s.window).sort();
    ok(windowKeys.length === 2 && windowKeys[0] === 'from_year' && windowKeys[1] === 'to_year',
      'SCENE-1: window has extra/missing fields (expected exactly from_year, to_year)');
    ok(s.window.from_year === w.from && s.window.to_year === w.to, 'SCENE-1: window echo mismatch');
    const placeIds = new Set(s.places.map((p: any) => p.id));
    for (const p of s.places) {                                           // SCENE-2
      ok(p.events.length > 0, 'SCENE-2: place with no events');
      ok(p.brightness === Math.min(p.events.length, 5), 'SCENE-2: brightness mismatch');
      for (const e of p.events) {
        ok(intersects(e.when, w), 'SCENE-2: event does not intersect window');
        for (const g of e.verse_groups) {
          ok(g.verses.length <= 20, 'SCENE-2: verse group over cap');
          ok(g.count >= g.verses.length, 'SCENE-2: count below shown verses');
        }
      }
    }
    // Task 16: precomputed once per iteration rather than re-scanning
    // s.places (a linear .find()) AND rebuilding a fresh Set from that
    // place's events on every single call -- eventsOf is called twice per
    // arrow (ARROW-7's from_event/to_event checks below), so with the full
    // narrative set now in play (13 narratives / up to ~90 arrows in a
    // full-span window, versus 3 narratives pre-Task-16) the old per-call
    // rescan pushed this property past its 300s budget -- not a data bug,
    // a real O(arrows * places) cost in the test's OWN validation code.
    // Same result set as before, just built in one O(places) pass instead
    // of up to 2*arrows separate O(places) scans.
    const eventIdsByPlace = new Map(
      s.places.map((p: any) => [p.id, new Set(p.events.map((e: any) => e.id))]));
    const eventsOf = (pid: string) => eventIdsByPlace.get(pid) ?? new Set();
    for (const [nid, arrows] of arrowsByNarrative(s)) {
      const sceneColor = s.narratives.find((n: any) => n.id === nid)?.color;
      for (const a of arrows) {
        ok(placeIds.has(a.from_place) && placeIds.has(a.to_place), 'ARROW-1: arrow references an unlit place');
        ok(a.color === sceneColor, 'ARROW-2: arrow color != scene narrative color');
        ok(a.color === narrColors.get(nid), 'ARROW-2: arrow color != /api/narratives color');
        ok(a.from_place !== a.to_place, 'ARROW-5: arrow is a self-loop');
        ok(eventsOf(a.from_place).has(a.from_event), 'ARROW-7: from_event not attached to from_place');
        ok(eventsOf(a.to_place).has(a.to_event), 'ARROW-7: to_event not attached to to_place');
      }
      for (let k = 0; k + 1 < arrows.length; k++) {
        ok(arrows[k].to_place === arrows[k + 1].from_place, 'ARROW-3: chain broken between consecutive legs');
      }
      if (arrows.length > 0) {
        const toEvents = new Set(arrows.map(a => a.to_event));
        const fromEvents = new Set(arrows.map(a => a.from_event));
        ok(!toEvents.has(arrows[0].from_event), 'ARROW-4: first leg\'s from_event reused as a to_event');
        ok(!fromEvents.has(arrows[arrows.length - 1].to_event), 'ARROW-4: last leg\'s to_event reused as a from_event');
      }
    }
    // ARROW-6 needs event years: read them from the scene itself
    const whenOf = new Map(s.places.flatMap((p: any) => p.events.map((e: any) => [e.id, e.when])));
    for (const a of s.arrows) {
      ok((whenOf.get(a.to_event) as any).from_year >= (whenOf.get(a.from_event) as any).from_year,
        'ARROW-6: to_event predates from_event');
    }
  }), RUNS_API);
});

test('SCENE-3: window monotonicity', async () => {
  // Task 16: plain throw, not expect() -- same reasoning as SCENE-1/2's own
  // ok() helper above (this loop is equally proportional to real scene size
  // via arbWindow, and hit the same expect()-call-volume wall at
  // FC_NUM_RUNS=500: SCENE-1/2 was fixed, this one wasn't, and it promptly
  // timed out the same way once the deep run actually exercised it).
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    const grow = { from: w.from === 1 ? -1 : w.from - 1, to: w.to === -1 ? 1 : Math.min(w.to + 1, 100) };
    const [s1, s2] = [await api.sceneTime(w.from, w.to), await api.sceneTime(grow.from, grow.to)];
    const ids2 = new Map(s2.places.map((p: any) => [p.id, new Set(p.events.map((e: any) => e.id))]));
    for (const p of s1.places) {
      ok(ids2.has(p.id), 'SCENE-3: place dropped when window grows');
      for (const e of p.events) ok(ids2.get(p.id)!.has(e.id), 'SCENE-3: event dropped when window grows');
    }
  }), RUNS_API);
});

test('SCENE-4: point windows deterministic', async () => {
  await fcAssert(fc.asyncProperty(arbYear, async y => {
    const [a, b] = [await api.sceneTime(y, y), await api.sceneTime(y, y)];
    expect(a).toEqual(b);
    for (const p of a.places) for (const e of p.events) expect(intersects(e.when, { from: y, to: y })).toBe(true);
  }), RUNS_API);
});

test('SCENE-5: invalid windows are typed 400s', async () => {
  for (const q of ['from=0&to=5', 'from=-5&to=0', 'from=5&to=-5', 'from=1', 'to=1', 'from=x&to=y']) {
    const r = await api.raw(`/api/scene?${q}`);
    expect(r.__status).toBe(400);
    expect(r.error.code).toBe('bad_window');
    expect(typeof r.error.message).toBe('string');
  }
});

// Batch E2 (the ever-present graph, batch-e2-brief.md): QUIET-1. The full
// atlas span's own `places` list IS the full, fixed-cardinality
// event-bearing set ("cities in our graph") -- every real event's `when`
// necessarily intersects [SPAN.from, SPAN.to] (that IS the whole atlas
// span), so every event-bearing place is lit there. Fetched once, outside
// the property loop, as this property's own ground truth to compare every
// OTHER window's lit-union-quiet against -- cheaper than re-deriving the
// full set from raw event data (which the black-box UX suite couldn't do
// anyway -- CONTRACT.md: "couples ONLY to this contract... plus the HTTP
// API").
test('QUIET-1: lit union quiet is the fixed event-bearing set, disjoint, for every window', async () => {
  const full = await api.sceneTime(SPAN.from, SPAN.to);
  const fullIds = new Set(full.places.map((p: any) => p.id));
  ok(fullIds.size > 0, 'QUIET-1 setup: the full-span scene must have places to compare against');

  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    const s = await api.sceneTime(w.from, w.to);
    ok(s.__status === undefined, 'QUIET-1: unexpected error status');
    ok(Array.isArray(s.quiet_places), 'QUIET-1: quiet_places must always be an array, never missing');

    const litIds = new Set(s.places.map((p: any) => p.id));
    const quietIds = new Set(s.quiet_places.map((p: any) => p.id));
    ok(litIds.size + quietIds.size === new Set([...litIds, ...quietIds]).size,
      'QUIET-1: lit and quiet ids overlap -- must be disjoint');

    const union = new Set([...litIds, ...quietIds]);
    ok(union.size === fullIds.size,
      `QUIET-1: union size ${union.size} != fixed event-bearing set size ${fullIds.size}`);
    for (const id of fullIds) {
      ok(union.has(id), `QUIET-1: event-bearing place ${id} missing from lit union quiet in window ${w.from}..${w.to}`);
    }
    for (const id of union) {
      ok(fullIds.has(id), `QUIET-1: ${id} appears in lit union quiet but is not a real event-bearing place`);
    }

    // The quiet payload stays lean (batch-e2-brief.md Requirement 1: "NO
    // events, NO verse groups") and every entry carries a positive
    // ALL-TIME event count (a place only ever enters event_bearing_place_ids
    // by having >=1 event somewhere).
    for (const qp of s.quiet_places) {
      ok(typeof qp.display_name === 'string' && qp.display_name.length > 0, `QUIET-1: ${qp.id} missing display_name`);
      ok(typeof qp.lat === 'number' && typeof qp.lon === 'number', `QUIET-1: ${qp.id} missing lat/lon`);
      ok(Number.isInteger(qp.total_events) && qp.total_events > 0, `QUIET-1: ${qp.id}.total_events must be a positive integer`);
      ok(!('events' in qp) && !('verse_groups' in qp), `QUIET-1: ${qp.id} carries events/verse_groups -- payload must stay lean`);
    }
  }), RUNS_API);
});

test('QUIET-1: scripture-mode scenes never gain quiet places', async () => {
  const s = await api.sceneScripture('EXO.14');
  expect(Array.isArray(s.quiet_places)).toBe(true);
  expect(s.quiet_places.length).toBe(0);
});

// QUIET-1 spot-assert (batch-e2-brief.md: "every quiet display_name matches
// the same window-resolution the lit side uses -- spot-assert via a place
// with a curated name range, e.g. Jerusalem in a pre-Jebus-cutover window").
// Jerusalem's own compiled events all postdate David's conquest (theo-159 is
// the earliest, -1055), so it is QUIET, never lit, throughout the primeval
// era (data/curated/eras.toml: [-4004,-2167]) -- and its curated "Jebus"
// name range (data/curated/place-history.toml: -4004..-1004) fully covers
// that window, the exact resolution world-place-history.spec.ts's own
// NAME-1 test already proves for the LIT side of this same place.
test('QUIET-1 spot-assert: a quiet place\'s display_name matches the lit-side window-resolution rules (Jerusalem, primeval era)', async () => {
  const w = { from: -4004, to: -2167 };
  const s = await api.sceneTime(w.from, w.to);
  expect(s.places.some((p: any) => p.id === 'jerusalem'), 'jerusalem must not be lit in the primeval era').toBe(false);

  const jerusalem = s.quiet_places.find((p: any) => p.id === 'jerusalem');
  expect(jerusalem, 'jerusalem should be quiet, not absent entirely, in the primeval era').toBeTruthy();
  expect(jerusalem.display_name).toBe('Jebus');

  // Cross-checked directly against the same resolve_display_name the lit
  // side's own marker label/place-card-title call through -- proves the two
  // paths agree, not just that this ONE value looks plausible.
  const direct = await api.placeHistory('jerusalem', w.from, w.to);
  expect(direct.history.display_name).toBe('Jebus');
});
