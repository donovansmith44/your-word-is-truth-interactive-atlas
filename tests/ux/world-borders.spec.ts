import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch B2 ("borders v2, the cartographer's edition"): every test in this
// file was rewritten against the new per-polity timerange border model
// (GET /api/polities?from=&to=) -- the old snapshot-year GeoJSON model
// (GET /api/borders, the "Borders c. X" tag, "nearest snapshot") is gone
// entirely; see CONTRACT.md's own BORDER-1 note for the invariant every
// test below ultimately checks a facet of. Black-box mirror of map.js's
// own slugify (kebab-case of a name) -- must match it exactly for
// landmark-{slug}/polity-label-{slug} testids to resolve. Kept local (no
// client imports, per this suite's own black-box rule).
function kebab(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
}

test('BORDERS-1: every era resolves to at least one polity (exhaustive)', async () => {
  const eras = await api.eras();
  expect(eras.length).toBeGreaterThan(0);
  for (const e of eras) {
    const res = await api.polities(e.from_year, e.to_year);
    expect(res.__status, `era ${e.id}: ${JSON.stringify(res)}`).toBeUndefined();
    expect(Array.isArray(res.polities), `era ${e.id}`).toBe(true);
    expect(res.polities.length, `era ${e.id} (${e.from_year}..${e.to_year}) unexpectedly empty`).toBeGreaterThan(0);
  }
});

// Requirement 6: "single-era window: solid ring + wash". -30..100 (early-
// church) intersects EXACTLY roman-empire's own "Roman Empire" era
// (-30..100) and parthian-empire's own single era -- neither polity has a
// second era anywhere near this window, so both render data-age="newest"
// (the single-era default): a plain solid line, no dash, and the full
// (0.32) wash opacity -- no year tag either (BORDERS-VISIBLE-ONLY-ONE-ERA
// never gets one, see CONTRACT's own polity-year-tag-* note).
test('BORDERS-2: single-era window renders a solid ring + full wash, no year tag', async ({ page }) => {
  await page.goto('/world?from=30&to=90');
  const line = page.getByTestId('polity-ring-roman-empire--30-0');
  await expect(line).toBeAttached();
  await expect(line).toHaveAttribute('data-age', 'newest');

  const style = await line.evaluate(el => {
    const cs = getComputedStyle(el);
    return { dasharray: cs.strokeDasharray, opacity: cs.strokeOpacity };
  });
  // "solid" -- no dash pattern at all (Chromium reports an unset
  // stroke-dasharray as the literal string "none").
  expect(style.dasharray).toBe('none');
  expect(parseFloat(style.opacity)).toBeGreaterThan(0.8); // .atlas-border-line[data-age="newest"] is 0.88

  const wash = page.locator('path.atlas-border-wash[data-age="newest"]').first();
  await expect(wash).toBeAttached();
  const washOpacity = await wash.evaluate(el => parseFloat(getComputedStyle(el).fillOpacity));
  expect(washOpacity).toBeCloseTo(0.32, 2);

  await expect(page.getByTestId(/^polity-year-tag-roman-empire-/)).toHaveCount(0);
});

test('BORDERS-3: every curated landmark is attached and styled by kind', async ({ page }) => {
  const landmarks = await api.landmarks();
  expect(landmarks.length).toBeGreaterThan(0);

  await page.goto('/world');

  for (const l of landmarks) {
    const slug = kebab(l.name);
    await expect(page.getByTestId(`landmark-${slug}`), `landmark "${l.name}" (kind ${l.kind})`).toBeAttached();
  }

  const water = landmarks.find((l: any) => l.kind === 'water');
  const mountain = landmarks.find((l: any) => l.kind === 'mountain');
  expect(water, 'expected at least one water landmark in the curated set').toBeTruthy();
  expect(mountain, 'expected at least one mountain landmark in the curated set').toBeTruthy();

  const waterStyle = await page.getByTestId(`landmark-${kebab(water.name)}`).evaluate(el => getComputedStyle(el).fontStyle);
  expect(waterStyle).toBe('italic');

  const mountainStyle = await page.getByTestId(`landmark-${kebab(mountain.name)}`).evaluate(el => getComputedStyle(el).fontStyle);
  expect(mountainStyle).not.toBe('italic');
});

