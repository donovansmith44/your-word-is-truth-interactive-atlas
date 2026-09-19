import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { LIT_MARKER_TESTID } from './lib/markers';

// Batch ST-1 -- the SYNC-1 repro AS the agreement-law regression test (the
// motivating defect, owner verbatim: "when we follow text in map, there is
// a box on the map that allows you to pick which chapter you're on but it's
// out of sync with the analogous box on the reader side"). SYNC-1 is now
// retired BY CONSTRUCTION (both ScripturePicker mounts render from the SAME
// shared Locus atom, client/State/Locus.cs) -- these tests pin the
// user-visible contract that construction guarantees, not any one bugfix.
// See CONTRACT.md's own SYNC-1/ST-1-AMENDMENT notes for the full mechanism.
//
// Split view renders TWO <ScripturePicker> instances at once (reader +
// atlas panes) -- every picker locator below is scoped to its own pane
// container (`reader-root` / `split-pane-atlas`) so a bare
// `page.getByTestId('picker-book')` never hits Playwright's strict-mode
// ambiguity.

test('SYNC-1: both pickers agree after a reader-picker-driven chapter change, while following', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'true'); // follow ON, the default

  const readerRoot = page.getByTestId('reader-root');
  const atlasPane = page.getByTestId('split-pane-atlas');

  // D1 LAW (owner, 2026-09-15): while FOLLOWING there is ONE selector on
  // screen, the reader's -- the atlas-side picker is not mounted at all
  // (World.razor's guard), so "both pickers agree" is now "the atlas pane
  // has no picker; the reader's carries the chapter". (SYNC-1's own
  // projection -- the world picker rendering FROM the shared Locus atom --
  // still holds the moment follow is released, below.)
  await expect(atlasPane.getByTestId('picker')).toHaveCount(0);

  // Navigate via the READER's OWN picker.
  await readerRoot.getByTestId('picker-book').selectOption('EXO');
  await readerRoot.getByTestId('picker-chapter').selectOption('3');
  await readerRoot.getByTestId('picker-apply').click();

  await expect(page).toHaveURL(/\/read\/EXO\/3/);
  await expect(page.getByTestId('chapter-head')).toContainText('3');
  await expect(page.getByTestId('follow-chip')).toHaveText('Following EXO.3');

  // Agreement (D1): the atlas pane still has no picker of its own; the
  // reader's carries EXO.3.
  await expect(atlasPane.getByTestId('picker')).toHaveCount(0);
  await expect(readerRoot.getByTestId('picker-book')).toHaveValue('EXO');
  await expect(readerRoot.getByTestId('picker-chapter')).toHaveValue('3');

  const scene = await api.sceneScripture('EXO.3');
  await expect(page.getByTestId(LIT_MARKER_TESTID)).toHaveCount(scene.places.length);
});

test('SYNC-1: both pickers agree after reader-next arrow navigation, while following', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('split-view')).toBeVisible();

  const readerRoot = page.getByTestId('reader-root');
  const atlasPane = page.getByTestId('split-pane-atlas');

  await page.getByTestId('reader-next').click();
  await page.waitForURL(/\/read\/GEN\/13/);

  await expect(page.getByTestId('chapter-head')).toContainText('13');
  await expect(page.getByTestId('follow-chip')).toHaveText('Following GEN.13');
  await expect(atlasPane.getByTestId('picker')).toHaveCount(0); // D1: one selector while following
  await expect(readerRoot.getByTestId('picker-book')).toHaveValue('GEN');
  await expect(readerRoot.getByTestId('picker-chapter')).toHaveValue('13');
});

