import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch HOTFIX-4 requirement 1 ("whole-DAG chronological traversal"),
// requirement 4 (acceptance, the owner's own report inverted), requirement
// 5 (tests), and Amendment C (Baptism-before-Temptation, red-then-green).
// TRAV-1 (CHRONO-1/2/3, PEEK-1): the "PRIOR IN TIME"/"FOLLOWING IN TIME"
// sections these tests originally pinned are RETIRED WHOLE -- ONE
// "Chronology" block (`popover-section-event-chronology`,
// `event-chrono-{prior,following}-event-global`) replaces them, name-only
// arrows via the SAME `Components.ArrowNav` the narrative nav uses. The
// real-data ACCEPTANCE SUBJECTS/facts below (gen_binding_isaac's own
// narrative-less dated status, pw_gethsemane's dual membership, theo-1/
// theo-385's own true first/last position, Baptism-precedes-Temptation,
// jm_egypt's own traversability) are UNCHANGED -- only the testids/DOM
// shape asserting them moved. See CONTRACT.md's own "GLOBAL TIMELINE" note
// (under EVENT-1, now RESPEC'D -- see its own "RESPEC'D WHOLE" paragraph)
// and CHRONO-1/PEEK-1 (below EVENT-1) for the full, CURRENT, binding
// ordering/wire-shape/presentation/dwell-hover rules these tests pin.
//
// CHRONO-MERGE-1 (batch-chrono-merge-brief.md, owner NOD 2026-08-24: "put
// chronology up top... nix the narrative thing from hover menu... just
// don't clutter with story line where story doesn't exist"): the
// NARRATIVE nav (`event-nav`, `event-{prior,following}-event-{narrativeId}`)
// this file's own tests used to assert alongside the Chronology block is
// now RETIRED WHOLE -- CONTRACT.md's own CHRONO-MERGE-1 note (after
// TITLE-WRAP-1) has the full divergence rule, the retired testid list, and
// the per-test disposition (rewritten in place vs. retired) for every test
// in this file and `popover-sections.spec.ts`/`world-narrative-focus.spec.ts`/
// `world-pin.spec.ts` that used to reach a narrative-nav testid. Tests
// below that only ever used `event-chrono-*`/`popover-section-event-*`
// testids are untouched by that retirement and are not individually
// re-flagged.

