import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbAnyRef, arbVerseRef } from './lib/canon';
import { fcAssert, RUNS_API } from './lib/fc';

function refContains(sref: string, verseId: string): boolean {
  const [b, c, v] = verseId.split('.');
  const m = sref.match(/^([A-Z0-9]{3})(?:\.(\d+)(?:\.(\d+)(?:-(\d+))?)?)?$/)!;
  if (m[1] !== b) return false;
  if (m[2] === undefined) return true;
  if (Number(m[2]) !== Number(c)) return false;
  if (m[3] === undefined) return true;
  const [lo, hi] = [Number(m[3]), Number(m[4] ?? m[3])];
  return Number(v) >= lo && Number(v) <= hi;
}

test('SCRIP-1/3: scripture scenes are sound', async () => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbAnyRef(toc), async sref => {
    const s = await api.sceneScripture(sref);
    expect(s.__status).toBeUndefined();
    expect(s.mode).toBe('scripture');
    expect(s.ref).toBe(sref);                                              // SCRIP-1
    for (const p of s.places) {
      const all = p.events.flatMap((e: any) => e.verse_groups.flatMap((g: any) => g.verses));
      expect(all.some((v: string) => refContains(sref, v))).toBe(true);    // SCRIP-1
    }
    const eventVerses = new Map(s.places.flatMap((p: any) => p.events.map((e: any) =>
      [e.id, e.verse_groups.flatMap((g: any) => g.verses)])));
    for (const a of s.arrows) {                                            // SCRIP-3
      for (const ev of [a.from_event, a.to_event]) {
        expect((eventVerses.get(ev) as string[]).some(v => refContains(sref, v))).toBe(true);
      }
    }
  }), RUNS_API);
});

test('SCRIP-2: ref monotonicity verse ⊆ passage ⊆ chapter ⊆ book', async () => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    const [b, c, v] = vref.split('.');
    const chain = [vref, `${b}.${c}.${v}-${Number(v) + 1}`, `${b}.${c}`, b];
    let prev: Set<string> | null = null;
    for (const sref of chain) {
      const s = await api.sceneScripture(sref);
      if (s.__status) continue; // v+1 may exceed chapter; skip that link
      const ids = new Set<string>(s.places.map((p: any) => p.id));
      if (prev) for (const id of prev) expect(ids.has(id)).toBe(true);
      prev = ids;
    }
  }), RUNS_API);
});

test('bad refs are typed 400s', async () => {
  for (const bad of ['NOPE', 'GEN.0', 'GEN.1.0', 'GEN.1.9-2', 'gen..1']) {
    const r = await api.raw(`/api/scene/scripture?ref=${encodeURIComponent(bad)}`);
    expect(r.__status).toBe(400);
    expect(r.error.code).toBe('bad_ref');
  }
});
