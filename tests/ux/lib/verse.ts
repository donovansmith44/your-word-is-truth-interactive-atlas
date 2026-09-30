import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';

export async function openVerse(page: Page, verse: number | string, suffix = ''): Promise<void> {
  const row = page.getByTestId(`verse-line-${verse}${suffix}`);
  await expect(row).toBeVisible();
  await row.focus();
  await row.press('Enter');
}
