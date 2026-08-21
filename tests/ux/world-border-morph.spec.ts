import { test, expect, Page } from '@playwright/test';

// Batch M requirement 5: the scrub state machine, delta ring hover/click,
// minimal popover, and land-clip-holds-mid-morph tests -- CONTRACT-bound
// (DELTA-1 in tests/ux/CONTRACT.md), --workers=1. Real curated data used
// throughout, no mocking, matching this suite's own house rule.
//
// A note on interaction mechanics: a border ring's own hit-stroke is
// `pointer-events: stroke` on a `fill:none` closed path -- Playwright's own
// default click/hover point (a locator's bounding-box CENTER) lands in the
// ring's EMPTY INTERIOR for most real curated shapes, where the stroke has
// no hit surface at all (confirmed live, not assumed: a bounding-box-center
// click was intercepted by whatever unrelated element/terrain sits at that
// interior pixel, not this app's own path). Every test below therefore
// activates via KEYBOARD (`.focus()` + `.press('Enter')`) as the PRIMARY,
// reliable mechanism -- which the brief itself separately requires
// ("keyboard reachable") -- and verifies the HOVER-darken mechanism by
// dispatching a real `mouseover`/`mouseout` DOM event directly at the known
// element (page.evaluate), the standard technique for testing an event
// HANDLER's own behavior independent of screen-geometry hit-testing
// specifics -- the same class of "best-effort, not a per-target geometric
// guarantee" limitation CONTRACT.md's own "Marker hover-target resolution"
// note already discloses for this app's dense map content generally.

async function isPointOnLand(page: Page, lat: number, lon: number): Promise<boolean | null> {
  return page.evaluate(async ({ lat, lon }) => {
    const m: any = await import('/js/map.js');
    const ids: number[] = m.debugLiveInstanceIds();
    return m.debugIsPointOnLand(ids[ids.length - 1], lat, lon);
  }, { lat, lon });
}

