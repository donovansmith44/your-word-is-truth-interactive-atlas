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
// O4 (owner live-preview correction, 2026-08-23: "remove persons from
// hover menus for now"): the verse popover's own PERSONS section
// (VersePersonsSection, the FIRST half of the premise above) is retired --
// removed-for-now by owner order, its own machinery (the class itself)
// retained per the ruling's own explicit words, just unregistered from
// PopoverSectionRegistry.Providers (see PopoverSections.cs's own comment).
// The verse -> person -> mentions loop and the person popover itself (the
// SECOND half) are UNCHANGED and NOT named by the order -- only the ENTRY
// PATH moves: every test below that used to enter via the retired
// section's own `verse-person-{slug}` row now enters via the SAME in-text
// mention link (M-D3/U5, `verse-mention-person-{verse}-{personId}`,
// Reader.razor's own primary verse text -- a different, pre-existing
// affordance the owner's own order never touched) instead, so person-node
// coverage (the card, the honestly-clamped mentions frontier, the onward
// hop back to a verse) stays fully alive. PERSONS-1's own first test
// (conditional PRESENCE of the section) is inverted to prove its
// unconditional ABSENCE instead -- see that test's own comment.
//
// Every scenario reads REAL compiled data (never a hardcoded book/chapter/
// person id) -- the SAME "sample sweep, not hardcoded verses" discipline
// reader-xref-superscripts.spec.ts's own findVerseByXrefCount already
// establishes, via the pre-existing api.node/api.nodeEdges helpers
// (lib/api.ts, added M-D2 for exactly this "CONTRACT-lockstep assertions
// against the generic wire" purpose).

type NodeRef = { id: string; kind: string; label: string };
type EdgePage = { kind: string; entries: { edge: string; node: NodeRef }[]; next: number | null; version: string };

// O4 rework: scans real chapters (first 3 of each of up to maxChapters
// books, the SAME sampling shape findVerseByXrefCount uses) for a verse
// whose own `persons` field (ChapterOut.Verses[].Persons, the wire's own
// ATTESTED-mentions index -- PersonNode.cs's own doc comment) names someone
// whose display name is a genuine, exact-case LITERAL substring of that
// verse's own text -- the SAME plain-text match PlaceMentions.Scan (client)
// performs (R-D1, 2026-08-23: case-SENSITIVE, Ordinal), so this is exactly
// the condition under which `verse-mention-person-{verse}-{personId}`
// (Reader.razor's own in-text link, the surviving entry path -- see this
// file's own header comment) actually renders, not merely "the wire knows
// about a relationship." Returns every such person found (a verse may
// literally mention more than one) plus the discovering verse ref itself,
// so a caller can click straight into the in-text span with no intermediate
// verse-popover hop.
async function findVerseWithPersonMentions(
  toc: any,
  maxChapters = 40,
): Promise<{ book: string; chapter: number; verse: number; persons: { id: string; name: string }[] } | null> {
  for (const b of toc.slice(0, maxChapters)) {
    for (const ch of b.chapters.slice(0, 3)) {
      const chapterOut = await api.chapter(`${b.code}.${ch}`);
      for (const v of chapterOut.verses as { verse: number; text: string; persons?: { id: string; name: string }[] }[]) {
        const persons = (v.persons || []).filter(p => v.text.includes(p.name));
        if (persons.length > 0) {
          return { book: b.code, chapter: ch, verse: v.verse, persons };
        }
      }
    }
  }
  return null;
}