// Exact-membership class check (never a regex against the whole attribute
// string, which can't cleanly distinguish "explorable" from "explorable-quiet"
// as substrings of each other) -- splits the real `class` attribute on
// whitespace and checks the token list directly.
async function hasClass(locator: any, className: string): Promise<boolean> {
  const attr = await locator.getAttribute('class');
  return (attr ?? '').split(/\s+/).includes(className);
}

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
  // M-D3/U5, a real, live-caught regression (this shared helper's own
  // verse-line click hung indefinitely on a real witness verse whose
  // attested mentions happened to sit at the click's own geometric
  // center -- the SAME class of hazard CONTRACT.md's own MENTION-1 note
  // documents): keyboard activation sidesteps the coordinates entirely.
  await page.getByTestId(`verse-line-${v.verse}`).focus();
  await page.keyboard.press('Enter');
  await page.getByTestId(`verse-event-${eventId}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);
}

test('HOTFIX-4 req 1/4, TRAV-1/CHRONO-1: gen_binding_isaac (a real W1 container event, no narrative membership) traverses the global timeline via the Chronology block -- FOLLOWING and PRIOR each walk >=5 hops, every destination rendering its own traversal sections', async ({ page }) => {
  // Ground truth: gen_binding_isaac is dated, a leg of NO narrative at all
  // -- exactly the owner's own report ("adjacent nodes in the dag are dead
  // ends from where we start"). Confirmed via the wire, not assumed.
  const positions = await api.narrativeEventPositions('gen_binding_isaac');
  expect(positions.narrative, 'gen_binding_isaac is a leg of no narrative').toEqual([]);
  expect(positions.timeline, 'gen_binding_isaac must have a REAL global-timeline position (this is the fix)').toBeTruthy();

  await openEventPopover(page, 'gen_binding_isaac');

  // No NARRATIVE nav at all (conditional presence -- there is no narrative
  // to show; CHRONO-MERGE-1 retires that nav whole regardless -- see this
  // file's own header comment); the Chronology block is what carries
  // traversal here (CHRONO-1: ONE section now, never two separate
  // "IN TIME" ones).
  await expect(page.getByTestId('popover-section-event-chronology')).toBeVisible();
  await expect(page.getByTestId('event-chronology-heading')).toHaveText('CHRONOLOGY');
  // CHRONO-MERGE-1/MERGE-3: a narrative-less event never has a story
  // line -- nothing non-redundant to show, and nothing TO show either.
  await expect(page.getByTestId('event-story-thread')).toHaveCount(0);

  // Walk FOLLOWING >=5 hops -- hop targets read from the wire (whatever
  // `event-chrono-following-event-global` actually links to next), never
  // hardcoded. Each destination renders its own date+places section (proof
  // the popover actually re-anchored, not just changed its title) AND its
  // own Chronology section (proof recursion -- the traversed node offers
  // FURTHER traversal, "arbitrarily far," not a one-hop special case).
  for (let hop = 0; hop < 5; hop++) {
    await expect(page.getByTestId('event-chrono-following-event-global'), `expected a FOLLOWING target at hop ${hop}`).toBeVisible();
    await page.getByTestId('event-chrono-following-event-global').click();
    await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();
  }

  // Walk PRIOR >=5 hops from a FRESH open of the same starting popover
  // (walking FOLLOWING above already moved Current away from it).
  await openEventPopover(page, 'gen_binding_isaac');
  for (let hop = 0; hop < 5; hop++) {
    await expect(page.getByTestId('event-chrono-prior-event-global'), `expected a PRIOR target at hop ${hop}`).toBeVisible();
    await page.getByTestId('event-chrono-prior-event-global').click();
    await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();
  }
});

test('CHRONO-MERGE-1/MERGE-1: pw_gethsemane (a passion-week leg whose own narrative order AGREES with the global timeline on BOTH directions) shows exactly the ONE Chronology block, no story-thread line at all', async ({ page }) => {
  // RESPEC'D from the pre-CHRONO-MERGE-1 "shows BOTH its narrative rows
  // and the Chronology block" test this replaces (CONTRACT.md's own
  // CHRONO-MERGE-1 note, after TITLE-WRAP-1, has the full retirement
  // story) -- pw_gethsemane's own narrative and timeline positions are
  // byte-identical on BOTH prior and following, ground-truthed live, so
  // this is now a clean MERGE-1 fixture: a real narrative member whose
  // order carries nothing non-redundant to show (POPOVER-LAW-1).
  const positions = await api.narrativeEventPositions('pw_gethsemane');
  const narrativePos = positions.narrative.find((p: any) => p.narrative_id === 'passion-week');
  expect(narrativePos, 'pw_gethsemane must be a passion-week leg').toBeTruthy();
  expect(positions.timeline, 'pw_gethsemane must ALSO have a global-timeline position').toBeTruthy();
  expect(narrativePos.prior?.id, 'this fixture needs a genuinely AGREEING prior').toBe(positions.timeline.prior?.id);
  expect(narrativePos.following?.id, 'this fixture needs a genuinely AGREEING following').toBe(positions.timeline.following?.id);

  await openEventPopover(page, 'pw_gethsemane');

  // ONE traversal block -- the narrative nav this event used to ALSO show
  // is retired whole (`event-nav` structurally cannot exist anywhere in
  // the DOM now, not merely absent for this particular event).
  await expect(page.getByTestId('event-nav')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-chronology')).toBeVisible();
  await expect(page.getByTestId('event-chronology-heading')).toHaveText('CHRONOLOGY');

  // No story-thread line: agreement (both directions) is NOT a divergence.
  await expect(page.getByTestId('event-story-thread')).toHaveCount(0);

  // The Chronology block's own arrows still correctly carry this event's
  // real global-timeline neighbor (unaffected by any of this).
  await expect(page.getByTestId('event-chrono-following-event-global').locator('.popover-event-nav-label')).toHaveText(positions.timeline.following.label);
});

test('HOTFIX-4 req 1/5, TRAV-1/CHRONO-1: chain-end conditional presence at the atlas\'s TRUE first/last dated event only -- the Chronology block itself still renders, only the qualifying ARROW is absent', async ({ page }) => {
  // theo-1 "Creation of all things" (-4004) is the atlas's own true first
  // dated event -- verified against the real compiled data
  // (batch-hotfix4-report.md).
  //
  // FIX ROUND 1 CORRECTION: the true LAST dated event changed. Before
  // nt_calibration reconciled the surviving Theographic-scale NT events
  // onto the AD-33 anchor, `pr_rome` ("Paul arrives at Rome") WAS the true
  // last event. `theo-385` ("Paul's First Roman imprisonment," year 60)
  // now sorts strictly after it -- an imprisonment that begins at/after
  // Paul's own arrival and (Acts 28:30, "two whole years") continues past
  // the mere arrival moment `pr_rome` itself captures. See
  // batch-hotfix4-report.md's own "Fix round 1" section.
  const first = await api.narrativeEventPositions('theo-1');
  expect(first.timeline).toBeTruthy();
  expect(first.timeline.prior, 'the true first dated event of the whole atlas has no prior').toBeFalsy();
  expect(first.timeline.following, 'but DOES have a following -- one direction present IS honest').toBeTruthy();

  const last = await api.narrativeEventPositions('theo-385');
  expect(last.timeline).toBeTruthy();
  expect(last.timeline.following, 'the true last dated event of the whole atlas has no following').toBeFalsy();
  expect(last.timeline.prior, 'but DOES have a prior').toBeTruthy();

  // CHRONO-1: the Chronology block renders whenever `timeline` is present
  // at all (i.e. the event is dated) -- INCLUDING at a true chain end,
  // where it still honestly shows a real block, just with one side the
  // empty placeholder (no testid) rather than a real arrow.
  await openEventPopover(page, 'theo-1');
  await expect(page.getByTestId('popover-section-event-chronology')).toBeVisible();
  await expect(page.getByTestId('event-chrono-prior-event-global')).toHaveCount(0);
  await expect(page.getByTestId('event-chrono-following-event-global')).toBeVisible();

  await openEventPopover(page, 'theo-385');
  await expect(page.getByTestId('popover-section-event-chronology')).toBeVisible();
  await expect(page.getByTestId('event-chrono-following-event-global')).toHaveCount(0);
  await expect(page.getByTestId('event-chrono-prior-event-global')).toBeVisible();
});

test('HOTFIX-4 req 2/5, TRAV-1/CHRONO-3: a general-kind container shows no traversal section at all -- no Chronology block (conditional presence, "NOT part of time traversal")', async ({ page }) => {
  const detail = await api.event('rob_luke_preface');
  expect(detail.kind, 'rob_luke_preface must be general-kind for this test to mean anything').toBe('general');

  const positions = await api.narrativeEventPositions('rob_luke_preface');
  expect(positions.timeline, 'a general-kind event carries NO timeline key at all, not an empty object').toBeFalsy();

  await page.goto('/read/LUK/1');
  await page.getByTestId('verse-line-1').click();
  await page.getByTestId('verse-event-rob_luke_preface').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Luke\'s preface to Theophilus');

  // CHRONO-MERGE-1: the narrative nav this used to also check is retired
  // whole (structurally absent everywhere, not merely for this event) --
  // see CONTRACT.md's own CHRONO-MERGE-1 note.
  await expect(page.getByTestId('popover-section-event-chronology')).toHaveCount(0);
});

// ---------------------------------------------------------------------
// CHRONO-MERGE-1 (batch-chrono-merge-brief.md, owner NOD 2026-08-24): the
// divergence-only story-thread line. MERGE-1/2/3/4, the brief's own
// acceptance checklist, verbatim. Every fixture's own narrative-vs-timeline
// agreement/divergence is confirmed live against the wire in each test
// below, never assumed from the brief's own illustrative names alone.
// ---------------------------------------------------------------------

test('CHRONO-MERGE-1/MERGE-1: ab_hebron (abraham-migration -- prior AGREES, following is the narrative\'s own chain end) shows exactly ONE traversal block, no story line', async ({ page }) => {
  const positions = await api.narrativeEventPositions('ab_hebron');
  const narrativePos = positions.narrative.find((p: any) => p.narrative_id === 'abraham-migration');
  expect(narrativePos, 'ab_hebron must be an abraham-migration leg').toBeTruthy();
  expect(narrativePos.prior?.id, 'this fixture needs a genuinely AGREEING prior').toBe(positions.timeline.prior?.id);
  expect(narrativePos.following, 'this fixture needs ab_hebron to be the narrative\'s own LAST leg (a chain end, not a divergence)').toBeFalsy();

  await openEventPopover(page, 'ab_hebron');
  await expect(page.getByTestId('popover-section-event-chronology')).toBeVisible();
  await expect(page.getByTestId('event-story-thread')).toHaveCount(0);
});

test('CHRONO-MERGE-1/MERGE-2: df_adullam (David\'s Flight from Saul -- FOLLOWING only diverges) shows the Chronology block PLUS the story line naming the narrative and the diverging leg; clicking the leg commits traversal to it', async ({ page }) => {
  // Ground truth, live: David's Flight from Saul's own next leg is
  // df_keilah, but the GLOBAL next event is a different thread's own
  // Chronicles entry -- the owner's own worked example (progress.md,
  // batch-chrono-merge-brief.md), confirmed against the real wire, never
  // hardcoded from the brief's own illustrative text.
  const positions = await api.narrativeEventPositions('df_adullam');
  const narrativePos = positions.narrative.find((p: any) => p.narrative_id === 'david-flight');
  expect(narrativePos, 'df_adullam must be a david-flight leg').toBeTruthy();
  expect(narrativePos.prior?.id, 'this fixture needs an AGREEING prior (single-clause line)').toBe(positions.timeline.prior?.id);
  expect(narrativePos.following?.id).toBe('df_keilah');
  expect(narrativePos.following.id, 'this fixture needs a genuinely DIVERGING following').not.toBe(positions.timeline.following?.id);
  const keilahDetail = await api.event('df_keilah');

  await openEventPopover(page, 'df_adullam');
  await expect(page.getByTestId('popover-section-event-chronology')).toBeVisible();

  const line = page.getByTestId(`event-story-thread-${narrativePos.narrative_id}`);
  await expect(line).toBeVisible();
  await expect(line).toContainText(narrativePos.narrative_name);
  await expect(line).toContainText(narrativePos.following.label);
  // Single-clause: prior agrees, so no prior leg affordance on this line.
  await expect(page.getByTestId(`event-story-thread-prior-event-${narrativePos.narrative_id}`)).toHaveCount(0);

  // MERGE-4 (retirement), live: df_adullam is exactly the class of event
  // that WOULD have rendered the old per-narrative nav -- confirms every
  // one of its own retired testids is genuinely gone, not merely absent
  // because this particular fixture never exercised them.
  await expect(page.getByTestId('event-nav')).toHaveCount(0);
  await expect(page.getByTestId(`event-prior-event-${narrativePos.narrative_id}`)).toHaveCount(0);
  await expect(page.getByTestId(`event-following-event-${narrativePos.narrative_id}`)).toHaveCount(0);
  await expect(page.getByTestId(`event-prior-label-${narrativePos.narrative_id}`)).toHaveCount(0);
  await expect(page.getByTestId(`event-following-label-${narrativePos.narrative_id}`)).toHaveCount(0);

  // Click commits: the SAME traversal affordance the block arrows give.
  const leg = page.getByTestId(`event-story-thread-following-event-${narrativePos.narrative_id}`);
  await expect(leg).toHaveText(`next → ${narrativePos.following.label}`);
  await leg.click();
  await expect(page.getByTestId('popover-title')).toHaveText(keilahDetail.title);
});

test('CHRONO-MERGE-1/MERGE-2 (dual divergence): pw_jerusalem_entry (Passion Week -- BOTH directions diverge) joins both clauses on the one line with the middle dot', async ({ page }) => {
  const positions = await api.narrativeEventPositions('pw_jerusalem_entry');
  const narrativePos = positions.narrative.find((p: any) => p.narrative_id === 'passion-week');
  expect(narrativePos, 'pw_jerusalem_entry must be a passion-week leg').toBeTruthy();
  expect(narrativePos.prior?.id, 'this fixture needs a genuinely DIVERGING prior').not.toBe(positions.timeline.prior?.id);
  expect(narrativePos.following?.id, 'this fixture needs a genuinely DIVERGING following').not.toBe(positions.timeline.following?.id);

  await openEventPopover(page, 'pw_jerusalem_entry');
  const line = page.getByTestId(`event-story-thread-${narrativePos.narrative_id}`);
  await expect(line).toBeVisible();
  await expect(line).toContainText(narrativePos.narrative_name);

  const priorLeg = page.getByTestId(`event-story-thread-prior-event-${narrativePos.narrative_id}`);
  const followingLeg = page.getByTestId(`event-story-thread-following-event-${narrativePos.narrative_id}`);
  await expect(priorLeg).toHaveText(`← ${narrativePos.prior.label}`);
  await expect(followingLeg).toHaveText(`next → ${narrativePos.following.label}`);
  await expect(line).toContainText('·'); // the one line joins both clauses, never two separate lines
});

test('CHRONO-MERGE-1/MERGE-3: gen_binding_isaac (a narrative-less dated event, the theo-* class) shows the Chronology block and NO story line', async ({ page }) => {
  const positions = await api.narrativeEventPositions('gen_binding_isaac');
  expect(positions.narrative, 'gen_binding_isaac is a leg of no narrative').toEqual([]);

  await openEventPopover(page, 'gen_binding_isaac');
  await expect(page.getByTestId('popover-section-event-chronology')).toBeVisible();
  await expect(page.getByTestId('event-story-thread')).toHaveCount(0);
});

test('AMENDMENT C: exactly one Baptism and one Temptation event exist; Baptism is chronologically PRIOR to Temptation; walking FOLLOWING from Baptism reaches Temptation directly via the Chronology block (red-then-green: RED against the pre-merge theo-267/theo-268 shape)', async ({ page }) => {
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
  await page.getByTestId('event-chrono-following-event-global').click();
  await expect(page.getByTestId('popover-title')).toHaveText(temptation.title);
});

test('HOTFIX-4 req 3, TRAV-1: map coherence -- traversing the Chronology block from a map-side event popover behaves exactly like narrative traversal (shared code path, no special case)', async ({ page }) => {
  // Split view (`?split=world`, SPLIT-1) puts the reader AND the atlas pane on
  // screen together -- the same "map-side" surface a narrative popover's
  // own MAP FOCUS SYNC already targets today, reached without inventing a
  // new navigation path for this test alone.
  //
  // Part 1: open a NARRATIVE event (pw_gethsemane) -- real focus state
  // must exist (proves there's something live to get wrong, not asserting
  // against a scene that never had any arrows to begin with) -- then
  // traverse ONE hop via the CHRONOLOGY row (not the narrative one,
  // requirement 3's own surface). jesus-ministry/passion-week are now
  // dense (122/49 legs, verified against the real compiled data), so the
  // immediate timeline neighbor is very likely STILL a narrative member --
  // that's fine: "shared code path, no special case" means focus just
  // correctly re-syncs to whatever Current actually is, which this proves
  // either way (still-narrative or not).
  const gethsemaneDetail = await api.event('pw_gethsemane');
  const vref = gethsemaneDetail.witnesses[0].verse_groups[0].verses[0];
  const v = parseVerse(vref);
  await page.goto(`/read/${v.book}/${v.chapter}?split=world`);
  // Keyboard activation -- see openEventPopover's own comment above for
  // why a plain coordinate click on a verse-line is unsafe now.
  await page.getByTestId(`verse-line-${v.verse}`).focus();
  await page.keyboard.press('Enter');
  await page.getByTestId('verse-event-pw_gethsemane').click();
  await expect(page.getByTestId('popover-title')).toHaveText(gethsemaneDetail.title);
  await expect(page.locator('[data-narrative-focus]').first()).toBeAttached();

  const gethsemanePositions = await api.narrativeEventPositions('pw_gethsemane');
  await page.getByTestId('event-chrono-following-event-global').click();
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
  await page.goto(`/read/${iv.book}/${iv.chapter}?split=world`);
  await page.getByTestId(`verse-line-${iv.verse}`).focus();
  await page.keyboard.press('Enter');
  await page.getByTestId('verse-event-gen_binding_isaac').click();
  await expect(page.getByTestId('popover-title')).toHaveText(isaacDetail.title);

  const staleFocus = await page.locator('[data-narrative-focus]').count();
  expect(staleFocus, 'a narrative-less Current must clear every arrow\'s own data-narrative-focus attribute to baseline').toBe(0);
});

// ---------------------------------------------------------------------
// AFFORDANCE-1 (requirement 6): "if something isn't traversable it
// shouldn't look like other things that are actually traversable."
// ---------------------------------------------------------------------

test('AFFORDANCE-1: a general-kind container\'s own reader heading renders visibly distinct from a dated event\'s (discriminating class asserted, not just wording)', async ({ page }) => {
  const detail = await api.event('rob_luke_preface');
  expect(detail.kind).toBe('general');

  await page.goto('/read/LUK/1');
  const heading = page.getByTestId('pericope-heading-rob_luke_preface');
  await expect(heading).toBeVisible();
  expect(await hasClass(heading, 'explorable'), 'a general-kind heading must NOT carry .explorable (that would claim traversal it does not have)').toBe(false);
  expect(await hasClass(heading, 'explorable-quiet'), 'a general-kind heading MUST carry the discriminating .explorable-quiet class').toBe(true);

  // Still honestly clickable -- opens its own real popover, just doesn't
  // LOOK like a chain link beforehand.
  await heading.click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);
  await expect(page.getByTestId('popover-section-event-chronology')).toHaveCount(0);
});

test('AFFORDANCE-1: a dated event\'s own reader heading and verse EVENT-membership row keep the traversable .explorable class, and every one of a dated event\'s own affordances actually traverses', async ({ page }) => {
  // jm_egypt is dated, real, heading-worthy (a narrative leg AND a merge
  // survivor -- HOTFIX-4's own event_merge.rs) -- the positive control
  // against the general-kind test immediately above.
  const detail = await api.event('jm_egypt');
  expect(detail.kind).toBe('event');
  const vref = detail.witnesses[0].verse_groups[0].verses[0];
  const v = parseVerse(vref);

  await page.goto(`/read/${v.book}/${v.chapter}`);
  const heading = page.getByTestId(`pericope-heading-jm_egypt`);
  expect(await hasClass(heading, 'explorable')).toBe(true);
  expect(await hasClass(heading, 'explorable-quiet')).toBe(false);

  // Click the heading -- a real dated event's own affordance -- confirm it
  // actually traverses (opens the popover, the Chronology block present).
  await heading.click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);
  await expect(page.getByTestId('popover-section-event-chronology')).toBeVisible();
  await page.keyboard.press('Escape'); // close the popover before the next click -- it otherwise intercepts pointer events over the reader

  // The SAME event's own verse-membership row also keeps .explorable --
  // checked directly against the verse popover's own EVENT section.
  // Keyboard activation -- see openEventPopover's own comment above.
  await page.getByTestId(`verse-line-${v.verse}`).focus();
  await page.keyboard.press('Enter');
  const row = page.getByTestId('verse-event-jm_egypt');
  expect(await hasClass(row, 'explorable')).toBe(true);
  expect(await hasClass(row, 'explorable-quiet')).toBe(false);
  await row.click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);
});

// ---------------------------------------------------------------------
// TRUNC-1 (requirement 7): the 20-verse wire cap's own honest truncation
// signal -- the owner's own temple-dedication acceptance case. Unrelated
// to timeline/Chronology traversal -- untouched by TRAV-1.
// ---------------------------------------------------------------------

test('TRUNC-1/ACCT-COALESCE-1: the temple-dedication popover\'s 1KI.8 witness shows the +46-more affordance and opens the full chapter; the 2CH witness (fix round 1) coalesces its own 3 chapters into ONE account, disclosing truncation summed across every chapter it spans', async ({ page }) => {
  const detail = await api.event('1ki_temple_dedication');
  const kingsWitness = detail.witnesses.find((w: any) => w.book === '1KI');
  const chroniclesWitness = detail.witnesses.find((w: any) => w.book === '2CH');
  expect(kingsWitness, '1 Kings 8 is the owner\'s own named acceptance witness').toBeTruthy();
  // Ground truth, at the wire level: the true count vs. what's delivered.
  const kingsGroup = kingsWitness.verse_groups.find((g: any) => g.chapter === 8);
  expect(kingsGroup.count, '1 Kings 8 has 66 real verses').toBe(66);
  expect(kingsGroup.verses.length, 'the wire caps the delivered verses at 20').toBe(20);
  const missing = kingsGroup.count - kingsGroup.verses.length;
  expect(missing).toBe(46);

  // M-D3/U6, owner verbatim: "'read the whole chapter' affordance REMOVED
  // when already reading that chapter" -- correctly true here of a WITNESS
  // entry's own expand button, same as any other: opening this event via a
  // 1KI.8 verse-line click means popover-verse-expand-event-witness-1KI.8.1-20
  // is now the chapter the reader is already showing (nothing left for it
  // to honestly offer -- the FULL, real chapter is already the page behind
  // it), so it is correctly absent, not merely inert. Verified from TWO
  // separate navigations instead of one -- this event has exactly two
  // witnesses (1KI.8, 2CH.5-7), each needs to NOT be the reader's own
  // displayed chapter to stay expand-testable, and no third witness exists
  // to anchor a single neutral navigation for both at once.
  const kingsVref = kingsWitness.verse_groups[0].verses[0];
  const kingsV = parseVerse(kingsVref);
  const chroniclesVref = chroniclesWitness.verse_groups[0].verses[0];
  const chroniclesV = parseVerse(chroniclesVref);

  // Pass 1: reader on 2 Chronicles -- 1 Kings 8's own witness entry is a
  // DIFFERENT chapter, so its own truncation affordance is fully testable.
  await page.goto(`/read/${chroniclesV.book}/${chroniclesV.chapter}`);
  // Keyboard activation -- see openEventPopover's own comment above.
  await page.getByTestId(`verse-line-${chroniclesV.verse}`).focus();
  await page.keyboard.press('Enter');
  await page.getByTestId('verse-event-1ki_temple_dedication').click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);
  await expect(page.getByTestId('popover-section-event-witnesses')).toBeVisible();

  // The truncated 1 Kings 8 entry specifically (the cap keeps the LOWEST-
  // numbered 20 of 66, so its own delivered span is deterministically
  // 1KI.8.1-20): quiet "+46 more — read the chapter" affordance. O2
  // (2026-08-23) retired the old text-button (and its own `data-truncated`
  // marker) in favor of RevealControls' shared down/double-down arrow pair
  // -- the identical truncation-aware wording now lives in the button's own
  // title/aria-label (RevealControls' MoreLabel override, MiniReaderExpand.razor's
  // own O2 comment) rather than its visible text (an icon glyph now), still
  // wired to the SAME MiniReaderExpand control, no parallel affordance --
  // clicking it opens the real, full chapter.
  const kingsExpand = page.getByTestId('popover-verse-expand-event-witness-1KI.8.1-20');
  await expect(kingsExpand).toHaveAttribute('title', `+${missing} more — read the chapter`);
  await kingsExpand.click();
  const chapter = await api.chapter('1KI.8');
  await expect(page.getByTestId(/^popover-reader-verse-/).first()).toBeVisible();
  await expect(page.getByTestId(/^popover-reader-verse-/)).toHaveCount(chapter.verses.length);

  // Pass 2: reader on 1 Kings -- the 2 Chronicles witness entry is now the
  // DIFFERENT chapter, so ITS OWN affordance is testable instead.
  //
  // ACCT-COALESCE-1 (fix round 1, owner bug report -- "in parallel
  // accounts... accounts from the same book + chapter are listed. makes
  // no sense"): this witness's own 3 VerseGroups (2CH.5/6/7, ONE curated
  // witness row) now COALESCE into a single account, span "2CH.5.2-7.10"
  // -- never 3 separate same-book entries. Truncation is summed across
  // EVERY distinct chapter the coalesced block spans (chapter 5: 13
  // verses, under cap; chapter 6: 42 real, capped at 20, 22 missing;
  // chapter 7: 10 verses, under cap) -- 22 total, disclosed on the ONE
  // entry's own expand affordance (PassageBlockBuilder.BuildCoalescedBlock's
  // own "sum across every group" math, mirroring ArrowNav.ComputeTruncatedBy).
  const ch5 = chroniclesWitness.verse_groups.find((g: any) => g.chapter === 5);
  const ch6 = chroniclesWitness.verse_groups.find((g: any) => g.chapter === 6);
  const ch7 = chroniclesWitness.verse_groups.find((g: any) => g.chapter === 7);
  expect(ch5.count, '2 Chronicles 5 (within this witness) is under the cap').toBeLessThan(20);
  expect(ch6.count, '2 Chronicles 6 is genuinely over the cap -- the SAME honest signal fires for a MIDDLE chapter of the coalesced span, not just the owner\'s own named 1 Kings case').toBeGreaterThan(20);
  expect(ch7.count, '2 Chronicles 7 (within this witness) is under the cap').toBeLessThan(20);
  const coalescedMissing = (ch5.count - ch5.verses.length) + (ch6.count - ch6.verses.length) + (ch7.count - ch7.verses.length);
  expect(coalescedMissing).toBe(ch6.count - ch6.verses.length); // only ch6 contributes, but summed generically, not hardcoded to "only ch6 matters"

  await page.goto(`/read/${kingsV.book}/${kingsV.chapter}`);
  await page.getByTestId(`verse-line-${kingsV.verse}`).focus();
  await page.keyboard.press('Enter');
  await page.getByTestId('verse-event-1ki_temple_dedication').click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);

  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  const entries = witnessesSection.locator('[data-testid^="event-witness-"]');
  await expect(entries).toHaveCount(2); // ONE per witness ROW (1KI + 2CH), never one per chapter

  // Resolve the coalesced entry's own real span directly off the DOM
  // (never reconstruct PassageGrouping.SpanRef's own formatting logic by
  // hand in the test -- popover-sections.spec.ts's own established
  // discipline) rather than guessing the testid.
  const allTestIds = await entries.evaluateAll((els) => els.map((el) => el.getAttribute('data-testid')));
  const chroniclesEntryTestId = allTestIds.find((id) => id?.startsWith('event-witness-2CH.'));
  expect(chroniclesEntryTestId, 'exactly one coalesced 2CH account entry must exist').toBeTruthy();
  const chroniclesEntry = witnessesSection.getByTestId(chroniclesEntryTestId!);
  const chroniclesExpandBtn = chroniclesEntry.getByTestId(`popover-verse-expand-${chroniclesEntryTestId}`);
  await expect(chroniclesExpandBtn).toHaveAttribute('title', `+${coalescedMissing} more — read the chapter`);

  // Expanding opens the coalesced entry's own FIRST chapter (2 Chronicles
  // 5, MiniReaderExpand's own one-chapter-at-a-time limit -- PassageList.razor's
  // own FocalToOf comment has the "why" story) in full.
  await chroniclesExpandBtn.click();
  const chapter5 = await api.chapter('2CH.5');
  await expect(chroniclesEntry.locator('[data-testid^="popover-reader-verse-"]')).toHaveCount(chapter5.verses.length);
});

// ---------------------------------------------------------------------
// HOVER-KILL-1 (fix round 2, owner verbatim: "get rid of the box that
// comes up when hovering over prior/following event buttons"): an OWNER
// REVERSAL of UX-1/PEEK-1's own dwell-hover verse peek, scoped to the
// Chronology block's own BLOCK-mode PRIOR/FOLLOWING buttons specifically
// -- "these buttons" names them, not the story-thread line's own Inline
// legs (a DIFFERENT surface the SAME peek mechanism still lives on,
// untouched -- see "PEEK-1/CHRONO-MERGE-1," below, the surviving proof
// the underlying mechanism is not removed, only gated off block-mode's
// own entry point, `Components.ArrowNav.OnPointerEnter`). RETIRED WHOLE
// (not rewritten, per this house's own CHRONO-MERGE-1 retirement
// precedent -- their own fixture, a block-mode arrow, can no longer ever
// trigger a peek, so PEEK-2 through PEEK-5's own detailed content/
// placement/truncation assertions and both EVENT-HOVER-HATCH-1 close/
// escape tests have nothing left to exercise on this surface; the
// mechanism they proved is unchanged code, still real, just no longer
// reachable from a block-mode arrow): the former `PEEK-1`,
// `EVENT-HOVER-HATCH-1` (x2), `PEEK-2`, `PEEK-3`, `PEEK-3b`, `PEEK-4`,
// `PEEK-5` tests (all targeting `event-chrono-{prior,following}-event-global`)
// are gone; `TITLE-2` (below) is TRIMMED, not retired -- its own two-line-
// clamp/grid-alignment assertions are unrelated to the peek and stay
// live, only its trailing "peek header carries the full name" paragraph
// is removed.
// ---------------------------------------------------------------------

test('HOVER-KILL-1: hovering a Chronology block PRIOR/FOLLOWING button never shows a peek, however long the dwell; click still commits the traversal', async ({ page }) => {
  const positions = await api.narrativeEventPositions('gen_binding_isaac');
  expect(positions.timeline.following, 'gen_binding_isaac must have a real FOLLOWING target for this test to mean anything').toBeTruthy();
  const followingDetail = await api.event(positions.timeline.following.id);

  await openEventPopover(page, 'gen_binding_isaac');
  const arrow = page.getByTestId('event-chrono-following-event-global');
  await expect(arrow).toBeVisible();
  const peek = page.getByTestId('event-chrono-following-event-global-peek');

  // A quick, un-lingering hover shows nothing (unchanged baseline).
  await arrow.hover({ force: true });
  await expect(peek).toHaveCount(0);

  // A genuine, SUSTAINED dwell -- comfortably past DwellTiming.PeekDelayMs
  // (375ms), the exact window PEEK-1 used to require for the box to
  // appear -- must STILL show nothing: the owner's own reversal, proven
  // by waiting through the window that used to trigger it and finding no
  // peek regardless.
  await page.waitForTimeout(600);
  await expect(peek).toHaveCount(0);

  // Click still commits exactly as before -- HOVER-KILL-1 touches only
  // the hover affordance, never the click one.
  await arrow.click();
  await expect(page.getByTestId('popover-title')).toHaveText(followingDetail.title);
});

test('PEEK-1/CHRONO-MERGE-1: the SAME dwell-hover peek works identically on the story-thread line\'s own INLINE leg (df_adullam, David\'s Flight from Saul) -- one shared component, not a parallel implementation, HOVER-KILL-1\'s own surviving proof the mechanism is not removed', async ({ page }) => {
  // RESPEC'D from the pre-CHRONO-MERGE-1 "narrative-nav arrow" fixture
  // this test used (pw_gethsemane/passion-week) -- that whole affordance
  // is retired; the SAME "second consumer of the shared peek" proof this
  // test always existed for now targets the surviving second consumer,
  // the story-thread line's own inline leg (CONTRACT.md's own
  // CHRONO-MERGE-1 note has the retirement story).
  const positions = await api.narrativeEventPositions('df_adullam');
  const narrativePos = positions.narrative.find((p: any) => p.narrative_id === 'david-flight');
  expect(narrativePos?.following?.id, 'df_adullam must have a real, genuinely DIVERGING following leg for this test to mean anything').toBe('df_keilah');
  expect(narrativePos.following.id).not.toBe(positions.timeline.following?.id);

  await openEventPopover(page, 'df_adullam');
  const arrow = page.getByTestId(`event-story-thread-following-event-${narrativePos.narrative_id}`);
  await expect(arrow).toBeVisible();
  const peek = page.getByTestId(`event-story-thread-following-event-${narrativePos.narrative_id}-peek`);

  await expect(peek).toHaveCount(0); // nothing before any hover at all

  await arrow.hover({ force: true });
  await expect(peek).toBeVisible({ timeout: 2000 });
  // Real content, not an empty shell -- the peek actually resolved and
  // rendered the target's own verse text. PEEK-TRUNC-1: no longer via
  // PassageList (`.popover-passage-text`) -- ArrowNav's own peek renders
  // its (at most one, by default) verse directly, `.popover-arrow-peek-verse`.
  await expect(peek.locator('.popover-arrow-peek-verse')).toHaveCount(1);
  expect((await peek.locator('.popover-arrow-peek-verse').first().textContent())?.trim().length, 'the peek must carry real, non-empty verse text').toBeGreaterThan(0);

  await page.mouse.move(2, 2);
  await expect(peek).toHaveCount(0);
});

// PEEK-TRUNC-1's own former PEEK-2/PEEK-3/PEEK-3b/PEEK-4 tests (content/
// placement/transit-corridor behavior on a BLOCK-mode arrow's own peek)
// are RETIRED WHOLE by HOVER-KILL-1, above -- their own fixture can no
// longer ever trigger a peek at all. See that section's own header
// comment for the full retirement citation.

// ---------------------------------------------------------------------
// TITLE-WRAP-1 (owner report, 2026-08-24, verbatim: "i don't like that
// arrow titles are getting cut off with elipses... we need to find a way
// to have a nice presentation while showing a relatively full title.").
// CONTRACT.md's own TITLE-WRAP-1 note has the full, binding contract.
// ---------------------------------------------------------------------

test('TITLE-2: a long event name renders via the two-line clamp (never a single-line ellipsis) and the fixed grid holds', async ({ page }) => {
  // rob_elijah_puzzle's own PRIOR (rob_transfiguration, "The
  // Transfiguration", 19 chars) and FOLLOWING (rob_demoniac_boy, "Jesus
  // heals a demoniac boy the disciples could not heal", 57 chars) are real
  // neighbors on this event's own GLOBAL TIMELINE, one clearly short and
  // one clearly long -- opening ITS popover renders both Chronology arrows
  // side by side in one row, so the grid-alignment comparison below is
  // between two REAL rows on the SAME live page, not a synthetic fixture.
  // CHRONO-MERGE-1: RESPEC'D from the narrative-nav fixture this test used
  // (jesus-ministry) -- this fixture's own narrative and timeline
  // positions are byte-identical live, so retargeting onto the Chronology
  // block's own testids changes nothing about what TITLE-WRAP-1 itself is
  // proving (that contract lives entirely in `Components.ArrowNav`'s
  // BLOCK-mode rendering, shared by both families).
  const positions = await api.narrativeEventPositions('rob_elijah_puzzle');
  expect(positions.timeline.prior?.id).toBe('rob_transfiguration');
  expect(positions.timeline.following?.id).toBe('rob_demoniac_boy');
  const shortLabel: string = positions.timeline.prior.label;
  const longLabel: string = positions.timeline.following.label;
  expect(shortLabel.length, 'this test needs a genuinely short neighbor label').toBeLessThanOrEqual(24);
  expect(longLabel.length, 'this test needs a genuinely long neighbor label').toBeGreaterThan(50);

  await openEventPopover(page, 'rob_elijah_puzzle');
  const shortNameLabel = page.getByTestId('event-chrono-prior-event-global').locator('.popover-event-nav-label');
  const longNameLabel = page.getByTestId('event-chrono-following-event-global').locator('.popover-event-nav-label');
  await expect(shortNameLabel).toHaveText(shortLabel);
  await expect(longNameLabel).toHaveText(longLabel);

  // The short name renders one line, standard size -- never the two-line
  // clamp class.
  expect(await hasClass(shortNameLabel, 'popover-event-nav-label-long')).toBe(false);
  // The long name renders via the two-line clamp -- never a single-line
  // ellipsis. Checked two ways: the class itself (app.css's own
  // .popover-event-nav-label-long), and the COMPUTED -webkit-line-clamp
  // value it declares (2) -- never a bounding-box height comparison
  // against the short label: app.css's own min-height on the BASE rule
  // deliberately reserves the SAME worst-case height for BOTH variants
  // (that reservation is exactly what keeps the grid aligned, asserted
  // below), so short-vs-long rendered height is expected to be IDENTICAL
  // by design, not a signal of which variant is active.
  expect(await hasClass(longNameLabel, 'popover-event-nav-label-long')).toBe(true);
  const longClamp = await longNameLabel.evaluate((el) => getComputedStyle(el).getPropertyValue('-webkit-line-clamp'));
  expect(longClamp, 'the long name must genuinely clamp to two lines, not just carry an inert class').toBe('2');

  // FIXED GRID: the PRIOR/FOLLOWING role-caption row stays aligned even
  // though one side's own name is two lines and the other is one --
  // compare the two `.popover-event-nav-role` positions directly, a
  // small pixel tolerance for sub-pixel/font-metric rounding only.
  const shortRole = page.getByTestId('event-chrono-prior-label-global');
  const longRole = page.getByTestId('event-chrono-following-label-global');
  const shortRoleBox = await shortRole.boundingBox();
  const longRoleBox = await longRole.boundingBox();
  expect(shortRoleBox).not.toBeNull();
  expect(longRoleBox).not.toBeNull();
  expect(Math.abs(shortRoleBox!.y - longRoleBox!.y), 'the PRIOR/FOLLOWING role captions must not shift out of alignment').toBeLessThanOrEqual(3);

  // HOVER-KILL-1 (fix round 2): this test's own former trailing paragraph
  // ("the peek header always carries the full name," dwelling the
  // long-named arrow) is REMOVED, not merely modified -- a block-mode
  // arrow's own peek is gone entirely now; see that section's own header
  // comment. Everything above (the clamp/grid-alignment contract itself)
  // is unrelated to the peek and stays fully live.
});

// PEEK-5 (the server-capped-group truncation disclosure on a BLOCK-mode
// arrow's own peek) is RETIRED WHOLE by HOVER-KILL-1, above, same
// citation as PEEK-2/3/3b/4.

// ---------------------------------------------------------------------
// EVT-3 Ticket 2 (owner ruling, SUPERSEDES EV-1): the Chronology block's
// own block-mode arrow rows used to render an ALWAYS-VISIBLE primary-
// witness-VERSE-TEXT line (EV-1, Batch CHRON-1). RETIRED -- the owner's
// own later, more specific ruling on this exact surface (UX-2 feedback,
// 2026-09-06, verbatim: "it does not appear to me that you did what i told
// you for the chronological thing (not showing the verse contents, just
// the list of refs)"; EVENT-ACCOUNTS-1's own governing law, "events are
// keys that map to sets of Biblical accounts... chronological traversal
// can return a set of passages with each hop"): the row now shows the SET
// of the adjacent event's own account REFS -- via the shared, reusable
// RefsList component -- NEVER verse text, gated behind nothing (the dwell
// peek, PEEK-1 above, is UNCHANGED, still the ONE place a hover reveals
// real passage text). See ArrowNav.razor's own header comment and
// CONTRACT.md's own EV-1 note (superseded in place) for the full rule.
// ---------------------------------------------------------------------

test('EVT-3/RefsList: a Chronology traversal row shows the target event\'s own SET of account refs immediately, with NO verse contents anywhere outside the dwell peek', async ({ page }) => {
  const positions = await api.narrativeEventPositions('gen_binding_isaac');
  expect(positions.timeline.following, 'gen_binding_isaac must have a real FOLLOWING target for this test to mean anything').toBeTruthy();

  await openEventPopover(page, 'gen_binding_isaac');
  const arrow = page.getByTestId('event-chrono-following-event-global');
  await expect(arrow).toBeVisible();

  // No hover/dwell needed -- the refs list renders immediately off the
  // (possibly still-fallback) Adjacent.VerseGroups, then upgrades in
  // place once ACCT-SET-MISMATCH-1's own eager EventDetail fetch resolves
  // (ArrowNav.razor's own RefDescriptors doc comment) -- Playwright's
  // auto-retrying `toBeVisible`/`getByTestId` below tolerate either
  // timing; gen_binding_isaac's own FOLLOWING (gen_death_of_sarah) is a
  // real single-implicit-witness event (no curated parallel account), so
  // BOTH derivations converge on the identical single ref regardless.
  const refsList = page.getByTestId('event-chrono-following-event-global-refs');
  await expect(refsList).toBeVisible();

  // Ground truth: ONE ref per VerseGroup on the wire -- the SET, not just
  // the first ("primary") one EV-1 used to show alone (ArrowNav.SelectRefs's
  // own pure-logic proof lives in client.Tests/ArrowNavTests.cs; this is
  // the real end-to-end wire-through-DOM confirmation).
  const groups = positions.timeline.following.verse_groups;
  expect(groups.length).toBeGreaterThan(0);
  for (const group of groups) {
    const firstVref = group.verses[0];
    const lastVref = group.verses[group.verses.length - 1];
    const expectedRef = firstVref === lastVref ? firstVref : `${firstVref.split('.').slice(0, 2).join('.')}.${firstVref.split('.')[2]}-${lastVref.split('.')[2]}`;
    await expect(refsList.getByTestId(`event-chrono-following-event-global-refs-${expectedRef}`)).toBeVisible();
  }

  // NO verse contents anywhere in the row -- the owner's own words,
  // verbatim -- never a compact-passage/mention render outside the peek.
  await expect(refsList.locator('.popover-passage-text')).toHaveCount(0);
  await expect(refsList.locator('[class*="mention"]')).toHaveCount(0);

  // HOVER-KILL-1 (fix round 2): the peek is GONE for block-mode arrows --
  // superseding this test's own former "the peek still exists alongside
  // this" assertion. Hovering, even a sustained dwell, must show nothing.
  const peek = page.getByTestId('event-chrono-following-event-global-peek');
  await expect(peek).toHaveCount(0);
  await arrow.hover({ force: true });
  await page.waitForTimeout(600);
  await expect(peek).toHaveCount(0);
});

test('EVT-3/RefsList: clicking a ref under a Chronology arrow explores directly to that account\'s own first verse (a genuine shortcut, distinct from the arrow\'s own whole-event traversal)', async ({ page }) => {
  const positions = await api.narrativeEventPositions('gen_binding_isaac');
  const group = positions.timeline.following.verse_groups[0];
  const firstVref = group.verses[0];
  const lastVref = group.verses[group.verses.length - 1];
  const expectedRef = firstVref === lastVref ? firstVref : `${firstVref.split('.').slice(0, 2).join('.')}.${firstVref.split('.')[2]}-${lastVref.split('.')[2]}`;

  await openEventPopover(page, 'gen_binding_isaac');
  await page.getByTestId(`event-chrono-following-event-global-refs-${expectedRef}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(firstVref);
});

