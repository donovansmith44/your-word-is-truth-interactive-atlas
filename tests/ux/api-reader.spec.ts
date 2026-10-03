import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbVerseRef, arbChapterRef, arbPassageRef } from './lib/canon';
import { fcAssert, RUNS_API } from './lib/fc';
import { neighbourNode } from './lib/edges';

const CITES_PAGE = 20;
const VERSE_RE = /^[A-Z0-9]{3}\.\d+\.\d+$/;
const VERSE_PREFIX_RE = /^[A-Z0-9]{3}\.\d+\.\d+/;
const SPAN_RE = /^[A-Z0-9]{3}\.\d+\.\d+(-(\d+|[A-Z0-9]{3}\.\d+\.\d+))?$/;

test('XREF-1: a verse\'s record and its first page of cross references are sound', async () => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    // Arrange
    const id = `text-unit:${vref}`;

    // Act
    const [record, cites] = await Promise.all([api.node(id), api.nodeEdges(id, 'cites', { limit: CITES_PAGE })]);

    // Assert
    expect(record.__status).toBeUndefined();
    expect(record.label).toBe(vref);
    expect(record.text.text.length).toBeGreaterThan(0);
    for (const target of cites.entries.map(neighbourNode)) {
      expect(VERSE_RE.test(target.label)).toBe(true);
    }
  }), RUNS_API);
});

test('XREF-2: a single-verse span cites the verses its first page of cites edges reaches, in the same order; passage spans stay sound', async () => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    // Act
    const [got, cites] = await Promise.all([api.xrefs(vref), api.nodeEdges(`text-unit:${vref}`, 'cites', { limit: CITES_PAGE })]);

    // Assert
    expect(got.__status).toBeUndefined();
    expect(Array.isArray(got)).toBe(true);
    expect(got.length).toBeLessThanOrEqual(CITES_PAGE);
    expect(got.map((x: { target: string }) => x.target.match(VERSE_PREFIX_RE)![0])).toEqual(cites.entries.map(entry => neighbourNode(entry).label));
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
