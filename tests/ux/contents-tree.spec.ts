import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { RUNS_UI } from './lib/fc';

// D4 (owner, 2026-09-15, verbatim: "Table of contents = a tree. Current ToC
// navigation 'sucks'. Contents is a tree; clicking a node toggles visibility
// of its children. Stop at the level of ARTICLE (BoC) or TOPIC (Small
// Catechism). Pages are not a meaningful way of thinking about things.").
// The tree IS /api/contents/{corpus} -- the graph's own containment forest.

test('contents tree: roots collapsed, toggle shows children, toggle again hides them, chapter leaf navigates', async ({ page }) => {
  await page.goto('/read/JHN/3');
  await page.getByTestId('contents-open').click();
  const panel = page.getByTestId('contents-panel');
  await expect(panel).toBeVisible();
  await expect(panel.getByTestId('contents-current')).toContainText('JHN.3');
  // the current chapter's own book is open on arrival, its chapter current
  await expect(panel.getByTestId('contents-item-Container-bible-chapter-JHN-3')).toHaveAttribute('aria-current', 'true');
  await expect(panel.getByTestId('contents-toggle-Container-bible-book-JHN')).toHaveAttribute('aria-label', /Collapse/);

  const gen = panel.getByTestId('contents-toggle-Container-bible-book-GEN');
  await expect(panel.getByTestId('contents-node-Container-bible-chapter-GEN-1')).toHaveCount(0);
  await gen.click();
  await expect(panel.getByTestId('contents-node-Container-bible-chapter-GEN-1')).toBeVisible();
  await gen.click();
  await expect(panel.getByTestId('contents-node-Container-bible-chapter-GEN-1')).toHaveCount(0);
  // a root LABEL toggles too (the panel's own click grammar)
  await panel.getByTestId('contents-node-Container-bible-book-GEN').click();
  await expect(panel.getByTestId('contents-node-Container-bible-chapter-GEN-2')).toBeVisible();
  await panel.getByTestId('contents-node-Container-bible-chapter-GEN-2').click();
  await expect(page).toHaveURL(/\/read\/GEN\/2/);
  await expect(page.getByTestId('contents-panel')).toHaveCount(0);
});

test('contents tree property: any toggle sequence applied twice restores the visible rows', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('contents-open').click();
  const panel = page.getByTestId('contents-panel');
  await expect(panel.locator('[role=treeitem]').first()).toBeVisible();
  const books = ['GEN', 'EXO', 'PSA', 'ISA', 'MAT', 'JHN', 'ROM', 'REV'];
  await fc.assert(fc.asyncProperty(fc.array(fc.constantFrom(...books), { maxLength: 6 }), async seq => {
    const before = await panel.locator('[role=treeitem]').allInnerTexts();
    for (const pass of [0, 1]) {
      for (const b of seq) {
        await panel.getByTestId(`contents-toggle-Container-bible-book-${b}`).click();
      }
    }
    expect(await panel.locator('[role=treeitem]').allInnerTexts()).toEqual(before);
  }), { numRuns: RUNS_UI });
});

test('the hamburger reaches the contents tree from any page', async ({ page }) => {
  await page.goto('/world');
  await page.getByTestId('hamburger-menu').click();
  await page.getByTestId('hamburger-contents').click();
  const panel = page.getByTestId('contents-panel');
  await expect(panel).toBeVisible();
  await panel.getByTestId('contents-toggle-Container-bible-book-PSA').click();
  await panel.getByTestId('contents-node-Container-bible-chapter-PSA-23').click();
  await expect(page).toHaveURL(/\/read\/PSA\/23/);
});

test('concord contents: documents then articles; a document jumps to its opening paragraph, an article deep-links to its first paragraph', async ({ page }) => {
  await page.goto('/concord');
  const tree = page.getByTestId('concord-contents-tree');
  await expect(tree).toBeVisible();
  await expect(tree.locator('[role=treeitem]')).toHaveCount(11); // ten documents + the current (first) one's articles? no: roots only + the open Preface
  // the document being read is current and open; every other document is collapsed
  await expect(page.getByTestId('concord-toc-part-1')).toHaveAttribute('aria-current', 'true');
  const augsburg = tree.getByTestId('contents-toggle-Container-concord-doc-augsburg-confession');
  await augsburg.click();
  const article = tree.locator('[data-testid^="contents-node-Container-concord-art-augsburg-confession-"]').first();
  await expect(article).toBeVisible();
  await article.click();
  await expect(page).toHaveURL(/\/concord\?ref=\d+\.\d+\.\d+/);
  await expect(page.getByTestId('concord-position')).toContainText(/BoC 3\.\d+\.\d+/);
});
