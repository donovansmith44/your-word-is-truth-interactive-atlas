import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { edgePosition, positionNode } from './lib/edges';

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
});

test('A-EDGES: an edge is explored as its served card under its two ends, and its from end follows to the event', async ({ page }) => {
  const attests = await api.nodeEdges('text-unit:EXO.14.21', 'attests');
  const card = await api.edge(attests.entries[0].edge);
  const from = positionNode(card.from, `${card.id}'s from end`);
  const to = positionNode(card.to, `${card.id}'s to end`);
  await page.evaluate(save => localStorage.setItem('explorations-v3', JSON.stringify([save])), {
    id: 'edge', name: card.label, createdUtc: '2026-10-01T00:00:00+00:00', start: edgePosition(card), steps: [],
  });

  await page.goto('/read/EXO/14');
  await page.getByTestId('hamburger-menu').click();
  await page.locator('[data-testid^="exploration-item-"] .hamburger-exploration-summary').click();
  await page.getByTestId('exploration-node-0').click();

  await expect(page.getByTestId('popover-card-title')).toHaveText(card.label);
  await expect(page.getByTestId(`popover-up-to-${to.id}`)).toHaveText(to.label);
  await page.getByTestId(`popover-up-from-${from.id}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(from.label);
});