// The negative twin of findVerseWithPersonMentions above -- same bounded
// sample, looking for the first verse whose own `persons` field is empty
// (a real, common case: most verses attest no person at all). Bounded the
// SAME way (never an unbounded whole-canon scan) so this stays cheap even
// though a "not found" result is honestly possible in principle.
async function findVerseWithoutPersonMentions(toc: any, maxChapters = 40): Promise<{ book: string; chapter: number; verse: number } | null> {
  for (const b of toc.slice(0, maxChapters)) {
    for (const ch of b.chapters.slice(0, 3)) {
      const chapterOut = await api.chapter(`${b.code}.${ch}`);
      for (const v of chapterOut.verses as { verse: number; persons?: { id: string; name: string }[] }[]) {
        if (!v.persons || v.persons.length === 0) {
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
// than the initial page) rather than an arbitrary one. O4 rework: sourced
// from `v.persons` (see findVerseWithPersonMentions above) rather than the
// generic `mentions` edge, and returns the DISCOVERING verse itself (book/
// chapter/verse) alongside the person -- that verse is, by construction,
// one where the person's own name is a genuine literal text match, so the
// caller can click straight into `verse-mention-person-{verse}-{id}` with
// no separate "is this vref actually mentionable" check of its own.
async function findPersonWithManyMentions(
  toc: any,
  minCount: number,
  maxChapters = 60,
): Promise<{ id: string; label: string; total: number; book: string; chapter: number; verse: number } | null> {
  const seen = new Set<string>();
  for (const b of toc.slice(0, maxChapters)) {
    for (const ch of b.chapters.slice(0, 3)) {
      const chapterOut = await api.chapter(`${b.code}.${ch}`);
      for (const v of chapterOut.verses as { verse: number; text: string; persons?: { id: string; name: string }[] }[]) {
        for (const p of v.persons || []) {
          if (seen.has(p.id) || !v.text.includes(p.name)) continue;
          seen.add(p.id);
          const card = await api.node(`Person:${p.id}`);
          const count = (card.edge_summary as { kind: string; count: number }[]).find(s => s.kind === 'mentioned-in')?.count ?? 0;
          if (count > minCount) {
            return { id: p.id, label: p.name, total: count, book: b.code, chapter: ch, verse: v.verse };
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
  // O4 (owner live-preview correction, 2026-08-23: "remove persons from
  // hover menus for now") inverts this test's own original premise --
  // PERSONS-1 used to prove the section's CONDITIONAL presence (shown when
  // a verse attests a person, absent when it doesn't); it now proves the
  // section is UNCONDITIONALLY absent either way, on the exact same two
  // real-data cases the original test already sampled (a person-attesting
  // verse AND a zero-person one) -- the strongest available proof that the
  // retirement is total, not merely "usually doesn't show." VersePersonsSection
  // itself still exists (dead-code law's own exception, per the ruling's
  // explicit "machinery retained" words) but is unregistered from
  // PopoverSectionRegistry.Providers -- see PopoverSections.cs's own
  // comment -- so `persons-section-heading`/`verse-person-*`/
  // `persons-section-more` can never render from ANY node, ever, regardless
  // of what the wire says.
  test('PERSONS-1: a verse popover shows no PERSONS section at all, whether or not the verse attests a person (removed for now, O4)', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseWithPersonMentions(toc);
    test.skip(!found, 'no sampled verse carried a literally-text-mentioned Person');
    if (!found) return;

    await page.goto(`/read/${found.book}/${found.chapter}`);
    await page.getByTestId(`verse-line-${found.verse}`).click();
    await expect(page.getByTestId('popover-title')).toHaveText(`${found.book}.${found.chapter}.${found.verse}`);
    await expect(page.getByTestId('persons-section-heading')).toHaveCount(0);
    for (const person of found.persons) {
      await expect(page.getByTestId(`verse-person-${person.id}`)).toHaveCount(0);
    }
    await expect(page.getByTestId('persons-section-more')).toHaveCount(0);
    await page.getByTestId('popover-close').click();

    // Second case, same test: a verse the wire itself says attests ZERO
    // persons (never assumed/hardcoded -- matches this suite's own
    // "wire-driven" rule throughout) -- absent here too, for the ordinary,
    // unremarkable reason (nothing to show), not the O4 retirement.
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
    test.skip(!found, 'no sampled verse carried a literally-text-mentioned Person');
    if (!found) return;
    const person = found.persons[0];
    const wireId = `Person:${person.id}`; // v.persons[].id is the RAW id (PersonNode.cs's own doc comment) -- the generic node/edges endpoints below want the "Person:"-prefixed wire form OpenPersonMention itself constructs.

    // Independently confirm, at the wire, what the Person's own card +
    // mentioned-in frontier should say -- the test's own oracle, not a
    // second UI-derived guess.
    const card = await api.node(wireId);
    const mentionedInCount = (card.edge_summary as { kind: string; count: number }[]).find(s => s.kind === 'mentioned-in')?.count ?? 0;
    const firstPage: EdgePage = await api.nodeEdges(wireId, 'mentioned-in', { limit: 12 });
    expect(firstPage.entries.length).toBeGreaterThan(0);
    const firstVref = firstPage.entries[0].node.label;

    // O4: enters via the SAME in-text mention link (M-D3/U5,
    // `verse-mention-person-{verse}-{id}`, Reader.razor's own primary verse
    // text) the retired PERSONS section's own `verse-person-*` row used to
    // stand in front of -- straight to the person popover, no intermediate
    // verse-popover hop needed at all (unlike a place mention, a person
    // mention has never required opening the verse popover first; see
    // Reader.razor's own OpenPersonMention).
    await page.goto(`/read/${found.book}/${found.chapter}`);
    await page.getByTestId(`verse-mention-person-${found.verse}-${person.id}`).click();

    // Landed on the PERSON card: title is the person's own name (the
    // popover's title chrome, not a section), and the mentions frontier
    // shows the WIRE's own true count up front (design doc §7's own
    // "honest count up front" law).
    await expect(page.getByTestId('popover-title')).toHaveText(person.name);
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
    const wireId = `Person:${busy.id}`; // findPersonWithManyMentions' own raw id -- see PERSONS-1's own wireId comment above.

    const firstPage: EdgePage = await api.nodeEdges(wireId, 'mentioned-in', { limit: 12 });
    expect(firstPage.next).not.toBeNull();

    // O4: reach the person's own popover via the SAME in-text mention link
    // (M-D3/U5) every OTHER test in this file now uses -- the DISCOVERING
    // verse itself (findPersonWithManyMentions' own return), guaranteed by
    // construction to be one where busy.label is a genuine literal text
    // match (unlike an arbitrary mentioned-in page entry, which the retired
    // PERSONS-section entry path never actually required to be one).
    await page.goto(`/read/${busy.book}/${busy.chapter}`);
    await page.getByTestId(`verse-mention-person-${busy.verse}-${busy.id}`).click();
    await expect(page.getByTestId('popover-title')).toHaveText(busy.label);

    // Fix round 1, self-caught (a real async-timing bug, the SAME class
    // M-D2's own fix round already named elsewhere in this app: "an async
    // race asserting section order immediately after popover-title, which
    // renders synchronously, without waiting for the section list's own
    // async population"): `popover-title` resolves off `Current.Title`
    // the instant the node is pushed, BEFORE PersonCardAndMentionsSection's
    // own concurrent Card()+Edges() fetch (Task.WhenAll) has necessarily
    // finished and rendered. A bare, non-retrying `.count()` snapshot taken
    // right after the title assertion can genuinely read 0 -- `toHaveCount`
    // (auto-retrying) is what the ExplorerPopover's own async render
    // lifecycle actually requires here, not a plain snapshot.
    const personMentionRows = page.locator('[data-testid^="person-mention-"]');
    await expect(personMentionRows).toHaveCount(12);
    const rowsBefore = 12;
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
    test.skip(!found, 'no sampled verse carried a literally-text-mentioned Person');
    if (!found) return;

    const page: EdgePage = await api.nodeEdges(`Person:${found.persons[0].id}`, 'mentioned-in', { limit: 200 });
    const keys = page.entries.map(e => canonKey(toc, e.node.label));
    const sorted = [...keys].sort(cmpCanon);
    expect(keys, `${found.persons[0].name}'s own mentioned-in page must already arrive in canon order -- no client re-sort exists anywhere on this path`).toEqual(sorted);
  });
});

// O4 (2026-08-23) retired this file's own last dependency on
// VersePersonsSection.Slug (server label -> testid derivation) along with
// the retired `verse-person-{slug}` entry path itself -- every test above
// now reaches a person via `verse-mention-person-{verse}-{id}` (M-D3/U5),
// keyed by the person's own RAW id, never a slugified label; this helper
// had no remaining caller and is removed with it, dead-code law.
