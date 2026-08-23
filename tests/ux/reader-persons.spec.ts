import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { loadToc } from './lib/canon';

// Batch P (the extensibility proof, batch-p-brief.md): "the verse popover
// gains a PERSONS section (mentioned persons, conditional presence) and the
// person node popover (card + mentions frontier with honest clamps) --
// BOTH through IExplorableClient + the edge-kind registry." This is the
// verse -> person -> mentions ONE-HOP EXPLORATION LOOP the brief's own
// requirement 5 names explicitly, plus wire-driven conditional-presence and
// honest-clamp coverage. See CONTRACT.md's own PERSONS-1 note for the full
// law this file proves.
//
// Every scenario reads REAL compiled data (never a hardcoded book/chapter/
// person id) -- the SAME "sample sweep, not hardcoded verses" discipline
// reader-xref-superscripts.spec.ts's own findVerseByXrefCount already
// establishes, via the pre-existing api.node/api.nodeEdges helpers
// (lib/api.ts, added M-D2 for exactly this "CONTRACT-lockstep assertions
// against the generic wire" purpose).

type NodeRef = { id: string; kind: string; label: string };
type EdgePage = { kind: string; entries: { edge: string; node: NodeRef }[]; next: number | null; version: string };

function textUnitId(book: string, chapter: number, verse: number): string {
  return `text-unit:${book}.${chapter}.${verse}`;
}

// Scans real chapters (first 3 of each of up to maxChapters books, the SAME
// sampling shape findVerseByXrefCount uses) for a verse whose OWN generic
// `mentions` edge page carries >=1 Person-kind entry. Returns the verse ref
// plus every Person entry found there (a verse may mention more than one).
async function findVerseWithPersonMentions(
  toc: any,
  maxChapters = 40,
): Promise<{ book: string; chapter: number; verse: number; persons: NodeRef[] } | null> {
  for (const b of toc.slice(0, maxChapters)) {
    for (const ch of b.chapters.slice(0, 3)) {
      const chapterOut = await api.chapter(`${b.code}.${ch}`);
      for (const v of chapterOut.verses as { verse: number }[]) {
        const page: EdgePage = await api.nodeEdges(textUnitId(b.code, ch, v.verse), 'mentions', { limit: 50 });
        const persons = page.entries.map(e => e.node).filter(n => n.kind === 'Person');
        if (persons.length > 0) {
          return { book: b.code, chapter: ch, verse: v.verse, persons };
        }
      }
    }
  }
  return null;
}

// The negative twin of findVerseWithPersonMentions above -- same bounded
// sample, looking for the first verse whose own `mentions` page carries
// ZERO Person-kind entries (a real, common case: most verses cite no
// person at all). Bounded the SAME way (never an unbounded whole-canon
// scan) so this stays cheap even though a "not found" result is honestly
// possible in principle.
async function findVerseWithoutPersonMentions(toc: any, maxChapters = 40): Promise<{ book: string; chapter: number; verse: number } | null> {
  for (const b of toc.slice(0, maxChapters)) {
    for (const ch of b.chapters.slice(0, 3)) {
      const chapterOut = await api.chapter(`${b.code}.${ch}`);
      for (const v of chapterOut.verses as { verse: number }[]) {
        const page: EdgePage = await api.nodeEdges(textUnitId(b.code, ch, v.verse), 'mentions', { limit: 50 });
        if (!page.entries.some(e => e.node.kind === 'Person')) {
          return { book: b.code, chapter: ch, verse: v.verse };
        }
      }
    }
  }
  return null;
}

