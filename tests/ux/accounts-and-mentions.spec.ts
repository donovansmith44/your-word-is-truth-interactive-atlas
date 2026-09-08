import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch ATTEST-1 -- ACCOUNTS vs. MENTIONS, and the Analogue relation.
//
// THE OWNER'S TWO REPORTS, verbatim, which these fixtures exist to hold
// fixed:
//
//   1. "A leper healed; a great popular excitement is given a parallel
//      where there shouldn't be from Mat.8.1-4; another leprosy story. We
//      need to come up with an idiom for stories that are very similar in
//      this regard, but distinct events." -> "let's call it Analogue;
//      that's ok for now."
//
//   2. "I'm seeing a fundamental error. The Espousal of Mary event has
//      parallel accounts Mat.1.18 + Luke.1.27, and that's a distinct event
//      from The Angel Gabriel Announces Jesus'... which has Luk1.26-36;
//      Even worse, the appearance of Gabriel to Zacharias is BETWEEN the
//      espousal of mary and the announcement of Gabriel to Mary."
//
// Both were the SAME defect class: `Attests` (the relation that means "this
// passage NARRATES this event") was being used for verses that merely
// REFERENCE an event, and for two similar-but-distinct stories. Because the
// verse popover's own event membership and the event popover's own PARALLEL
// ACCOUNTS both walk `Attests`, the bad rows rendered as confident, wrong
// claims. The fixes are three new facts on the wire -- an event's
// `mentioned_in`, an event's `analogues`, and the corrected `Attests`
// partition itself -- and these tests assert every one of them at the DOM,
// not just at the API.

function parseVerse(vref: string): { book: string; chapter: number; verse: number } {
  const [book, chapter, verse] = vref.split('.');
  return { book, chapter: Number(chapter), verse: Number(verse) };
}

// Opens a verse's own popover the keyboard way -- the SAME hazard-free
// activation `event-timeline.spec.ts`'s own helper documents (a verse-line
// click can land on an attested mention sitting at the click's geometric
// centre and hang).
async function openVersePopover(page: any, vref: string) {
  const v = parseVerse(vref);
  await page.goto(`/read/${v.book}/${v.chapter}`);
  await page.getByTestId(`verse-line-${v.verse}`).focus();
  await page.keyboard.press('Enter');
}

test('ATTEST-1 (owner report 2): LUK.1.27 belongs to exactly ONE event -- the annunciation -- and the espousal is no longer among its accounts', async ({ page }) => {
  // THE FOUNDING CASE, at the exact surface the owner was looking at. Before
  // this batch LUK.1.27 sat in the `Attests` set of BOTH theo-249 (Espousal
  // of Mary) and rob_annunciation_mary (LUK.1.26-38), so this popover
  // offered two event rows and the espousal claimed a verse from inside the
  // annunciation narrative as one of its own "parallel accounts".
  await openVersePopover(page, 'LUK.1.27');
  await expect(page.getByTestId('verse-event-rob_annunciation_mary')).toBeVisible();
  await expect(
    page.getByTestId('verse-event-theo-249'),
    'the espousal must NOT claim LUK.1.27 as an account: "To a virgin espoused to a man whose name was Joseph" MENTIONS the betrothal inside Luke\'s account of the ANNUNCIATION -- it does not narrate it'
  ).toHaveCount(0);

  // And the same at the wire, so a green DOM can never rest on a missing
  // testid: the API itself must partition the verse to one event.
  const verse = await api.verse('LUK.1.27');
  const eventIds = (verse.events ?? []).map((e: any) => e.id);
  expect(eventIds).toContain('rob_annunciation_mary');
  expect(eventIds).not.toContain('theo-249');
});