test('BORDERS-4: scripture mode hides every polity ring and label', async ({ page }) => {
  await page.goto('/world?from=-1446&to=-1400'); // time mode first -- confirms rings exist to hide
  await expect(page.getByTestId(/^polity-ring-/).first()).toBeAttached();

  await page.goto('/world?ref=EXO.14');
  await expect(page.getByTestId(/^polity-ring-/)).toHaveCount(0);
  await expect(page.getByTestId(/^polity-label-/)).toHaveCount(0);
  await expect(page.locator('svg.atlas-borders')).toBeHidden();
});

// Requirement 6: "spanning window: >=2 rings, distinct dash classes, year
// tags". The batch brief's own named example -- 1600 BC to 900 BC -- spans
// Egypt's OWN border changes so thoroughly it actually catches THREE of
// egypt's own curated eras at once (found empirically while writing this
// test, a richer result than the brief's own two-era framing anticipated,
// not a bug): the tail of the Old/Middle Kingdom era (-4004..-1550, just
// its last 50 years), the New Kingdom empire (-1549..-1069, large, extends
// into Canaan), and the contracted post-Empire footprint (-1068..-332) --
// oldest/middle/newest respectively, dotted/dashed/solid, lightest/
// intermediate/full wash. A real bonus for this test: it's the one window
// in this app's own curated data that ever reaches the "middle" tier at
// all (every other multi-era window in the roster has exactly 2 visible
// eras of any one polity).
test('BORDERS-5: a spanning window (1600-900 BC) shows 3 rings of the same polity with distinct dash classes and year tags', async ({ page }) => {
  await page.goto('/world?from=-1600&to=-900');

  const oldest = page.getByTestId('polity-ring-egypt--4004-0');
  const middle = page.getByTestId('polity-ring-egypt--1549-0');
  const newest = page.getByTestId('polity-ring-egypt--1068-0');
  await expect(oldest).toBeAttached();
  await expect(middle).toBeAttached();
  await expect(newest).toBeAttached();
  await expect(oldest).toHaveAttribute('data-age', 'oldest');
  await expect(middle).toHaveAttribute('data-age', 'middle');
  await expect(newest).toHaveAttribute('data-age', 'newest');

  const [oldestDash, middleDash, newestDash] = await Promise.all([
    oldest.evaluate(el => getComputedStyle(el).strokeDasharray),
    middle.evaluate(el => getComputedStyle(el).strokeDasharray),
    newest.evaluate(el => getComputedStyle(el).strokeDasharray),
  ]);
  expect(newestDash, 'newest era: solid').toBe('none');
  expect(oldestDash, 'oldest era: dotted').not.toBe('none');
  expect(middleDash, 'middle era: dashed').not.toBe('none');
  // "distinct dash classes" -- all three read differently from one another.
  expect(new Set([oldestDash, middleDash, newestDash]).size).toBe(3);

  await expect(page.getByTestId('polity-year-tag-egypt--4004-0')).toBeVisible();
  await expect(page.getByTestId('polity-year-tag-egypt--1549-0')).toBeVisible();
  await expect(page.getByTestId('polity-year-tag-egypt--1068-0')).toBeVisible();
  await expect(page.getByTestId('polity-year-tag-egypt--4004-0')).toContainText('4004 BC');
  await expect(page.getByTestId('polity-year-tag-egypt--1549-0')).toContainText('1549 BC');
  await expect(page.getByTestId('polity-year-tag-egypt--1068-0')).toContainText('1068 BC');

  // "the polity name label renders once per era-name" -- all three eras
  // here are named "Egypt" (redraws, not renames), so exactly ONE label,
  // not three, even though three rings/year-tags are visible.
  await expect(page.getByTestId('polity-label-egypt')).toHaveCount(1);
});

