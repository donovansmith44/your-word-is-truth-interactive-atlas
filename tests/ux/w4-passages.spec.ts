import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch W4 ("whole-Bible titled verse containers," fourth run -- the
// Prophets: Isaiah, Jeremiah, Lamentations, Ezekiel, Daniel, and the
// twelve Minor Prophets). See batch-w4-report.md for the full coverage
// table. Targeted acceptance only, per the brief's own scope for this run:
// one prophet-heading rendering case and one Kings || Isaiah parallel-
// witness popover case, plus this run's own new req 1b named case
// (Isaiah 2:2-4 || Micah 4:1-3, both sides authored this run).

test('a prophet burden-superscription renders verbatim as the reader heading, and its own popover title', async ({ page }) => {
  // Isaiah 13:1's own KJV superscription, quoted verbatim: "The burden of
  // Babylon, which Isaiah the son of Amoz did see." kjv_superscription
  // provenance, GENERAL-kind (no date/place chip), the SAME "wire-only, no
  // UI element renders the provenance field itself" disposition every
  // other provenance field has (client/Dtos.cs) -- proves the
  // superscription text itself reached the heading via Event::label.
  const detail = await api.event('isa_burden_of_babylon');
  expect(detail.title).toBe('The burden of Babylon, which Isaiah the son of Amoz did see.');
  expect(detail.kind).toBe('general');
  expect(detail.when).toBeUndefined(); // omitted, not null, for general-kind

  await page.goto('/read/ISA/13');
  const heading = page.getByTestId('pericope-heading-isa_burden_of_babylon');
  await expect(heading).toBeVisible();
  await expect(heading).toHaveText('The burden of Babylon, which Isaiah the son of Amoz did see.');
  await heading.click();
  await expect(page.getByTestId('popover-title')).toHaveText('The burden of Babylon, which Isaiah the son of Amoz did see.');
  await expect(page.getByTestId('popover-section-event-date-places')).toHaveCount(0);
  await expect(page.getByTestId('popover-chip-map')).toHaveCount(0);
});

test('req 1b NAMED CASE (2 Kings 18-20 || Isaiah 36-39, Crockett sections 175-176, Batch W2): Rabshakeh\'s message is a PARALLEL ACCOUNT across 2 Kings, Isaiah, and 2 Chronicles', async ({ page }) => {
  const detail = await api.event('2ki_rabshakeh_message');
  expect(detail.witnesses.length).toBe(3);
  const books = detail.witnesses.map((w: any) => w.book).sort();
  expect(books).toEqual(['2CH', '2KI', 'ISA']);

  const isaWitness = detail.witnesses.find((w: any) => w.book === 'ISA');
  expect(isaWitness.verse_groups[0].verses[0]).toBe('ISA.36.2');

  // Open via the Isaiah side (a real navigation path this run's own
  // Isaiah authoring makes possible) and confirm PARALLEL ACCOUNTS shows
  // all three witnesses.
  await page.goto('/read/ISA/36');
  await page.getByTestId('verse-line-2').click();
  await page.getByTestId('verse-event-2ki_rabshakeh_message').click();
  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection).toBeVisible();
  await expect(witnessesSection.getByTestId('event-section-heading')).toHaveText('PARALLEL ACCOUNTS');
  await expect(witnessesSection.locator('[data-testid^="event-witness-"]')).toHaveCount(3);

  // The reader heading itself also anchors correctly at Isaiah's own
  // witness range -- this run's own gap this book previously had no
  // container to render at all.
  await expect(page.getByTestId('pericope-heading-2ki_rabshakeh_message')).toBeVisible();
});

test('req 1b NAMED CASE, this run\'s own new authoring (both sides): Isaiah 2:2-4 and Micah 4:1-3 (swords into plowshares) are PARALLEL ACCOUNTS', async ({ page }) => {
  const detail = await api.event('isa_mountain_of_the_lords_house');
  expect(detail.witnesses.length).toBe(2);
  const books = detail.witnesses.map((w: any) => w.book).sort();
  expect(books).toEqual(['ISA', 'MIC']);

  const micWitness = detail.witnesses.find((w: any) => w.book === 'MIC');
  expect(micWitness.verse_groups[0].verses[0]).toBe('MIC.4.1');

  await page.goto('/read/ISA/2');
  await page.getByTestId('verse-line-2').click();
  await page.getByTestId('verse-event-isa_mountain_of_the_lords_house').click();
  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection).toBeVisible();
  await expect(witnessesSection.getByTestId('event-section-heading')).toHaveText('PARALLEL ACCOUNTS');
  await expect(witnessesSection.locator('[data-testid^="event-witness-"]')).toHaveCount(2);

  // The shared title is TRUE AT EVERY ANCHOR (the controller's own binding
  // ruling, batch-w3-report.md fix round 1) -- it is this project's own
  // CC0 paraphrase (not a literal kjv_superscription quote, since the two
  // accounts genuinely differ in wording), so it must render identically,
  // and honestly, above BOTH Isaiah 2 and Micah 4.
  const title: string = detail.title;
  await expect(page.getByTestId('pericope-heading-isa_mountain_of_the_lords_house')).toHaveText(title);
  await page.goto('/read/MIC/4');
  await expect(page.getByTestId('pericope-heading-isa_mountain_of_the_lords_house')).toHaveText(title);
  // Micah's own chapter-4 container begins at 4:4, not 4:1 (avoiding the
  // real anchor collision this shared witness's own MIC.4.1 anchor would
  // otherwise cause) -- so the FIRST heading in Micah 4 is this shared
  // one, not a separate Micah-only container.
});