test('SYNC-1: follow OFF -- a world-picker Apply still works and leaves the reader undisturbed', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('split-view')).toBeVisible();

  const readerRoot = page.getByTestId('reader-root');
  const atlasPane = page.getByTestId('split-pane-atlas');

  await page.getByTestId('reader-next').click();
  await page.waitForURL(/\/read\/GEN\/13/);
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'true');

  // D1 LAW: the world picker is not mounted while following, so follow is
  // released FIRST (the chip) -- that is what brings the picker back, showing
  // the reader's current chapter (SYNC-1's projection). Then the world-side
  // picker jumps to something unrelated to the reader -- the pre-existing
  // "look at something on the map" contract (World's own picker Apply does
  // NOT dispatch onto the shared Locus atom -- see ApplyScriptureRef's own
  // doc comment).
  await expect(atlasPane.getByTestId('picker')).toHaveCount(0);
  await page.getByTestId('follow-chip').click();
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');
  await expect(atlasPane.getByTestId('picker-book')).toHaveValue('GEN');
  await expect(atlasPane.getByTestId('picker-chapter')).toHaveValue('13');
  await atlasPane.getByTestId('picker-book').selectOption('JOS');
  await atlasPane.getByTestId('picker-chapter').selectOption('6');
  await atlasPane.getByTestId('picker-apply').click();

  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');
  await expect(page.getByTestId('mode-chip')).toContainText('JOS.6');
  const scene = await api.sceneScripture('JOS.6');
  await expect(page.getByTestId(LIT_MARKER_TESTID)).toHaveCount(scene.places.length);

  // "World picker stops tracking" -- its own dropdown now shows the applied
  // pick, no longer the reader's chapter.
  await expect(atlasPane.getByTestId('picker-book')).toHaveValue('JOS');
  await expect(atlasPane.getByTestId('picker-chapter')).toHaveValue('6');

  // The reader itself is COMPLETELY undisturbed: still on GEN.13, its own
  // picker still shows GEN.13 -- proving the world-side Apply never touched
  // the shared Locus atom (the reader's actual reading position).
  await expect(page).toHaveURL(/\/read\/GEN\/13/);
  await expect(page.getByTestId('chapter-head')).toContainText('13');
  await expect(readerRoot.getByTestId('picker-book')).toHaveValue('GEN');
  await expect(readerRoot.getByTestId('picker-chapter')).toHaveValue('13');
});

// Fix round 1 (review finding S-1, CRITICAL -- the rejection defect): the
// pre-fix version of this test asserted the follow-chip label, the
// mode-chip's absence, and the marker count after re-toggling follow back
// ON -- but never the picker dropdowns themselves, which is EXACTLY where
// SYNC-1 was still reachable. ScripturePicker's own B4 `_synced` guard only
// re-syncs when CurrentBook/CurrentChapter genuinely CHANGE; since a
// world-picker Apply deliberately never touches Locus (see
// ApplyScriptureRef's own doc comment) and the reader's own chapter hasn't
// moved either across this OFF/ON cycle, Locus itself never changes, so
// without the SyncToken fix (ScripturePicker.razor/World.razor) the world
// picker's dropdowns stayed on the applied ref (JOS/6) FOREVER -- follow ON,
// map scened to GEN.13, reader picker GEN 13, world picker JOS 6, stable --
// the owner's defect verbatim. The two `atlasPane` assertions below are the
// actual regression lock; everything else in this test already passed
// before the fix.
test('SYNC-1: follow back ON re-converges the atlas pane to the reader\'s actual current chapter -- BOTH the scene and the picker dropdowns', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await page.getByTestId('reader-next').click();
  await page.waitForURL(/\/read\/GEN\/13/);

  const readerRoot = page.getByTestId('reader-root');
  const atlasPane = page.getByTestId('split-pane-atlas');

  // Diverge via the world picker (as the prior test: release follow first,
  // D1), then re-toggle follow.
  await page.getByTestId('follow-chip').click(); // off (D1: brings the picker back)
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');
  await atlasPane.getByTestId('picker-book').selectOption('JOS');
  await atlasPane.getByTestId('picker-chapter').selectOption('6');
  await atlasPane.getByTestId('picker-apply').click();
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');
  await expect(page.getByTestId('mode-chip')).toContainText('JOS.6');
  await expect(atlasPane.getByTestId('picker-book')).toHaveValue('JOS'); // the stale-before-reconvergence baseline
  await expect(atlasPane.getByTestId('picker-chapter')).toHaveValue('6');

  await page.getByTestId('follow-chip').click(); // back ON

  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByTestId('follow-chip')).toHaveText('Following GEN.13');
  await expect(page.getByTestId('mode-chip')).toHaveCount(0); // suppressed while following (FOLLOW-1)
  const scene = await api.sceneScripture('GEN.13');
  await expect(page.getByTestId(LIT_MARKER_TESTID)).toHaveCount(scene.places.length);

  // D1: while following the atlas pane has no picker at all -- the reader's
  // is the one selector, on GEN.13.
  await expect(atlasPane.getByTestId('picker')).toHaveCount(0);
  await expect(readerRoot.getByTestId('picker-book')).toHaveValue('GEN');
  await expect(readerRoot.getByTestId('picker-chapter')).toHaveValue('13');

  // THE REGRESSION LOCK (S-1), in its D1 form: releasing follow AGAIN brings
  // the world picker back RE-CONVERGED to GEN.13 -- not stuck on the applied
  // JOS.6 forever (the SyncToken fix) -- so both pickers genuinely agree.
  await page.getByTestId('follow-chip').click(); // off again
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');
  await expect(atlasPane.getByTestId('picker-book')).toHaveValue('GEN');
  await expect(atlasPane.getByTestId('picker-chapter')).toHaveValue('13');
  await expect(readerRoot.getByTestId('picker-book')).toHaveValue('GEN');
  await expect(readerRoot.getByTestId('picker-chapter')).toHaveValue('13');
});

