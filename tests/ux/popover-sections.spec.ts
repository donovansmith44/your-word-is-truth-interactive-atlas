import { test, expect, Page, Locator } from '@playwright/test';
import { openVerse } from './lib/verse';
import { popoverSectionsHolding } from './lib/popover';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbVerseRef } from './lib/canon';
import { elementNode, neighbourNode } from './lib/edges';

// Batch R requirement 3 ("the popover becomes a content-first section
// platform") + requirement 4 (expandable popover / in-context chapter
// reading) + requirement 5 (place-in-verse hover -> marker blink), all
// exercised through the real, live popover -- see CONTRACT.md's own
// REGISTRY-1/READER-1/BLINK-1 notes for the exact behavior each test below
// pins.

type VerseRecord = {
  id: string;
  label: string;
  text: { locus: { book: string; chapter: number; verse: number } };
  edge_summary: { kind: string; count: number }[];
};

const PAGE_SIZE = 20;

function served(record: VerseRecord, kind: string): number {
  return record.edge_summary.find(group => group.kind === kind)?.count ?? 0;
}

async function findVerse(predicate: (record: VerseRecord) => boolean, maxTries = 120): Promise<VerseRecord | null> {
  const toc = await loadToc();
  for (const vref of fc.sample(arbVerseRef(toc), maxTries)) {
    const record: VerseRecord = await api.node(`text-unit:${vref}`);
    if (predicate(record)) {
      return record;
    }
  }
  return null;
}

async function openRecord(page: Page, record: VerseRecord): Promise<void> {
  const { book, chapter, verse } = record.text.locus;
  await page.goto(`/read/${book}/${chapter}`);
  await openVerse(page, verse);
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
}

// ---------------------------------------------------------------------
// REGISTRY-1: VERSE node sections, in order, conditional presence
// ---------------------------------------------------------------------

test('REGISTRY-1: a verse with real cross-references shows them inline, no button press', async ({ page }) => {
  // Arrange
  const record = await findVerse(candidate => served(candidate, 'cites') > 0);
  test.skip(!record, 'no sampled verse had cross-references');
  if (!record) return;
  const first = neighbourNode((await api.nodeEdges(record.id, 'cites')).entries[0]);

  // Act
  await openRecord(page, record);

  // Assert
  const section = page.getByTestId('popover-section-cites');
  await expect(section.getByTestId('popover-section-cites-heading')).toHaveText(`Cites (${served(record, 'cites')})`);
  await expect(section.getByTestId(/^popover-link-cites-/)).toHaveCount(Math.min(served(record, 'cites'), PAGE_SIZE));
  await expect(page.getByTestId(`popover-link-cites-${first.id}`)).toHaveText(first.label);
  await expect(page.getByTestId('popover-chip-xrefs')).toHaveCount(0);
});

test('VERSE-WORDS-1 (owner, 2026-10-02): a verse\'s Cites list shows each cited verse\'s served words under its link', async ({ page }) => {
  // Arrange
  const verseId = 'text-unit:GEN.1.1';
  const cited = (await api.nodeEdges(verseId, 'cites')).entries.map(neighbourNode);
  const records = (await api.elements(cited.map(unit => unit.id))).elements.map((element, n) => elementNode(element, cited[n].id) as any);

  // Act
  await page.goto('/read/GEN/1');
  await openVerse(page, 1);

  // Assert
  const section = page.getByTestId('popover-section-cites');
  await expect(section.locator('[data-testid^="popover-words-cites-"][data-testid$="-text"]')).toHaveText(records.map(record => record.text.text));
});

test('REGISTRY-1: a verse with zero cross-references shows no cites section at all (conditional presence)', async ({ page }) => {
  // Arrange
  const record = await findVerse(candidate => served(candidate, 'cites') === 0);
  test.skip(!record, 'no sampled verse had zero cross-references');
  if (!record) return;

  // Act
  await openRecord(page, record);

  // Assert
  await expect(page.getByTestId('popover-text')).toBeVisible();
  await expect(page.getByTestId('popover-section-cites')).toHaveCount(0);
  await expect(page.getByTestId(/^popover-link-cites-/)).toHaveCount(0);
});

test('CATECH-1: a verse with zero catechism citations shows no catechism section', async ({ page }) => {
  // Arrange
  const record = await findVerse(candidate => served(candidate, 'catechism-link') === 0);
  test.skip(!record, 'no sampled verse had zero catechism citations');
  if (!record) return;

  // Act
  await openRecord(page, record);

  // Assert
  await expect(page.getByTestId('popover-text')).toBeVisible();
  await expect(page.getByTestId('popover-section-catechism-link')).toHaveCount(0);
});

test('FRONTIER-ORDER-1: a content-rich verse (MAT.26.28) shows its text, its events, its catechism and its cross references in the owner\'s ruled order', async ({ page }) => {
  // Arrange
  const inTheOwnersOrder = [
    'popover-section-text',
    'popover-section-attests',
    'popover-section-catechism-link',
    'popover-section-cites',
  ];
  await page.goto('/read/MAT/26');

  // Act
  await openVerse(page, 28);

  // Assert
  const sectionIds = await popoverSectionsHolding(page, inTheOwnersOrder);
  expect(sectionIds.filter(id => inTheOwnersOrder.includes(id))).toEqual(inTheOwnersOrder);
});

test('FRONTIER-ORDER-1: a verse with no other category present shows its text alone (conditional presence; the text always present)', async ({ page }) => {
  // Arrange
  const record = await findVerse(candidate =>
    served(candidate, 'cites') === 0 && served(candidate, 'catechism-link') === 0 && served(candidate, 'attests') === 0);
  test.skip(!record, 'no sampled verse had zero of every other category');
  if (!record) return;

  // Act
  await openRecord(page, record);

  // Assert
  await expect(page.getByTestId('popover-text')).toBeVisible();
  await expect(page.getByTestId('popover-section-attests')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-catechism-link')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-cites')).toHaveCount(0);
});

test('CHROME-1: a verse popover offers read in context, save and close, and no book chip (OPEN 4b)', async ({ page }) => {
  // Arrange
  await page.goto('/read/GEN/1');

  // Act
  await openVerse(page, 3);

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.3');
  await expect(page.getByTestId('popover-chip-context')).toBeVisible();
  await expect(page.getByTestId('popover-save-exploration')).toBeVisible();
  await expect(page.getByTestId('popover-close')).toBeVisible();
  await expect(page.getByTestId('popover-chip-book')).toHaveCount(0);
});

// The owner's own named candidate ("Read in context") -- traced AND
// live-exercised (not assumed) as genuinely functional: it re-scrolls the
// Reader to the clicked verse, even without leaving the current page.
test('CHROME-1 trace: "Read in context" genuinely scrolls the reader to the clicked verse, even from the same page', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await openVerse(page, 20);
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.20');

  await page.evaluate(() => window.scrollTo(0, 0));
  await expect(async () => {
    expect(await page.evaluate(() => window.scrollY)).toBe(0);
  }).toPass({ timeout: 3000 });

  await page.getByTestId('popover-chip-context').click();

  await expect(async () => {
    const y = await page.evaluate(() => window.scrollY);
    expect(y).toBeGreaterThan(500); // genuinely scrolled back down to GEN.1.20
  }).toPass({ timeout: 3000 });
  await expect(page.getByTestId('verse-line-20')).toBeInViewport();
});

test('CHROME-2: popover-close and popover-breadcrumb-back both show a native tooltip naming the button', async ({ page }) => {
  // Arrange
  const chapter = neighbourNode((await api.nodeEdges('text-unit:GEN.1.3', 'member-of')).entries[0]);
  await page.goto('/read/GEN/1');
  await openVerse(page, 3);
  await expect(page.getByTestId('popover-close')).toHaveAttribute('title', 'Close');

  // Act
  await page.getByTestId(`popover-up-member-of-${chapter.id}`).click();

  // Assert
  await expect(page.getByTestId('popover-breadcrumb-back')).toHaveAttribute('title', 'Back');
});

// HATCH-DELIVERABLE-1 (from the CHROME-1b safety-valve mystery this batch's
// own dispatch names: "the owner's §4e law applied to hatches" -- an
// affordance whose target has genuinely nothing to show is worse than no
// affordance at all): "popover-chip-map" renders ONLY when the map scene it
// would open has real content -- for EventNodes, at least one located place
// witness (EventDetail.Places) in the event's own window. theo-1 ("Creation
// of all things," GEN.1.1's own primary heading anchor) is the real,
// already-CHROME-1b-traced no-located-places fixture (`GET /api/event/theo-1`
// -> `"places":[]` -- a cosmic, non-geographic event; CHROME-1b's own live
// trace confirmed the resulting /world scene rendered zero lit markers,
// server-truthfully, not a client bug) -- the chip must not tease it.
// ab_haran ("Sojourn in Haran; the call of Abram," GEN.11.31's own primary
// heading anchor, is_continuation:false) is the real, well-located sibling
// (`"places":[{"id":"haran","name":"Haran"}]`) -- the chip must still show.
test('HATCH-DELIVERABLE-1: EventNode\'s map chip is absent for a no-located-places event (Creation) and present for a located one (Haran)', async ({ page }) => {
  await page.goto('/read/GEN/1');
  const creationHeading = page.getByTestId('pericope-heading-theo-1');
  await expect(creationHeading).toBeVisible();
  await creationHeading.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Creation of all things');
  await expect(page.getByTestId('popover-chip-map')).toHaveCount(0);
  // The empty-scene tease is dead: not merely absent from chrome, but there
  // is no map-navigating control anywhere in this popover to click at all
  // (the chip's own accessible name, not its visible glyph -- ExplorerPopover
  // renders every chip as a bare crosshair icon, "Show on the map" lives in
  // its title/aria-label only).
  await expect(page.getByRole('button', { name: 'Show on the map' })).toHaveCount(0);
  await page.getByTestId('popover-close').click();

  await page.goto('/read/GEN/11');
  const haranHeading = page.getByTestId('pericope-heading-ab_haran');
  await expect(haranHeading).toBeVisible();
  await haranHeading.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Sojourn in Haran; the call of Abram');
  await expect(page.getByTestId('popover-chip-map')).toBeVisible();
  await expect(page.getByTestId('popover-chip-map')).toHaveAttribute('aria-label', 'Show on the map');

  // The chip genuinely works, end to end -- not just present -- for the
  // located case (CHROME-1b's own live trace already proved this for a
  // DIFFERENT located event; re-proven here for the SAME fixture this test
  // pins as "must show").
  await page.getByTestId('popover-chip-map').click();
  await expect(page).toHaveURL(/\/world\?from=-2092&to=-2091/);
  await expect(async () => {
    expect(await page.getByTestId(/^marker-/).count()).toBeGreaterThan(0);
  }).toPass({ timeout: 5000 });
});

