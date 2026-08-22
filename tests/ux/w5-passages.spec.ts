import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch W5 ("whole-Bible titled verse containers," fifth and FINAL run --
// Romans through Revelation, completing the whole Bible's own container
// coverage: all 66 canonical books, all 31,102 KJV verses). See
// batch-w5-report.md for the full coverage table. Targeted acceptance
// only, per this batch's own slice-specific req 7: one epistle-structure
// heading case and one Revelation vision-heading case, plus the book's
// own sole kjv_superscription case (Revelation's own real, text-native
// title verse).

test('an epistle chapter renders real letter-structure headings, in order, not an arbitrary chapter chunk', async ({ page }) => {
  // Romans 1 alone carries FOUR distinct argument units per this batch's
  // own genre-granularity requirement (salutation, thanksgiving, the
  // letter's own thesis statement, and the indictment of Gentile
  // ungodliness) -- proof this run's own epistle authoring follows real
  // letter structure, not a bare "one heading per chapter" default.
  const chapterOut = await api.chapter('ROM.1');
  const headed = chapterOut.verses.filter((v: any) => v.heading);
  expect(headed.map((v: any) => v.heading.event_id)).toEqual([
    'rom_salutation',
    'rom_thanksgiving_and_longing_to_see_rome',
    'rom_the_just_shall_live_by_faith',
    'rom_the_wrath_of_god_against_gentile_ungodliness',
  ]);

  await page.goto('/read/ROM/1');
  const salutation = page.getByTestId('pericope-heading-rom_salutation');
  const thanksgiving = page.getByTestId('pericope-heading-rom_thanksgiving_and_longing_to_see_rome');
  const thesis = page.getByTestId('pericope-heading-rom_the_just_shall_live_by_faith');
  const wrath = page.getByTestId('pericope-heading-rom_the_wrath_of_god_against_gentile_ungodliness');
  await expect(salutation).toHaveText('Salutation: Paul, a servant of Jesus Christ, called to be an apostle');
  await expect(thanksgiving).toHaveText("Thanksgiving, and Paul's longing to come to Rome");
  await expect(thesis).toHaveText('The gospel of God: the righteousness of God revealed, the just shall live by faith');
  await expect(wrath).toHaveText('The wrath of God revealed against the ungodliness of the Gentile world');

  // DOM order matches the text's own verse order (1:1, 1:8, 1:16, 1:18).
  const ys = await Promise.all([salutation, thanksgiving, thesis, wrath].map(async (h) => (await h.boundingBox())!.y));
  expect(ys[0]).toBeLessThan(ys[1]);
  expect(ys[1]).toBeLessThan(ys[2]);
  expect(ys[2]).toBeLessThan(ys[3]);

  // GENERAL-kind throughout this whole slice (req 2, KIND DISCIPLINE): a
  // letter's own content is not a datable occurrence -- no date/place chip
  // renders on any of these containers' own popovers, the same
  // "wire-only, no UI element renders the provenance field itself"
  // disposition every prior W-series general-kind container already has.
  const detail = await api.event('rom_salutation');
  expect(detail.kind).toBe('general');
  expect(detail.when).toBeUndefined();
  await salutation.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Salutation: Paul, a servant of Jesus Christ, called to be an apostle');
  await expect(page.getByTestId('popover-section-event-date-places')).toHaveCount(0);
  await expect(page.getByTestId('popover-chip-map')).toHaveCount(0);
});

test("Revelation's own real superscription (REV.1.1, the source of the book's own traditional name) renders verbatim as the reader heading", async ({ page }) => {
  // REV.1.1 is this whole 22-book slice's own SOLE kjv_superscription
  // case (req 2's own "epistles have almost no canon-text titles" -- every
  // other W5 container is atlas_section, our own CC0 sectioning) -- the
  // one place in Romans-Revelation where the KJV's own text supplies a
  // real, whole-book title, exactly as Isaiah 13's own burden-
  // superscription did for the Prophets (w4-passages.spec.ts).
  const detail = await api.event('rev_prologue_and_greeting');
  expect(detail.title).toBe(
    'The Revelation of Jesus Christ, which God gave unto him, to shew unto his servants things which must shortly come to pass; and he sent and signified it by his angel unto his servant John:'
  );
  expect(detail.kind).toBe('general');
  expect(detail.when).toBeUndefined();

  await page.goto('/read/REV/1');
  const heading = page.getByTestId('pericope-heading-rev_prologue_and_greeting');
  await expect(heading).toBeVisible();
  await expect(heading).toHaveText(detail.title);
  await heading.click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);
  await expect(page.getByTestId('popover-section-event-date-places')).toHaveCount(0);
});

test('Revelation 2-3: the seven letters to the churches each anchor their own SEPARATE heading (the master brief\'s own named per-vision granularity)', async ({ page }) => {
  // req 2's own named Revelation granularity, verbatim: "the seven
  // letters individually (REV.2-3)". Chapter 2 alone carries four of the
  // seven (Ephesus, Smyrna, Pergamos, Thyatira); the remaining three
  // (Sardis, Philadelphia, Laodicea) are chapter 3's own opening headings
  // -- never one bundled "the seven letters" mega-container.
  const chapterOut = await api.chapter('REV.2');
  const headed = chapterOut.verses.filter((v: any) => v.heading);
  expect(headed.map((v: any) => v.heading.event_id)).toEqual([
    'rev_letter_to_ephesus',
    'rev_letter_to_smyrna',
    'rev_letter_to_pergamos',
    'rev_letter_to_thyatira',
  ]);

  await page.goto('/read/REV/2');
  await expect(page.getByTestId('pericope-heading-rev_letter_to_ephesus')).toHaveText('The letter to the church at Ephesus');
  await expect(page.getByTestId('pericope-heading-rev_letter_to_smyrna')).toHaveText('The letter to the church at Smyrna');
  await expect(page.getByTestId('pericope-heading-rev_letter_to_pergamos')).toHaveText('The letter to the church at Pergamos');
  await expect(page.getByTestId('pericope-heading-rev_letter_to_thyatira')).toHaveText('The letter to the church at Thyatira');

  // Each letter is independently explorable (ONE-RULE) -- opening Ephesus's
  // own popover shows Ephesus's own title, not a bundled "seven letters" title.
  await page.getByTestId('pericope-heading-rev_letter_to_ephesus').click();
  await expect(page.getByTestId('popover-title')).toHaveText('The letter to the church at Ephesus');
});
