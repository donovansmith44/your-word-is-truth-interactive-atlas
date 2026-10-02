import { expect, type Page } from '@playwright/test';

export async function expectPopoverChips(page: Page, chipTestIds: string[]): Promise<void> {
  await expect
    .poll(() => page.getByTestId('popover').locator('.popover-head-actions [data-testid]').evaluateAll(
      els => els.map(el => el.getAttribute('data-testid'))))
    .toEqual(chipTestIds);
}