// ---------------------------------------------------------------------
// READER-1: expand -> lazy chapter fetch -> scrollable mini-reader ->
// focal verse visible + highlighted; collapse restores the compact view.
// ---------------------------------------------------------------------

test('READER-1: a verse reached from another chapter is read in its chapter through its read-in-context hatch', async ({ page }) => {
  // Arrange
  await page.goto('/read/GEN/1');
  await openVerse(page, 3);
  await page.getByTestId('popover-link-cites-text-unit:2CO.4.6').click();
  await expect(page.getByTestId('popover-title')).toHaveText('2CO.4.6');

  // Act
  await page.getByTestId('popover-chip-context').click();

  // Assert
  await expect(page).toHaveURL(/\/read\/2CO\/4/);
  await expect(page.getByTestId('verse-line-6')).toBeInViewport();
});

test('READER-1: a passage\'s whole focal range is highlighted when expanded', async ({ page }) => {
  // Arrange
  await page.goto('/read/JER/39');
  await openVerse(page, 1);
  await page.getByTestId('popover-link-attests-Event:exl_jerusalem').click();
  const account = page.getByTestId('event-witness-2KI.25.1-10');
  await expect(account).toBeVisible();
  await account.locator('.popover-passage-ref-label').click();
  await expect(page.getByTestId('popover-title')).toHaveText('2KI.25.1-10');

  // Act
  await page.getByTestId('popover-verse-expand').click();

  // Assert
  await expect(page.getByTestId('popover-verse-reader')).toBeVisible();
  for (const n of [1, 9, 10]) {
    await expect(page.getByTestId(`popover-reader-verse-${n}`)).toHaveAttribute('data-focal', 'true');
  }
  await expect(page.getByTestId('popover-reader-verse-11')).toHaveAttribute('data-focal', 'false');
});

async function openJerusalem(page: Page): Promise<any> {
  const record = await api.node('Place:jerusalem');
  await page.goto('/world?from=-1000&to=-900');
  const marker = page.getByTestId('marker-jerusalem').or(page.getByTestId('quiet-marker-jerusalem'));
  await expect(marker).toBeAttached();
  await marker.dispatchEvent('click');
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  return record;
}

test('REGISTRY-1: a PLACE popover shows its window-free record, then its site-of neighbours, capped and revealable', async ({ page }) => {
  // Arrange
  const record = await openJerusalem(page);
  const siteOf = record.edge_summary.find((s: any) => s.kind === 'site-of').count;
  const firstPage = await api.nodeEdges(record.id, 'site-of', { limit: 20 });
  const cap = 20;

  // Act
  const sectionIds = await popoverSectionsHolding(page, ['popover-section-card', 'popover-section-site-of'], /^popover-section-(card|site-of|mentioned-in)$/);

  // Assert
  expect(siteOf).toBeGreaterThan(cap);
  expect(sectionIds.indexOf('popover-section-card')).toBeLessThan(sectionIds.indexOf('popover-section-site-of'));
  await expect(page.getByTestId('popover-card-title')).toHaveText(record.label);
  const fields = await page.getByTestId(/^popover-field-/).evaluateAll(els => els.map(el => el.getAttribute('data-testid')));
  expect(fields).toEqual(['popover-field-Established', 'popover-field-Destroyed', 'popover-field-Provenance']);
  await expect(page.getByTestId('popover-section-site-of-heading')).toHaveText(`Site of (${siteOf})`);
  const links = page.getByTestId('popover-section-site-of').locator('[data-testid^="popover-link-site-of-"]');
  await expect(links).toHaveCount(cap);
  await expect(links.first()).toHaveText(neighbourNode(firstPage.entries[0]).label);
  await page.getByTestId('popover-section-site-of-more').click();
  await expect.poll(() => links.count()).toBeGreaterThan(cap);
  await expect(page.getByTestId('popover-section-site-of-collapse')).toBeVisible();
});

test('REGISTRY-1/XREF-1: a PLACE popover\'s established/destroyed dates are plain record fields carrying the served labels', async ({ page }) => {
  // Arrange
  const record = await openJerusalem(page);

  // Act
  const established = page.getByTestId('popover-field-Established');
  const destroyed = page.getByTestId('popover-field-Destroyed');

  // Assert
  await expect(established).toBeVisible();
  await expect(established).not.toHaveJSProperty('tagName', 'BUTTON');
  await expect(established.locator('dd')).toHaveText(record.place.established.label);
  await expect(destroyed.locator('dd')).toHaveText(record.place.destroyed.label);
  await expect(page.getByTestId('popover').locator('button', { hasText: record.place.established.label })).toHaveCount(0);
});

// ---------------------------------------------------------------------
// CHAPTER-CARD-1 (M-D3 fix round, R-D2/review I1): ChapterCardSection --
// U4/B3's own metadata-and-context card, owner verbatim (progress.md):
// "when you're reading a chapter, you're in its focus. you can focus
// further by clicking chapter heading and you get metadata and context...
// container title, position in book, edge summary -- what the graph knows
// ABOUT the chapter" -- NEVER the chapter's own verse text (B3, the
// standing "first verse" bug). This section had zero direct Playwright
// content assertions before this fix round -- every `chapter-card-*`
// testid lived only in CONTRACT.md prose. JOS.6 is this file's own real,
// curated exemplar: exactly one heading container ("The walls of Jericho
// fall", cq_jericho) and exactly one attested place (Jericho) for the
// whole chapter -- small enough to assert precisely, real enough to prove
// the card's own data plumbing, not a tautology.
// ---------------------------------------------------------------------

test('CHAP-HOVER-1: quick pass over chapter-head produces nothing (the tickle test); dwell produces a transient, non-interactive peek with real position/heading content and no close button; pointer-leave dismisses it; click opens the real, sticky card -- real position/verse-count/headings/places, never the chapter\'s own first verse or its text', async ({ page }) => {
  const toc = await loadToc();
  const jos = toc.find((b: any) => b.code === 'JOS');
  const totalChapters = jos.chapters.length;
  const chapterOut = await api.chapter('JOS.6');
  const verse1Text = chapterOut.verses.find((v: any) => v.verse === 1).text;

  await page.goto('/read/JOS/6');
  const head = page.getByTestId('chapter-head');
  const peek = page.getByTestId('chapter-head-peek');

  // The tickle test (owner defect report, 2026-08-24, verbatim: "if i so
  // much as tickle the chapter button. that is awful."): a quick,
  // un-lingering hover must produce NOTHING -- no popover, no peek, no
  // backdrop, no state change of any kind.
  await head.hover({ force: true });
  await expect(peek).toHaveCount(0);
  await expect(page.getByTestId('popover')).toHaveCount(0);
  await page.mouse.move(2, 2);
  await expect(peek).toHaveCount(0);

  // A genuine DWELL: hover again and wait comfortably past
  // DwellTiming.PeekDelayMs (375ms, client/DwellTiming.cs) -- a transient,
  // real-content peek appears, WITHOUT a backdrop and WITHOUT the real
  // ExplorerPopover/ChapterCardSection card at all.
  await head.hover({ force: true });
  await expect(peek).toBeVisible({ timeout: 2000 });
  await expect(page.getByTestId('popover')).toHaveCount(0); // never the real card, even mid-dwell
  await expect(page.getByTestId('chapter-head-peek-position')).toHaveText(`Chapter 6 of ${totalChapters}`);
  await expect(page.getByTestId('chapter-head-peek-heading-cq_jericho')).toHaveText('The walls of Jericho fall');
  await expect(peek.getByTestId('popover-close')).toHaveCount(0); // NO x needed on the peek
  const peekText = (await peek.textContent()) ?? '';
  expect(peekText, 'never the chapter\'s own verse text, even in the peek').not.toContain(verse1Text);

  // Pointer-leave dismisses it immediately, no grace period, no x needed.
  await page.mouse.move(2, 2);
  await expect(peek).toHaveCount(0);

  // CLICK commits: the real, sticky, x-dismissable card -- unchanged
  // content from B3-CARD's own original spec.
  await head.click();
  await expect(page.getByTestId('popover')).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText('JOS.6');
  await expect(peek).toHaveCount(0); // the click supersedes any lingering peek -- never both at once

  // Real content, read straight off the wire -- not just "a popover opened."
  await expect(page.getByTestId('chapter-card-position')).toHaveText(`Chapter 6 of ${totalChapters}`);
  await expect(page.getByTestId('chapter-card-verse-count')).toHaveText(`${chapterOut.verses.length} verses.`);
  await expect(page.getByTestId('chapter-card-headings-heading')).toHaveText('CONTAINERS IN THIS CHAPTER');
  await expect(page.getByTestId('chapter-card-heading-cq_jericho')).toHaveText('The walls of Jericho fall');
  await expect(page.getByTestId('chapter-card-places-heading')).toHaveText('PLACES MENTIONED');
  await expect(page.getByTestId('chapter-card-place-jericho-1')).toHaveText('Jericho');

  // B3's own standing bug, concretely disproven: the chapter's own first
  // verse (or any verse text at all) never appears -- neither as a
  // dedicated reader testid nor as literal text anywhere in the popover.
  await expect(page.getByTestId(/^popover-reader-verse-/)).toHaveCount(0);
  await expect(page.getByTestId('popover-verse-text')).toHaveCount(0);
  const popoverText = (await page.getByTestId('popover').textContent()) ?? '';
  expect(popoverText).not.toContain(verse1Text);

  // Both entry points are conditionally explorable -- proves this card is
  // real outward-connection content, not a dead-end summary.
  await page.getByTestId('chapter-card-heading-cq_jericho').click();
  await expect(page.getByTestId('popover-title')).toHaveText('The walls of Jericho fall');
});

