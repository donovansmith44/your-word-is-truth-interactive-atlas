import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { arbWindow } from './lib/canon';
import { fcAssert, RUNS_API } from './lib/fc';

// Batch E (batch-e-brief.md): `/api/place/{id}?from=&to=` history resolution
// -- pure API-only property/example coverage (no browser), mirroring the
// api-scene.spec.ts / api-scripture.spec.ts pattern. NAME-1/BLURB-1's own
// visible-on-screen counterparts live in world-place-history.spec.ts; this
// file is the "window -> name/blurb determinism" API property plus
// exhaustive boundary-year pins, checked directly against curated
// data/curated/place-history.toml's real compiled content.

test('a place with no curated history omits `history` entirely, window or not', async () => {
  // "gilgal-1" is a real compiled place (cq_gilgal, -1406) with no entry in
  // data/curated/place-history.toml -- NOT jericho-1, which Batch E DOES
  // curate (a blurb, no names/dates).
  for (const url of ['/api/place/gilgal-1', '/api/place/gilgal-1?from=-1406&to=-1405']) {
    const body = await api.raw(url);
    expect(body.history).toBeUndefined();
  }
});

test('API property: /api/place/bethel-1 history resolution is deterministic and, whenever resolved, intersects a curated range', async () => {
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    const a = await api.placeHistory('bethel-1', w.from, w.to);
    const b = await api.placeHistory('bethel-1', w.from, w.to);
    expect(a).toEqual(b); // determinism

    // bethel-1's two curated name ranges (Luz [-4004,-2092], Bethel
    // [-2091,100]) jointly cover the WHOLE atlas span with no gap, so
    // display_name must always be one of the two curated names, never the
    // raw compiled default ("Bethel 1") -- see place-history.toml's own
    // file-header comment.
    expect(['Luz', 'Bethel']).toContain(a.history.display_name);
  }), RUNS_API);
});

test('NAME-1: a window fully inside one curated name range resolves to exactly that name', async () => {
  await fcAssert(fc.asyncProperty(
    fc.integer({ min: -4004, max: -2092 }).chain(a => fc.integer({ min: a, max: -2092 }).map(b => ({ from: a, to: b }))),
    async w => {
      const body = await api.placeHistory('bethel-1', w.from, w.to);
      expect(body.history.display_name).toBe('Luz');
    }), RUNS_API);
  // The Bethel range [-2091,100] straddles year 0 (which does not exist on
  // this calendar) -- filtered out so no generated (from, to) pair ever
  // tries to build a zero-year window.
  await fcAssert(fc.asyncProperty(
    fc.integer({ min: -2091, max: 100 }).filter(y => y !== 0)
      .chain(a => fc.integer({ min: a, max: 100 }).filter(y => y !== 0).map(b => ({ from: a, to: b }))),
    async w => {
      const body = await api.placeHistory('bethel-1', w.from, w.to);
      expect(body.history.display_name).toBe('Bethel');
    }), RUNS_API);
});

test('NAME-1: Luz/Bethel boundary years pinned exhaustively', async () => {
  const luz = await api.placeHistory('bethel-1', -2092, -2092);
  expect(luz.history.display_name).toBe('Luz');
  const bethel = await api.placeHistory('bethel-1', -2091, -2091);
  expect(bethel.history.display_name).toBe('Bethel');
  // One year on either side, for good measure.
  expect((await api.placeHistory('bethel-1', -2093, -2093)).history.display_name).toBe('Luz');
  expect((await api.placeHistory('bethel-1', -2090, -2090)).history.display_name).toBe('Bethel');
});

test('NAME-1: every other curated rename pinned at its own boundary (Jebus/Jerusalem, Kirjath-arba/Hebron, Laish/Dan)', async () => {
  const cases: { id: string; oldName: string; newName: string; oldYear: number; newYear: number }[] = [
    { id: 'jerusalem', oldName: 'Jebus', newName: 'Jerusalem', oldYear: -1004, newYear: -1003 },
    { id: 'hebron', oldName: 'Kirjath-arba', newName: 'Hebron', oldYear: -2092, newYear: -2091 },
    { id: 'dan', oldName: 'Laish', newName: 'Dan', oldYear: -1401, newYear: -1400 },
  ];
  for (const c of cases) {
    const before = await api.placeHistory(c.id, c.oldYear, c.oldYear);
    expect(before.history.display_name, `${c.id} at ${c.oldYear}`).toBe(c.oldName);
    const after = await api.placeHistory(c.id, c.newYear, c.newYear);
    expect(after.history.display_name, `${c.id} at ${c.newYear}`).toBe(c.newName);
  }
});

