import { test, expect, type Page } from '@playwright/test';
import { api } from './lib/api';
import { openVerse } from './lib/verse';
import type { NodeRef } from './lib/edges';

type ServedAnchor = { kind: string; node: NodeRef };

async function anchorIds(verseId: string): Promise<string[]> {
  const anchors = (await api.node(verseId)).text.anchors as ServedAnchor[];
  return anchors.map((anchor, n) => `popover-anchor-${anchor.kind}-${anchor.node.id}-${n}`);
}

async function shownAffordances(page: Page): Promise<{ anchors: (string | null)[]; cites: string }> {
  await expect(page.getByTestId('popover-section-cites-heading')).toBeVisible();
  return {
    anchors: await page.getByTestId('popover-text').locator('[data-testid]').evaluateAll(els => els.map(el => el.getAttribute('data-testid'))),
    cites: (await page.getByTestId('popover-section-cites-heading').textContent()) ?? '',
  };
}

test.describe('M-D4: the recursive reader', () => {
  test('RECURSE-1: a verse carries the identical affordance set whether opened in the main reader or reached by a cross reference', async ({ page }) => {
    // Arrange
    const expectedAnchors = await anchorIds('text-unit:2CO.4.6');
    await page.goto('/read/2CO/4');
    await openVerse(page, 6);
    await expect(page.getByTestId('popover-title')).toHaveText('2CO.4.6');
    const fromReader = await shownAffordances(page);
    await page.getByTestId('popover-close').click();
    await expect(page.getByTestId('popover')).toHaveCount(0);

    // Act
    await page.goto('/read/GEN/1');
    await openVerse(page, 3);
    await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.3');
    await page.getByTestId('popover-link-cites-text-unit:2CO.4.6').click();
    await expect(page.getByTestId('popover-title')).toHaveText('2CO.4.6');
    const recursed = await shownAffordances(page);

    // Assert
    expect(fromReader.anchors).toEqual(expectedAnchors);
    expect(recursed).toEqual(fromReader);
    await expect(page.getByTestId('popover-breadcrumb-back')).toBeVisible();
  });

  test('RECURSE-3: reading a cross-referenced verse in context lands in its chapter, where its pericope heading interleaves above it', async ({ page }) => {
    // Arrange
    const gen1 = await api.chapter('GEN.1');
    const v1Heading = gen1.verses.find((v: any) => v.verse === 1).heading;
    expect(v1Heading, 'GEN.1.1 must anchor a real heading for this test to mean anything').toBeTruthy();
    await page.goto('/read/JHN/1');
    await openVerse(page, 1);
    await expect(page.getByTestId('popover-title')).toHaveText('JHN.1.1');
    await page.getByTestId('popover-link-cites-text-unit:GEN.1.1').click();
    await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.1');

    // Act
    await page.getByTestId('popover-chip-context').click();

    // Assert
    await expect(page).toHaveURL(/\/read\/GEN\/1\b/);
    const heading = page.getByTestId(`pericope-heading-${v1Heading.event_id}`);
    await expect(heading).toHaveText(v1Heading.title);
    const headingBox = await heading.boundingBox();
    const v1Box = await page.getByTestId('verse-line-1').boundingBox();
    expect(headingBox && v1Box && headingBox.y).toBeLessThan(v1Box!.y);
  });

  test('RECURSE-4: the verse popover\'s own focus text carries an in-text mention link too', async ({ page }) => {
    // Arrange
    const [godAnchor] = await anchorIds('text-unit:2CO.4.4');
    await page.goto('/read/2CO/4');
    await openVerse(page, 4);
    await expect(page.getByTestId('popover-title')).toHaveText('2CO.4.4');
    const mention = page.getByTestId(godAnchor);
    await expect(mention).toHaveText('God');

    // Act
    await mention.click();

    // Assert
    await expect(page.getByTestId('popover-title')).toHaveText('God');
    await expect(page.getByTestId('popover-breadcrumb-back'), 'a PUSH onto the SAME stack, not a dead end').toBeVisible();
  });
});