// S-2: deliverable 7's own most load-bearing scenario ("split view, follow
// on, navigate chapters via reader picker / arrows / world picker; BOTH
// pickers agree after each mutation") applied specifically to the WORLD
// picker, which the earlier tests in this file never actually exercised
// with follow starting ON. The honest, deliberate contract for this
// specific path (ratified by the controller, see the batch report's fix
// round 1 addendum) is: applying the world picker while following turns
// follow OFF as an immediate side effect (ApplyExternalQuery's own first
// statement), so the two pickers are NEVER simultaneously "follow ON AND
// disagreeing" -- they only ever disagree once follow has already, visibly,
// turned itself off. This test pins that as a real, asserted contract
// rather than leaving it implicit.
test('SYNC-1/deliverable 7 (D1 form): while follow is ON there is no world picker to apply; released, the two pickers legitimately diverge', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await page.getByTestId('reader-next').click();
  await page.waitForURL(/\/read\/GEN\/13/);

  const readerRoot = page.getByTestId('reader-root');
  const atlasPane = page.getByTestId('split-pane-atlas');

  // Before: follow ON -- D1 LAW: the reader's picker is the ONE selector on
  // screen (GEN.13); the atlas pane has none, so "follow ON AND the two
  // pickers disagree" is not merely a transient the Apply closes -- it is
  // structurally unreachable now.
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'true');
  await expect(readerRoot.getByTestId('picker-book')).toHaveValue('GEN');
  await expect(readerRoot.getByTestId('picker-chapter')).toHaveValue('13');
  await expect(atlasPane.getByTestId('picker')).toHaveCount(0);

  // Release follow: the world picker appears, agreeing (SYNC-1's projection).
  await page.getByTestId('follow-chip').click();
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');
  await expect(atlasPane.getByTestId('picker-book')).toHaveValue('GEN');
  await expect(atlasPane.getByTestId('picker-chapter')).toHaveValue('13');

  await atlasPane.getByTestId('picker-book').selectOption('JOS');
  await atlasPane.getByTestId('picker-chapter').selectOption('6');
  await atlasPane.getByTestId('picker-apply').click();

  // After: follow stays OFF and the divergence is real and expected, not a
  // bug: the reader stays exactly where it was.
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');
  await expect(atlasPane.getByTestId('picker-book')).toHaveValue('JOS');
  await expect(atlasPane.getByTestId('picker-chapter')).toHaveValue('6');
  await expect(readerRoot.getByTestId('picker-book')).toHaveValue('GEN');
  await expect(readerRoot.getByTestId('picker-chapter')).toHaveValue('13');
  await expect(page).toHaveURL(/\/read\/GEN\/13/); // the reader's own route: completely undisturbed
});

test('SYNC-1: the world-side picker shows the current chapter standalone too (not split-only)', async ({ page }) => {
  // The ONE user-visible change this batch permits/requires, checked
  // outside split view entirely: a bare standalone /world visit, after the
  // reader was visited this session, shows that chapter in its own picker
  // -- the shared Locus atom is an app-lifetime singleton, not scoped to
  // split mode.
  await page.goto('/read/LEV/5');
  await expect(page.getByTestId('chapter-head')).toContainText('5');

  await page.getByTestId('nav-world').click();
  await page.waitForURL(u => u.pathname === '/world');

  await expect(page.getByTestId('picker-book')).toHaveValue('LEV');
  await expect(page.getByTestId('picker-chapter')).toHaveValue('5');
});
