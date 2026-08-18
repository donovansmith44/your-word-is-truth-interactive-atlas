import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { arbWindow, arbYear } from './lib/canon';
import { fcAssert, RUNS_API } from './lib/fc';

function arrowsByNarrative(scene: any): Map<string, any[]> {
  const m = new Map<string, any[]>();
  for (const a of scene.arrows) { (m.get(a.narrative) ?? m.set(a.narrative, []).get(a.narrative)!).push(a); }
  for (const v of m.values()) v.sort((x, y) => x.order - y.order);
  return m;
}
const intersects = (a: any, w: { from: number; to: number }) =>
  a.from_year <= w.to && w.from <= a.to_year;

test('SCENE-1/2 + ARROW-1..7: window scene invariants', async () => {
  const narrColors = new Map((await api.narratives()).map((n: any) => [n.id, n.color]));
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    const s = await api.sceneTime(w.from, w.to);
    expect(s.__status).toBeUndefined();                                   // SCENE-1
    expect(s.mode).toBe('time');
    expect(s.window).toEqual({ from_year: w.from, to_year: w.to });
    const placeIds = new Set(s.places.map((p: any) => p.id));
    for (const p of s.places) {                                           // SCENE-2
      expect(p.events.length).toBeGreaterThan(0);
      expect(p.brightness).toBe(Math.min(p.events.length, 5));
      for (const e of p.events) {
        expect(intersects(e.when, w)).toBe(true);
        for (const g of e.verse_groups) {
          expect(g.verses.length).toBeLessThanOrEqual(20);
          expect(g.count).toBeGreaterThanOrEqual(g.verses.length);
        }
      }
    }
    const eventsOf = (pid: string) => new Set(
      s.places.find((p: any) => p.id === pid)?.events.map((e: any) => e.id) ?? []);
    for (const [nid, arrows] of arrowsByNarrative(s)) {
      const sceneColor = s.narratives.find((n: any) => n.id === nid)?.color;
      for (const a of arrows) {
        expect(placeIds.has(a.from_place) && placeIds.has(a.to_place)).toBe(true); // ARROW-1
        expect(a.color).toBe(sceneColor);                                          // ARROW-2
        expect(a.color).toBe(narrColors.get(nid));                                 // ARROW-2
        expect(a.from_place).not.toBe(a.to_place);                                 // ARROW-5
        expect(eventsOf(a.from_place).has(a.from_event)).toBe(true);               // ARROW-7
        expect(eventsOf(a.to_place).has(a.to_event)).toBe(true);                   // ARROW-7
      }
      for (let k = 0; k + 1 < arrows.length; k++) {
        expect(arrows[k].to_place).toBe(arrows[k + 1].from_place);                 // ARROW-3
      }
      if (arrows.length > 0) {
        const toEvents = new Set(arrows.map(a => a.to_event));
        const fromEvents = new Set(arrows.map(a => a.from_event));
        expect(toEvents.has(arrows[0].from_event)).toBe(false);                    // ARROW-4
        expect(fromEvents.has(arrows[arrows.length - 1].to_event)).toBe(false);    // ARROW-4
      }
    }
    // ARROW-6 needs event years: read them from the scene itself
    const whenOf = new Map(s.places.flatMap((p: any) => p.events.map((e: any) => [e.id, e.when])));
    for (const a of s.arrows) {
      expect((whenOf.get(a.to_event) as any).from_year)
        .toBeGreaterThanOrEqual((whenOf.get(a.from_event) as any).from_year);      // ARROW-6
    }
  }), RUNS_API);
});

test('SCENE-3: window monotonicity', async () => {
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    const grow = { from: w.from === 1 ? -1 : w.from - 1, to: w.to === -1 ? 1 : Math.min(w.to + 1, 100) };
    const [s1, s2] = [await api.sceneTime(w.from, w.to), await api.sceneTime(grow.from, grow.to)];
    const ids2 = new Map(s2.places.map((p: any) => [p.id, new Set(p.events.map((e: any) => e.id))]));
    for (const p of s1.places) {
      expect(ids2.has(p.id)).toBe(true);
      for (const e of p.events) expect(ids2.get(p.id)!.has(e.id)).toBe(true);
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