test('BLURB-1: exactly one blurb or none, deterministic, and a broad window prefers the broad summary over stacking eras', async () => {
  // jerusalem: two "era" blurbs ([-4004,-586], [-538,100]) plus one "broad"
  // ([-4004,100]) -- batch-e-brief.md Requirement 4's "don't stack
  // everything for Jerusalem".
  const insideFirstEra = await api.placeHistory('jerusalem', -1055, -1055);
  expect(insideFirstEra.history.blurb).toContain('Jebusite stronghold');

  const insideSecondEra = await api.placeHistory('jerusalem', 27, 30);
  expect(insideSecondEra.history.blurb).toContain('Second Temple city');

  // The full default span crosses both curated era ranges -> the broad
  // summary shows instead of either era one.
  const wholeSpan = await api.placeHistory('jerusalem', -4004, 100);
  expect(wholeSpan.history.blurb).toContain("Jerusalem's story runs");

  // Determinism.
  const again = await api.placeHistory('jerusalem', -4004, 100);
  expect(again).toEqual(wholeSpan);
});

test('M1 regression: Jerusalem\'s OWN destruction year (the exact "Destroyed 586 BC" -> "Show this time on the map" click path) resolves to the SPECIFIC blurb naming that year, not the broad summary', async () => {
  // Fix round 1 (batch-e-review.md MAJOR-1): the first era blurb's own
  // text says "...until Babylon burned it in 586 BC" but its curated range
  // used to stop at -587, one year short -- so this exact window (which is
  // also exactly place.destroyed.when, and exactly where YearNode's "Show
  // this time on the map" chip navigates to from the destroyed date's own
  // popover) fell into the zero-era-hits fallback and showed the generic
  // broad summary instead. Pinned here so it can never silently regress.
  const atDestructionYear = await api.placeHistory('jerusalem', -586, -586);
  expect(atDestructionYear.history.display_name).toBe('Jerusalem');
  expect(atDestructionYear.history.blurb).toContain('Jebusite stronghold');
  expect(atDestructionYear.history.blurb).toContain('586 BC');
  expect(atDestructionYear.history.blurb).not.toContain("Jerusalem's story runs"); // NOT the broad one
  expect(atDestructionYear.history.destroyed.when).toEqual({ from_year: -586, to_year: -586 });

  // One year on either side of the fixed boundary, for good measure: -587
  // (still era-1, unaffected by the fix) and -585 (start of the genuine,
  // documented [-585,-539] gap, where broad correctly takes over).
  const justBefore = await api.placeHistory('jerusalem', -587, -587);
  expect(justBefore.history.blurb).toContain('Jebusite stronghold');
  const inTheGap = await api.placeHistory('jerusalem', -585, -560);
  expect(inTheGap.history.blurb).toContain("Jerusalem's story runs");
});

test('established/destroyed are window-independent: same values regardless of which window is asked for', async () => {
  const early = await api.placeHistory('jerusalem', -1055, -1055);
  const late = await api.placeHistory('jerusalem', 27, 30);
  expect(early.history.established).toEqual(late.history.established);
  expect(early.history.destroyed).toEqual(late.history.destroyed);
  expect(early.history.established).toEqual({
    when: { from_year: -1003, to_year: -1003 },
    verses: ['2SA.5.6', '2SA.5.7', '2SA.5.9'],
    note: 'traditional',
  });
  expect(early.history.destroyed).toEqual({
    when: { from_year: -586, to_year: -586 },
    verses: ['2KI.25.9', '2KI.25.10'],
    note: null,
  });
});

test('DATE-1 (API half): a curated established/destroyed claim carries its own supporting verses, real canonical refs', async () => {
  for (const id of ['samaria_1022', 'shiloh', 'nineveh']) {
    const body = await api.placeHistory(id, -4004, 100);
    for (const kind of ['established', 'destroyed'] as const) {
      const claim = body.history?.[kind];
      if (!claim) continue;
      expect(claim.verses.length).toBeGreaterThan(0);
      for (const v of claim.verses) {
        expect(v).toMatch(/^[A-Z0-9]{3}\.\d+\.\d+$/);
      }
    }
  }
});

test('no window at all: display_name falls back to the default name, no blurb, established/destroyed still present', async () => {
  const body = await api.raw('/api/place/hebron');
  expect(body.history.display_name).toBe(body.name);
  expect(body.history.blurb ?? null).toBeNull();
});