test('PLACE-ONE-VIEW: a place on the chapter card and a place mentioned in the text both open its served record', async ({ page }) => {
  const jericho = await api.node('Place:jericho-1');

  await page.goto('/read/JOS/6');
  await page.getByTestId('chapter-head').click();
  await page.getByTestId('chapter-card-place-jericho-1').click();
  await expect(page.getByTestId('popover-card-title')).toHaveText(jericho.label);

  await page.goto('/read/JOS/6');
  await page.getByTestId('verse-mention-1-jericho-1').click();
  await expect(page.getByTestId('popover-card-title')).toHaveText(jericho.label);
});

test('CHAPTER-CARD-1: clicking chapter-head opens the real card directly -- CHAP-HOVER-1 respec\'s "click is the only entry point" (hover no longer opens it at all, even quietly -- see that note)', async ({ page }) => {
  await page.goto('/read/JOS/6');
  await page.getByTestId('chapter-head').click();
  await expect(page.getByTestId('popover-title')).toHaveText('JOS.6');
  await expect(page.getByTestId('chapter-card-position')).toBeVisible();
  await expect(page.getByTestId('chapter-card-heading-cq_jericho')).toHaveText('The walls of Jericho fall');
  await expect(page.getByTestId('chapter-card-place-jericho-1')).toHaveText('Jericho');
  await expect(page.getByTestId(/^popover-reader-verse-/)).toHaveCount(0);

  // A genuine click persists (no auto-dismiss at all, ever, for this
  // card -- CHAP-HOVER-1: there is no more hover-opened/upgraded-to-
  // persistent state for chapter-head; clicking always means the real,
  // backdrop-shown, sticky popover from the start).
  await page.mouse.move(2, 2);
  await page.waitForTimeout(1200);
  await expect(page.getByTestId('popover')).toBeVisible();
});

// The fix round's own live-caught bug, now covered directly: PSA.119's real
// 22 acrostic-stanza heading containers (confirmed against the compiled
// data) made this card tall enough to self-overlap chapter-head before the
// cap existed. No places are attested anywhere in PSA.119 (real, confirmed
// data) -- this test exercises the headings cap specifically, the one the
// live bug actually turned on; the places cap shares the identical
// ListCap/ChapterCardSection code path, not independently re-proven here.
test('CHAPTER-CARD-1: the 8-row containers cap renders an honest "+N more" line on a many-container chapter (PSA.119, 22 real acrostic sections)', async ({ page }) => {
  const chapterOut = await api.chapter('PSA.119');
  const distinctHeadings = new Set(chapterOut.verses.filter((v: any) => v.heading).map((v: any) => v.heading.event_id));
  const cap = 8;
  expect(distinctHeadings.size, 'PSA.119 must still real-carry MORE than the cap for this assertion to exercise it').toBeGreaterThan(cap);

  await page.goto('/read/PSA/119');
  await page.getByTestId('chapter-head').click();
  await expect(page.getByTestId('popover-title')).toHaveText('PSA.119');

  await expect(page.locator('[data-testid^="chapter-card-heading-"]')).toHaveCount(cap);
  const moreLine = page.getByTestId('chapter-card-headings-more');
  await expect(moreLine).toBeVisible();
  await expect(moreLine).toHaveText(`+ ${distinctHeadings.size - cap} more containers in this chapter.`);

  // The exact bug this cap fixes: chapter-head itself must still be
  // reachable and clickable with the card open -- proving the card no
  // longer covers its own trigger the way the uncapped version did.
  await expect(page.getByTestId('chapter-head')).toBeVisible();
});

// ---------------------------------------------------------------------
// BLINK-1: hovering a place mention inside the mini-reader blinks the
// SAME place's own live marker.
// ---------------------------------------------------------------------

async function openBethelAccountReader(page: Page): Promise<Locator> {
  const detail = await api.event('jj_bethel_dream');
  await openVerse(page, 8);
  await page.getByTestId('popover-link-cites-text-unit:GEN.28.19').click();
  await page.getByTestId('popover-link-attests-Event:jj_bethel_dream').click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);
  const entry = page.getByTestId('popover-section-event-witness').locator('[data-testid^="event-witness-"]');
  const entryTestId = await entry.getAttribute('data-testid');
  await entry.getByTestId(`popover-verse-expand-${entryTestId}`).click();
  return entry.getByTestId(`popover-reader-mention-1-canaan-${entryTestId}`);
}

test('BLINK-1: hovering a place mention in an account\'s mini-reader blinks its map marker; leaving unblinks it', async ({ page }) => {
  // Arrange
  await page.goto('/read/GEN/12?split=world&follow=1');
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'true');
  const marker = page.getByTestId('marker-canaan').or(page.getByTestId('quiet-marker-canaan'));
  await expect(marker).toBeAttached({ timeout: 15000 });
  const mention = await openBethelAccountReader(page);
  await expect(mention).toBeVisible();
  await expect(marker).not.toHaveClass(/atlas-blink/);

  // Act
  await mention.hover();

  // Assert
  await expect(marker).toHaveClass(/atlas-blink/);
  await page.mouse.move(2, 2);
  await expect(marker).not.toHaveClass(/atlas-blink/);
});

// prefers-reduced-motion: the pulse animation itself is disabled -- a
// class/state assertion (computed animation-name), not pixel timing, per
// the batch report's own testing approach.
test('BLINK-1: prefers-reduced-motion disables the pulse animation on .atlas-blink', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/world?from=-1000&to=-900');
  const marker = page.getByTestId('marker-jerusalem').or(page.getByTestId('quiet-marker-jerusalem'));
  await expect(marker).toBeAttached();
  await marker.evaluate(el => el.classList.add('atlas-blink'));
  const animationName = await marker.evaluate(el => getComputedStyle(el).animationName);
  expect(animationName).toBe('none');
});

// ---------------------------------------------------------------------
// MENTION (M-D3/U5, "in-text mentions-attested links"): PlaceMentions.Scan
// widened to a second entity kind (Explore/PlaceMentions.cs) and wired into
// Reader.razor's own PRIMARY verse text (previously plain @v.Text, no
// scanning at all -- only the nested mini-reader, BLINK-1 above, ever did
// this) -- see CONTRACT.md's own MENTION note for the full behavior.
// ---------------------------------------------------------------------

test('MENTION-1: clicking a place mention in the reader\'s own verse text opens that place, not the verse underneath it', async ({ page }) => {
  // GEN.28.1's real text ("...a wife of the daughters of Canaan") carries
  // one clean, punctuation-free "Canaan" -- attested as BOTH a Place and a
  // (unrelated, Genesis 9-10) Person for this same verse; PlaceMentions'
  // own stable-sort tie-break resolves the ambiguity to the Place, which is
  // also the linguistically correct reading here (see that class's own doc
  // comment for the full disclosed story).
  await page.goto('/read/GEN/28');
  const mention = page.getByTestId('verse-mention-1-canaan');
  await expect(mention).toBeVisible();
  await expect(mention).toHaveText('Canaan');

  await mention.click();
  // @onclick:stopPropagation on the mention span is what's under test here
  // -- without it, this click would ALSO bubble into the verse-line's own
  // handler and open a VerseNode (popover-title "GEN.28.1") instead.
  await expect(page.getByTestId('popover-title')).toHaveText('Canaan');
});

test('MENTION-2: clicking a person mention in the reader\'s own verse text opens that person', async ({ page }) => {
  // EXO.4.14's real text names Aaron and Moses both cleanly (this file's
  // own graph_api.rs sibling, chapter_verse_persons_is_always_present_and_
  // matches_the_generic_mentions_frontier, pins the same verse server-side).
  await page.goto('/read/EXO/4');
  const aaron = page.getByTestId('verse-mention-person-14-aaron_1');
  await expect(aaron).toBeVisible();
  await expect(aaron).toHaveText('Aaron');

  await aaron.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Aaron');
});

test('MENTION-3: a common word is never linked just because it collides with an attested place name under a different case', async ({ page }) => {
  // The positive half of the guard: EXO.16.1 genuinely attests the place
  // "Sin" (a real wilderness name, capitalized, standing alone in real KJV
  // prose) -- confirmed linked.
  await page.goto('/read/EXO/16');
  const placeMention = page.getByTestId('verse-mention-1-sin');
  await expect(placeMention).toBeVisible();
  await expect(placeMention).toHaveText('Sin');

  // The negative half: GEN.4.7 ("...sin lieth at the door...") uses the
  // common noun, lowercase, and attests NEITHER the place Sin NOR any
  // person for this verse (VerseOut.Places/Persons both empty, confirmed
  // against the real compiled data) -- were the old case-INSENSITIVE
  // search still in place, "sin" here would have matched "Sin" and wrongly
  // linked. No mention span of any kind renders in this verse's text.
  await page.goto('/read/GEN/4');
  const verseLine = page.getByTestId('verse-line-7');
  await expect(verseLine).toContainText('sin lieth at the door');
  await expect(verseLine.locator('.verse-mention')).toHaveCount(0);
});

test('MENTION-4: clicking a place mention inside an account\'s mini-reader pushes a new popover level', async ({ page }) => {
  // Arrange
  await page.goto('/read/GEN/12?split=world');
  const mention = await openBethelAccountReader(page);

  // Act
  await mention.click();

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText('Canaan');
});

