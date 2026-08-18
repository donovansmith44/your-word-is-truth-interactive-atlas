import fc from 'fast-check';
import { api } from './api';
import { SPAN } from './years';

export type Toc = { code: string; name: string; chapters: number[] }[];
let toc: Toc | null = null;
export async function loadToc(): Promise<Toc> { return (toc ??= await api.books()); }

export const arbYear = fc.integer({ min: SPAN.from, max: SPAN.to }).filter(y => y !== 0);
export const arbWindow = fc.tuple(arbYear, arbYear)
  .map(([a, b]) => (a <= b ? { from: a, to: b } : { from: b, to: a }));

export function arbChapterRef(t: Toc) {
  return fc.integer({ min: 0, max: t.length - 1 }).chain(bi =>
    fc.integer({ min: 1, max: t[bi].chapters.length })
      .map(ch => ({ book: t[bi].code, chapter: ch, verses: t[bi].chapters[ch - 1] })));
}
export function arbVerseRef(t: Toc) {
  return arbChapterRef(t).chain(c =>
    fc.integer({ min: 1, max: c.verses }).map(v => `${c.book}.${c.chapter}.${v}`));
}
export function arbPassageRef(t: Toc) {
  return arbChapterRef(t).chain(c =>
    fc.tuple(fc.integer({ min: 1, max: c.verses }), fc.integer({ min: 1, max: c.verses }))
      .filter(([a, b]) => a < b)
      .map(([a, b]) => `${c.book}.${c.chapter}.${a}-${b}`));
}
export function arbAnyRef(t: Toc) {
  return fc.oneof(
    fc.integer({ min: 0, max: t.length - 1 }).map(i => t[i].code),
    arbChapterRef(t).map(c => `${c.book}.${c.chapter}`),
    arbVerseRef(t),
    arbPassageRef(t));
}
