import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch CORP-1 (R3/R5): the Book of Concord structure browser. Coverage: tab
// navigation from both existing pages; part/article/paragraph browse (the
// corpus's OWN shape, not scripture locus) + explore-on-click (ONE-RULE,
// with REAL paragraph text -- unlike Kretzmann, Concord's own reading-spine
// fetch already carries full text, no disclosed gap here); read-beside split
// with clean degradation (R3: "works via VC-1's generality; no follow
// anything").

test('CONCORD-1: nav-concord reaches /concord from both Reader and World', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByTestId('nav-concord')).toBeVisible();
  await page.getByTestId('nav-concord').click();
  await expect(page).toHaveURL(/\/concord$/);
  await expect(page.getByTestId('concord-page')).toBeVisible();

  await page.goto('/world');
  await expect(page.getByTestId('nav-concord')).toBeVisible();
  await page.getByTestId('nav-concord').click();
  await expect(page).toHaveURL(/\/concord$/);
  await expect(page.getByTestId('concord-page')).toBeVisible();
});

test('CONCORD-2: opens on the Preface (BoC 1.1.1) and every rendered row is real, non-fabricated paragraph text', async ({ page }) => {
  const ground = await api.reading('BoC 1.1.1', 20, { corpus: 'concord' });
  expect(ground.units.length).toBeGreaterThan(0);

  await page.goto('/concord');
  await expect(page.getByTestId('concord-page')).toBeVisible();
  await expect(page.getByTestId('concord-position')).toContainText('BoC 1.1.1');

  for (const unit of ground.units) {
    const row = page.getByTestId(`concord-unit-${unit.ref.replace(/ /g, '-').replace(/\./g, '-')}`);
    await expect(row).toBeVisible();
    await expect(row).toContainText(unit.text);
  }
});

test('CONCORD-3: the part/article/paragraph picker navigates the corpus\'s OWN shape -- jumping to 7.2.1 shows the real First Commandment text', async ({ page }) => {
  const ground = await api.reading('BoC 7.2.1', 1, { corpus: 'concord' });
  expect(ground.units).toHaveLength(1);
  const realText = ground.units[0].text as string;

  await page.goto('/concord');
  await page.getByTestId('concord-picker-part').fill('7');
  await page.getByTestId('concord-picker-article').fill('2');
  await page.getByTestId('concord-picker-paragraph').fill('1');
  await page.getByTestId('concord-picker-go').click();

  await expect(page.getByTestId('concord-position')).toContainText('BoC 7.2.1');
  await expect(page.getByTestId('concord-unit-BoC-7-2-1')).toContainText(realText);
});

test('CONCORD-4 (ONE-RULE): plain click on a paragraph row opens the existing explore/popover, carrying the REAL full paragraph text', async ({ page }) => {
  const ground = await api.reading('BoC 1.1.1', 1, { corpus: 'concord' });
  const realText = ground.units[0].text as string;

  await page.goto('/concord');
  await expect(page.getByTestId('concord-unit-BoC-1-1-1')).toBeVisible();

  await page.getByTestId('concord-unit-BoC-1-1-1').click();

  await expect(page.getByTestId('popover-title')).toContainText('BoC 1.1.1');
  await expect(page.getByTestId('popover-body')).toContainText(realText);
});

test('CONCORD-5: next/previous page the reading spine', async ({ page }) => {
  await page.goto('/concord');
  await expect(page.getByTestId('concord-next')).toBeVisible();
  await expect(page.getByTestId('concord-prev')).toHaveCount(0); // nothing precedes the Preface's own first paragraph

  const firstPosition = await page.getByTestId('concord-position').textContent();
  await page.getByTestId('concord-next').click();
  await expect(page.getByTestId('concord-position')).not.toHaveText(firstPosition ?? '');
  await expect(page.getByTestId('concord-prev')).toBeVisible();
});

test('CONCORD-6: declares its own "read-beside" hatch -- split opens with Concord hosting, Reader as a genuine, live guest', async ({ page }) => {
  await page.goto('/concord');
  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page.getByTestId('split-open-concord')).toBeVisible();

  await page.getByTestId('split-open-concord').click();

  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('split-divider')).toBeVisible();
  await expect(page.getByTestId('concord-page')).toBeVisible(); // both members live
  await expect(page.getByTestId('split-open-concord')).toHaveCount(0);

  await expect(page.getByTestId('reader-root')).toBeVisible();
  await expect(page.getByTestId('verse-line-1')).toBeVisible();
  await expect(page).toHaveURL(/\/concord$/); // Concord stays the route; no navigation

  await page.getByTestId('split-close-reader-guest').click();
  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page.getByTestId('concord-page')).toBeVisible();
});

test('CONCORD-7 (degraded-link law): concord+reader has no BearsWindow/BearsLocus member -- no follow chip, no follow link, no world map, ever', async ({ page }) => {
  await page.goto('/concord');
  await page.getByTestId('split-open-concord').click();
  await expect(page.getByTestId('split-view')).toBeVisible();

  await expect(page.getByTestId('follow-chip')).toHaveCount(0);
  await expect(page.getByTestId('mode-chip')).toHaveCount(0);
  await expect(page.getByTestId('world-map')).toHaveCount(0);
});