test('R-M1: a mentioned name in an account\'s compact preview links exactly the way the same mention links in the main reader', async ({ page }) => {
  // Arrange
  const detail = await api.event('gen_table_of_nations');
  const shem = await api.node('Person:shem_2613');
  await page.goto('/read/GEN/10');
  await openVerse(page, 1);
  await page.getByTestId('popover-link-attests-Event:gen_table_of_nations').click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);
  const entry = page.locator('[data-testid^="event-witness-"]').first();
  const entryTestId = await entry.getAttribute('data-testid');
  const previewMention = entry.getByTestId(`popover-passage-mention-person-18-shem_2613-${entryTestId}`);
  await expect(previewMention).toHaveText(shem.label);

  // Act
  await previewMention.click();

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(shem.label);
  await page.getByTestId('popover-close').click();
  await page.goto('/read/GEN/9');
  const mainMention = page.getByTestId('verse-mention-person-18-shem_2613');
  await expect(mainMention).toHaveText(shem.label);
  await mainMention.click();
  await expect(page.getByTestId('popover-title')).toHaveText(shem.label);
});

// ---------------------------------------------------------------------
// CATECH-1 (batch-f-brief.md, "the small catechism" -- user direction,
// asked three separate times): the verse->item->proof-verse hop, Luther's
// own verbatim explanation heading, and passage aggregation. All against
// REAL curated data/curated/catechism.toml content (MAT.28.19, "Baptism —
// Part One," is Luther's own institution-of-Baptism proof text --
// the exact "Baptism institution verse" example the batch brief itself
// names) -- not the demo fixture (that's atlas-server's own Rust-level
// integration test; this suite runs against the real compiled dataset).
// ---------------------------------------------------------------------

test('CATECH-1: the Baptism institution verse lists every catechism item that cites it, Baptism Part One among them', async ({ page }) => {
  // Arrange
  const items = (await api.nodeEdges('text-unit:MAT.28.19', 'catechism-link')).entries.map(neighbourNode);
  await page.goto('/read/MAT/28');

  // Act
  await openVerse(page, 19);

  // Assert
  const section = page.getByTestId('popover-section-catechism-link');
  await expect(section.getByTestId('popover-section-catechism-link-heading')).toHaveText(`Catechism link (${items.length})`);
  await expect(section.getByTestId(/^popover-link-catechism-link-/)).toHaveText(items.map(item => item.label));
  await expect(page.getByTestId('popover-link-catechism-link-CatechismItem:baptism-1')).toHaveText('Baptism — Part One');
});

test('CATECH-1: verse -> catechism item -> proof verse hop, with Luther\'s own verbatim heading', async ({ page }) => {
  // Arrange
  await page.goto('/read/MAT/28');
  await openVerse(page, 19);

  // Act
  await page.getByTestId('popover-link-catechism-link-CatechismItem:baptism-1').click();

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText('Baptism — Part One');
  const explanationSection = page.getByTestId('popover-section-catechism-explanation');
  await expect(explanationSection.getByTestId('catechism-section-heading')).toHaveText('What is Baptism?');
  await expect(explanationSection).toContainText('Baptism is not simple water only');
  await expect(page.getByTestId('popover-section-catechism-text')).toHaveCount(0);
  const whereWritten = page.getByTestId('popover-section-catechism-where-written');
  await expect(whereWritten.getByTestId('catechism-section-heading')).toHaveText('Where is this written?');
  await expect(whereWritten).toContainText('Go ye into all the world');
  const scriptures = page.getByTestId('popover-section-catechism-scriptures');
  await expect(scriptures.getByTestId('catechism-section-heading')).toHaveText('THE SCRIPTURES');
  const proofVerse = page.getByTestId('catechism-verse-MAT.28.19');
  await expect(proofVerse).toContainText('MAT.28.19');
  await expect(proofVerse).toContainText('baptizing them in the name of the Father');
  await expect(page.locator('.popover-head-actions')).toHaveCount(0);
  await proofVerse.click();
  await expect(page.getByTestId('popover-title')).toHaveText('MAT.28.19');
  await expect(page.getByTestId('popover-text')).toBeVisible();
  await expect(page.getByTestId('popover-section-catechism-link')).toBeVisible();
});

test('CATECH-1: a same-chapter passage selection aggregates catechism citations across member verses (item-level dedup)', async ({ page }) => {
  // MAT.26.26-28: all three verses cite the SAME item-level citation
  // (altar-1's own `verses`, the Sacrament of the Altar's institution
  // words, Luther's own embedded citation) -- the passage's own section
  // must list that BARE hit exactly ONCE (union+dedup), not three times,
  // even though (Batch F2) the repo mapping now ALSO cites altar-1 (and
  // several other items) via multiple separate QUESTIONS across this same
  // span -- dedup is by (item, question) pair, so those are each their own
  // row, additional to (not replacing) the bare dedup this test pins.
  await page.goto('/read/MAT/26');
  await page.getByTestId('verse-num-26').click();
  await page.keyboard.down('Shift');
  await page.getByTestId('verse-num-28').click();
  await page.keyboard.up('Shift');
  await page.getByTestId('passage-chip').click();
  await expect(page.getByTestId('popover-title')).toHaveText('MAT.26.26-28');

  await expect(page.getByTestId('popover-section-catechism')).toBeVisible();
  // M-D3/U2/U6: THE SMALL CATECHISM now defaults to 2 shown -- MAT.26.26-28
  // cites well over that (this test's own dedup subject below proves it),
  // so reveal everything first; this test's own concern is dedup, not the
  // reveal mechanic itself (XREF-1/U2's own dedicated test covers that).
  const catechismMore = page.getByTestId('catechism-more');
  if (await catechismMore.count() > 0) {
    // M-D4 fix round 1/P2: same conditional-"all" fallback as above.
    const catechismAll = page.getByTestId('catechism-more-all');
    if (await catechismAll.count() > 0) {
      await catechismAll.click();
    } else {
      await catechismMore.click();
    }
  }
  // The bare (item-level, no question) altar-1 row -- first occurrence,
  // unsuffixed testid -- appears exactly once despite being cited by all
  // three member verses.
  await expect(page.getByTestId('catechism-item-altar-1')).toHaveText('What Is the Sacrament of the Altar?');
  await expect(page.getByTestId('catechism-item-altar-1')).toHaveCount(1);

  // Batch F2: the repo mapping also cites altar-1 via several DIFFERENT
  // questions across this same span -- each is its own, separately
  // numbered-suffix row (id, question) pair, not folded into the bare hit.
  const altarRows = page.getByTestId(/^catechism-item-altar-1/);
  const altarCount = await altarRows.count();
  expect(altarCount).toBeGreaterThan(1);
  const altarTexts = await altarRows.allTextContents();
  expect(altarTexts.some(t => t.includes('What Is the Sacrament of the Altar? — '))).toBeTruthy();
});

test('CATECH-1/6-ARCH: a verse reachable only via the repo mapping links its catechism item, whose scriptures carry the question-titled passage (Luke 12:13-14)', async ({ page }) => {
  // Arrange
  const [item] = (await api.nodeEdges('text-unit:LUK.12.13', 'catechism-link')).entries.map(neighbourNode);
  await page.goto('/read/LUK/12');
  await openVerse(page, 13);

  // Act
  await page.getByTestId(`popover-link-catechism-link-${item.id}`).click();

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(item.label);
  const scriptures = page.getByTestId('popover-section-catechism-scriptures');
  const godAloneEntry = scriptures.getByTestId('catechism-verse-LUK.12.13-14');
  await expect(godAloneEntry).toContainText('God Alone as Judge');
  const title = godAloneEntry.getByTestId('catechism-verse-LUK.12.13-14-title');
  const ref = godAloneEntry.getByTestId('catechism-verse-LUK.12.13-14-ref');
  await expect(title).toHaveText('God Alone as Judge');
  await expect(ref).toHaveText('LUK.12.13-14');
  const titleBox = await title.boundingBox();
  const refBox = await ref.boundingBox();
  const firstVerseBox = await godAloneEntry.locator('.popover-passage-verse-num').first().boundingBox();
  expect(titleBox!.x).toBeLessThan(refBox!.x);
  expect(titleBox!.y).toBeLessThan(firstVerseBox!.y);
  expect(refBox!.y).toBeLessThan(firstVerseBox!.y);
  await expect(ref).toHaveClass(/\bpopover-passage-ref-label\b/);
});

