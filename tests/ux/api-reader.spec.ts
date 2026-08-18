import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbVerseRef, arbChapterRef } from './lib/canon';
import { fcAssert, RUNS_API } from './lib/fc';

const SPAN_RE = /^[A-Z0-9]{3}\.\d+\.\d+(-(\d+|[A-Z0-9]{3}\.\d+\.\d+))?$/;

test('XREF-1: verse details are sound', async () => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    const d = await api.verse(vref);
    expect(d.__status).toBeUndefined();
    expect(d.ref).toBe(vref);
    expect(d.text.length).toBeGreaterThan(0);
    expect(typeof d.book_meta.author).toBe('string');
    let last = Infinity;
    for (const x of d.cross_refs) {
      expect(x.votes).toBeLessThanOrEqual(last); last = x.votes;   // votes descending
      expect(SPAN_RE.test(x.target)).toBe(true);                   // canon-parseable target
      expect(x.target).not.toBe(vref);                             // no self (exact match only)
      expect(x.preview.length).toBeGreaterThan(0);
    }
  }), RUNS_API);
});

test('CHAP-1: chapters match the TOC', async () => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbChapterRef(toc), async c => {
    const ch = await api.chapter(`${c.book}.${c.chapter}`);
    expect(ch.verses.length).toBe(c.verses);
    ch.verses.forEach((v: any, i: number) => {
      expect(v.verse).toBe(i + 1);
      expect(v.text.length).toBeGreaterThan(0);
    });
  }), RUNS_API);
});
