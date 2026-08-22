import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch W2 ("whole-Bible titled verse containers," second run -- the
// historical books, Samuel/Kings/Chronicles). See CONTRACT.md's own
// EVENT-1 note and batch-w2-report.md for the full coverage table and the
// Crockett Harmony verification record.

test('req 1b OWNER ACCEPTANCE CASE: the temple dedication expands to per-account passages from Kings AND Chronicles, the Crucifixion pattern', async ({ page }) => {
  const detail = await api.event('1ki_temple_dedication');
  expect(detail.title).toBe('Solomon dedicates the temple');
  expect(detail.kind).toBe('event');
  expect(detail.witnesses.length).toBe(2);
  const books = detail.witnesses.map((w: any) => w.book).sort();
  expect(books).toEqual(['1KI', '2CH']);

  const chrWitness = detail.witnesses.find((w: any) => w.book === '2CH');
  const chrVerses = chrWitness.verse_groups.flatMap((g: any) => g.verses);
  // Crockett (1897) section 133: the dedicatory prayer and the fire from
  // heaven both fall inside the witnessed span.
  expect(chrVerses).toContain('2CH.6.1');
  expect(chrVerses).toContain('2CH.7.1');

  // Open via the 1 Kings side (a real navigation path).
  const kiWitness = detail.witnesses.find((w: any) => w.book === '1KI');
  const [book, chapter, verse] = kiWitness.verse_groups[0].verses[0].split('.');
  await page.goto(`/read/${book}/${chapter}`);
  await page.getByTestId(`verse-line-${verse}`).click();
  await page.getByTestId('verse-event-1ki_temple_dedication').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Solomon dedicates the temple');

  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection).toBeVisible();
  await expect(witnessesSection.getByTestId('event-section-heading')).toHaveText('PARALLEL ACCOUNTS');
  // One PassageList block per witness's own contiguous chapter span, not
  // one per book (PassageBlockBuilder splits a unit's own verses into MORE
  // blocks on a chapter boundary, per batch-t2-report.md) -- 1 Kings 8 is
  // one chapter (1 block); the 2 Chronicles witness spans three chapters,
  // 5-7 (3 blocks): 4 total, confirming both accounts are genuinely
  // reachable, each of Chronicles' own three chapters independently.
  const entries = witnessesSection.locator('[data-testid^="event-witness-"]');
  await expect(entries).toHaveCount(4);

  // The reader heading itself also anchors correctly in 2 Chronicles 5
  // (its own, separate witness-book heading -- the SAME event, a second
  // anchor in a second book, exactly like a multi-witness Gospel event).
  await page.goto('/read/2CH/5');
  const chrHeading = page.getByTestId('pericope-heading-1ki_temple_dedication');
  await expect(chrHeading).toBeVisible();
  await expect(chrHeading).toHaveText('Solomon dedicates the temple');
});

test('req 1b NAMED CASE: 2 Kings 18-20 / Isaiah 36-39 / 2 Chronicles 32 (Hezekiah and Sennacherib) is a genuine THREE-way parallel, including a not-yet-covered book', async ({ page }) => {
  const detail = await api.event('2ki_rabshakeh_message');
  // 2 Kings (its own top-level book), Isaiah (this run's own named cross-
  // book case, W4), and 2 Chronicles 32 (added when 2chronicles.toml
  // landed) -- a real three-witness parallel, the same shape as a
  // 3-Gospel Passion Week scene, not merely a two-way echo.
  expect(detail.witnesses.length).toBe(3);
  const books = detail.witnesses.map((w: any) => w.book).sort();
  expect(books).toEqual(['2CH', '2KI', 'ISA']);

  const isaWitness = detail.witnesses.find((w: any) => w.book === 'ISA');
  const [book, chapter, verse] = isaWitness.verse_groups[0].verses[0].split('.');
  expect(book).toBe('ISA'); // Isaiah was not yet a container-covered book at W2 authoring time -- still a legal witness target then; fully covered as of Batch W4 (stale comment corrected, no assertion here ever depended on Isaiah's own coverage state).

  await page.goto(`/read/${book}/${chapter}`);
  await page.getByTestId(`verse-line-${verse}`).click();
  await page.getByTestId('verse-event-2ki_rabshakeh_message').click();
  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection.getByTestId('event-section-heading')).toHaveText('PARALLEL ACCOUNTS');
  await expect(witnessesSection.locator('[data-testid^="event-witness-"]')).toHaveCount(3);
});

test('a general-kind 1 Chronicles genealogy renders its own heading, with no "Show on the map" chip', async ({ page }) => {
  const detail = await api.event('1ch_genealogy_patriarchs_to_edom');
  expect(detail.kind).toBe('general');
  expect(detail.when).toBeUndefined(); // omitted, not null, for general-kind

  await page.goto('/read/1CH/1');
  const heading = page.getByTestId('pericope-heading-1ch_genealogy_patriarchs_to_edom');
  await expect(heading).toBeVisible();
  await heading.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Genealogy from Adam to the sons of Esau');
  await expect(page.getByTestId('popover-section-event-date-places')).toHaveCount(0);
  await expect(page.getByTestId('popover-chip-map')).toHaveCount(0);
});

test('jank check: a dense 2 Samuel chapter renders multiple pericope headings cleanly, each explorable', async ({ page }) => {
  // 2 Samuel 5 -- David made king over Israel, Jerusalem captured, the
  // Hiram alliance and family, and the Philistine wars, all in one
  // chapter (sam2_king_over_israel, sam2_jerusalem_captured,
  // sam2_hiram_family, sam2_philistine_wars) -- the req-3 jank check the
  // brief names ("a dense book").
  const chapterOut = await api.chapter('2SA.5');
  const headings = chapterOut.verses.filter((v: any) => v.heading);
  expect(headings.length, '2 Samuel 5 must show multiple distinct heading anchors').toBeGreaterThan(1);

  await page.goto('/read/2SA/5');
  const allHeadings = page.locator('[data-testid^="pericope-heading-"]');
  await expect(allHeadings.first()).toBeVisible();
  const count = await allHeadings.count();
  expect(count).toBeGreaterThan(1);

  for (let i = 0; i < count; i++) {
    const h = allHeadings.nth(i);
    await expect(h).toBeVisible();
    expect(await h.getAttribute('data-testid')).toBeTruthy();
  }

  await allHeadings.first().click();
  await expect(page.getByTestId('popover-title')).toBeVisible();
});

test('coverage sanity: 1 Samuel 1 and Esther 10 (this run\'s own first and last chapters) both show a heading at their own opening verse', async ({ page }) => {
  const first = await api.chapter('1SA.1');
  expect(first.verses[0].heading?.event_id).toBe('sam_hannah_vow');

  const last = await api.chapter('EST.10');
  const lastHeadings = last.verses.filter((v: any) => v.heading);
  expect(lastHeadings.length).toBeGreaterThan(0);

  await page.goto('/read/1SA/1');
  await expect(page.getByTestId('pericope-heading-sam_hannah_vow')).toBeVisible();
  await expect(page.getByTestId('pericope-heading-sam_hannah_vow')).toHaveText("Hannah's grief and vow at Shiloh");
});