test.describe('DELTA-1: delta ring explorability + popover', () => {
  test('a fall-eligible ring is keyboard-reachable, hover-darkens, and opens a full popover with event/scriptures/grounding/chip', async ({ page }) => {
    // Israel's own fall era: "Kingdom of Israel" (-930..-722), fall = Assyria carries Israel captive, 2 Kings 17:6,23.
    await page.goto('/world?from=-750&to=-700');
    const hit = page.getByTestId('polity-delta-israel--930-0');
    await expect(hit).toBeAttached();

    // Real, keyboard-reachable element.
    await expect(hit).toHaveAttribute('tabindex', '0');
    await expect(hit).toHaveAttribute('role', 'button');
    // SVG elements keep their AUTHORED-CASE tagName (never uppercased the
    // way HTML elements' tagName always is) -- confirmed live: the actual
    // DOM property here reads "path", not "PATH".
    await expect(hit).toHaveJSProperty('tagName', 'path');

    // Hover darkens the ring's own wash+line (ONE-RULE's own ~120ms language, adapted for an SVG shape).
    const line = page.getByTestId('polity-ring-israel--930-0');
    await expect(line).not.toHaveAttribute('data-delta-hover', 'true');
    await hit.dispatchEvent('mouseover');
    await expect(line).toHaveAttribute('data-delta-hover', 'true');
    const brightnessOnHover = await line.evaluate(el => getComputedStyle(el).filter);
    expect(brightnessOnHover).toContain('brightness');
    await hit.dispatchEvent('mouseout');
    await expect(line).not.toHaveAttribute('data-delta-hover', 'true');

    // Click (via keyboard activation) opens the popover.
    await hit.focus();
    await hit.press('Enter');
    await expect(page.getByTestId('popover-title')).toHaveText('Kingdom of Israel, 930 BC → 722 BC');

    await expect(page.getByTestId('popover-section-polity-delta-event')).toContainText('Assyria carries Israel captive');
    await expect(page.getByTestId('popover-section-polity-delta-scriptures')).toBeVisible();
    const verse = page.getByTestId('polity-delta-verse-2KI.17.6');
    await expect(verse).toContainText('In the ninth year of Hoshea');
    await expect(page.getByTestId('polity-delta-verse-2KI.17.23')).toContainText('So was Israel carried away');
    await expect(page.getByTestId('popover-section-polity-delta-grounding')).toContainText('2 Kings 17');
    await expect(page.getByTestId('popover-chip-map')).toBeVisible();

    // The verses are independently explorable, PASSAGE-1's own rule.
    await verse.click();
    await expect(page.getByTestId('popover-title')).toHaveText('2KI.17.6');
  });

  test('a transition-eligible ring (rise, first era) opens a full popover', async ({ page }) => {
    // Israel's own rise: "Kingdom of Israel (United Monarchy)" (-1050..-931), transition = David anointed king, 2 Samuel 5:1,3.
    // The window must actually CONTAIN entry.from (-1050) for the transition
    // to be a hit target -- DELTA-1's own rule -- not merely overlap the era.
    await page.goto('/world?from=-1060&to=-1010');
    const hit = page.getByTestId('polity-delta-israel--1050-0');
    await expect(hit).toBeAttached();
    await hit.focus();
    await hit.press('Enter');
    await expect(page.getByTestId('popover-title')).toHaveText('Kingdom of Israel (United Monarchy), 1050 BC → 931 BC');
    await expect(page.getByTestId('popover-section-polity-delta-event')).toContainText('David is anointed king over all Israel');
    // 2SA.5.1 and 2SA.5.3 are NOT numerically consecutive (verse 2 is not
    // cited) -- PassageGrouping never merges across a gap, so these render
    // as two SEPARATE lone-verse entries, not one "2SA.5.1-3" span.
    await expect(page.getByTestId('polity-delta-verse-2SA.5.1')).toContainText('Then came all the tribes of Israel');
    await expect(page.getByTestId('polity-delta-verse-2SA.5.3')).toContainText('anointed David king over Israel');
  });

  test('an internal transition (neither first nor last era) opens a full popover, title years span the previous era\'s end to this era\'s own end', async ({ page }) => {
    // Judah's own Yehud era (-538..-333), transition = Cyrus decrees the return, Ezra 1:1-3.
    await page.goto('/world?from=-540&to=-500');
    const hit = page.getByTestId('polity-delta-judah--538-0');
    await expect(hit).toBeAttached();
    await hit.focus();
    await hit.press('Enter');
    await expect(page.getByTestId('popover-title')).toHaveText('Yehud, 587 BC → 333 BC');
    await expect(page.getByTestId('popover-section-polity-delta-event')).toContainText('Cyrus decrees the return from exile');
    await expect(page.getByTestId('polity-delta-verse-EZR.1.1-3')).toContainText('Now in the first year of Cyrus');
  });

  test('minimal popover: an honestly uneventful boundary is still explorable, but offers no event/scriptures/grounding sections -- only the map chip', async ({ page }) => {
    // egypt's own Late Period transition (New Kingdom collapses) -- authored with NO verses.
    await page.goto('/world?from=-1100&to=-1000');
    await expect(page.getByTestId(/^polity-ring-egypt-/).first()).toBeAttached();
    const hit = page.getByTestId('polity-delta-egypt--1068-0');
    await expect(hit).toBeAttached();
    await hit.focus();
    await hit.press('Enter');

    await expect(page.getByTestId('popover-title')).toHaveText('Egypt, 1069 BC → 332 BC');
    // Event text IS present (a real, if verse-less, curated event) --
    // "minimal" here means no SCRIPTURES section, not no content at all.
    await expect(page.getByTestId('popover-section-polity-delta-event')).toContainText('The New Kingdom collapses');
    await expect(page.getByTestId('popover-section-polity-delta-scriptures')).toHaveCount(0);
    await expect(page.getByTestId('popover-section-polity-delta-grounding')).toContainText('general Egyptological knowledge');
    await expect(page.getByTestId('popover-chip-map')).toBeVisible();
  });

  test('a boundary with NO curated delta block at all is still explorable with a fully minimal popover: only years, no sections', async ({ page }) => {
    // Sumer has zero authored deltas (disclosed, see the batch report) -- its only era's own start (-4004) is never in-window for any real window, but its own end (-2004) is reachable.
    await page.goto('/world?from=-2050&to=-1990');
    await expect(page.getByTestId('polity-ring-sumer--4004-0')).toBeAttached();
    // Sumer's single era is trivially its own "final" era -- its own end (-2004) is fall-eligible even with no [era.fall] curated.
    const hit = page.getByTestId('polity-delta-sumer--4004-0');
    await expect(hit).toBeAttached();
    await hit.focus();
    await hit.press('Enter');

    await expect(page.getByTestId('popover-title')).toHaveText('Sumer, 4004 BC → 2004 BC');
    await expect(page.getByTestId('popover-section-polity-delta-event')).toHaveCount(0);
    await expect(page.getByTestId('popover-section-polity-delta-scriptures')).toHaveCount(0);
    await expect(page.getByTestId('popover-section-polity-delta-grounding')).toHaveCount(0);
    await expect(page.getByTestId('popover-chip-map')).toBeVisible();
  });

  test('an era boundary OUTSIDE the current window is not a hit target at all', async ({ page }) => {
    // Israel's own fall (-722) is far outside this window -- the ring itself renders (era intersects), but no delta hit-stroke exists for it.
    await page.goto('/world?from=-850&to=-800');
    await expect(page.getByTestId('polity-ring-israel--930-0')).toBeAttached();
    await expect(page.getByTestId('polity-delta-israel--930-0')).toHaveCount(0);
  });
});

