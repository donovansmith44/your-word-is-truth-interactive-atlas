import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbVerseRef, arbChapterRef, arbPassageRef } from './lib/canon';
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

// Batch G1: GET /api/xrefs/{sref}. atlas_core::xrefs's own Rust proptest
// (server/atlas-core/src/xrefs.rs, "xref_2_span_aggregation") already proves
// the aggregation algebra itself (sum, subset-drop, sort, cap) against a
// small hand-built fixture -- this is the endpoint's real-data counterpart,
// same relationship XREF-1 above already has to the ETL-time xref checks:
// exercises the ACTUAL compiled dataset (hundreds of thousands of real
// cross-ref rows) through the real HTTP handler, not a fixture. A single-
// verse span's aggregation reduces to exactly that verse's OWN cross_refs
// (mod the 20-cap XREF-1's own endpoint never applies) -- a strong,
// precise equality check requiring no shadow reimplementation of the
// subset-drop logic in TypeScript; a passage span gets the general
// soundness checks alone (every target parses, votes non-increasing,
// capped at 20, non-empty preview).
test('XREF-2: single-verse span equals that verse\'s own cross_refs (capped); passage spans stay sound', async () => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    const [got, verseDetail] = await Promise.all([api.xrefs(vref), api.verse(vref)]);
    expect(got.__status).toBeUndefined();
    expect(Array.isArray(got)).toBe(true);
    expect(got.length).toBeLessThanOrEqual(20);
    expect(got).toEqual(verseDetail.cross_refs.slice(0, 20));
  }), RUNS_API);

  await fcAssert(fc.asyncProperty(arbPassageRef(toc), async sref => {
    const got = await api.xrefs(sref);
    expect(got.__status).toBeUndefined();
    expect(Array.isArray(got)).toBe(true);
    expect(got.length).toBeLessThanOrEqual(20);
    let last = Infinity;
    for (const x of got) {
      expect(x.votes).toBeLessThanOrEqual(last); last = x.votes;   // votes non-increasing
      expect(SPAN_RE.test(x.target)).toBe(true);                   // every target parses
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
