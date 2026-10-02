import { expect, type Page } from '@playwright/test';

export async function expectPopoverChips(page: Page, chipTestIds: string[]): Promise<void> {
  await expect
    .poll(() => page.getByTestId('popover').locator('.popover-head-actions [data-testid]').evaluateAll(
      els => els.map(el => el.getAttribute('data-testid'))))
    .toEqual(chipTestIds);
}

export async function popoverSectionIds(page: Page, pattern: RegExp = /^popover-section-/): Promise<string[]> {
  return page.getByTestId(pattern).evaluateAll(els => els.map(el => el.getAttribute('data-testid') ?? ''));
}

export async function popoverSectionsHolding(page: Page, held: string[], pattern: RegExp = /^popover-section-/): Promise<string[]> {
  await expect
    .poll(async () => {
      const present = await popoverSectionIds(page, pattern);
      return held.filter(id => !present.includes(id));
    })
    .toEqual([]);
  return popoverSectionIds(page, pattern);
}
