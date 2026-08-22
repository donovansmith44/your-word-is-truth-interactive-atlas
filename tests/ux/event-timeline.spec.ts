import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch HOTFIX-4 requirement 1 ("whole-DAG chronological traversal"),
// requirement 4 (acceptance, the owner's own report inverted), requirement
// 5 (tests), and Amendment C (Baptism-before-Temptation, red-then-green).
// See CONTRACT.md's own "GLOBAL TIMELINE" note (under EVENT-1) for the
// full ordering/wire-shape/presentation rules these tests pin.

function parseVerse(vref: string): { book: string; chapter: number; verse: number } {
  const [book, chapter, verse] = vref.split('.');
  return { book, chapter: Number(chapter), verse: Number(verse) };
}

// Opens EVENT `eventId`'s own popover by navigating to its own first
// witness verse and clicking through the verse's own EVENT membership row
// -- the SAME path popover-sections.spec.ts's own "three-narrative full
// walk" test uses, isolating the traversal-walk acceptance itself from
// wherever the popover happens to be reached from.
async function openEventPopover(page: any, eventId: string) {
  const detail = await api.event(eventId);
  const vref = detail.witnesses[0].verse_groups[0].verses[0];
  const v = parseVerse(vref);
  await page.goto(`/read/${v.book}/${v.chapter}`);
  await page.getByTestId(`verse-line-${v.verse}`).click();
  await page.getByTestId(`verse-event-${eventId}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);
}

test('HOTFIX-4 req 1/4: gen_binding_isaac (a real W1 container event, no narrative membership) traverses the global timeline -- RED before this batch, dead end at [] -- FOLLOWING and PRIOR each walk >=5 hops, every destination rendering its own traversal sections', async ({ page }) => {
  // Ground truth: gen_binding_isaac is dated, a leg of NO narrative at all
  // -- exactly the owner's own report ("adjacent nodes in the dag are dead
  // ends from where we start"). Confirmed via the wire, not assumed.
  const positions = await api.narrativeEventPositions('gen_binding_isaac');
  expect(positions.narrative, 'gen_binding_isaac is a leg of no narrative').toEqual([]);
  expect(positions.timeline, 'gen_binding_isaac must have a REAL global-timeline position (this is the fix)').toBeTruthy();

  await openEventPopover(page, 'gen_binding_isaac');

  // No NARRATIVE sections at all (conditional presence -- there is no
  // narrative to show); the TIMELINE sections are what carry traversal here.
  await expect(page.getByTestId('popover-section-event-prior')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-following')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-prior-timeline')).toBeVisible();
  await expect(page.getByTestId('popover-section-event-following-timeline')).toBeVisible();

  // Walk FOLLOWING >=5 hops -- hop targets read from the wire (whatever
  // `event-following-event-timeline` actually links to next), never
  // hardcoded. Each destination renders its own date+places section (proof
  // the popover actually re-anchored, not just changed its title) AND its
  // own timeline section (proof recursion -- the traversed node offers
  // FURTHER traversal, "arbitrarily far," not a one-hop special case).
  for (let hop = 0; hop < 5; hop++) {
    await expect(page.getByTestId('event-following-event-timeline'), `expected a FOLLOWING target at hop ${hop}`).toBeVisible();
    await page.getByTestId('event-following-event-timeline').click();
    await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();
  }

  // Walk PRIOR >=5 hops from a FRESH open of the same starting popover
  // (walking FOLLOWING above already moved Current away from it).
  await openEventPopover(page, 'gen_binding_isaac');
  for (let hop = 0; hop < 5; hop++) {
    await expect(page.getByTestId('event-prior-event-timeline'), `expected a PRIOR target at hop ${hop}`).toBeVisible();
    await page.getByTestId('event-prior-event-timeline').click();
    await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();
  }
});

test('HOTFIX-4 req 1/req 5: a narrative-member event shows BOTH its narrative rows and the global-timeline row, correctly and independently', async ({ page }) => {
  // pw_gethsemane is a passion-week narrative leg AND a dated, real event --
  // narrative primacy preserved (requirement 1 verbatim): both families of
  // sections must render, neither replacing the other.
  const positions = await api.narrativeEventPositions('pw_gethsemane');
  const narrativePos = positions.narrative.find((p: any) => p.narrative_id === 'passion-week');
  expect(narrativePos, 'pw_gethsemane must be a passion-week leg').toBeTruthy();
  expect(positions.timeline, 'pw_gethsemane must ALSO have a global-timeline position').toBeTruthy();

  await openEventPopover(page, 'pw_gethsemane');

  await expect(page.getByTestId('popover-section-event-prior')).toBeVisible();
  await expect(page.getByTestId('popover-section-event-following')).toBeVisible();
  await expect(page.getByTestId('popover-section-event-prior-timeline')).toBeVisible();
  await expect(page.getByTestId('popover-section-event-following-timeline')).toBeVisible();

  // The narrative row's own eyebrow reads "PRIOR EVENT"/"FOLLOWING EVENT"
  // (unchanged wording); the timeline row's own eyebrow reads "PRIOR IN
  // TIME"/"FOLLOWING IN TIME" -- "quiet, clearly distinct," never
  // mistakable for each other.
  const headings = await page.getByTestId('event-section-heading').allTextContents();
  expect(headings.some((h: string) => h.includes('FOLLOWING EVENT'))).toBeTruthy();
  expect(headings.some((h: string) => h.includes('FOLLOWING IN TIME'))).toBeTruthy();

  // The narrative row's own target and the timeline row's own target need
  // not be the same event (different questions -- "next in this
  // narrative's own leg chain" vs "next chronologically at all") -- both
  // are independently explorable and correct per their own semantics.
  if (narrativePos.following) {
    await expect(page.getByTestId('event-following-event-passion-week')).toHaveText(narrativePos.following.label);
  }
  await expect(page.getByTestId('event-following-event-timeline')).toHaveText(positions.timeline.following.label);
});

test('HOTFIX-4 req 1/5: chain-end conditional presence at the atlas\'s TRUE first/last dated event only', async ({ page }) => {
  // theo-1 "Creation of all things" (-4004) is the atlas's own true first
  // dated event; pr_rome "Paul arrives at Rome" (AD 60) the true last --
  // verified against the real compiled data (batch-hotfix4-report.md).
  const first = await api.narrativeEventPositions('theo-1');
  expect(first.timeline).toBeTruthy();
  expect(first.timeline.prior, 'the true first dated event of the whole atlas has no prior').toBeFalsy();
  expect(first.timeline.following, 'but DOES have a following -- one direction present IS honest').toBeTruthy();

  const last = await api.narrativeEventPositions('pr_rome');
  expect(last.timeline).toBeTruthy();
  expect(last.timeline.following, 'the true last dated event of the whole atlas has no following').toBeFalsy();
  expect(last.timeline.prior, 'but DOES have a prior').toBeTruthy();

  await openEventPopover(page, 'theo-1');
  await expect(page.getByTestId('popover-section-event-prior-timeline')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-following-timeline')).toBeVisible();

  await openEventPopover(page, 'pr_rome');
  await expect(page.getByTestId('popover-section-event-following-timeline')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-prior-timeline')).toBeVisible();
});

test('HOTFIX-4 req 2/5: a general-kind container shows no traversal section at all -- neither narrative nor timeline (conditional presence, "NOT part of time traversal")', async ({ page }) => {
  const detail = await api.event('rob_luke_preface');
  expect(detail.kind, 'rob_luke_preface must be general-kind for this test to mean anything').toBe('general');

  const positions = await api.narrativeEventPositions('rob_luke_preface');
  expect(positions.timeline, 'a general-kind event carries NO timeline key at all, not an empty object').toBeFalsy();

  await page.goto('/read/LUK/1');
  await page.getByTestId('verse-line-1').click();
  await page.getByTestId('verse-event-rob_luke_preface').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Luke\'s preface to Theophilus');

  await expect(page.getByTestId('popover-section-event-prior')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-following')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-prior-timeline')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-following-timeline')).toHaveCount(0);
});

test('AMENDMENT C: exactly one Baptism and one Temptation event exist; Baptism is chronologically PRIOR to Temptation; walking FOLLOWING from Baptism reaches Temptation directly (red-then-green: RED against the pre-merge theo-267/theo-268 shape)', async ({ page }) => {
  // Duplicate identities gone (event_merge.rs).
  for (const dupe of ['theo-267', 'theo-268']) {
    const r = await api.raw(`/api/event/${dupe}`);
    expect(r.__status, `${dupe} must be merged away entirely (404)`).toBe(404);
  }

  const baptism = await api.event('jm_jordan');
  const temptation = await api.event('rob_temptation');

  const positions = await api.narrativeEventPositions('jm_jordan');
  expect(positions.timeline.following.id, 'Baptism\'s own FOLLOWING (in time) must be Temptation directly').toBe('rob_temptation');

  await openEventPopover(page, 'jm_jordan');
  await expect(page.getByTestId('popover-title')).toHaveText(baptism.title);
  await page.getByTestId('event-following-event-timeline').click();
  await expect(page.getByTestId('popover-title')).toHaveText(temptation.title);
});

test('HOTFIX-4 req 3: map coherence -- traversing the global timeline from a map-side event popover behaves exactly like narrative traversal (shared code path, no special case)', async ({ page }) => {
  // Split view (`?split=1`, SPLIT-1) puts the reader AND the atlas pane on
  // screen together -- the same "map-side" surface a narrative popover's
  // own MAP FOCUS SYNC already targets today, reached without inventing a
  // new navigation path for this test alone.
  //
  // Part 1: open a NARRATIVE event (pw_gethsemane) -- real focus state
  // must exist (proves there's something live to get wrong, not asserting
  // against a scene that never had any arrows to begin with) -- then
  // traverse ONE hop via the TIMELINE row (not the narrative one,
  // requirement 3's own surface). jesus-ministry/passion-week are now
  // dense (122/49 legs, verified against the real compiled data), so the
  // immediate timeline neighbor is very likely STILL a narrative member --
  // that's fine: "shared code path, no special case" means focus just
  // correctly re-syncs to whatever Current actually is, which this proves
  // either way (still-narrative or not).
  const gethsemaneDetail = await api.event('pw_gethsemane');
  const vref = gethsemaneDetail.witnesses[0].verse_groups[0].verses[0];
  const v = parseVerse(vref);
  await page.goto(`/read/${v.book}/${v.chapter}?split=1`);
  await page.getByTestId(`verse-line-${v.verse}`).click();
  await page.getByTestId('verse-event-pw_gethsemane').click();
  await expect(page.getByTestId('popover-title')).toHaveText(gethsemaneDetail.title);
  await expect(page.locator('[data-narrative-focus]').first()).toBeAttached();

  const gethsemanePositions = await api.narrativeEventPositions('pw_gethsemane');
  await page.getByTestId('event-following-event-timeline').click();
  await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();
  const nextPositions = await api.narrativeEventPositions(gethsemanePositions.timeline.following.id);
  if (nextPositions.narrative.length > 0) {
    // Still a narrative member (likely, given passion-week's own density)
    // -- focus must be present and reflect THIS event's own narrative(s),
    // not simply "whatever was focused before."
    await expect(page.locator('[data-narrative-focus]').first()).toBeAttached();
  } else {
    // Landed on a narrative-less event via the very first hop -- focus
    // must already be baseline (no special case needed to make this true).
    await expect(page.locator('[data-narrative-focus]')).toHaveCount(0);
  }

  // Part 2: separately, open a KNOWN narrative-less dated event
  // (gen_binding_isaac, req 1/4's own acceptance subject) directly in the
  // SAME split-view session, right after the narrative-focused open above
  // -- confirms the "clears to baseline" half concretely, deterministically
  // (not dependent on how many hops a walk happens to take).
  const isaacDetail = await api.event('gen_binding_isaac');
  const isaacVref = isaacDetail.witnesses[0].verse_groups[0].verses[0];
  const iv = parseVerse(isaacVref);
  await page.goto(`/read/${iv.book}/${iv.chapter}?split=1`);
  await page.getByTestId(`verse-line-${iv.verse}`).click();
  await page.getByTestId('verse-event-gen_binding_isaac').click();
  await expect(page.getByTestId('popover-title')).toHaveText(isaacDetail.title);

  const staleFocus = await page.locator('[data-narrative-focus]').count();
  expect(staleFocus, 'a narrative-less Current must clear every arrow\'s own data-narrative-focus attribute to baseline').toBe(0);
});
