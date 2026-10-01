import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { elementEdge, elementNode, neighbourNode, positionNode } from './lib/edges';

const verseId = 'text-unit:EXO.14.21';

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
});

test('A-EDGES: a step onto an attestation shows the served edge under its two ends; its event end follows to the event and Back retraces to the verse', async ({ page }) => {
  const attests = await api.nodeEdges(verseId, 'attests');
  const entry = attests.entries[0];
  const [verseElement, edgeElement] = (await api.elements([verseId, entry.edge.id])).elements;
  const verse = elementNode(verseElement, verseId);
  const edge = elementEdge(edgeElement, entry.edge.id);
  const subject = positionNode(edge.subject, `${edge.id}'s subject`);
  const object = positionNode(edge.object, `${edge.id}'s object`);
  const event = neighbourNode(entry);
  await page.evaluate(save => localStorage.setItem('explorations-v3', JSON.stringify([save])), {
    id: 'edge', name: edge.label, createdUtc: '2026-10-01T00:00:00+00:00',
    start: { position: 'node', node: verse },
    steps: [{ kind: 'attests', target: { position: 'edge', edge: entry.edge } }],
  });

  await page.goto('/read/EXO/14');
  await page.getByTestId('hamburger-menu').click();
  await page.locator('[data-testid^="exploration-item-"] .hamburger-exploration-summary').click();
  await page.getByTestId('exploration-node-1').click();

  await expect(page.getByTestId('popover-card-title')).toHaveText(edge.label);
  await expect(page.getByTestId(`popover-end-${subject.id}`)).toHaveText(subject.label);
  await expect(page.getByTestId(`popover-end-${object.id}`)).toHaveText(object.label);
  await page.getByTestId(`popover-end-${event.id}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(event.label);
  await page.getByTestId('popover-breadcrumb-back').click();
  await expect(page.getByTestId('popover-card-title')).toHaveText(edge.label);
  await page.getByTestId('popover-breadcrumb-back').click();
  await expect(page.getByTestId('popover-title')).toHaveText(verse.label);
});
