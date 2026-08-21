import type { Page } from '@playwright/test';

// Requirement 0b/0c (Batch C2): the ember marker's own >=14px hit target
// (map.js's NUDGE_STEP_DEG comment has the full empirical derivation) means
// two markers whose REAL rendered centers sit closer together than that can
// no longer be trusted to resolve a forced hover to the intended one -- not
// only for exact/near-coincident places nudgeCloseLatLng fixes, but for any
// two genuinely different, unnudged places a given scene's own fitScene zoom
// happens to render close together. A concrete, real example (not a
// hypothetical): "Philippi" and its own port city "Neapolis" (apostolic
// window, AD 46-48) are 13.92km apart in reality -- never coincident, never
// nudged -- yet render within single-digit px of each other at that scene's
// natural zoom. `document.elementsFromPoint` at Philippi's own marker center
// resolved to NEAPOLIS on top, even though Philippi is the LATER sibling in
// DOM order (confirmed live, not assumed): Leaflet's own default marker
// stacking assigns each marker a z-index from its OWN screen Y-position (a
// marker lower on screen paints in front of one higher up, so nearer pins
// visually overlap farther ones correctly) -- once two markers' expanded hit
// areas overlap at all, THAT, not DOM/insertion order, decides which one a
// pointer at the ambiguous pixel actually lands on.
//
// This is the shared, self-updating safety check every UI spec that forces
// a hover onto a specific marker uses to avoid depending on luck: it reads
// each candidate's REAL rendered position on the CURRENTLY loaded page (not
// a hardcoded id list or a offline km estimate, so it tracks the live app
// exactly and never goes stale as curated data or nudge tuning changes).
//
// RAISED 20 -> 26, Batch HOTFIX-2: 20px was narrower than the ember marker's
// own actual hit-box DIAMETER (10px core + `::after` inset:-8px padding,
// Batch C2's own derivation -- 13px RADIUS, i.e. 26px across, documented in
// map.js's own NUDGE_TRIGGER_PX comment) -- two markers 20-26px apart clear
// this file's own OLD bound while their real hit circles still overlap, a
// latent gap this constant's own reasoning above already implies (">=14px
// hit target... centers closer together than that can no longer be
// trusted") but its chosen NUMBER didn't actually close. Exposed live by
// this batch's own nudge redesign: map.js's close-marker nudge used to move
// a colliding marker by a fixed, large geographic delta (tens of km),
// which -- as an unintended side effect -- kept almost every real pair
// either far apart or freshly scattered far apart, rarely landing in this
// 20-26px gap; now that the nudge is a small, bounded SCREEN-PIXEL amount
// (map.js's own NUDGE_STEP_PX, ~16px) markers that are genuinely close in
// real life legitimately render close together on screen too (correctly --
// see map.js's own applyMarkerNudges comment), which surfaced two real
// pairs sitting exactly in this gap in the exodus window (mount-sinai/
// rephidim and elim/marah, ~21px apart each) and, transitively, broke a
// LATER, otherwise-independent hover on an unrelated marker in the same
// property run (a stuck/mis-set card from one of those two ambiguous pairs
// carrying over). Matches NUDGE_TRIGGER_PX exactly, deliberately -- both
// constants now name the SAME real geometric fact (the marker hit-box's own
// diameter), just consumed by two different files for two different
// purposes (map.js: when to nudge a marker; this file: when a forced test
// hover can no longer be trusted) -- see NUDGE_TRIGGER_PX's own comment.
export const SAFE_NEIGHBOR_PX = 26;

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

// Batch W1 self-review finding: a candidate's MARKER (dot) having no close
// neighbor (the check below) does NOT mean its own LABEL is safe to force a
// hover/click onto -- labels go through a SEPARATE, coarser collision-
// damping pass (map.js's own COLLISION_CELL_PX 72px grid, keyed off each
// label's own on-screen anchor, competing against every OTHER lit place's
// label AND every landmark/quiet-place label in the scene, not just this
// caller's own candidate pool) and can be hidden even when the dot itself
// is well clear of every other dot -- a real, live case, not a hypothetical:
// in the exodus window (-1446..-1406, WORLD-1's own reference scene),
// Hebron's own marker sits with no other DOT within 26px, yet 9 of the
// scene's 20 place labels (including Hebron's) are hidden by collision
// damping at that scene's natural zoom -- confirmed via a throwaway
// diagnostic script reading `.atlas-label`'s own `visibility`, not assumed.
// Checked here too (in addition to the marker-proximity check below) so a
// caller forcing a hover/click onto a LABEL specifically (world-labels.spec.ts)
// never lands on a place whose label collision-damping already dropped --
// harmless for a caller that only ever targets the DOT (a hidden label
// simply narrows the safe pool a little further, never widens it past what
// was already correct).
async function labelIsVisible(page: Page, id: string): Promise<boolean> {
  const label = page.getByTestId(`marker-${id}`).locator('.atlas-label, .quiet-label');
  if ((await label.count()) === 0) {
    return true; // no label at all for this marker kind -- nothing to hide, not this check's concern
  }
  return label.first().isVisible();
}

// Every id in `ids` whose real rendered marker center is at least
// SAFE_NEIGHBOR_PX away from every OTHER id's own center, on the page as
// currently rendered, AND whose own label (if any) survived collision
// damping. `ids` is deliberately the caller's own candidate pool (not
// necessarily every place in the scene) so a caller already filtering by
// some other criterion (e.g. "has a place-card-more button") only pays for
// measuring the markers it actually cares about.
export async function independentlyHoverableIds(page: Page, ids: string[]): Promise<Set<string>> {
  const centers = await markerCenters(page, ids);
  const safe = new Set<string>();
  for (const id of ids) {
    const a = centers.get(id);
    if (!a) {
      continue;
    }
    const clear = ids.every(otherId => {
      if (otherId === id) {
        return true;
      }
      const b = centers.get(otherId);
      return !b || Math.hypot(a.x - b.x, a.y - b.y) >= SAFE_NEIGHBOR_PX;
    });
    if (clear && (await labelIsVisible(page, id))) {
      safe.add(id);
    }
  }
  return safe;
}
