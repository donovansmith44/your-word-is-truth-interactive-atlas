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

  // Both pickers start agreeing (GEN.12) -- the ONE user-visible change this
  // batch permits/requires: the atlas-side picker shows the current chapter
  // at all, standalone or split (SYNC-1's own root mechanism -- see
  // ScripturePicker's own mount in World.razor).
  await expect(atlasPane.getByTestId('picker-book')).toHaveValue('GEN');
  await expect(atlasPane.getByTestId('picker-chapter')).toHaveValue('12');

  // Navigate via the READER's OWN picker.
  await readerRoot.getByTestId('picker-book').selectOption('EXO');
  await readerRoot.getByTestId('picker-chapter').selectOption('3');
  await readerRoot.getByTestId('picker-apply').click();

  await expect(page).toHaveURL(/\/read\/EXO\/3/);
  await expect(page.getByTestId('chapter-head')).toContainText('3');
  await expect(page.getByTestId('follow-chip')).toHaveText('Following EXO.3');

  // Agreement: the atlas pane's OWN picker -- never touched directly --
  // now reflects EXO.3 too, live, via the shared Locus atom.
  await expect(atlasPane.getByTestId('picker-book')).toHaveValue('EXO');
  await expect(atlasPane.getByTestId('picker-chapter')).toHaveValue('3');
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
  await expect(atlasPane.getByTestId('picker-book')).toHaveValue('GEN');
  await expect(atlasPane.getByTestId('picker-chapter')).toHaveValue('13');
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

  // The world-side picker jumps to something unrelated to the reader --
  // this is the pre-existing "look at something on the map" contract,
  // deliberately UNCHANGED this batch (World's own picker Apply does NOT
  // dispatch onto the shared Locus atom -- see ApplyScriptureRef's own doc
  // comment / the batch report). Turns follow off as a side effect, exactly
  // as it always has.
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

  // Diverge via the world picker (as the prior test), then re-toggle follow.
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

  // THE REGRESSION LOCK (S-1): the world picker's own dropdowns re-converge
  // to GEN.13 too -- not left stuck on the applied JOS.6 forever -- so both
  // pickers genuinely agree again, matching the reader's own picker.
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
test('SYNC-1/deliverable 7: applying the world picker while follow is ON turns follow off immediately, and the two pickers then legitimately diverge', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await page.getByTestId('reader-next').click();
  await page.waitForURL(/\/read\/GEN\/13/);

  const readerRoot = page.getByTestId('reader-root');
  const atlasPane = page.getByTestId('split-pane-atlas');

  // Before: follow ON, both pickers agree on GEN.13.
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'true');
  await expect(readerRoot.getByTestId('picker-book')).toHaveValue('GEN');
  await expect(readerRoot.getByTestId('picker-chapter')).toHaveValue('13');
  await expect(atlasPane.getByTestId('picker-book')).toHaveValue('GEN');
  await expect(atlasPane.getByTestId('picker-chapter')).toHaveValue('13');

  await atlasPane.getByTestId('picker-book').selectOption('JOS');
  await atlasPane.getByTestId('picker-chapter').selectOption('6');
  await atlasPane.getByTestId('picker-apply').click();

  // After: follow flipped OFF as an immediate, visible side effect of the
  // Apply itself (never a state where follow reads ON while the two
  // pickers disagree) -- and the divergence that follows is real and
  // expected, not a bug: the reader stays exactly where it was.
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
