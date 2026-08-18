import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { loadToc } from './lib/canon';

const nextYear = (y: number) => (y === -1 ? 1 : y + 1);

test('ERA-1: eras contiguous, zero-free, covering the span (exhaustive)', async () => {
  const eras = await api.eras();
  expect(eras[0].from_year).toBe(-4004);
  expect(eras[eras.length - 1].to_year).toBe(100);
  for (const e of eras) {
    expect(e.from_year).not.toBe(0); expect(e.to_year).not.toBe(0);
    expect(e.from_year).toBeLessThanOrEqual(e.to_year);
  }
  for (let i = 0; i + 1 < eras.length; i++) {
    expect(nextYear(eras[i].to_year)).toBe(eras[i + 1].from_year);
  }
});

test('BOOKS-1/2: all 66 books, each with a working book scene (exhaustive)', async () => {
  const toc = await loadToc();
  expect(toc.map(b => b.code)).toHaveLength(66);
  expect(new Set(toc.map(b => b.code)).size).toBe(66);
  expect(toc[0].code).toBe('GEN'); expect(toc[39].code).toBe('MAT'); expect(toc[65].code).toBe('REV');
  for (const b of toc) {
    expect(b.chapters.length).toBeGreaterThanOrEqual(1);
    const s = await api.sceneScripture(b.code);           // BOOKS-2: every single book
    expect(s.__status).toBeUndefined();
    expect(s.mode).toBe('scripture');
  }
});