test('ATTEST-1 (L3, owner report 2): the Espousal of Mary is a MENTION-ONLY event -- "MENTIONED IN" renders, "PARALLEL ACCOUNTS" does not', async ({ page }) => {
  // Wire first: total capture. Both former "accounts" survive as MENTIONS;
  // neither was deleted, and the event has no accounts left at all.
  const espousal = await api.event('theo-249');
  expect(espousal.witnesses ?? [], 'a mention-only event has zero narrative accounts').toHaveLength(0);
  expect(espousal.mentioned_in, 'both retyped facts must still reach the event, as mentions').toEqual([
    'MAT.1.18',
    'LUK.1.27',
  ]);

  // DOM: reached the only way a mention-only event now can be -- through the
  // chronology, from its own temporal neighbour. That path is itself the
  // owner's OTHER complaint about this event ("the appearance of Gabriel to
  // Zacharias is BETWEEN the espousal of mary and the announcement"), so
  // walking it proves the ordering as a side effect: Zacharias's PRIOR is
  // the espousal, and Zacharias's FOLLOWING is the annunciation.
  const zacharias = await api.event('rob_zacharias_vision');
  await openVersePopover(page, zacharias.witnesses[0].verse_groups[0].verses[0]);
  await page.getByTestId('verse-event-rob_zacharias_vision').click();
  await expect(page.getByTestId('popover-title')).toHaveText(zacharias.title);

  await expect(
    page.getByTestId('event-chrono-following-event-global').locator('.popover-event-nav-label'),
    'the annunciation follows Zacharias in time'
  ).toHaveText((await api.event('rob_annunciation_mary')).title);

  await page.getByTestId('event-chrono-prior-event-global').click();
  await expect(
    page.getByTestId('popover-title'),
    'the espousal precedes Zacharias -- which is only coherent once LUK.1.27 stops being an account of it'
  ).toHaveText(espousal.title);

  // THE SECTION SPLIT ITSELF: mentions render under their own heading, and
  // the parallel-accounts section is ABSENT -- not empty, absent (the
  // smart-frontier law: a section shows iff instances exist).
  await expect(page.getByTestId('popover-section-event-mentions')).toBeVisible();
  await expect(page.getByTestId('event-mentioned-in-MAT.1.18')).toBeVisible();
  await expect(page.getByTestId('event-mentioned-in-LUK.1.27')).toBeVisible();
  await expect(
    page.getByTestId('popover-section-event-witnesses'),
    'a mention-only event must never fabricate a PARALLEL ACCOUNTS section'
  ).toHaveCount(0);

  // A mention is explorable like everything else: clicking one opens the
  // verse it names.
  await page.getByTestId('event-mentioned-in-LUK.1.27').click();
  await expect(page.getByTestId('popover-title')).toHaveText('LUK.1.27');
});