// Requirement 1's own color_key refinement, verified LIVE end to end
// (server hash -> wire -> map.js POLITY_TINTS lookup): egypt's two windows
// below each show a DIFFERENT era name ("Egypt" vs "Ptolemaic Egypt") with
// NOTHING else in common except the shared polity id -- if color_key were
// still hashed from the era NAME (the retired Batch C2 behavior), these
// two would very likely disagree.
test('BORDERS-6: a polity keeps the same fill color across a rename (color_key hashes the id, not the era name)', async ({ page }) => {
  await page.goto('/world?from=-500&to=-400'); // era "Egypt" (-1068..-332)
  await expect(page.getByTestId('polity-ring-egypt--1068-0')).toBeAttached();
  const firstFill = await page.locator('path.atlas-border-wash[data-age="newest"]').first().evaluate(el => getComputedStyle(el).fill);

  await page.goto('/world?from=-100&to=-50'); // era "Ptolemaic Egypt" (-331..-30)
  await expect(page.getByTestId('polity-ring-egypt--331-0')).toBeAttached();
  const secondFill = await page.locator('path.atlas-border-wash[data-age="newest"]').first().evaluate(el => getComputedStyle(el).fill);

  expect(secondFill, 'Egypt -> Ptolemaic Egypt: same polity id, same fill color').toBe(firstFill);
});

// Requirement 3's own far-field smoke test (Batch C2's "populated far
// field": broad standing regions/waters spread across the whole biblical-
// world lock, each curated with a `size` hint precisely so it stays
// visible at the widest, most-zoomed-out view). Batch B2's OWN much richer
// polity roster (14 polities, 25 eras -- vs the retired snapshot model's
// single active snapshot at a time) means the full 4004 BC - AD 100 span
// now shows real historical polities across nearly the WHOLE biblical-
// world bbox at once (a good thing -- a fuller, more populated plate than
// the old model ever managed on its own) -- so "outside the active
// polities' own bbox" is no longer a meaningfully SPARSE comparison set
// the way it was against one snapshot's few features (confirmed
// empirically while writing this test: only 4 curated landmarks fall
// outside the union of every polity's own rings at this window, down from
// a much larger set under the retired model). This checks the same
// underlying feature a more robust way instead: the curated far-field
// landmarks themselves (every entry carrying a `size` hint, i.e. the
// entries Batch C2 added specifically for this) render visible at the
// widest natural view, regardless of how much of the plate polities now
// also cover. Full 4004 BC - AD 100 span is the SAME "default zoom-out"
// window WORLD-10 (world-map.spec.ts) already confirms lands in the FAR
// zoom tier.
test('BORDERS-7: the populated far field -- most curated far-field-hinted landmarks render visible at the widest natural view', async ({ page }) => {
  const landmarks = await api.landmarks();
  const farField = landmarks.filter((l: any) => l.size);
  expect(farField.length, 'expected a real curated far-field set to check').toBeGreaterThan(0);

  await page.goto('/world?from=-4004&to=100');
  await page.waitForSelector('.landmark-label', { timeout: 15000 });
  await page.waitForTimeout(500);

  let visibleCount = 0;
  for (const l of farField) {
    if (await page.getByTestId(`landmark-${kebab(l.name)}`).isVisible().catch(() => false)) {
      visibleCount++;
    }
  }
  expect(visibleCount, `expected most of ${farField.length} curated far-field landmarks visible; the plate must not read as bereft at its widest view`).toBeGreaterThanOrEqual(Math.ceil(farField.length * 0.6));
});