test('ACCT-SET-MISMATCH-1: rob_twelve_apostles -- the refs shown under the Chronology arrow pointing at it match the SAME SET and ORDER as its own landed PARALLEL ACCOUNTS frontier (the owner\'s own repro: MRK-only under the button, but a real LUKE parallel once landed)', async ({ page }) => {
  const detail = await api.event('rob_twelve_apostles');
  expect(detail.witnesses.length, 'ground truth: rob_twelve_apostles must have 2+ real witnesses (LUK + MRK) for this test to mean anything -- the owner\'s own named repro').toBeGreaterThan(1);

  const positions = await api.narrativeEventPositions('rob_twelve_apostles');
  const neighbor = positions.timeline.prior ?? positions.timeline.following;
  expect(neighbor, 'rob_twelve_apostles needs a real timeline neighbor to open an arrow pointing at it').toBeTruthy();
  const direction = positions.timeline.prior ? 'following' : 'prior'; // opening the NEIGHBOR, the arrow pointing BACK at rob_twelve_apostles is the opposite direction

  await openEventPopover(page, neighbor.id);
  const arrow = page.getByTestId(`event-chrono-${direction}-event-global`);
  await expect(arrow).toBeVisible();
  const refsList = page.getByTestId(`event-chrono-${direction}-event-global-refs`);

  // Wait for the CORRECT (fetched, witness-derived) refs to land -- not
  // merely the narrower Adjacent.VerseGroups fallback (which is exactly
  // the owner's own bug: MRK-only, missing the LUK parallel).
  const refButtons = refsList.locator('[data-testid^="event-chrono-' + direction + '-event-global-refs-"]');
  await expect.poll(() => refButtons.count()).toBe(detail.witnesses.length);
  const refTexts = await refButtons.allTextContents();

  // Land on the event and confirm the SAME set+order on the PARALLEL
  // ACCOUNTS frontier -- ONE source (EventDetail.Witnesses, coalesced),
  // rendered two ways, never two independent derivations.
  await arrow.click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);
  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection).toBeVisible();
  const entries = witnessesSection.locator('[data-testid^="event-witness-"]');
  await expect(entries).toHaveCount(detail.witnesses.length);
  const entryTexts = await entries.locator('.popover-passage-ref-label').allTextContents();

  expect(entryTexts, 'the landed frontier\'s own account refs must match the SET the button showed').toEqual(refTexts);
});

