import { test, expect } from '@playwright/test';

// D5 (owner, 2026-09-15, verbatim: "when we click on a person's name
// there's no point to just see every verse that name is mentioned ... i
// want to see the years that person is alive -- years are explorable
// positions; the events (explorable) in which they are mentioned;
// optionally a family tree whose names are all explorable; the exception
// is God because he is eternal"). Every assertion below runs against the
// real compiled data through the live popover: Aaron (aaron_1, EXO.4.14 --
// the same mention MENTION-2 already pins) carries the source's own life
// dates (Theographic "-1574"/"-1451" -> -1575/-1452 through the ETL's pinned
// parse_theo_year convention), parents, siblings, partner, children and timeline; God (god_1324,
// GEN.1.1) is the curated eternal exception (data/curated/people-eternal.toml).

test('PERSON-1: a person card reads LIFE (years as explorable positions), EVENTS, FAMILY, then the verse list collapsed last', async ({ page }) => {
  await page.goto('/read/EXO/4');
  await page.getByTestId('verse-mention-person-14-aaron_1').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Aaron');

  // LIFE: the source's own life dates, never a corpus span dressed up as one.
  await expect(page.getByTestId('person-life-heading')).toHaveText('LIFE');
  await expect(page.getByTestId('person-life')).toHaveText('Born c. 1575 BC · Died c. 1452 BC');
  await expect(page.getByTestId('popover-chip-year-born')).toBeVisible();
  await expect(page.getByTestId('popover-chip-year-died')).toBeVisible();
  await expect(page.getByTestId('popover-chip-year-span')).toHaveCount(0);

  // EVENTS: Aaron's own participates-in frontier, each an explorable event.
  await expect(page.getByTestId('person-events-heading')).toHaveText(/^EVENTS \(\d+\)$/);
  const events = page.locator('[data-testid^="person-event-"]');
  expect(await events.count()).toBeGreaterThan(0);

  await expect(page.getByTestId('person-family-heading')).toHaveText('FAMILY');
  await expect(page.getByTestId('person-family-parents')).toHaveText('Parents (2)');
  await expect(page.getByTestId('person-parents-amram_242')).toHaveText('Amram');
  await expect(page.getByTestId('person-parents-jochebed_1645')).toHaveText('Jochebed');
  await expect(page.getByTestId('person-family-spouses')).toHaveText('Spouses (1)');
  await expect(page.getByTestId('person-spouses-elisheba_1162')).toBeVisible();
  await expect(page.getByTestId('person-family-children')).toHaveText('Children (4)');
  await expect(page.getByTestId('person-family-siblings')).toHaveText('Siblings (2)');
  await expect(page.getByTestId('person-siblings-moses_2108')).toHaveText('Moses');

  // The verse list is still there -- last, and collapsed until asked for.
  const disclosure = page.getByTestId('person-mentions-disclosure');
  await expect(page.getByTestId('person-mentions-heading')).toHaveText(/^MENTIONED IN SCRIPTURE \(\d+\)$/);
  await expect(disclosure).not.toHaveAttribute('open', '');
  const sections = page.locator('[data-testid^="popover-section-"]');
  const ids = await sections.evaluateAll(els => els.map(e => e.getAttribute('data-testid')));
  const order = ['popover-section-person-life', 'popover-section-person-events', 'popover-section-person-family', 'popover-section-person-mentions'];
  expect(ids.filter(id => order.includes(id!))).toEqual(order);

  // A sibling's name is an explorable person: it pushes Moses's own card.
  await page.getByTestId('person-siblings-moses_2108').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Moses');
  await expect(page.getByTestId('person-siblings-aaron_1')).toHaveText('Aaron');
});

test('PERSON-2: a year is an explorable position -- the born chip opens the world at that year', async ({ page }) => {
  await page.goto('/read/EXO/4');
  await page.getByTestId('verse-mention-person-14-aaron_1').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Aaron');
  await page.getByTestId('popover-chip-year-born').click();
  await expect(page).toHaveURL(/\/world\?from=-1575&to=-1575/);
});

test('PERSON-3: God is eternal -- no years, no year chips, the grounds are explorable verses', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('verse-mention-person-1-god_1324').first().click();
  await expect(page.getByTestId('popover-title')).toHaveText('God');
  await expect(page.getByTestId('person-life')).toHaveText('Eternal');
  await expect(page.locator('[data-testid^="popover-chip-year-"]')).toHaveCount(0);
  await expect(page.getByTestId('person-eternal-ground-PSA-90-2')).toHaveText('PSA.90.2');
  await expect(page.getByTestId('person-eternal-ground-REV-1-8')).toBeVisible();
  await page.getByTestId('person-eternal-ground-PSA-90-2').click();
  await expect(page.getByTestId('popover-title')).toHaveText('PSA.90.2');
});

test('PERSON-4: Jesus is the eternal Son of God, born of the Virgin Mary, the son of Joseph as was supposed', async ({ page }) => {
  await page.goto('/read/LUK/3');
  await page.getByTestId('verse-mention-person-23-jesus_905').first().click();
  await expect(page.getByTestId('popover-title')).toHaveText('Jesus');
  await expect(page.getByTestId('person-family-parents-eternal')).toHaveText('Father (eternal Son of God)');
  await expect(page.getByTestId('person-parents-eternal-god_1324')).toHaveText('God');
  await expect(page.getByTestId('person-family-parents-virgin')).toHaveText('Mother (born of the Virgin)');
  await expect(page.getByTestId('person-parents-virgin-mary_1938')).toBeVisible();
  await expect(page.getByTestId('person-family-parents-legal')).toHaveText('Legal father (as was supposed)');
  await expect(page.getByTestId('person-parents-legal-joseph_1715')).toBeVisible();
  await expect(page.getByTestId('person-family-parents')).toHaveCount(0);
  await expect(page.getByTestId('person-siblings-adam_78')).toHaveCount(0);
});