test.describe('MORPH-1: scrub state machine (drag -> morphing, release -> settled)', () => {
  test('dragging the FROM handle across a polity\'s own era boundary morphs its ring live, then settles on release', async ({ page }) => {
    await page.goto('/world?from=-750&to=100'); // TO pinned far right, clear of the FROM handle this test drags (WORLD-9's own established reasoning)
    await expect(page.getByTestId('polity-ring-israel--930-0')).toBeAttached();

    const handle = page.getByRole('slider', { name: 'Window start year' });
    const box = (await handle.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();

    // Sweep leftward (further into the past) in several steps -- crosses the -931/-930 boundary between Israel's own two eras.
    await page.mouse.move(box.x - 60, box.y + box.height / 2, { steps: 8 });

    // While dragging: the settled ring is hidden, a morphing one (data-morph-state) is visible somewhere on the plate.
    await expect(async () => {
      const morphing = page.locator('[data-morph-state="morphing"]').first();
      await expect(morphing).toBeVisible();
    }).toPass();
    const settledGroupDisplay = await page.locator('.atlas-border-settled-group').evaluate(el => getComputedStyle(el).display);
    expect(settledGroupDisplay).toBe('none');

    await page.mouse.up();

    // On release: settles back into the static layered-era presentation -- morphing paths gone, settled group visible again, a real committed URL change.
    await page.waitForFunction(prev => new URL(location.href).searchParams.get('from') !== prev, '-750');
    await expect(page.locator('[data-morph-state="morphing"]')).toHaveCount(0);
    await expect(async () => {
      const d = await page.locator('.atlas-border-settled-group').evaluate(el => getComputedStyle(el).display);
      expect(d).not.toBe('none');
    }).toPass();
  });

  test('C1 regression: dragging one handle keeps the OTHER, still-committed edge fully in the per-frame lookup window -- non-crossing polities never vanish mid-drag', async ({ page }) => {
    // Batch M review, fix round 1, Critical finding C1: the per-frame morph
    // lookup used to derive lo/hi from [the dragged handle's OWN pre-drag
    // value, the live probe] -- never the OTHER, still-committed handle's
    // current value -- so any polity/era outside that narrow sweep vanished
    // for the whole gesture. Replays the reviewer's own live-verified
    // scenario: /world?from=-750&to=100, drag FROM left. The TRUE live
    // window during the drag is [probe, 100] (TO never moves) -- under the
    // bug, alexander-empire/babylon/parthian-empire/persia/roman-empire/
    // seleucid-empire (none of whose own eras cross the dragged FROM edge)
    // silently disappeared because TO's own live value (100) was never fed
    // into the per-frame `lookup` at all.
    await page.goto('/world?from=-750&to=100');
    await expect(page.getByTestId('polity-ring-israel--930-0')).toBeAttached();

    const handle = page.getByRole('slider', { name: 'Window start year' });
    const box = (await handle.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    // A generous sweep -- well past -1000 regardless of the slider's exact
    // px-per-year scale (the six previously-vanishing polities above are
    // each expected present for ANY probe year <= -539, per their own
    // curated era spans, so overshooting costs nothing here).
    await page.mouse.move(box.x - 300, box.y + box.height / 2, { steps: 15 });

    await expect(async () => {
      const morphing = page.locator('[data-morph-state="morphing"]').first();
      await expect(morphing).toBeVisible();
    }).toPass();

    const morphingIds: string[] = await page.evaluate(async () => {
      const m: any = await import('/js/map.js');
      const ids: number[] = m.debugLiveInstanceIds();
      const instId = ids[ids.length - 1];
      return m.debugMorphingPolityIds(instId);
    });

    // The count itself, not just individual membership -- the settled
    // repaint of the equivalent full window shows all 12 polities (per the
    // reviewer's own live count); the bug's own narrow sweep showed only 6.
    expect(morphingIds.length).toBeGreaterThanOrEqual(12);
    for (const id of ['alexander-empire', 'babylon', 'parthian-empire', 'persia', 'roman-empire', 'seleucid-empire']) {
      expect(morphingIds).toContain(id);
    }

    await page.mouse.up();
  });

  test('prefers-reduced-motion: dragging snaps directly between discrete era shapes, never showing an interpolated in-between ring', async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/world?from=-750&to=100');
    await expect(page.getByTestId('polity-ring-israel--930-0')).toBeAttached();

    // The two "snap candidate" shapes: israel's own two eras' SETTLED `d`,
    // computed via the exact same `_ringPathData` projection production
    // rendering uses (map.js's own debugSettledRingPathData) -- NOT read
    // off a live `polity-ring-israel--1050-0` element, which does not
    // exist in this window at all (that era, -1050..-931, sits entirely
    // before this window's own -750 start) -- reading it as a locator would
    // hang forever (this project sets no Playwright actionTimeout, so a
    // locator that never attaches never rejects either) rather than fail
    // fast, exactly the bug a first draft of this test caught the hard way.
    const [dNewer, dOlder] = await page.evaluate(async () => {
      const m: any = await import('/js/map.js');
      const ids: number[] = m.debugLiveInstanceIds();
      const instId = ids[ids.length - 1];
      return [
        m.debugSettledRingPathData(instId, 'israel', -930),
        m.debugSettledRingPathData(instId, 'israel', -1050),
      ];
    });
    expect(dNewer.length).toBeGreaterThan(0);
    expect(dOlder.length).toBeGreaterThan(0);

    const handle = page.getByRole('slider', { name: 'Window start year' });
    const box = (await handle.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x - 60, box.y + box.height / 2, { steps: 8 });

    await expect(async () => {
      const morphing = page.locator('[data-morph-state="morphing"]').first();
      await expect(morphing).toBeVisible();
    }).toPass();

    // Israel's own currently-morphing line(s) -- scoped BY POLITY ID
    // (debugMorphLineData), not a bare `.atlas-border-morph-group`
    // CSS-class sweep, which would also catch OTHER polities simultaneously
    // mid-drag in this same wide (-750..100) window (Judah, Rome, etc. --
    // several of their own era boundaries also sit inside it). Every one of
    // israel's own morphing `d` values matches EXACTLY one of the two real,
    // settled era shapes (never a blended value) -- "snap directly," per
    // the brief, checked as a real geometric claim, not merely "some class
    // is present."
    const morphD: string[] = await page.evaluate(async () => {
      const m: any = await import('/js/map.js');
      const ids: number[] = m.debugLiveInstanceIds();
      const instId = ids[ids.length - 1];
      return m.debugMorphLineData(instId, 'israel');
    });
    expect(morphD.length).toBeGreaterThan(0);
    const candidates = new Set([...dNewer, ...dOlder]);
    for (const d of morphD) {
      expect(candidates.has(d)).toBe(true);
    }

    await page.mouse.up();
    await page.waitForFunction(prev => new URL(location.href).searchParams.get('from') !== prev, '-750');
  });
});

test('MORPH-2: land-clip holds mid-morph (deterministic sea pixel stays excluded while dragging)', async ({ page }) => {
  await page.goto('/world?from=-1446&to=-1400'); // egypt-exodus: a real egypt wash is present to morph
  await expect(page.getByTestId(/^polity-ring-egypt-/).first()).toBeAttached();

  const handle = page.getByRole('slider', { name: 'Window start year' });
  const box = (await handle.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x - 40, box.y + box.height / 2, { steps: 6 });

  await expect(async () => {
    const morphing = page.locator('[data-morph-state="morphing"]').first();
    await expect(morphing).toBeVisible();
  }).toPass();

  // Mediterranean south of Cyprus (the same deterministic sea point LAND-1
  // already uses) stays excluded from the land-mask clip WHILE a morph is
  // actively in progress, not just at rest.
  await expect.poll(() => isPointOnLand(page, 33.5, 29.5)).toBe(false);
  await expect.poll(() => isPointOnLand(page, 31.78, 35.22)).toBe(true); // Jerusalem, land, same-mechanism sanity check

  await page.mouse.up();
});