// Same sweep, but keeps searching until it finds a PERSON whose own
// `mentioned-in` edge_summary count exceeds `minCount` -- used by the
// honest-clamp test, which needs a genuinely busy person (more mentions
// than the initial page) rather than an arbitrary one.
async function findPersonWithManyMentions(toc: any, minCount: number, maxChapters = 60): Promise<{ id: string; label: string; total: number } | null> {
  const seen = new Set<string>();
  for (const b of toc.slice(0, maxChapters)) {
    for (const ch of b.chapters.slice(0, 3)) {
      const chapterOut = await api.chapter(`${b.code}.${ch}`);
      for (const v of chapterOut.verses as { verse: number }[]) {
        const page: EdgePage = await api.nodeEdges(textUnitId(b.code, ch, v.verse), 'mentions', { limit: 50 });
        for (const e of page.entries) {
          if (e.node.kind !== 'Person' || seen.has(e.node.id)) continue;
          seen.add(e.node.id);
          const card = await api.node(e.node.id);
          const count = (card.edge_summary as { kind: string; count: number }[]).find(s => s.kind === 'mentioned-in')?.count ?? 0;
          if (count > minCount) {
            return { id: e.node.id, label: e.node.label, total: count };
          }
        }
      }
    }
  }
  return null;
}

// Extracts (bookIndex-free) a comparable canon-order key straight off a
// "BOOK.C.V" dot-ref string using the real book table order (toc's own
// array order IS canon order -- Books.csv/canon.json's own order, the same
// assumption findVerseByXrefCount's own book iteration already relies on).
function canonKey(toc: any, vref: string): [number, number, number] {
  const [book, chapter, verse] = vref.split('.');
  const bookIndex = toc.findIndex((b: any) => b.code === book);
  return [bookIndex, Number(chapter), Number(verse)];
}
function cmpCanon(a: [number, number, number], b: [number, number, number]): number {
  return a[0] - b[0] || a[1] - b[1] || a[2] - b[2];
}

