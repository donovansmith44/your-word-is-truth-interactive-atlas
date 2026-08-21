import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch T requirement 5 ("event titles throughout the reader" -- owner
// direction 2026-08-21, verbatim: "add the event titles throughout the
// reader... Headings are explorable... clicking opens the event/passage
// node"). See CONTRACT.md's own EVENT-1 note and the reader testid-
// inventory's own `pericope-heading-{eventId}` entry.

test('a covered book renders a pericope heading above its anchor verse, explorable, opens the EventNode', async ({ page }) => {
  // pw_golgotha's own Matthew witness anchors at MAT.27.33 (event-witnesses.toml).
  const detail = await api.event('pw_golgotha');
  const matWitness = detail.witnesses.find((w: any) => w.book === 'MAT');
  const anchorVref = matWitness.verse_groups[0].verses[0];
  const [book, chapter] = anchorVref.split('.');

  const chapterOut = await api.chapter(`${book}.${chapter}`);
  const anchorVerse = chapterOut.verses.find((v: any) => `${book}.${chapter}.${v.verse}` === anchorVref);
  expect(anchorVerse.heading, 'the API itself must mark this verse as a heading anchor').toBeTruthy();
  expect(anchorVerse.heading.event_id).toBe('pw_golgotha');
  expect(anchorVerse.heading.title).toBe('The crucifixion at Golgotha');

  await page.goto(`/read/${book}/${chapter}`);
  const heading = page.getByTestId('pericope-heading-pw_golgotha');
  await expect(heading).toBeVisible();
  await expect(heading).toHaveText('The crucifixion at Golgotha');

  // Heading renders immediately ABOVE its own anchor verse's line (DOM
  // order, not just "present somewhere on the page").
  const headingBox = await heading.boundingBox();
  const verseBox = await page.getByTestId(`verse-line-${anchorVerse.verse}`).boundingBox();
  expect(headingBox && verseBox && headingBox.y).toBeLessThan(verseBox!.y);

  // Explorable (ONE-RULE): click opens a fresh EventNode popover.
  await heading.click();
  await expect(page.getByTestId('popover-title')).toHaveText('The crucifixion at Golgotha');
  await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();
});

test('a pericope heading is keyboard-explorable (Enter opens the popover, same as click)', async ({ page }) => {
  const detail = await api.event('pw_jerusalem_resurrection');
  const matWitness = detail.witnesses.find((w: any) => w.book === 'MAT');
  const anchorVref = matWitness.verse_groups[0].verses[0];
  const [book, chapter] = anchorVref.split('.');

  await page.goto(`/read/${book}/${chapter}`);
  const heading = page.getByTestId('pericope-heading-pw_jerusalem_resurrection');
  await expect(heading).toBeVisible();
  await heading.focus();
  await heading.press('Enter');
  await expect(page.getByTestId('popover-title')).toHaveText('The empty tomb in Jerusalem');
});

test('an uncovered book/chapter shows no pericope heading at all (conditional presence)', async ({ page }) => {
  // Leviticus 5 -- no leg of any of the 13 curated narratives, no
  // witnesses/robertson_section authored this batch (Gospels+Acts coverage
  // + the existing narratives only, per the owner's own coverage decision).
  const chapterOut = await api.chapter('LEV.5');
  expect(chapterOut.verses.some((v: any) => v.heading), 'LEV.5 must carry zero heading-anchored verses on the wire').toBeFalsy();

  await page.goto('/read/LEV/5');
  await expect(page.getByTestId(/^pericope-heading-/)).toHaveCount(0);
});

test('a multi-witness event anchors a SEPARATE heading in each of its own witness books, all sharing the same title', async ({ page }) => {
  const detail = await api.event('pw_golgotha');
  expect(detail.witnesses.length).toBe(4);

  for (const w of detail.witnesses) {
    const anchorVref = w.verse_groups[0].verses[0];
    const [book, chapter] = anchorVref.split('.');
    await page.goto(`/read/${book}/${chapter}`);
    const heading = page.getByTestId('pericope-heading-pw_golgotha');
    await expect(heading, `${w.book}'s own witness must anchor its own heading`).toBeVisible();
    await expect(heading).toHaveText('The crucifixion at Golgotha');
  }
});

test('split view: the reader pane shows the SAME pericope heading (shared code path, requirement 5)', async ({ page }) => {
  const detail = await api.event('pw_golgotha');
  const matWitness = detail.witnesses.find((w: any) => w.book === 'MAT');
  const [book, chapter] = matWitness.verse_groups[0].verses[0].split('.');

  await page.goto(`/read/${book}/${chapter}?split=1`);
  await expect(page.getByTestId('split-view')).toBeVisible();
  const heading = page.getByTestId('reader-root').getByTestId('pericope-heading-pw_golgotha');
  await expect(heading).toBeVisible();
  await expect(heading).toHaveText('The crucifixion at Golgotha');
  await heading.click();
  await expect(page.getByTestId('popover-title')).toHaveText('The crucifixion at Golgotha');
  // The split's own atlas pane stays present and untouched (SPLIT-1) --
  // opening the heading's popover never disturbs the other pane.
  await expect(page.getByTestId('split-pane-atlas')).toBeVisible();
});

test('every curated event\'s reader heading text equals its own title exactly (a sample sweep, not just one event)', async ({ page }) => {
  // Every existing-narrative event already carries a title via Event.label
  // (no new authoring needed) -- spot-check a handful across different
  // narratives/books to confirm the heading-worthy rule reaches them too,
  // not just this batch's own newly-witnessed Gospel events.
  const sample = ['ex_red_sea', 'cq_jericho', 'pw_emmaus'];
  const narratives = await api.narratives();
  const knownIds = new Set(narratives.flatMap((n: any) => n.legs));
  for (const eventId of sample) {
    if (!knownIds.has(eventId)) continue; // skip gracefully if a curated id ever changes
    const detail = await api.event(eventId);
    const anchorVref = detail.witnesses[0].verse_groups[0].verses[0];
    const [book, chapter] = anchorVref.split('.');
    await page.goto(`/read/${book}/${chapter}`);
    const heading = page.getByTestId(`pericope-heading-${eventId}`);
    await expect(heading, `${eventId} must anchor a real reader heading`).toBeVisible();
    await expect(heading).toHaveText(detail.title);
  }
});