test('ATTEST-1 (L4, owner report 1): Matthew\'s leper is its own event, joined to Mark/Luke\'s by SIMILAR ACCOUNTS directly below PARALLEL ACCOUNTS -- never by a false parallel account', async ({ page }) => {
  // Wire: the two leprosy events are genuinely two, each with its own
  // accounts, and the Analogue relation is what joins them.
  const matthews = await api.event('mat_leper_healed');
  const marks = await api.event('rob_leper_healed');
  expect(matthews.witnesses).toHaveLength(1);
  expect(matthews.witnesses[0].book).toBe('MAT');
  expect(marks.witnesses.map((w: any) => w.book).sort()).toEqual(['LUK', 'MRK']);
  expect(
    marks.witnesses.map((w: any) => w.book),
    'MAT.8.1-4 was the false parallel the owner reported; it must no longer be an account of Mark\'s and Luke\'s occasion'
  ).not.toContain('MAT');
  expect(matthews.analogues.map((a: any) => a.id)).toEqual(['rob_leper_healed']);
  expect(marks.analogues.map((a: any) => a.id), 'Analogue is SYMMETRIC -- both ends see it').toEqual(['mat_leper_healed']);

  // DOM: MAT.8.2 (the leper's own request verse) now offers Matthew's event
  // and only Matthew's.
  await openVersePopover(page, 'MAT.8.2');
  await expect(page.getByTestId('verse-event-mat_leper_healed')).toBeVisible();
  await expect(page.getByTestId('verse-event-rob_leper_healed')).toHaveCount(0);

  await page.getByTestId('verse-event-mat_leper_healed').click();
  await expect(page.getByTestId('popover-title')).toHaveText(matthews.title);
  // ONE account -> no "PARALLEL ACCOUNTS" eyebrow (the pre-existing
  // single-witness rule, now reaching the honest answer).
  await expect(page.getByTestId('popover-section-event-witnesses')).toHaveCount(0);
  // ... and the similarity is stated as similarity.
  await expect(page.getByTestId('popover-section-event-analogues')).toBeVisible();
  const similar = page.getByTestId('event-analogues-rob_leper_healed');
  await expect(similar).toBeVisible();

  // Walk it: the analogue is explorable, and the far end shows the relation
  // back (symmetry, live in the DOM, not just in the row table) alongside
  // its own two real accounts.
  await similar.click();
  await expect(page.getByTestId('popover-title')).toHaveText(marks.title);
  const parallels = page.getByTestId('popover-section-event-witnesses');
  const analogues = page.getByTestId('popover-section-event-analogues');
  await expect(
    parallels,
    'Mark and Luke ARE two accounts of one event -- this event keeps its real PARALLEL ACCOUNTS section'
  ).toBeVisible();
  await expect(analogues).toBeVisible();

  // THE OWNER'S PLACEMENT AMENDMENT, at the DOM (verbatim: "let's have a
  // 'Similar Accounts' or something similar added to the frontier part of
  // the UI where it was getting pulled in as a parallel account. Have that
  // section be right below the 'Parallel ..' section.").
  //
  // (a) the row appears under SIMILAR ACCOUNTS ...
  await expect(analogues.getByTestId('event-section-heading')).toHaveText('SIMILAR ACCOUNTS');
  await expect(analogues.getByTestId('event-analogues-mat_leper_healed')).toBeVisible();
  // (b) ... and NOT under PARALLEL ACCOUNTS. This is the owner's actual
  // report, inverted into an assertion: Matthew's leper used to render as
  // one of this event's parallel accounts.
  await expect(
    parallels.getByTestId('event-analogues-mat_leper_healed'),
    "Matthew's leper must not appear inside the PARALLEL ACCOUNTS section -- that WAS the defect"
  ).toHaveCount(0);
  await expect(parallels.getByTestId('event-section-heading')).toHaveText('PARALLEL ACCOUNTS');
  const parallelRefs = await parallels.getByTestId(/^event-witness/).count();
  expect(parallelRefs, 'the parallel-accounts section still renders its real accounts -- that they are Mark and Luke AND ONLY THEM is asserted at the wire above (:124-128), not by this count').toBeGreaterThan(0);

  // (c) the two sections are ADJACENT, in that order -- the wrongly-placed
  // row moved down exactly ONE section, it did not disappear from where the
  // reader last saw it. Read off the real rendered DOM order, so a future
  // provider slipping in between fails here as well as in
  // PopoverSectionRegistryTests.
  const sectionIds = await page.locator('[data-testid^="popover-section-"]').evaluateAll(
    (els) => els.map((e) => e.getAttribute('data-testid'))
  );
  const pIdx = sectionIds.indexOf('popover-section-event-witnesses');
  const aIdx = sectionIds.indexOf('popover-section-event-analogues');
  expect(pIdx, 'PARALLEL ACCOUNTS must be rendered').toBeGreaterThanOrEqual(0);
  expect(aIdx, 'SIMILAR ACCOUNTS must be rendered directly below it').toBe(pIdx + 1);
});

test('ATTEST-1: an ordinary event is untouched -- no MENTIONED IN, no SIMILAR ACCOUNTS, PARALLEL ACCOUNTS exactly as before', async ({ page }) => {
  // The conditional-presence half of the smart-frontier law, pinned against
  // an event this batch never touched: two new sections must not start
  // appearing everywhere. `pw_bethany` has three real curated witnesses and
  // no mentions or analogues of its own.
  const bethany = await api.event('pw_bethany');
  expect(bethany.mentioned_in ?? [], 'the wire OMITS the key when empty').toHaveLength(0);
  expect(bethany.analogues ?? []).toHaveLength(0);

  await openVersePopover(page, bethany.witnesses[0].verse_groups[0].verses[0]);
  await page.getByTestId('verse-event-pw_bethany').click();
  await expect(page.getByTestId('popover-title')).toHaveText(bethany.title);
  await expect(page.getByTestId('popover-section-event-witnesses')).toBeVisible();
  await expect(page.getByTestId('popover-section-event-mentions')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-analogues')).toHaveCount(0);
});