test.describe('Batch P: PERSONS section + the person popover', () => {
  test('PERSONS-1: a verse mentioning a person shows a PERSONS section; a verse mentioning none shows no section at all', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseWithPersonMentions(toc);
    test.skip(!found, 'no sampled verse carried a Person mention');
    if (!found) return;

    await page.goto(`/read/${found.book}/${found.chapter}`);
    await page.getByTestId(`verse-line-${found.verse}`).click();
    await expect(page.getByTestId('popover-title')).toHaveText(`${found.book}.${found.chapter}.${found.verse}`);

    const heading = page.getByTestId('persons-section-heading');
    await expect(heading).toBeVisible();
    await expect(heading).toHaveText('PERSONS');
    for (const person of found.persons) {
      await expect(page.getByTestId(`verse-person-${slug(person.label)}`)).toHaveText(person.label);
    }
    await page.getByTestId('popover-close').click();

    // Negative case, same test: pick a verse the wire itself says carries
    // ZERO person mentions (never assumed/hardcoded -- matches this
    // suite's own "wire-driven" rule throughout).
    const empty = await findVerseWithoutPersonMentions(toc);
    test.skip(!empty, 'no sampled verse carried zero Person mentions');
    if (!empty) return;
    await page.goto(`/read/${empty.book}/${empty.chapter}`);
    await page.getByTestId(`verse-line-${empty.verse}`).click();
    await expect(page.getByTestId('popover-title')).toHaveText(`${empty.book}.${empty.chapter}.${empty.verse}`);
    await expect(page.getByTestId('persons-section-heading')).toHaveCount(0);
  });

  test('PERSONS-1: the verse -> person -> mentions one-hop exploration loop', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseWithPersonMentions(toc);
    test.skip(!found, 'no sampled verse carried a Person mention');
    if (!found) return;
    const person = found.persons[0];

    // Independently confirm, at the wire, what the Person's own card +
    // mentioned-in frontier should say -- the test's own oracle, not a
    // second UI-derived guess.
    const card = await api.node(person.id);
    const mentionedInCount = (card.edge_summary as { kind: string; count: number }[]).find(s => s.kind === 'mentioned-in')?.count ?? 0;
    const firstPage: EdgePage = await api.nodeEdges(person.id, 'mentioned-in', { limit: 12 });
    expect(firstPage.entries.length).toBeGreaterThan(0);
    const firstVref = firstPage.entries[0].node.label;

    await page.goto(`/read/${found.book}/${found.chapter}`);
    await page.getByTestId(`verse-line-${found.verse}`).click();
    await page.getByTestId(`verse-person-${slug(person.label)}`).click();

    // Landed on the PERSON card: title is the person's own name (the
    // popover's title chrome, not a section), and the mentions frontier
    // shows the WIRE's own true count up front (design doc §7's own
    // "honest count up front" law).
    await expect(page.getByTestId('popover-title')).toHaveText(person.label);
    await expect(page.getByTestId('person-mentions-heading')).toHaveText(`MENTIONED IN SCRIPTURE (${mentionedInCount})`);
    await expect(page.getByTestId(`person-mention-${firstVref}`)).toBeVisible();

    // ONE HOP FURTHER: click that first mentioned verse -- lands back on a
    // real VERSE popover for exactly that ref, closing the loop
    // (verse -> person -> mentions -> verse).
    await page.getByTestId(`person-mention-${firstVref}`).click();
    await expect(page.getByTestId('popover-title')).toHaveText(firstVref);
  });

  test('PERSONS-2: the mentioned-in frontier is honestly clamped, and Reveal fetches a genuine second page', async ({ page }) => {
    const toc = await loadToc();
    const busy = await findPersonWithManyMentions(toc, 12);
    test.skip(!busy, 'no sampled person exceeded the initial 12-entry clamp');
    if (!busy) return;

    const firstPage: EdgePage = await api.nodeEdges(busy.id, 'mentioned-in', { limit: 12 });
    expect(firstPage.next).not.toBeNull();

    // Reach the person's own popover the same way a real user would --
    // through a verse that mentions them (PersonNode has no direct /read
    // route of its own; every card exists only via the mentions frontier,
    // by design -- see PersonNode.cs's own doc comment).
    const entryVref = firstPage.entries[0].node.label;
    const [book, chapterStr] = entryVref.split('.');
    const chapter = Number(chapterStr);
    await page.goto(`/read/${book}/${chapter}`);
    const verseNum = Number(entryVref.split('.')[2]);
    await page.getByTestId(`verse-line-${verseNum}`).click();
    await page.getByTestId(`verse-person-${slug(busy.label)}`).click();
    await expect(page.getByTestId('popover-title')).toHaveText(busy.label);

    const rowsBefore = await page.locator('[data-testid^="person-mention-"]').count();
    expect(rowsBefore).toBe(12);
    const reveal = page.getByTestId('person-mentions-more');
    await expect(reveal).toBeVisible();

    await reveal.click();
    await expect
      .poll(async () => page.locator('[data-testid^="person-mention-"]').count(), { message: 'Reveal must fetch and append a genuine second page' })
      .toBeGreaterThan(rowsBefore);
  });

  test('PERSONS-3: a person’s mentioned-in frontier is canon-ordered on the wire (server-only, no browser needed)', async ({}) => {
    const toc = await loadToc();
    const found = await findVerseWithPersonMentions(toc);
    test.skip(!found, 'no sampled verse carried a Person mention');
    if (!found) return;

    const page: EdgePage = await api.nodeEdges(found.persons[0].id, 'mentioned-in', { limit: 200 });
    const keys = page.entries.map(e => canonKey(toc, e.node.label));
    const sorted = [...keys].sort(cmpCanon);
    expect(keys, `${found.persons[0].label}'s own mentioned-in page must already arrive in canon order -- no client re-sort exists anywhere on this path`).toEqual(sorted);
  });
});

// A stable, DOM-safe testid fragment from a person's own display name --
// MUST match PopoverSectionProviders.cs's own VersePersonsSection.Slug
// exactly (server label -> client testid, one shared derivation rule; if
// the two ever drift, this test's own selectors simply stop matching,
// which is the point -- a silent testid mismatch would otherwise hide a
// real regression).
function slug(label: string): string {
  return label
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}