test('EVT-3/RefsList: the Inline story-thread leg (a diverging narrative row) stays title-only, unaffected by the block-mode refs list', async ({ page }) => {
  // pw_jerusalem_entry is CONTRACT.md's own named dual-divergence fixture
  // (passion-week's own prior AND following both differ from the global
  // timeline) -- guaranteed to render at least one story-thread leg.
  await openEventPopover(page, 'pw_jerusalem_entry');
  const storyThread = page.getByTestId('event-story-thread');
  await expect(storyThread).toBeVisible();
  const leg = storyThread.locator('.popover-story-thread-leg').first();
  await expect(leg).toBeVisible();
  // No refs-list anywhere inside an inline leg -- this ticket's own
  // scoping decision (ArrowNav.razor's own header comment): Inline rows
  // are a running-prose leg reference, never a traversal row of their own.
  await expect(leg.locator('.popover-refs-list')).toHaveCount(0);
});

// DUP-DEATH REGRESSION (the owner's own original repro, ledgered in
// .superpowers/sdd/2026-08-17-bible-atlas-m1/dup-events-investigation.md):
// before Batch CHRON-1's own charter merge (rob_leper_healed/theo-286),
// MAT.8.3 cited TWO independently-dated events for the identical Gospel
// pericope -- two PARALLELS cards, two conflicting dates. THE CHRONOLOGY
// AUTHORITY LAW (owner: "why are we pulling chronology from conflicting
// sources? we should have one absolute source of truth") fixes this at
// the data layer (server/atlas-core/src/event_merge.rs); this test proves
// it end to end, through the real popover a reader actually sees.
test('EV-1/dup-death regression: MAT.8.3\'s own popover shows exactly ONE event (the leper pair, rob_leper_healed/theo-286, is merged)', async ({ page }) => {
  const verseOut = await api.verse('MAT.8.3');
  expect(verseOut.events.map((e: any) => e.id), 'MAT.8.3 must cite exactly one event id, never two independently-dated opinions about the identical pericope').toEqual(['rob_leper_healed']);

  await page.goto('/read/MAT/8');
  await page.getByTestId('verse-line-3').click();
  await expect(page.getByTestId('popover-title')).toHaveText('MAT.8.3');

  const eventSection = page.getByTestId('popover-section-event-membership');
  await expect(eventSection).toBeVisible();
  await expect(eventSection.getByTestId('event-section-heading')).toHaveText('EVENT');
  await expect(eventSection.getByTestId('verse-event-rob_leper_healed')).toBeVisible();
  // No second, independently-dated event card for the same pericope.
  await expect(eventSection.locator('[data-testid^="verse-event-"]')).toHaveCount(1);

  // The PARALLELS section (this verse's own OTHER witnesses, Mark/Luke)
  // still shows exactly one event group -- the plain "PARALLELS" heading,
  // never the multi-event "PARALLELS — {label}" variant that a live
  // duplicate would have produced.
  const parallels = page.getByTestId('popover-section-parallels');
  await expect(parallels).toBeVisible();
  await expect(parallels.getByTestId('event-section-heading')).toHaveCount(1);
  await expect(parallels.getByTestId('event-section-heading')).toHaveText('PARALLELS');
});
