import type { Page } from '@playwright/test';

const SAFE_NEIGHBOR_PX = 26;

async function markerCenters(page: Page, ids: string[]): Promise<Map<string, { x: number; y: number }>> {
  const centers = new Map<string, { x: number; y: number }>();
  for (const id of ids) {
    const box = await page.getByTestId(`marker-${id}`).boundingBox();
    if (box) {
      centers.set(id, { x: box.x + box.width / 2, y: box.y + box.height / 2 });
    }
  }
  return centers;
}

async function otherCandidateCenters(page: Page): Promise<{ x: number; y: number }[]> {
  return page.evaluate(() => {
    const out: { x: number; y: number }[] = [];
    for (const el of document.querySelectorAll('[data-testid^="quiet-marker-"], [data-testid^="marker-cluster-"]')) {
      const r = (el as HTMLElement).getBoundingClientRect();
      if (r.width !== 0 || r.height !== 0) {
        out.push({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
      }
    }
    return out;
  });
}

async function labelIsVisible(page: Page, id: string): Promise<boolean> {
  const label = page.getByTestId(`marker-${id}`).locator('.atlas-label, .quiet-label');
  if ((await label.count()) === 0) {
    return true;
  }
  return label.first().isVisible();
}

export async function independentlyHoverableIds(page: Page, ids: string[]): Promise<Set<string>> {
  const centers = await markerCenters(page, ids);
  const others = await otherCandidateCenters(page);
  const safe = new Set<string>();
  for (const id of ids) {
    const a = centers.get(id);
    if (!a) {
      continue;
    }
    const clearOfPool = ids.every(otherId => {
      const b = centers.get(otherId);
      return otherId === id || !b || Math.hypot(a.x - b.x, a.y - b.y) >= SAFE_NEIGHBOR_PX;
    });
    const clearOfOthers = others.every(b => Math.hypot(a.x - b.x, a.y - b.y) >= SAFE_NEIGHBOR_PX);
    if (clearOfPool && clearOfOthers && (await labelIsVisible(page, id))) {
      safe.add(id);
    }
  }
  return safe;
}
