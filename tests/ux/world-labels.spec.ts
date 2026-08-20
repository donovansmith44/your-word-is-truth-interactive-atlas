import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { independentlyHoverableIds } from './lib/hoverSafety';

// Batch R requirement 2 (regression + enhancement, user 2026-08-19: "i can
// no longer click on a location's (like Beersheba) name, where i used to be
// able to and i would get the est/dest dates. we need that functionality
// back"): a place's own label (.atlas-label / .quiet-label) is a full
// hover/click target equivalent to its dot -- see CONTRACT.md's own LABEL-1
// note. independentlyHoverableIds (lib/hoverSafety.ts) filters out any place
// whose real rendered marker sits too close to a neighbor for a forced hover
// to be trusted -- the SAME safety net WORLD-2/world-hover-text.spec.ts
// already rely on, needed here too since the label sits right next to the
// exact same dot.

test('LABEL-1: hovering a LIT place\'s label opens the same place-card a hover on its dot would', async ({ page }) => {
  const w = { from: -1446, to: -1406 }; // exodus window: rich scene
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  const safeIds = await independentlyHoverableIds(page, scene.places.map((p: any) => p.id));
  const p = scene.places.find((pl: any) => safeIds.has(pl.id));
  expect(p, 'expected at least one independently-hoverable lit place').toBeTruthy();

  const label = page.getByTestId(`marker-${p.id}`).locator('.atlas-label');
  await expect(label).toBeVisible();
  await label.hover({ force: true });

  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();
  await expect(page.getByTestId('place-card-title')).toHaveText(p.display_name);
});

test('LABEL-1: clicking a LIT place\'s label pins the card exactly like clicking its dot (PIN-1)', async ({ page }) => {
  const w = { from: -1446, to: -1406 };
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  const safeIds = await independentlyHoverableIds(page, scene.places.map((p: any) => p.id));
  const p = scene.places.find((pl: any) => safeIds.has(pl.id));
  expect(p).toBeTruthy();

  const label = page.getByTestId(`marker-${p.id}`).locator('.atlas-label');
  await label.click({ force: true });

  const card = page.getByTestId('place-card');
  await expect(card).toHaveAttribute('data-pinned', 'true');
  await expect(page.getByTestId('place-card-close')).toBeVisible();

  // Moving away must NOT close a pinned card (PIN-1) -- proves the pin
  // really landed via the label click, not a transient hover artifact.
  await page.mouse.move(5, 5);
  await page.waitForTimeout(600);
  await expect(card).toBeVisible();
  await expect(card).toHaveAttribute('data-pinned', 'true');
});

// A pinned label's own card title still promotes into a real PlaceNode
// popover -- est/dest dates (now rendered via REGISTRY-1's own
// popover-place-date-established/-destroyed) are reachable in exactly 2
// clicks from the label: label pins the card, title opens the popover.
// Jerusalem is heavily curated (established/destroyed both present) and
// event-bearing at essentially every historical window, so it is present
// (lit or quiet) here regardless of exact scene composition.
test('LABEL-1: est/dest dates are reachable in <=2 clicks from a label (regression: "i used to be able to")', async ({ page }) => {
  await page.goto('/world?from=-1000&to=-900');
  const jerusalemMarker = page.getByTestId('marker-jerusalem').or(page.getByTestId('quiet-marker-jerusalem'));
  await expect(jerusalemMarker).toBeAttached();

  const label = jerusalemMarker.locator('.atlas-label, .quiet-label');
  await label.click({ force: true }); // click 1: pins the card
  await expect(page.getByTestId('place-card')).toHaveAttribute('data-pinned', 'true');

  await page.getByTestId('place-card-title').click(); // click 2: promotes into the popover
  await expect(page.getByTestId('popover')).toBeVisible();
  await expect(page.getByTestId('popover-place-date-established')).toBeVisible();
});

// Polity/landmark labels stay entirely non-interactive -- LABEL-1's own
// explicit scope boundary.
test('LABEL-1: polity and landmark labels stay non-interactive', async ({ page }) => {
  await page.goto('/world?from=-1446&to=-1400');
  const polityLabel = page.getByTestId(/^polity-label-/).first();
  await expect(polityLabel).toBeAttached();
  const polityPointerEvents = await polityLabel.evaluate(el => getComputedStyle(el).pointerEvents);
  expect(polityPointerEvents).toBe('none');

  const landmarkLabel = page.getByTestId(/^landmark-/).first();
  await expect(landmarkLabel).toBeAttached();
  const landmarkPointerEvents = await landmarkLabel.evaluate(el => getComputedStyle(el).pointerEvents);
  expect(landmarkPointerEvents).toBe('none');
});
