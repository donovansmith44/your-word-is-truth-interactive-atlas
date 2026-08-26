import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch S ("document our sources for everything and give it a dedicated
// page on the site"), requirement 4: page reachable from the nav (see
// smoke.spec.ts's own "credits link navigates to the Sources page"),
// every source row present (derived from the same source of truth, never
// a hardcoded list -- SOURCES-1 below), Esri/Leaflet attribution intact
// on the map itself AND on the page (SOURCES-2), contrast/viewport floors
// (SOURCES-3/4).

test.describe('Sources page (Batch S)', () => {
  test('SOURCES-1: every /api/sources entry renders as its own source-{id} card -- page is data-driven, not hardcoded', async ({ page }) => {
    const doc = await api.sources();
    expect(doc.categories.length).toBeGreaterThan(0);
    expect(doc.sources.length).toBeGreaterThan(0);

    await page.goto('/sources');
    await expect(page.getByTestId('sources-page')).toBeVisible();

    for (const category of doc.categories) {
      await expect(page.getByTestId(`sources-category-${category.id}`)).toContainText(category.label);
    }
    for (const source of doc.sources) {
      const card = page.getByTestId(`source-${source.id}`);
      await expect(card).toBeVisible();
      await expect(card).toContainText(source.title);
      await expect(card).toContainText(source.license);
    }

    // Bidirectional: the page renders EXACTLY the API's own source set, no
    // more, no fewer -- proves this is genuinely data-driven (requirement
    // 3), never a hardcoded list that could silently drift from
    // /api/sources. Scoped to .source-card (not every `[data-testid^=
    // "source-"]`) so an optional per-row `source-link-{id}` anchor never
    // inflates the count.
    const cardCount = await page.locator('.source-card').count();
    expect(cardCount).toBe(doc.sources.length);
  });

  test('SOURCES-2: Esri and Leaflet attribution are intact on the map itself AND on the Sources page', async ({ page }) => {
    // On the map itself -- Leaflet's own attribution control (untouched by
    // this batch) still carries the Esri tile service's own copyrightText
    // verbatim, per map.js's TILE_ATTRIBUTION -- this batch never changes
    // the map, only documents it.
    await page.goto('/world');
    const mapAttribution = page.locator('.leaflet-control-attribution');
    await expect(mapAttribution).toBeVisible();
    await expect(mapAttribution).toContainText('Esri');

    // On the Sources page -- the same license terms, generated straight
    // from LICENSES.md's own per-source table (never hardcoded prose).
    await page.goto('/sources');
    await expect(page.getByTestId('source-esri-tiles')).toContainText('Esri');
    await expect(page.getByTestId('source-esri-tiles')).toContainText('Copyright');
    await expect(page.getByTestId('source-leaflet')).toContainText('BSD-2-Clause');
    await expect(page.getByTestId('source-leaflet')).toContainText('Leaflet');
  });

  // Parchment contrast >= 10:1 (batch-s-brief.md requirement 2) -- computed
  // live against the Sources page's own background, the same way NAV-4
  // (reader.spec.ts) already establishes for --ink on --parchment
  // (13.98:1). rgb(43,33,23) is --ink (#2B2117), rgb(246,241,229) is
  // --parchment (#F6F1E5).
  test('SOURCES-3: source-card body text clears the >=10:1 parchment contrast floor', async ({ page }) => {
    await page.goto('/sources');
    const firstCard = page.locator('.source-card').first();
    await expect(firstCard).toBeVisible();

    const [whatColor, titleColor, bg] = await page.evaluate(() => {
      const card = document.querySelector('.source-card')!;
      return [
        getComputedStyle(card.querySelector('.source-what')!).color,
        getComputedStyle(card.querySelector('.source-title')!).color,
        getComputedStyle(document.querySelector('.sources-page')!).backgroundColor,
      ];
    });
    expect(bg).toBe('rgb(246, 241, 229)');
    expect(whatColor).toBe('rgb(43, 33, 23)');
    expect(titleColor).toBe('rgb(43, 33, 23)');
  });

  // Desktop+tablet >=1024px (batch-s-brief.md requirement 2) through
  // ultrawide -- no horizontal overflow at either floor, the same
  // "nothing breaks between 1024px and ultrawide" quality floor
  // design-direction.md states for every page.
  test('SOURCES-4: no horizontal overflow at the 1024px floor or ultrawide', async ({ page }) => {
    for (const width of [1024, 2560]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto('/sources');
      await expect(page.getByTestId('sources-page')).toBeVisible();
      const [scrollWidth, clientWidth] = await page.evaluate(() => [
        document.documentElement.scrollWidth,
        document.documentElement.clientWidth,
      ]);
      // +1px: sub-pixel layout rounding tolerance, not a real overflow budget.
      expect(scrollWidth, `width=${width}`).toBeLessThanOrEqual(clientWidth + 1);
    }
  });
});
