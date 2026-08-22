import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch W3 ("whole-Bible titled verse containers," third run -- Job,
// Psalms, Proverbs, Ecclesiastes, Song of Solomon). See CONTRACT.md's own
// EVENT-1 note (kjv_superscription provenance) and batch-w3-report.md for
// the full coverage table and the Psalm-119-nesting/Psalm-14-53 reasoning.
// Targeted acceptance only, per the brief's own scope for this run: a
// Psalm-superscription heading rendering, one parallel-witness popover
// case, and the master brief's own explicit "jank check... PSA
// superscriptions" density check.
//
// Fix round 1 (batch-w3-review.md, Important-1): psa_014's own title was
// retitled from Psalm 14's own superscription tag (which, reused as Psalm
// 53's own reader heading, silently misattributed a citation Psalm 53
// does not carry) to the psalm pair's own shared opening clause -- true,
// byte-for-byte, at BOTH anchors. The second test below pins this
// directly against live compiled text, not a hardcoded echo.

test('a Psalm superscription renders verbatim as the reader heading, and its own popover title', async ({ page }) => {
  // Psalm 3's own KJV superscription (PSA.3.1) -- the brief's own named
  // example verbatim: "A Psalm of David, when he fled from Absalom his
  // son." kjv_superscription provenance, no UI element renders the
  // provenance field itself (same "wire-only, no UI element" disposition
  // robertson_section/acts_section/atlas_section already have -- see
  // client/Dtos.cs), so this proves the SUPERSCRIPTION TEXT ITSELF reached
  // the heading via Event::label, not a separate rendered citation.
  const detail = await api.event('psa_003');
  expect(detail.title).toBe('A Psalm of David, when he fled from Absalom his son.');
  expect(detail.kind).toBe('general');
  expect(detail.when).toBeUndefined(); // omitted, not null, for general-kind

  await page.goto('/read/PSA/3');
  const heading = page.getByTestId('pericope-heading-psa_003');
  await expect(heading).toBeVisible();
  await expect(heading).toHaveText('A Psalm of David, when he fled from Absalom his son.');
  await heading.click();
  await expect(page.getByTestId('popover-title')).toHaveText('A Psalm of David, when he fled from Absalom his son.');
  // General-kind: no date/place chip, matching every other general-kind
  // passage's own popover shape (w2-passages.spec.ts's own precedent).
  await expect(page.getByTestId('popover-section-event-date-places')).toHaveCount(0);
  await expect(page.getByTestId('popover-chip-map')).toHaveCount(0);
});

test('req 1b NAMED CASE, this run\'s own new authoring: Psalm 14 and Psalm 53 (the fool hath said in his heart) are PARALLEL ACCOUNTS', async ({ page }) => {
  const detail = await api.event('psa_014');
  expect(detail.witnesses.length).toBe(2);
  const books = detail.witnesses.map((w: any) => w.book).sort();
  expect(books).toEqual(['PSA', 'PSA']); // both witnesses are book PSA -- two distinct psalms, same book code

  const psa53Witness = detail.witnesses.find((w: any) => w.verse_groups[0].verses[0].startsWith('PSA.53'));
  expect(psa53Witness).toBeTruthy();
  const [book, chapter, verse] = psa53Witness.verse_groups[0].verses[0].split('.');
  expect(chapter).toBe('53');

  // Open via Psalm 14 (a real navigation path) and confirm PARALLEL
  // ACCOUNTS shows both entries.
  await page.goto('/read/PSA/14');
  await page.getByTestId('verse-line-1').click();
  await page.getByTestId('verse-event-psa_014').click();
  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection).toBeVisible();
  await expect(witnessesSection.getByTestId('event-section-heading')).toHaveText('PARALLEL ACCOUNTS');
  await expect(witnessesSection.locator('[data-testid^="event-witness-"]')).toHaveCount(2);

  // The reader heading itself also anchors correctly in Psalm 53 (its own,
  // separate witness-book heading -- Psalm 53 carries no top-level
  // container of its own, deliberately, to avoid a real within-layer
  // anchor collision at PSA.53.1 -- see psalms.toml's own header note).
  await page.goto(`/read/${book}/${chapter}`);
  const psa53Heading = page.getByTestId('pericope-heading-psa_014');
  await expect(psa53Heading).toBeVisible();
  await expect(psa53Heading).toHaveText("The fool hath said in his heart, There is no God.");
});

test('fix round 1 (Important-1): psa_014\'s shared title is TRUE AT EVERY ANCHOR, in both Psalm 14 and Psalm 53 -- pins the controller\'s binding ruling', async ({ page }) => {
  // Live regression pin, not a hardcoded-string echo: fetches BOTH psalms'
  // own real verse-1 text from the API and proves the container's own
  // title is a genuine, verbatim substring of EACH -- if either the title
  // or the underlying compiled text ever drifts apart again, this fails.
  const detail = await api.event('psa_014');
  const title: string = detail.title;

  const psa14 = await api.chapter('PSA.14');
  const psa53 = await api.chapter('PSA.53');
  const psa14Verse1: string = psa14.verses[0].text;
  const psa53Verse1: string = psa53.verses[0].text;

  expect(psa14Verse1).toContain(title);
  expect(psa53Verse1).toContain(title);

  // The regression this pins directly: Psalm 14's own DISTINCT
  // superscription tag (never shared with Psalm 53, which reads "...upon
  // Mahalath, Maschil..." instead) must NEVER be the container's own
  // title -- that was the bug (batch-w3-review.md, Important-1). Confirms
  // the two psalms' own superscription tags really do differ (the reason
  // the bug was visible at all), then confirms the CURRENT title is
  // neither of them.
  const psa14Superscription = 'To the chief Musician, A Psalm of David.';
  const psa53Superscription = 'To the chief Musician upon Mahalath, Maschil, A Psalm of David.';
  expect(psa14Verse1.startsWith(psa14Superscription)).toBe(true);
  expect(psa53Verse1.startsWith(psa53Superscription)).toBe(true);
  expect(psa14Superscription).not.toBe(psa53Superscription);
  expect(title).not.toBe(psa14Superscription);
  expect(title).not.toBe(psa53Superscription);

  // Reader-visible: the SAME heading text renders above both chapters.
  await page.goto('/read/PSA/14');
  await expect(page.getByTestId('pericope-heading-psa_014')).toHaveText(title);
  await page.goto('/read/PSA/53');
  await expect(page.getByTestId('pericope-heading-psa_014')).toHaveText(title);
});

test('jank check (the master brief\'s own named case, "PSA superscriptions"): Psalm 119 renders 22 distinct acrostic-stanza headings cleanly, each explorable', async ({ page }) => {
  const chapterOut = await api.chapter('PSA.119');
  const headings = chapterOut.verses.filter((v: any) => v.heading);
  expect(headings.length).toBe(22);

  await page.goto('/read/PSA/119');
  const allHeadings = page.locator('[data-testid^="pericope-heading-psa_119_"]');
  await expect(allHeadings.first()).toBeVisible();
  const count = await allHeadings.count();
  expect(count).toBe(22);

  for (let i = 0; i < count; i++) {
    const h = allHeadings.nth(i);
    await expect(h).toBeVisible();
    expect(await h.getAttribute('data-testid')).toBeTruthy();
  }

  // Aleph (the first stanza) opens the chapter; Tau (the last) closes it.
  await expect(page.getByTestId('pericope-heading-psa_119_aleph')).toHaveText('Psalm 119: ALEPH');
  await allHeadings.last().click();
  await expect(page.getByTestId('popover-title')).toHaveText('Psalm 119: TAU');
});