test('CATECH-1: a verse whose catechism links fit in one page shows them all, with no way to page', async ({ page }) => {
  // Arrange
  const record = await findVerse(candidate => served(candidate, 'catechism-link') > 0 && served(candidate, 'catechism-link') <= PAGE_SIZE);
  test.skip(!record, 'no sampled verse had catechism links within one page');
  if (!record) return;

  // Act
  await openRecord(page, record);

  // Assert
  await expect(page.getByTestId(/^popover-link-catechism-link-/)).toHaveCount(served(record, 'catechism-link'));
  await expect(page.getByTestId('popover-section-catechism-link-more')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-catechism-link-collapse')).toHaveCount(0);
});

test('XREF-1: a verse\'s cross references page 20 at a time; More shows the next page beneath, Less goes back a page', async ({ page }) => {
  // Arrange
  const record = await findVerse(candidate => served(candidate, 'cites') > PAGE_SIZE);
  test.skip(!record, 'no sampled verse had more than a page of cross references');
  if (!record) return;
  const total = served(record, 'cites');
  await openRecord(page, record);
  const links = page.getByTestId(/^popover-link-cites-/);
  await expect(links).toHaveCount(PAGE_SIZE);
  await expect(page.getByTestId('popover-section-cites-position')).toHaveText(`1–${PAGE_SIZE} of ${total}`);
  await expect(page.getByTestId('popover-section-cites-collapse')).toHaveCount(0);

  // Act
  await page.getByTestId('popover-section-cites-more').click();

  // Assert
  await expect(links).toHaveCount(Math.min(total, 2 * PAGE_SIZE));
  await page.getByTestId('popover-section-cites-collapse').click();
  await expect(links).toHaveCount(PAGE_SIZE);
});

test('XREF-1: a verse whose cross references fit in one page shows them all, with no way to page', async ({ page }) => {
  // Arrange
  const record = await findVerse(candidate => served(candidate, 'cites') > 0 && served(candidate, 'cites') <= PAGE_SIZE);
  test.skip(!record, 'no sampled verse had cross references within one page');
  if (!record) return;

  // Act
  await openRecord(page, record);

  // Assert
  await expect(page.getByTestId(/^popover-link-cites-/)).toHaveCount(served(record, 'cites'));
  await expect(page.getByTestId('popover-section-cites-more')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-cites-collapse')).toHaveCount(0);
});

test('XREF-1/regression: a cross reference to a span opens the verse at the span\'s first verse, never a passage for the whole range', async ({ page }) => {
  // Arrange
  const entries = (await api.nodeEdges('text-unit:MRK.6.1', 'cites')).entries;
  const span = entries.find(entry => !entry.edge.label.endsWith(neighbourNode(entry).label))!;
  const first = neighbourNode(span);
  await page.goto('/read/MRK/6');
  await openVerse(page, 1);

  // Act
  await page.getByTestId(`popover-link-cites-${first.id}`).click();

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(first.label);
  await expect(page.getByTestId('popover-text')).toBeVisible();
  await expect(page.getByTestId('popover-section-verse-text')).toHaveCount(0);
});

test('XREF-CLAMP-1: a cross reference to a span (MRK.6.1 -> LUK.4.16-30) steps onto its edge, which names the whole span it cites (F-65)', async ({ page }) => {
  // Arrange
  const entries = (await api.nodeEdges('text-unit:MRK.6.1', 'cites')).entries;
  const span = entries.find(entry => entry.edge.label.endsWith('LUK.4.16-30'))!;
  await page.goto('/read/MRK/6');
  await openVerse(page, 1);
  const step = page.getByTestId(`popover-entry-edge-cites-${span.edge.id}`);
  await expect(step).toHaveAttribute('aria-label', span.edge.label);

  // Act
  await step.click();

  // Assert
  await expect(page.getByTestId('popover-card-title')).toHaveText(span.edge.label);
});

// ---------------------------------------------------------------------
// EVENT-1 (batch-t-brief.md, "events as the narrative nodes" -- SUPERSEDES
// batch-n-brief.md's own NARRATIVE-1 verse-level tests, retired: "rather
// than putting the next/previous event on every verse, add titles of
// events that correspond to passages... traversal lives on event nodes,"
// the owner verbatim). EXO.13.20 (the exodus narrative's own "First camp
// at Succoth" leg, `ex_succoth`) is the SAME known narrative verse Batch N
// used, still picked (not discovered) because its exact adjacency (prior:
// ex_rameses/EXO.12.37; following: ex_red_sea/EXO.14.21-31) is
// independently readable straight off data/curated/narratives/exodus.toml
// + events-extra.toml, letting these tests assert EXACT text.
//
// CHRONO-MERGE-1 (batch-chrono-merge-brief.md, owner NOD 2026-08-24):
// retires the per-narrative traversal nav these tests originally proved
// (`event-nav`, `event-{prior,following}-event-{narrativeId}`) whole --
// CONTRACT.md's own CHRONO-MERGE-1 note has the full divergence rule and
// per-test disposition. `ex_succoth`'s own narrative and timeline
// positions are byte-identical live (ground-truthed below), so the
// "clicking a verse's EVENT row..." test's own UI-rendering half
// retargets onto the Chronology block's testids with its assertions
// otherwise unchanged; its wire-level "one-graph property" half is
// untouched. The MULTI-NARRATIVE nav test, the "recursive traversal
// reaches both narrative ends" test, and the "three-narrative full walk"
// test are RETIRED (not rewritten) -- all three exercised the retired
// per-narrative UI walk specifically (a "walk this ONE narrative,
// arbitrarily far" affordance POPOVER-LAW-1's own non-redundancy
// admission deliberately removes; the MULTI-NARRATIVE test was already
// permanently `test.skip`-vacuous even before this batch -- the
// controller's own live sweep found zero of 255 real events belong to >1
// narrative). The underlying graph property they exercised (a
// narrative's own `.legs` form a real, walkable chain, conditional
// presence at genuine chain ends) stays covered server-side
// (`server/atlas-graph/tests/narrative_real_data.rs`) and, for the
// SURVIVING global-timeline walk, by `event-timeline.spec.ts`'s own
// CHRONO-1 chain-end test (theo-1/theo-385).
// ---------------------------------------------------------------------

test('EVENT-1: a verse popover lists the events it attests (not PRIOR/FOLLOWING) -- traversal lives on the EVENT node', async ({ page }) => {
  // Arrange
  const events = (await api.nodeEdges('text-unit:EXO.13.20', 'attests')).entries.map(neighbourNode);
  expect(events.map(event => event.id)).toContain('Event:ex_succoth');
  await page.goto('/read/EXO/13');

  // Act
  await openVerse(page, 20);

  // Assert
  const section = page.getByTestId('popover-section-attests');
  await expect(section.getByTestId('popover-section-attests-heading')).toHaveText(`Attests (${events.length})`);
  await expect(section.getByTestId(/^popover-link-attests-/)).toHaveText(events.map(event => event.label));
  await expect(page.getByTestId('event-nav')).toHaveCount(0);
});

test('EVENT-1: a verse with no titled event shows no attests section at all (conditional presence)', async ({ page }) => {
  // Arrange
  const record = await findVerse(candidate => served(candidate, 'attests') === 0);
  test.skip(!record, 'no sampled verse had zero events');
  if (!record) return;

  // Act
  await openRecord(page, record);

  // Assert
  await expect(page.getByTestId('popover-text')).toBeVisible();
  await expect(page.getByTestId('popover-section-attests')).toHaveCount(0);
});

test('EVENT-1: clicking a verse\'s EVENT row opens the EventNode, whose PRIOR/FOLLOWING verses equal the map arrows\' own endpoint events (one-graph property)', async ({ page }) => {
  // The one-graph proof, at the wire level, independent of the popover's
  // own rendering: ex_succoth's own event-id-keyed narrative position
  // (UNCHANGED endpoint/resolver from Batch N) must report the SAME
  // following-event verse_groups as the live map's own scene (via the SAME
  // event id an arrow's own to_event names) -- byte-for-byte.
  const positions = (await api.narrativeEventPositions('ex_succoth')).narrative;
  const position = positions.find((p: any) => p.narrative_id === 'exodus');
  expect(position.prior.id).toBe('ex_rameses');
  expect(position.prior.label).toBe('Israel departs Rameses');
  expect(position.following.id).toBe('ex_red_sea');
  expect(position.following.label).toBe('Crossing the Red Sea');

  const scene = await api.sceneTime(-1446, -1406); // EXODUS_WINDOW (world-pin.spec.ts's own established exodus window)
  const arrow = scene.arrows.find((a: any) => a.narrative === 'exodus' && a.from_event === 'ex_succoth' && a.to_event === 'ex_red_sea');
  expect(arrow, 'the ex_succoth -> ex_red_sea leg must be a real rendered arrow in this window').toBeTruthy();
  const redSeaPlace = scene.places.find((p: any) => p.events.some((e: any) => e.id === 'ex_red_sea'));
  const redSeaSceneEvent = redSeaPlace.events.find((e: any) => e.id === 'ex_red_sea');
  expect(position.following.verse_groups).toEqual(redSeaSceneEvent.verse_groups); // <- the one-graph assertion itself

  // Also confirm GET /api/event/ex_succoth (the EVENT node's own richer
  // fetch) agrees on the title/date -- a DIFFERENT wire source from the
  // narrative-position lookup above, both describing the same event.
  const eventDetail = await api.event('ex_succoth');
  expect(eventDetail.title).toBe('First camp at Succoth');

  // Now the SAME thing, live: verse -> EVENT row -> EventNode -> PRIOR/FOLLOWING.
  await page.goto('/read/EXO/13');
  await openVerse(page, 20);
  await page.getByTestId('popover-link-attests-Event:ex_succoth').click();
  await expect(page.getByTestId('popover-title')).toHaveText('First camp at Succoth');
  await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();

  // CHRONO-MERGE-1: ex_succoth's own narrative and timeline positions are
  // byte-identical (confirmed above, live) -- there is nothing
  // non-redundant for a story-thread line to add, so the Chronology
  // block's own GLOBAL arrows are what carries this traversal now (the
  // narrative-scoped nav this paragraph used to check -- `event-nav`,
  // `event-prior-event-exodus`, etc. -- is retired whole).
  await expect(page.getByTestId('event-nav')).toHaveCount(0);
  await expect(page.getByTestId('event-story-thread')).toHaveCount(0);
  const chronoSection = page.getByTestId('popover-section-event-chronology');
  await expect(chronoSection).toBeVisible();

  const priorBtn = page.getByTestId('event-chrono-prior-event-global');
  // .popover-event-nav-label -- the button's own full text also includes
  // its decorative directional glyph (a sibling span).
  await expect(priorBtn.locator('.popover-event-nav-label')).toHaveText('Israel departs Rameses');
  // M-D4 fix round 1/P4 (owner, verbatim: "we straight up should not have
  // [the verse text]. you get that when you traverse."): no verse content,
  // no attestation text in the affordance at all, the click is what
  // YIELDS the event, not what the button previews. In its place: a
  // static small-caps role caption naming the DIRECTION only, plus a
  // `title` on the (possibly ellipsis-truncated) name itself carrying the
  // untruncated event name for a native hover tooltip.
  await expect(page.getByTestId('event-chrono-prior-label-global')).toHaveText('PRIOR EVENT');
  await expect(priorBtn.locator('.popover-event-nav-label')).toHaveAttribute('title', 'Israel departs Rameses');

  const followingBtn = page.getByTestId('event-chrono-following-event-global');
  await expect(followingBtn.locator('.popover-event-nav-label')).toHaveText('Crossing the Red Sea');
  // The FOLLOWING event's own real verse groups span EXO.14.21-31 -- already
  // proven at the wire level above (position.following.verse_groups ==
  // redSeaSceneEvent.verse_groups, the test's own "one-graph" assertion);
  // P4 means that span is never echoed into the UI arrow itself anymore.
  await expect(page.getByTestId('event-chrono-following-label-global')).toHaveText('FOLLOWING EVENT');
  await expect(followingBtn.locator('.popover-event-nav-label')).toHaveAttribute('title', 'Crossing the Red Sea');
});

for (const { vref, eventId } of [
  { vref: 'PSA.119.105', eventId: 'Event:psa_119_nun' },
  { vref: 'GAL.1.8', eventId: 'Event:gal_no_other_gospel' },
]) {
  test(`PERI-1: ${vref}'s general-kind pericope is listed in its one attests group, with no PASSAGE group (OPEN 3)`, async ({ page }) => {
    // Arrange
    const record: VerseRecord = await api.node(`text-unit:${vref}`);
    const pericope = (await api.nodeEdges(record.id, 'attests')).entries.map(neighbourNode).find(event => event.id === eventId)!;

    // Act
    await openRecord(page, record);

    // Assert
    await expect(page.getByTestId('popover-section-attests').getByTestId(`popover-link-attests-${eventId}`)).toHaveText(pericope.label);
    await expect(page.getByTestId('popover-section-passage-membership')).toHaveCount(0);
  });
}

// CHRONO-MERGE-1 RETIREMENT (not a rewrite -- this file's own header
// comment, above, has the full reasoning): three tests used to live here --
// "EVENT-1: MULTI-NARRATIVE nav" (already permanently `test.skip`-vacuous,
// zero of 255 real events belong to >1 narrative), "EVENT-1: recursive
// traversal reaches both narrative ends" (exodus, walked hop-by-hop via
// `event-{prior,following}-event-exodus`), and "EVENT-1: the
// three-narrative full walk" (exodus/jesus-ministry/passion-week,
// identical mechanism). All three exercised the retired per-narrative UI
// walk specifically; none has a CHRONO-MERGE-1-era replacement to rewrite
// onto, because "walk this ONE narrative, arbitrarily far" is exactly the
// affordance this batch removes (POPOVER-LAW-1's own non-redundancy
// admission -- a diverging story-thread line shows at most one hop per
// direction, never a multi-hop per-narrative walk). Recoverable from git
// history if this UI affordance is ever reinstated.

// ---------------------------------------------------------------------
// EVENT-1: PARALLEL WITNESSES (requirement 4/7 -- "Crucifixion event shows
// 4 witness passages, each clamped to 2 verses, expandable") and
// chronological-vs-reading-order (requirement 2/7 -- "a JHN event whose
// FOLLOWING is not the next pericope in JHN").
// ---------------------------------------------------------------------

test('EVENT-1/PASSAGE-1: the Crucifixion event shows 4 witness passages under "PARALLEL ACCOUNTS", each clamped to 2 verses, each independently expandable to its own whole chapter', async ({ page }) => {
  const detail = await api.event('pw_golgotha');
  expect(detail.witnesses.length).toBe(4);
  const books = detail.witnesses.map((w: any) => w.book).sort();
  expect(books).toEqual(['JHN', 'LUK', 'MAT', 'MRK']);
  // Every real witness here spans well over 2 verses -- the clamp
  // affordance must be genuinely exercised, not vacuously absent.
  for (const w of detail.witnesses) {
    const total = w.verse_groups.reduce((n: number, g: any) => n + g.verses.length, 0);
    expect(total, `${w.book}'s own witness must span >2 verses for this test to exercise the clamp`).toBeGreaterThan(2);
  }

  // Open the event via one of its own witness verses (a real navigation
  // path, not a synthetic direct-open) -- Matthew's own first verse.
  const matWitness = detail.witnesses.find((w: any) => w.book === 'MAT');
  const firstVref = matWitness.verse_groups[0].verses[0];
  const [book, chapter, verse] = firstVref.split('.');
  await page.goto(`/read/${book}/${chapter}`);
  await openVerse(page, verse);
  await page.getByTestId('popover-link-attests-Event:pw_golgotha').click();
  await expect(page.getByTestId('popover-title')).toHaveText('The crucifixion at Golgotha');

  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection).toBeVisible();
  await expect(witnessesSection.getByTestId('event-section-heading')).toHaveText('PARALLEL ACCOUNTS');

  // Query generically by testid PREFIX (never reconstruct the exact
  // "{book}.{chapter}.{from}-{to}" span string by hand in the test itself
  // -- that duplicates PassageGrouping.SpanRef's own formatting logic and
  // would make this test fragile to a harmless future change there; the
  // real assertion is "4 witness entries, each independently clamps and
  // expands," not "the span text matches this exact reconstruction").
  const entries = witnessesSection.locator('[data-testid^="event-witness-"]');
  await expect(entries).toHaveCount(4);

  // M-D4 fix round 1/P5 (owner, verbatim: "we're wasting real estate...
  // it's obvious where they're coming from already"): NO standalone
  // book-name caption renders under any of these four entries' own
  // reference headers anymore -- exactly the 4-Gospel, book-disambiguation
  // scenario WitnessUnitsResolver's own retired doc comment used to call
  // "genuinely load-bearing," now proven unnecessary: PassageList's own
  // ref-label (e.g. "MRK...") already names the book. One header per
  // account entry, the reference -- never a second line duplicating it.
  await expect(witnessesSection.locator('.popover-passage-caption')).toHaveCount(0);

  // O2 (owner live-preview correction, 2026-08-23) retired the per-entry
  // popover-passage-clamp-expand/-collapse toggle this test used to
  // exercise (see PassageList.razor's own O2 comment) -- "each clamped to 2
  // verses with independent expand/collapse" now means MiniReaderExpand's
  // own control (a RevealControls-driven arrow pair, popover-verse-expand/
  // -collapse{-ENTRY-ID}) is the SOLE remaining affordance, and it always
  // jumps straight to that witness's own WHOLE chapter -- never a partial
  // reveal of just this entry's own remaining clamped verses (a disclosed
  // simplification; see PassageList.razor's own header comment). The
  // compact `.popover-passage-verse-num` list and the expanded mini-reader's
  // own `popover-reader-verse-*` are mutually exclusive (MiniReaderExpand.razor's
  // own `@if (!_expanded)`), so "expanded" is verified the same
  // structural-swap way READER-1 already verifies it, not by counting the
  // SAME clamped list grow.
  //
  // Real, live-caught (not guessed): this event was opened via MATTHEW's
  // own first verse, so the reader is now ACTIVELY showing Matthew's own
  // chapter -- M-D3/U6's chapter-aware suppression (READER-1's own
  // established rule, `ViewStateService.MountedReaderChapter`) correctly
  // makes the MATTHEW witness entry's own popover-verse-expand UNCONDITIONALLY
  // ABSENT, the exact same way a verse-line-opened verse popover's own
  // affordance always is -- there is no book/chapter this event could be
  // opened FROM that doesn't land the reader on one of its own four
  // witnesses' books, so exactly one of the four is always structurally
  // suppressed this way; asserted explicitly below rather than avoided.
  for (let i = 0; i < 4; i++) {
    const entry = entries.nth(i);
    const entryTestId = await entry.getAttribute('data-testid');
    expect(entryTestId).toBeTruthy();

    await expect(entry.locator('[data-testid^="popover-passage-clamp-"]'), `witness ${i} carries no retired clamp-toggle testid`).toHaveCount(0);
    const versesBeforeExpand = await entry.locator('.popover-passage-verse-num').count();
    expect(versesBeforeExpand, `witness ${i} must show exactly 2 clamped verses`).toBe(2);

    if (entryTestId!.startsWith(`event-witness-${book}.`)) {
      // The reader's own current book -- the expand control is correctly,
      // structurally absent (M-D3/U6); nothing further to exercise here.
      await expect(entry.getByTestId(`popover-verse-expand-${entryTestId}`), `witness ${i} (${book}, the reader's own chapter) suppresses its expand affordance`).toHaveCount(0);
      await expect(entry.getByTestId(`popover-verse-collapse-${entryTestId}`)).toHaveCount(0);
      continue;
    }

    // Exact getByTestId, not a prefix locator -- popover-verse-expand-{id}
    // and its own always-paired popover-verse-expand-{id}-all sibling both
    // start with this same prefix (R-D3's own double-arrow button), which
    // would make a prefix locator ambiguous (strict-mode violation).
    const expandBtn = entry.getByTestId(`popover-verse-expand-${entryTestId}`);
    await expect(expandBtn, `witness ${i}'s own expand affordance`).toBeVisible();
    await expect(entry.locator('[data-testid^="popover-reader-verse-"]')).toHaveCount(0);

    await expandBtn.click();
    const collapseBtn = entry.getByTestId(`popover-verse-collapse-${entryTestId}`);
    await expect(collapseBtn).toBeVisible();
    await expect(entry.locator('.popover-passage-verse-num')).toHaveCount(0); // compact clamp view torn down
    const readerVerses = await entry.locator('[data-testid^="popover-reader-verse-"]').count();
    expect(readerVerses, `witness ${i} must show its own WHOLE chapter once expanded`).toBeGreaterThan(2);

    await collapseBtn.click();
    await expect(entry.locator('.popover-passage-verse-num')).toHaveCount(2); // restored, compact clamp
    await expect(entry.locator('[data-testid^="popover-reader-verse-"]')).toHaveCount(0);
  }
});

// ---------------------------------------------------------------------
// ACCT-COALESCE-1 (owner bug report, verbatim: "in parallel accounts
// (sermon on the mount in particular), accounts from the same book +
// chapter are listed. makes no sense."): an event witness (one curated
// `[[witness]]` row) is ONE account even when its own VerseGroups span
// multiple chapters (a storage-syntax artifact, per event-witnesses.toml's
// own Sermon comment: "written as 3 same-chapter ranges... not spanning a
// chapter boundary in one string") -- PassageBlockBuilder used to render
// each chapter as its own separate "PARALLEL ACCOUNTS" entry. Fixed:
// PassageSourceUnit.CoalesceAcrossChapters (Explore/PassageBlock.cs),
// set true by WitnessUnitsResolver. See client.Tests/AcctCoalesceTests.cs
// for the pure-logic proof; these are the real end-to-end wire-through-DOM
// confirmations against the real compiled data.
// ---------------------------------------------------------------------

test('ACCT-COALESCE-1: the Sermon on the Mount shows exactly 2 PARALLEL ACCOUNTS (MAT.5.1-7.29 coalesced, LUK.6.17-49) -- never 3+ same-book entries split per chapter', async ({ page }) => {
  const detail = await api.event('rob_sermon_on_the_mount');
  expect(detail.witnesses.length, 'ground truth: exactly 2 witness ROWS (MAT, LUK) for this fixture to mean anything').toBe(2);
  const matWitness = detail.witnesses.find((w: any) => w.book === 'MAT');
  expect(matWitness.verse_groups.length, 'the MAT witness must genuinely span 3 chapters on the wire (the bug\'s own real shape) for this test to mean anything').toBe(3);
  expect(matWitness.verse_groups.map((g: any) => g.chapter)).toEqual([5, 6, 7]);

  await page.goto('/read/LUK/6');
  await openVerse(page, 17);
  await page.getByTestId('popover-link-attests-Event:rob_sermon_on_the_mount').click();
  await expect(page.getByTestId('popover-title')).toHaveText('The Sermon on the Mount');

  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection).toBeVisible();
  await expect(witnessesSection.getByTestId('event-section-heading')).toHaveText('PARALLEL ACCOUNTS');

  // Exactly TWO account entries -- one per WITNESS ROW, not one per
  // chapter (which would be 4: MAT.5, MAT.6, MAT.7, LUK.6).
  const entries = witnessesSection.locator('[data-testid^="event-witness-"]');
  await expect(entries).toHaveCount(2);

  // The MAT account's own ref-label reads the FULL coalesced span,
  // crossing all three chapters in one entry.
  await expect(witnessesSection.getByTestId('event-witness-MAT.5.1-7.29')).toBeVisible();
  await expect(witnessesSection.getByTestId('event-witness-LUK.6.17-49')).toBeVisible();

  // The coalesced MAT entry's own expand affordance still opens its FIRST
  // chapter (MAT 5) in full -- MiniReaderExpand's own one-chapter-at-a-time
  // limit, honestly degraded (PassageList.razor's own FocalToOf comment).
  const matEntry = witnessesSection.getByTestId('event-witness-MAT.5.1-7.29');
  const matExpand = matEntry.getByTestId('popover-verse-expand-event-witness-MAT.5.1-7.29');
  await expect(matExpand).toBeVisible();
  await matExpand.click();
  const chapter5 = await api.chapter('MAT.5');
  await expect(matEntry.locator('[data-testid^="popover-reader-verse-"]')).toHaveCount(chapter5.verses.length);
});

