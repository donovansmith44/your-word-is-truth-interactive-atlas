import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbChapterRef, arbVerseRef } from './lib/canon';
import { fcAssert, RUNS_UI } from './lib/fc';

test('READ-1 + NAV-1: chapter deep links render exactly the TOC verses', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbChapterRef(toc), async c => {
    await page.goto(`/read/${c.book}/${c.chapter}`);
    await expect(page.getByTestId(/^verse-line-/)).toHaveCount(c.verses);
    await expect(page.getByTestId('verse-num-1')).toHaveText('1');
    await expect(page.getByTestId(`verse-num-${c.verses}`)).toHaveText(String(c.verses));
  }), RUNS_UI);
});

test('READ-2: verse popover shows the API text', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    const [b, c, v] = vref.split('.');
    await page.goto(`/read/${b}/${c}`);
    await page.getByTestId(`verse-num-${v}`).click();
    await expect(page.getByTestId('popover-title')).toHaveText(vref);
    const detail = await api.verse(vref);
    await expect(page.getByTestId('popover')).toContainText(detail.text.slice(0, 40));
  }), RUNS_UI);
});

test('READ-3: cross-ref chains push and pop breadcrumbs faithfully', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), fc.array(fc.nat(4), { maxLength: 3 }), async (vref, picks) => {
    const [b, c, v] = vref.split('.');
    await page.goto(`/read/${b}/${c}`);
    await page.getByTestId(`verse-num-${v}`).click();
    const titles = [vref];
    for (const pick of picks) {
      await page.getByTestId('popover-chip-xrefs').click();
      const items = page.getByTestId(/^xref-item-/);
      const n = await items.count();
      if (n === 0) break;
      const detail = await api.verse(titles[titles.length - 1]);
      for (let i = 0; i < Math.min(n, detail.cross_refs.length); i++) {   // list order == API order
        await expect(items.nth(i)).toContainText(detail.cross_refs[i].target);
      }
      const chosen = Math.min(pick, n - 1);
      const target = detail.cross_refs[chosen].target;
      await items.nth(chosen).click();
      const head = target.match(/^[A-Z0-9]{3}\.\d+\.\d+/)![0];
      await expect(page.getByTestId('popover-title')).toHaveText(head);
      titles.push(head);
    }
    while (titles.length > 1) {                                            // walk back restores each title
      await page.getByTestId('popover-breadcrumb-back').click();
      titles.pop();
      await expect(page.getByTestId('popover-title')).toHaveText(titles[titles.length - 1]);
    }
  }), RUNS_UI);
});