// Requirement 6: "growth case: outer ring's bounding box strictly contains
// the inner's (bounding boxes only -- no geometry math)". Rome's own
// Republic -> Empire growth: -50..10 intersects BOTH roman-empire eras
// (Republic -200..-31 and Empire -30..100), and the Empire era's own
// curated rings were drawn to visibly extend further in all four
// directions (north into Gaul, south into Egypt, east toward the
// Euphrates) than the Republic era's -- a real, deliberate growth, not a
// coincidence of drawing order.
test('BORDERS-8: a growth window shows the newer ring\'s screen bounding box strictly containing the older ring\'s', async ({ page }) => {
  await page.goto('/world?from=-50&to=10');
  const older = page.getByTestId('polity-ring-roman-empire--200-0'); // Roman Republic
  const newer = page.getByTestId('polity-ring-roman-empire--30-0'); // Roman Empire
  await expect(older).toBeAttached();
  await expect(newer).toBeAttached();

  const olderBox = await older.boundingBox();
  const newerBox = await newer.boundingBox();
  expect(olderBox).not.toBeNull();
  expect(newerBox).not.toBeNull();
  const o = olderBox!, n = newerBox!;

  expect(n.x, 'newer ring starts further left').toBeLessThan(o.x);
  expect(n.y, 'newer ring starts further up').toBeLessThan(o.y);
  expect(n.x + n.width, 'newer ring extends further right').toBeGreaterThan(o.x + o.width);
  expect(n.y + n.height, 'newer ring extends further down').toBeGreaterThan(o.y + o.height);
});

// Requirement 5 (arrow/border zoom-animation sync fix), acceptance per the
// batch brief: "a UI test can assert final positions match projected
// coords right after zoom WITHOUT waiting long after zoomend". Both
// custom SVG layers (ArrowLayer, BorderLayer) reset their own zoomanim CSS
// transform back to identity at the top of _redraw (which 'zoomend' always
// triggers) -- this is the exact mechanism a forgotten reset, or wrong
// transform math, would break: an identity transform left over means the
// layer's already-correct, freshly-recomputed `d`/path geometry is
// rendered WITHOUT a stray leftover translate/scale distorting it.
// page.mouse.wheel centered on the viewport drives Leaflet's own animated
// scroll-wheel zoom directly (same real-user gesture world-map.spec.ts's
// own WORLD-10b already uses for exactly this reason -- more reliable
// than clicking the small top-left zoom control).
test('BORDERS-9 (zoom-sync): border and arrow SVG layers reset to an identity transform immediately after an animated zoom', async ({ page }) => {
  await page.goto('/world?from=-1446&to=-1400'); // egypt-exodus: has an egypt polity ring
  const ring = page.getByTestId('polity-ring-egypt--1549-0');
  await expect(ring).toBeAttached();
  const dBefore = await ring.getAttribute('d');

  for (let i = 0; i < 3; i++) {
    await page.mouse.move(640, 360);
    await page.mouse.wheel(0, -300);
  }

  // No manual settle-wait beyond Playwright's own auto-retrying `toPass` --
  // this IS the acceptance criterion itself.
  await expect(async () => {
    const bordersTransform = await page.locator('svg.atlas-borders').evaluate(el => (el as HTMLElement).style.transform);
    const arrowsTransform = await page.locator('svg.atlas-arrows').evaluate(el => (el as HTMLElement).style.transform);
    for (const t of [bordersTransform, arrowsTransform]) {
      expect(t, `expected an identity transform, got "${t}"`).toMatch(/scale\(1\)\s*$/);
      // Chromium renders L.DomUtil.setTransform's own 3D form as
      // "translate3d(0px, 0px, 0px) scale(1)" (a trailing "0px", not the
      // bare "0" the un-rendered JS template literal itself contains) --
      // accepting either the 2D or 3D form's zero either way.
      expect(t).toMatch(/translate3d\(0px,\s*0px,\s*0(?:px)?\)|translate\(0px,\s*0px\)/);
    }
  }).toPass();

  // And the ring's own geometry actually moved to the new zoom's real
  // projected coordinates -- not merely "no transform left over" but
  // "correctly redrawn".
  const dAfter = await ring.getAttribute('d');
  expect(dAfter).not.toBe(dBefore);
});