test('ACCT-COALESCE-1 counterexample: psa_014 (Psalm 14 + Psalm 53, two SEPARATE witness rows, same book) stays TWO accounts -- genuinely non-contiguous, must never coalesce', async ({ page }) => {
  const detail = await api.event('psa_014');
  expect(detail.witnesses.length).toBe(2);
  expect(detail.witnesses.map((w: any) => w.book)).toEqual(['PSA', 'PSA']);

  await page.goto('/read/PSA/14');
  await openVerse(page, 1);
  await page.getByTestId('popover-link-attests-Event:psa_014').click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);

  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection).toBeVisible();

  // Two genuinely separate accounts -- NOT coalesced into one PSA.14.1-53.6
  // span, since they came from two SEPARATE witness rows (curation-level
  // boundary), not one continuous account split by storage syntax.
  const entries = witnessesSection.locator('[data-testid^="event-witness-"]');
  await expect(entries).toHaveCount(2);
  await expect(witnessesSection.getByTestId('event-witness-PSA.14.1-7')).toBeVisible();
  await expect(witnessesSection.getByTestId('event-witness-PSA.53.1-6')).toBeVisible();
});

test('ACCT-COALESCE-1 fix round 2 (review N-1): rob_peter_denies -- a WITHIN-witness gap renders the honest compound ref ("MRK.14.54, 66-72"), never the fabricated envelope ("MRK.14.54-61") and never fake per-range accounts', async ({ page }) => {
  // The review's own sharpest live counterexample: the MRK witness row is
  // ["MRK.14.54", "MRK.14.66-72"] (data/curated/event-witnesses.toml) --
  // ONE account with a REAL gap, because verses 55-61 belong to a
  // DIFFERENT event (rob_tried_by_caiaphas, inside the house while Peter
  // is in the courtyard below). Ground truth at the wire level first: one
  // (MRK,14) verse group, 8 delivered verses, Count=8 -- the gap is
  // GENUINELY on the wire, so this test cannot pass vacuously.
  const detail = await api.event('rob_peter_denies');
  const mrkWitness = detail.witnesses.find((w: any) => w.book === 'MRK');
  expect(mrkWitness, 'the curated MRK witness must exist').toBeTruthy();
  const mrkGroup = mrkWitness.verse_groups.find((g: any) => g.chapter === 14);
  expect(mrkGroup.count, 'the witness has 8 verses in MRK 14 (54 + 66-72)').toBe(8);
  expect(mrkGroup.verses).toContain('MRK.14.54');
  expect(mrkGroup.verses).toContain('MRK.14.66');
  expect(mrkGroup.verses, 'verse 55 belongs to ANOTHER event -- the gap is real').not.toContain('MRK.14.55');

  await page.goto('/read/MRK/14');
  await openVerse(page, 54);
  await page.getByTestId('popover-link-attests-Event:rob_peter_denies').click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);

  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection).toBeVisible();

  // ONE entry per witness ROW -- a gapped witness stays ONE account
  // (never split into fake per-range accounts)...
  const entries = witnessesSection.locator('[data-testid^="event-witness-"]');
  await expect(entries).toHaveCount(detail.witnesses.length);

  // ...whose ref is the honest COMPOUND list of its real ranges. The old
  // envelope "MRK.14.54-61" both claimed another event's verses (55-61)
  // and dropped delivered ones (62-72) -- under this project's inerrancy
  // law, the worst class of bug.
  await expect(witnessesSection.getByTestId('event-witness-MRK.14.54, 66-72')).toBeVisible();
  await expect(witnessesSection.getByTestId('event-witness-MRK.14.54-61')).toHaveCount(0);
  await expect(witnessesSection.locator('[data-testid^="event-witness-MRK.14.54-"]')).toHaveCount(0);
});

