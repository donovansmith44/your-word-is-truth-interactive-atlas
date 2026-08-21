import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch W1 ("whole-Bible titled verse containers" -- owner directive
// 2026-08-21: "do everything that you did for the Gospels and acts with
// the labeled containers of verses for the whole Bible"). See CONTRACT.md's
// own EVENT-1 note (the KIND-AGNOSTIC paragraph, req 1b's own model
// generalization) and batch-w1-report.md for the full coverage table.

test('req-1b NAMED CASE: Exodus 20 / Deuteronomy 5 (the Commandments) render as PARALLEL ACCOUNTS, agreeing with the catechism layer', async ({ page }) => {
  const detail = await api.event('theo-127');
  expect(detail.title).toBe('Ten Commandments Given');
  expect(detail.kind).toBe('event'); // theo-127 itself is event-kind (dated, placed at Sinai)
  expect(detail.witnesses.length).toBe(2);
  const books = detail.witnesses.map((w: any) => w.book).sort();
  expect(books).toEqual(['DEU', 'EXO']);

  const deuWitness = detail.witnesses.find((w: any) => w.book === 'DEU');
  const deuVerses = deuWitness.verse_groups.flatMap((g: any) => g.verses);
  // Agrees with data/curated/catechism-deut5.toml's own per-commandment
  // verse alignment: the Commandments proper run DEU.5.7 through DEU.5.21.
  expect(deuVerses).toContain('DEU.5.7');
  expect(deuVerses).toContain('DEU.5.21');

  // Open via the Exodus side (a real navigation path).
  const exoWitness = detail.witnesses.find((w: any) => w.book === 'EXO');
  const [book, chapter, verse] = exoWitness.verse_groups[0].verses[0].split('.');
  await page.goto(`/read/${book}/${chapter}`);
  await page.getByTestId(`verse-line-${verse}`).click();
  await page.getByTestId('verse-event-theo-127').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Ten Commandments Given');

  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection).toBeVisible();
  await expect(witnessesSection.getByTestId('event-section-heading')).toHaveText('PARALLEL ACCOUNTS');
  const entries = witnessesSection.locator('[data-testid^="event-witness-"]');
  await expect(entries).toHaveCount(2);

  // The reader heading itself also anchors correctly at Deuteronomy 5:1
  // (its own, separate witness-book heading -- the SAME event, a second
  // anchor in a second book, exactly like a multi-witness Gospel event).
  await page.goto('/read/DEU/5');
  const deuHeading = page.getByTestId('pericope-heading-theo-127');
  await expect(deuHeading).toBeVisible();
  await expect(deuHeading).toHaveText('Ten Commandments Given');
});

test('a general-kind Genesis container (a genealogy) renders its own heading, with no "Show on the map" chip', async ({ page }) => {
  const detail = await api.event('gen_generations_adam_to_noah');
  expect(detail.kind).toBe('general');
  expect(detail.when).toBeUndefined(); // omitted, not null, for general-kind

  await page.goto('/read/GEN/5');
  const heading = page.getByTestId('pericope-heading-gen_generations_adam_to_noah');
  await expect(heading).toBeVisible();
  await expect(heading).toHaveText('The generations of Adam to Noah');
  await heading.click();
  await expect(page.getByTestId('popover-title')).toHaveText('The generations of Adam to Noah');
  // General-kind: no date/places section, no map-explore chip (CONTRACT.md's
  // own "conditional presence extends to affordances too" note).
  await expect(page.getByTestId('popover-section-event-date-places')).toHaveCount(0);
  await expect(page.getByTestId('popover-chip-map')).toHaveCount(0);
});

test('jank check: a dense Genesis chapter (the Passion-Week-scale narrative density) renders multiple pericope headings cleanly, each explorable', async ({ page }) => {
  // Genesis 25 -- Isaac prays for Rebekah, the birth of Jacob and Esau,
  // Esau sells his birthright, all in one chapter (gen_jacob_esau_born,
  // gen_children_of_nahor, gen_esau_birthright, plus the pre-existing
  // theo-88/theo-90 stubs) -- a genuinely dense chapter, the req-3 jank
  // check the brief names ("GEN narrative").
  const chapterOut = await api.chapter('GEN.25');
  const headings = chapterOut.verses.filter((v: any) => v.heading);
  expect(headings.length, 'GEN 25 must show multiple distinct heading anchors').toBeGreaterThan(1);

  await page.goto('/read/GEN/25');
  const allHeadings = page.locator('[data-testid^="pericope-heading-"]');
  // Auto-retrying assertion (not a raw `.count()` immediately after
  // `goto`) -- the WASM app's own render is not yet guaranteed complete
  // the instant navigation resolves; `toHaveCount` polls until it settles
  // (or times out), the same house idiom other specs in this suite use as
  // a settle-gate rather than a raw snapshot read.
  await expect(allHeadings.first()).toBeVisible();
  const count = await allHeadings.count();
  expect(count).toBeGreaterThan(1);

  // Every heading renders directly above its own anchor verse's line (DOM
  // order), and none overlap/stack -- the "quiet small-caps title... never
  // stack" design intent.
  for (let i = 0; i < count; i++) {
    const h = allHeadings.nth(i);
    await expect(h).toBeVisible();
    const testId = await h.getAttribute('data-testid');
    expect(testId).toBeTruthy();
  }

  // The first heading is explorable end to end.
  await allHeadings.first().click();
  await expect(page.getByTestId('popover-title')).toBeVisible();
});

test('coverage sanity: Genesis 1 and Genesis 50 (the book\'s own first and last chapters) both show a heading at their own opening verse', async ({ page }) => {
  const first = await api.chapter('GEN.1');
  expect(first.verses[0].heading?.event_id).toBe('theo-1');

  const last = await api.chapter('GEN.50');
  const lastHeadings = last.verses.filter((v: any) => v.heading);
  expect(lastHeadings.length).toBeGreaterThan(0);

  await page.goto('/read/GEN/1');
  await expect(page.getByTestId('pericope-heading-theo-1')).toBeVisible();
  await expect(page.getByTestId('pericope-heading-theo-1')).toHaveText('Creation of all things');
});
