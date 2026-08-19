import { api } from './api';

// Black-box mirrors of PlaceCard.razor's own grouping/reveal logic (Batch D,
// batch-d-brief.md Requirement 2 / CONTRACT.md's "Hover place card content"
// note). Shared by world-hover-text.spec.ts and world-map.spec.ts's WORLD-2
// so both specs build their EXPECTED shape from one authoritative model
// instead of two copies that could quietly drift apart. No client/server
// imports (CONTRACT's black-box UX suite rule) -- everything here is
// recomputed from plain scene JSON.

// Flattens every event's verse_groups' verse ids (already book/chapter/
// verse-ascending per atlas-core's verse_groups_for) in event order, deduped
// by canonical id -- PlaceCard.razor's MergedVerses(), the list both it and
// design-direction.md call "canonical order".
export function mergedVerses(place: any): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const e of place.events) {
    for (const g of e.verse_groups) {
      for (const v of g.verses) {
        if (!seen.has(v)) {
          seen.add(v);
          out.push(v);
        }
      }
    }
  }
  return out;
}

function parseVerse(vref: string): { book: string; chapter: number; verse: number } {
  const [book, chapter, verse] = vref.split('.');
  return { book, chapter: Number(chapter), verse: Number(verse) };
}

export type Group = { start: number; length: number };
export const isPassage = (g: Group): boolean => g.length >= 2;

// PlaceCard.razor's Groups(): maximal runs of numerically-consecutive
// same-book/chapter verses, by LIST POSITION within `verses` -- not a fresh
// sort. Two verses that are numerically back-to-back but arrived via
// non-adjacent events never merge into one run (see PlaceCard.razor's own
// file-header comment for why that's the deliberate reading of "canonical
// order").
export function groups(verses: string[]): Group[] {
  const out: Group[] = [];
  let i = 0;
  while (i < verses.length) {
    const start = i;
    let cur = parseVerse(verses[i]);
    i++;
    while (i < verses.length) {
      const next = parseVerse(verses[i]);
      if (next.book !== cur.book || next.chapter !== cur.chapter || next.verse !== cur.verse + 1) break;
      cur = next;
      i++;
    }
    out.push({ start, length: i - start });
  }
  return out;
}

// PlaceCard.razor's InitialShownCount(): CONTRACT's 4/2-initial rule -- up
// to 4 verses if the first group is a passage, else the first 2 verses of
// the flat list (necessarily non-consecutive with each other, since the
// first group being length-1 means the very next verse isn't consecutive
// with it).
export function initialShownCount(verses: string[]): number {
  if (verses.length === 0) return 0;
  const first = groups(verses)[0];
  return isPassage(first) ? Math.min(4, first.length) : Math.min(2, verses.length);
}

// PlaceCard.razor's VisibleGroups(): every group that starts before
// shownCount, clipped to however much of it is visible yet. A group only 1
// verse into its reveal comes back with length 1 (isPassage false) even if
// its full underlying run is longer -- it renders (and this model treats
// it) as a lone verse until a later step reveals a 2nd verse of it.
export function visibleGroups(gs: Group[], shownCount: number): Group[] {
  const out: Group[] = [];
  for (const g of gs) {
    if (g.start >= shownCount) break;
    out.push({ start: g.start, length: Math.min(g.length, shownCount - g.start) });
  }
  return out;
}

// PlaceCard.razor's ShowMore() step selection: CONTRACT's 5/2-step rule,
// keyed off whichever (full, unclipped) group contains the next
// not-yet-shown verse.
export function nextStep(gs: Group[], shownCount: number): number {
  const g = gs.find(g => shownCount >= g.start && shownCount < g.start + g.length);
  if (!g) throw new Error(`nextStep: no group covers index ${shownCount} (already exhausted?)`);
  return isPassage(g) ? 5 : 2;
}

// The full expected reveal sequence a place-card-more click-loop should
// produce, starting from the initial count and stepping via nextStep until
// `verses.length` is reached -- one array entry per click, in order.
export function revealSequence(verses: string[]): number[] {
  const gs = groups(verses);
  const seq: number[] = [];
  let shown = initialShownCount(verses);
  while (shown < verses.length) {
    shown = Math.min(shown + nextStep(gs, shown), verses.length);
    seq.push(shown);
  }
  return seq;
}

// The canonical span text for a (possibly partially-visible) group, e.g.
// "GEN.12.1-4" -- CONTRACT's hover-passage-{SPAN} testid suffix.
export function spanRef(verses: string[], g: Group): string {
  const first = parseVerse(verses[g.start]);
  const last = parseVerse(verses[g.start + g.length - 1]);
  return `${first.book}.${first.chapter}.${first.verse}-${last.verse}`;
}

// Looks up a single verse's real KJV text via the chapter endpoint -- the
// same source PlaceCard.razor's AtlasClient.Chapter fetch reads from.
export async function verseText(vref: string): Promise<string> {
  const [book, chapter, verseNum] = vref.split('.');
  const chapterOut = await api.chapter(`${book}.${chapter}`);
  const verse = chapterOut.verses.find((v: any) => v.verse === Number(verseNum));
  if (!verse) {
    throw new Error(`verse ${vref} missing from fetched chapter ${book}.${chapter}`);
  }
  return verse.text;
}