test('ACCT-COALESCE-1 fix round 3 (review NEW-1): clicking a coalesced account whose DISPLAY span is unparseable still lands a popover with its Cross References / Small Catechism sections -- never the silent section loss', async ({ page }) => {
  // NEW-1's own failure scenario, inverted into a live proof: a coalesced
  // account's display span ("MAT.5.1-7.29", "MRK.14.54, 66-72") is not a
  // shape ScriptureRef::parse accepts, and round 1/2 pushed it VERBATIM
  // as the explored PassageNode's sref -- /api/xrefs/{sref} and
  // /api/catechism/{sref} 400'd and both sections silently vanished for
  // 167 of 1212 real accounts. The fix explores by the account's own
  // FIRST contiguous same-chapter run (parser-safe by construction; the
  // display ref stays the honest compound). Ground truth first, so
  // neither half can pass vacuously:
  const compoundProbe = await api.xrefs('MAT.5.1-7.29');
  expect(compoundProbe.__status, 'the display span itself really is unparseable by the server -- the very thing that made the old behavior silent section loss').toBe(400);
  const sermonXrefs = await api.xrefs('MAT.5.1-48');
  const sermonCatechism = await api.catechism('MAT.5.1-48');
  expect(sermonXrefs.length, 'MAT.5.1-48 must have real cross-references for the section assertion to mean anything').toBeGreaterThan(0);
  expect(sermonCatechism.length, 'MAT.5.1-48 must have real catechism citations for the section assertion to mean anything').toBeGreaterThan(0);
  const peterXrefs = await api.xrefs('MRK.14.54');
  expect(peterXrefs.length, 'MRK.14.54 must have real cross-references for the VerseNode half to mean anything').toBeGreaterThan(0);

  // Pass 1 -- the PassageNode path: the Sermon's MAT account (cross-chapter
  // contiguous, display span "MAT.5.1-7.29") explores as its first
  // chapter's own run, "MAT.5.1-48".
  await page.goto('/read/LUK/6');
  await openVerse(page, 17);
  await page.getByTestId('popover-link-attests-Event:rob_sermon_on_the_mount').click();
  await expect(page.getByTestId('popover-title')).toHaveText('The Sermon on the Mount');
  const matEntry = page.getByTestId('event-witness-MAT.5.1-7.29');
  await expect(matEntry).toBeVisible(); // the DISPLAYED ref stays the honest full span
  await matEntry.locator('.popover-passage-ref-label').click();
  await expect(page.getByTestId('popover-title')).toHaveText('MAT.5.1-48');
  await expect(page.getByTestId('popover-section-xrefs')).toBeVisible();
  await expect(page.getByTestId('popover-section-catechism')).toBeVisible();

  await page.goto('/read/MRK/14');
  await openVerse(page, 54);
  await page.getByTestId('popover-link-attests-Event:rob_peter_denies').click();
  const peterEntry = page.getByTestId('event-witness-MRK.14.54, 66-72');
  await expect(peterEntry).toBeVisible();
  await peterEntry.locator('.popover-passage-ref-label').click();
  await expect(page.getByTestId('popover-title')).toHaveText('MRK.14.54');
  await expect(page.getByTestId('popover-section-cites')).toBeVisible();
});

test('EVENT-1: a single-witness event shows the one passage with no "PARALLEL ACCOUNTS" framing (requirement 4, n=1)', async ({ page }) => {
  // Batch T2 (owner's own live-review ruling, 2026-08-21): pw_emmaus,
  // this test's own original n=1 example, is no longer single-witness --
  // Mark 16:12-13 (also listed by Robertson for that section) is now
  // correctly curated as a genuine 2nd witness (the compiled KJV text is
  // the canon of witnesses; textual-critical dispute is never grounds for
  // omission -- see batch-t2-report.md). jm_temple_cleansing (John's own
  // FIRST cleansing, John 2:13-22) is genuinely, deliberately
  // single-witness -- Robertson's own table lists no Matthew/Mark/Luke
  // parallel for it -- and was Batch T's own original n=1 precedent
  // (batch-t-report.md's own words: "single witness (John alone)...
  // requirement 4's own 'no parallel framing when n=1' is exactly the
  // case this event demonstrates"), so it replaces pw_emmaus here as the
  // still-accurate n=1 acceptance case; the assertions themselves are
  // unchanged in shape.
  const detail = await api.event('jm_temple_cleansing');
  expect(detail.witnesses.length).toBe(1);
  expect(detail.witnesses[0].book).toBe('JHN');

  await page.goto('/read/JHN/2');
  await openVerse(page, 13);
  await page.getByTestId('popover-link-attests-Event:jm_temple_cleansing').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Jesus cleanses the temple for the first time');

  // Singular section id (event-witness, not event-witnesses) -- no eyebrow at all.
  await expect(page.getByTestId('popover-section-event-witness')).toBeVisible();
  await expect(page.getByTestId('popover-section-event-witnesses')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-witness').getByTestId('event-section-heading')).toHaveCount(0);
});

// ---------------------------------------------------------------------
// EVT-3 Ticket 2 (owner ruling, SUPERSEDES M-D1 requirement 3's own
// "SPAN-NOT-ECHO" law): M-D1 req 3 (owner live report #4, verbatim: "we
// should just see the passage span") RETIRED a single-witness event's own
// compact verse text down to a bare, click-to-expand span. The owner's own
// LATER, more specific ruling reverses this exact surface (EVENT-ACCOUNTS-1,
// verbatim: "i should see the passage, and it should be explorable like
// everything else" -- not just ref + read-whole-chapter): a single-witness
// event's own landed frontier now shows its real, clamped, explorable
// compact text -- IDENTICAL treatment to a multi-witness entry's own
// PARALLEL ACCOUNTS rendering (see EventWitnessesSection.cs's own EVT-3
// comment for the retirement of `SpanOnly`). RED-then-GREEN, inverted in
// place: this test used to assert the SPAN-ONLY shape; it now asserts the
// real-text shape supersedes it.
// ---------------------------------------------------------------------

test('EVT-3: a single-witness event\'s popover now shows its real, clamped compact TEXT -- never a bare span-only echo (M-D1 req 3, superseded)', async ({ page }) => {
  const detail = await api.event('jj_bethel_dream');
  expect(detail.witnesses.length).toBe(1);

  // M-D3/U6, owner verbatim: "'read the whole chapter' affordance REMOVED
  // when already reading that chapter" -- jm_temple_cleansing's own single
  // witness (JHN.2, the event's own former subject here) is, structurally,
  // reachable ONLY via a verse WITHIN that same chapter (a single-witness
  // event's own membership can never be cited from any OTHER chapter), so
  // popover-verse-expand there is now unconditionally suppressed -- this
  // test's own concern (span-not-echo) needs the button PRESENT and
  // clickable, so it now uses jj_bethel_dream instead (ALSO single-witness,
  // GEN.28.11-19), reached the same real, live-verified way READER-1/
  // BLINK-1 immediately above reach a different-chapter node: explore a
  // real, stable cross-reference (GEN.12.8 -> GEN.28.19, votes=3) so the
  // event's own witness chapter is never the one the reader is already
  // showing.
  await page.goto('/read/GEN/12');
  await openVerse(page, 8);
  await page.getByTestId('popover-link-cites-text-unit:GEN.28.19').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.28.19');
  await page.getByTestId('popover-link-attests-Event:jj_bethel_dream').click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);

  const section = page.getByTestId('popover-section-event-witness');
  await expect(section).toBeVisible();

  // The compact SPAN reference renders (the ref label PassageList.razor
  // always shows) --
  const entry = section.locator('[data-testid^="event-witness-"]');
  await expect(entry).toHaveCount(1);
  await expect(entry.locator('.popover-passage-ref-label')).toBeVisible();
  const entryTestId = await entry.getAttribute('data-testid');

  // EVT-3: the owner's own reversal, proven directly -- real compact
  // passage TEXT now renders by default (per-verse superscript numbers
  // included), the SAME treatment a multi-witness PARALLEL ACCOUNTS entry
  // already got. GEN.28.11-19 is 9 verses, well over the 2-verse clamp
  // (StandardVerseClamp), so the clamp-mark ellipsis is also expected.
  await expect(entry.locator('.popover-passage-text')).toBeVisible();
  await expect(entry.locator('.popover-passage-verse-num').first()).toBeVisible();
  await expect(entry.locator('[data-testid^="clamp-mark-"]')).toHaveCount(1);

  // The span click STILL reads the passage inline -- the existing
  // MiniReaderExpand control, reused, not reimplemented (O2: now a
  // RevealControls-driven arrow pair -- see that component's own O2
  // comment). Exact getByTestId, not a prefix locator: this entry's own
  // popover-verse-expand-{id} and its always-paired popover-verse-expand-
  // {id}-all sibling (R-D3's own double-arrow button) share this prefix,
  // which would otherwise make a prefix locator ambiguous.
  const expandBtn = entry.getByTestId(`popover-verse-expand-${entryTestId}`);
  await expect(expandBtn).toBeVisible();
  await expandBtn.click();
  await expect(entry.locator('[data-testid^="popover-verse-reader"]')).toBeVisible();
  await expect(entry.locator('.popover-reader-verse')).not.toHaveCount(0);
});

test('M-D1 req 3: a multi-witness event (Crucifixion) still shows other-book clamped text -- span-not-echo does not touch PARALLEL ACCOUNTS', async ({ page }) => {
  const detail = await api.event('pw_golgotha');
  expect(detail.witnesses.length).toBe(4);

  const matWitness = detail.witnesses.find((w: any) => w.book === 'MAT');
  const firstVref = matWitness.verse_groups[0].verses[0];
  const [book, chapter, verse] = firstVref.split('.');
  await page.goto(`/read/${book}/${chapter}`);
  await openVerse(page, verse);
  await page.getByTestId('popover-link-attests-Event:pw_golgotha').click();

  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection).toBeVisible();
  await expect(witnessesSection.getByTestId('event-section-heading')).toHaveText('PARALLEL ACCOUNTS');
  // UNCHANGED by this batch -- every one of the 4 witnesses (this book's
  // own included) still shows real clamped text, not a span-only line.
  const entries = witnessesSection.locator('[data-testid^="event-witness-"]');
  await expect(entries).toHaveCount(4);
  for (let i = 0; i < 4; i++) {
    await expect(entries.nth(i).locator('.popover-passage-text')).toBeVisible();
    await expect(entries.nth(i).locator('.popover-passage-verse-num').first()).toBeVisible();
  }
});

test('EVENT-1: chronological-vs-reading-order -- a JHN-witnessed event\'s FOLLOWING is not the next pericope in John (requirement 2/6/7, the owner\'s own "John doesn\'t have everything in order")', async ({ page }) => {
  // pw_jerusalem_entry is witnessed by John (JHN.12.12-19); its own
  // chronological FOLLOWING (pw_temple_cleansing, Robertson section 129)
  // is witnessed ONLY by Matthew/Mark/Luke -- John never repeats a
  // Passion-week temple cleansing, having already told a distinct, earlier
  // one in John 2 (jm_temple_cleansing). Ground truth, at the wire level:
  const entryDetail = await api.event('pw_jerusalem_entry');
  const johnWitness = entryDetail.witnesses.find((w: any) => w.book === 'JHN');
  expect(johnWitness, 'pw_jerusalem_entry must have a real John witness').toBeTruthy();

  const positions = await api.narrativeEventPositions('pw_jerusalem_entry');
  const passionWeek = positions.narrative.find((p: any) => p.narrative_id === 'passion-week');
  expect(passionWeek.following.id).toBe('pw_temple_cleansing');
  // CHRONO-MERGE-1: this event's own narrative-following genuinely
  // DIVERGES from its global-timeline following -- the "John doesn't have
  // everything in order" fact this test is named for IS a divergence, so
  // it now surfaces via the story-thread line rather than a dedicated
  // per-narrative arrow (CONTRACT.md's own CHRONO-MERGE-1 note).
  expect(passionWeek.following.id, 'this test needs a GENUINELY diverging following for the story-thread line to render at all').not.toBe(positions.timeline.following?.id);

  const cleansingDetail = await api.event('pw_temple_cleansing');
  expect(cleansingDetail.witnesses.some((w: any) => w.book === 'JHN'), 'pw_temple_cleansing must have NO John witness -- the whole point of this test').toBeFalsy();
  expect(cleansingDetail.witnesses.map((w: any) => w.book).sort()).toEqual(['LUK', 'MAT', 'MRK']);

  // Live, through the popover: open the John witness verse, traverse to
  // the event, then FOLLOWING -- lands on the temple-cleansing event, whose
  // OWN witness list (asserted above) proves this was never reachable by
  // "just keep reading John's text forward."
  const [book, chapter, verse] = johnWitness.verse_groups[0].verses[0].split('.');
  await page.goto(`/read/${book}/${chapter}`);
  await openVerse(page, verse);
  await page.getByTestId('popover-link-attests-Event:pw_jerusalem_entry').click();
  await expect(page.getByTestId('popover-title')).toHaveText('The triumphal entry into Jerusalem');

  const followingLeg = page.getByTestId('event-story-thread-following-event-passion-week');
  await expect(followingLeg).toHaveText('next → Jesus cleanses the temple a second time');
  await followingLeg.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Jesus cleanses the temple a second time');

  // This destination event's own PARALLEL ACCOUNTS never include John --
  // confirming, live, that the chronological target is NOT anything John's
  // own text narrates at all, let alone "the next pericope in JHN."
  const witnessSection = page.getByTestId(/^popover-section-event-witness/);
  await expect(witnessSection).toBeVisible();
  await expect(page.getByTestId(/^event-witness-JHN\./)).toHaveCount(0);
});
