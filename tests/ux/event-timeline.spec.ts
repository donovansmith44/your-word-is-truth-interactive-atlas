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
  // to show; M-D3/U1 folded it into event-date-places, no more separate
  // section -- `event-nav` is the presence signal now); the Chronology
  // block is what carries traversal here (CHRONO-1: ONE section now,
  // never two separate "IN TIME" ones).
  await expect(page.getByTestId('event-nav')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-chronology')).toBeVisible();
  await expect(page.getByTestId('event-chronology-heading')).toHaveText('CHRONOLOGY');

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

test('HOTFIX-4 req 1/req 5, TRAV-1/CHRONO-1: a narrative-member event shows BOTH its narrative rows and the Chronology block, correctly and independently', async ({ page }) => {
  // pw_gethsemane is a passion-week narrative leg AND a dated, real event --
  // narrative primacy preserved (requirement 1 verbatim): both families of
  // sections must render, neither replacing the other.
  const positions = await api.narrativeEventPositions('pw_gethsemane');
  const narrativePos = positions.narrative.find((p: any) => p.narrative_id === 'passion-week');
  expect(narrativePos, 'pw_gethsemane must be a passion-week leg').toBeTruthy();
  expect(positions.timeline, 'pw_gethsemane must ALSO have a global-timeline position').toBeTruthy();

  await openEventPopover(page, 'pw_gethsemane');

  // M-D3/U1: the narrative row is a compact flanking-arrow nav folded into
  // event-date-places (`event-nav`), not its own headed section -- the
  // Chronology block (CHRONO-1) is UNCHANGED in that respect from the
  // retired "IN TIME" rows it replaces: still its own separate, headed
  // section.
  await expect(page.getByTestId('event-nav')).toBeVisible();
  await expect(page.getByTestId('popover-section-event-chronology')).toBeVisible();

  // "quiet, clearly distinct," structurally, not just by wording: the
  // narrative row carries NO heading at all (M-D3/U1's own established
  // design, unchanged), while the Chronology block's own heading
  // (`event-chronology-heading`) is a DIFFERENT testid from the generic
  // `event-section-heading` OTHER headed sections in this popover platform
  // share (e.g. PARALLEL ACCOUNTS, present here too since pw_gethsemane
  // has real witnesses -- that generic testid is not unique to the retired
  // "IN TIME" sections, so this only asserts the wording itself is gone,
  // never that the shared testid has zero uses).
  await expect(page.getByTestId('event-chronology-heading')).toHaveText('CHRONOLOGY');
  const otherHeadings = await page.getByTestId('event-section-heading').allTextContents();
  expect(otherHeadings.some((h: string) => h.includes('IN TIME'))).toBeFalsy();

  // The narrative row's own target and the Chronology row's own target
  // need not be the same event (different questions -- "next in this
  // narrative's own leg chain" vs "next chronologically at all") -- both
  // are independently explorable and correct per their own semantics.
  // .popover-event-nav-label specifically -- the button's own FULL text
  // also includes its decorative directional glyph (a sibling span, never
  // meant to be part of an exact-text comparison).
  if (narrativePos.following) {
    await expect(page.getByTestId('event-following-event-passion-week').locator('.popover-event-nav-label')).toHaveText(narrativePos.following.label);
  }
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

test('HOTFIX-4 req 2/5, TRAV-1/CHRONO-3: a general-kind container shows no traversal section at all -- neither narrative nor Chronology (conditional presence, "NOT part of time traversal")', async ({ page }) => {
  const detail = await api.event('rob_luke_preface');
  expect(detail.kind, 'rob_luke_preface must be general-kind for this test to mean anything').toBe('general');

  const positions = await api.narrativeEventPositions('rob_luke_preface');
  expect(positions.timeline, 'a general-kind event carries NO timeline key at all, not an empty object').toBeFalsy();

  await page.goto('/read/LUK/1');
  await page.getByTestId('verse-line-1').click();
  await page.getByTestId('verse-event-rob_luke_preface').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Luke\'s preface to Theophilus');

  await expect(page.getByTestId('event-nav')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-chronology')).toHaveCount(0);
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
  // Split view (`?split=1`, SPLIT-1) puts the reader AND the atlas pane on
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
  await page.goto(`/read/${v.book}/${v.chapter}?split=1`);
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
  await page.goto(`/read/${iv.book}/${iv.chapter}?split=1`);
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
  await expect(page.getByTestId('event-nav')).toHaveCount(0);
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

test('TRUNC-1: the temple-dedication popover\'s 1KI.8 witness shows the +46-more affordance and it opens the full chapter (RED before this batch: nothing signaled); an under-cap witness group shows the ordinary wording (conditional presence)', async ({ page }) => {
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

  // Pass 2: reader on 1 Kings -- 2 Chronicles 5's own witness entry is now
  // the DIFFERENT chapter, so ITS OWN affordance is testable instead (an
  // UNDER-cap group, only 13 of the witness's own verses in that chapter --
  // well under 20 -- shows the ordinary wording, no truncation affordance
  // at all, conditional presence -- its own deterministic span is
  // 2CH.5.2-14, 13 verses from the witness's own start, none capped).
  const chroniclesCh5 = chroniclesWitness.verse_groups.find((g: any) => g.chapter === 5);
  expect(chroniclesCh5.count, '2 Chronicles 5 (within this witness) is under the cap').toBeLessThan(20);
  expect(chroniclesCh5.count).toBe(chroniclesCh5.verses.length);

  await page.goto(`/read/${kingsV.book}/${kingsV.chapter}`);
  await page.getByTestId(`verse-line-${kingsV.verse}`).focus();
  await page.keyboard.press('Enter');
  await page.getByTestId('verse-event-1ki_temple_dedication').click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);
  const chroniclesExpand = page.getByTestId('popover-verse-expand-event-witness-2CH.5.2-14');
  await expect(chroniclesExpand).toHaveAttribute('title', 'Read the whole chapter');

  // 2 Chronicles 6 (fully inside this same witness's own 5:2-7:10 span) is
  // ALSO over the cap (42 true verses) -- the SAME honest signal fires a
  // second time in this one popover, not just for the owner's own named
  // 1 Kings case. O2: no more `data-truncated` marker to count directly --
  // a truncated entry's own title CONTAINS "more — read the chapter"
  // (untruncated ones read exactly "Read the whole chapter", no "more"
  // substring), the same distinguishing signal one layer over. M-D4 fix
  // round 1, P2: `.popover-reveal-link` is the new shared class every
  // more/all/less button in the redesigned RevealControls.razor carries
  // (the old `.popover-reveal-more` arrow-glyph class, and its own
  // always-paired `.popover-reveal-more-all` sibling this comment used to
  // have to deliberately exclude, are both gone) -- no double-counting
  // risk to guard against here anymore, since MiniReaderExpand's own
  // binary Default=0/Total=1/Step=1 case never renders a same-titled
  // "all" sibling at all (RevealControls.razor's own ShowAll rule omits
  // "all" whenever it would coincide with "more" -- see that file's own
  // header comment), so a single class selector is now sufficient.
  const chroniclesCh6 = chroniclesWitness.verse_groups.find((g: any) => g.chapter === 6);
  const chroniclesCh6Missing = chroniclesCh6.count - chroniclesCh6.verses.length;
  expect(chroniclesCh6Missing).toBeGreaterThan(0);
  await expect(page.locator('.popover-reveal-link[title*="more — read the chapter"]')).toHaveCount(2);
});

// ---------------------------------------------------------------------
// PEEK-1 (TRAV-1, controller decision 4, owner verbatim: "also over the
// narrative/event arrows, you'll get a quick hover box of the verses if
// you hover over the arrows (not super sensitive, so some delay so that
// you're not accidentally getting hover boxes all the time)"): dwell-hover
// verse peek on traversal arrows, BOTH the narrative nav and the
// Chronology block (CHRONO-1), via the shared `Components.ArrowNav`.
// RESPEC'D BY PEEK-TRUNC-1 (below, CONTRACT.md's own note has the full
// story): the peek's own CONTENT (one verse + reveal controls, not an
// unbounded PassageList) and DISMISS grammar (a short grace-period
// corridor, not an instant hide) both changed; the two tests immediately
// below still pin the UNCHANGED dwell-IN half of this contract (tickle
// test, DwellTiming.PeekDelayMs, click always commits) and are updated
// only where PEEK-TRUNC-1 genuinely changed what they must assert.
// ---------------------------------------------------------------------

test('PEEK-1: a quick pointer pass over a Chronology arrow produces NO peek; a dwell past the delay reveals the target event\'s own verse text; pointer-leave (eventually) dismisses it; click still commits the traversal', async ({ page }) => {
  const positions = await api.narrativeEventPositions('gen_binding_isaac');
  expect(positions.timeline.following, 'gen_binding_isaac must have a real FOLLOWING target for this test to mean anything').toBeTruthy();
  const followingDetail = await api.event(positions.timeline.following.id);

  await openEventPopover(page, 'gen_binding_isaac');
  const arrow = page.getByTestId('event-chrono-following-event-global');
  await expect(arrow).toBeVisible();
  const peek = page.getByTestId('event-chrono-following-event-global-peek');

  // The tickle test: a quick, un-lingering hover must show NOTHING --
  // asserted immediately, no wait, matching the owner's own "not
  // accidentally getting hover boxes all the time."
  await arrow.hover({ force: true });
  await expect(peek).toHaveCount(0);

  // Move away immediately (before the dwell delay could ever elapse) --
  // the arrow itself must still be perfectly usable (this hover produced
  // no side effect that could linger and interfere with the dwell test
  // below).
  await page.mouse.move(2, 2);
  await expect(peek).toHaveCount(0);

  // A genuine DWELL: hover again and wait comfortably past
  // DwellTiming.PeekDelayMs (375ms) -- the peek must appear, carrying the
  // target event's own real verse text (never the arrow's own name-only
  // label a second time -- P4's own "no verse text in the arrow itself"
  // law stays true; the text lives ONLY in this peek).
  await arrow.hover({ force: true });
  await expect(peek).toBeVisible({ timeout: 2000 });
  // Ground truth for the peek's own content is the WIRE's own
  // `timeline.following.verse_groups` (exactly what ArrowNav resolves via
  // VerseTextResolver.ResolveGroupsAsync) -- never re-derived from
  // `witnesses` (a DIFFERENT, per-witness breakdown that need not start at
  // the identical verse). PEEK-TRUNC-1: the peek shows exactly ONE verse
  // by default -- the FIRST one -- so this is still exactly what must
  // render, unconditionally (PEEK-2, below, covers the multi-verse
  // more/all/less mechanics this single-verse case never exercises).
  const firstVref = positions.timeline.following.verse_groups[0].verses[0];
  const chapterOut = await api.chapter(firstVref.split('.').slice(0, 2).join('.'));
  const firstVerseNum = Number(firstVref.split('.')[2]);
  const firstVerseText = chapterOut.verses.find((v: any) => v.verse === firstVerseNum).text;
  await expect(peek).toContainText(firstVerseText);
  // PEEK-TRUNC-1: the peek header is the target's own FULL title too.
  await expect(page.getByTestId('event-chrono-following-event-global-peek-title')).toHaveText(followingDetail.title);

  // No close button of any kind on the peek (decision 4/5: "NO x needed").
  await expect(peek.getByTestId('popover-close')).toHaveCount(0);

  // PEEK-TRUNC-1: pointer-leave no longer dismisses on the SAME tick --
  // a short grace-period corridor (1000ms) now tolerates a genuine
  // transit into the box (PEEK-4 covers that side explicitly) -- but a
  // real, sustained departure like this one (mouse parked at a point far
  // from both the arrow and the box, never returning) still dismisses the
  // whole peek on its own once that window elapses; toHaveCount(0)'s own
  // auto-retry (Playwright's default 5s) comfortably covers the wait.
  await page.mouse.move(2, 2);
  await expect(peek).toHaveCount(0);

  // Click still commits exactly as before, completely independent of
  // whatever dwell state the arrow was last in.
  await arrow.click();
  await expect(page.getByTestId('popover-title')).toHaveText(followingDetail.title);
});

test('PEEK-1: the SAME dwell-hover peek works identically on a narrative-nav arrow (pw_gethsemane, passion-week) -- one shared component, not a parallel implementation', async ({ page }) => {
  const positions = await api.narrativeEventPositions('pw_gethsemane');
  const narrativePos = positions.narrative.find((p: any) => p.narrative_id === 'passion-week');
  expect(narrativePos?.following, 'pw_gethsemane must have a real FOLLOWING passion-week leg for this test to mean anything').toBeTruthy();

  await openEventPopover(page, 'pw_gethsemane');
  const arrow = page.getByTestId('event-following-event-passion-week');
  await expect(arrow).toBeVisible();
  const peek = page.getByTestId('event-following-event-passion-week-peek');

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

// ---------------------------------------------------------------------
// PEEK-TRUNC-1 (owner defect report, 2026-08-24, verbatim: "menus
// appearing on hover from arrow hover are getting cut off. needs to be
// truncated to an expandable menu limit one verse."): CONTRACT.md's own
// PEEK-TRUNC-1 note has the full, binding contract this batch adds on top
// of PEEK-1 (above) -- content (one verse + house reveal controls),
// dismiss (a grace-period corridor), and placement (viewport-aware
// above/below flip).
// ---------------------------------------------------------------------

test('PEEK-2: dwelling an arrow whose target resolves multiple verses shows exactly one by default plus more(n)/all(N); more/less operate inside the box; a real departure still dismisses the whole peek', async ({ page }) => {
  // rob_dedication_feast (JHN.10.22-39) is a real, many-verse single-group
  // event -- FOLLOWING of rob_crippled_woman_sabbath in jesus-ministry.
  // Total is derived from the LIVE wire (never hardcoded), the SAME
  // `verse_groups` shape PEEK-1's own test above reads its ground truth
  // from.
  const positions = await api.narrativeEventPositions('rob_crippled_woman_sabbath');
  const narrativePos = positions.narrative.find((p: any) => p.narrative_id === 'jesus-ministry');
  expect(narrativePos?.following?.id, 'rob_crippled_woman_sabbath must have a real FOLLOWING jesus-ministry leg for this test to mean anything').toBe('rob_dedication_feast');
  const total: number = narrativePos.following.verse_groups.reduce((n: number, g: any) => n + g.verses.length, 0);
  expect(total, 'this test needs a target with enough verses for more/all/less to mean anything').toBeGreaterThan(3);

  await openEventPopover(page, 'rob_crippled_woman_sabbath');
  const arrow = page.getByTestId('event-following-event-jesus-ministry');
  const peek = page.getByTestId('event-following-event-jesus-ministry-peek');

  await arrow.hover({ force: true });
  await expect(peek).toBeVisible({ timeout: 2000 });

  // Exactly ONE verse by default -- never the old unbounded list.
  await expect(peek.locator('.popover-arrow-peek-verse')).toHaveCount(1);
  const moreLink = page.getByTestId('event-following-event-jesus-ministry-peek-more');
  const allLink = page.getByTestId('event-following-event-jesus-ministry-peek-more-all');
  await expect(moreLink).toHaveText('more (2)');
  await expect(allLink).toHaveText(`all (${total})`);

  // `more` operates INSIDE the box -- the peek must not itself vanish as
  // a side effect of this click (a real, live-caught risk class: an
  // earlier design might treat any click near the peek as "the pointer
  // did something, tear it down").
  await moreLink.click({ force: true });
  await expect(peek.locator('.popover-arrow-peek-verse')).toHaveCount(3); // Default 1 + Step 2, RevealControls' own arithmetic
  await expect(peek, 'clicking more inside the box must not itself dismiss the peek').toBeVisible();

  // `less` is the SAME house one-op-undo mechanic every other
  // RevealControls consumer gets -- steps back toward (never below) the
  // one-verse floor.
  const lessLink = page.getByTestId('event-following-event-jesus-ministry-peek-collapse');
  await lessLink.click({ force: true });
  await expect(peek.locator('.popover-arrow-peek-verse')).toHaveCount(1);

  // A real, sustained departure (away from both the arrow and the box)
  // still dismisses the WHOLE peek -- the grace corridor (PEEK-4) only
  // ever tolerates a transit INTO the box, never a genuine goodbye.
  await page.mouse.move(2, 2);
  await expect(peek).toHaveCount(0);
});

test('PEEK-3: dwelling an arrow near the bottom viewport edge never clips the peek -- its own rendered bounding rect stays fully inside the viewport', async ({ page }) => {
  // A short viewport (real device proportions, just not much vertical
  // room) plus scrolling the popover's own internal overflow-y:auto
  // content so the arrow sits at the very bottom of what's scrolled into
  // view -- deterministically recreates "the arrow is near the bottom
  // screen edge" regardless of exactly how tall this event's own popover
  // content happens to be, or exactly where in section-registry order the
  // Chronology block falls.
  await page.setViewportSize({ width: 1280, height: 480 });

  await openEventPopover(page, 'gen_binding_isaac');
  const arrow = page.getByTestId('event-chrono-following-event-global');
  await arrow.evaluate((el: HTMLElement) => el.scrollIntoView({ block: 'end' }));
  await expect(arrow).toBeVisible();
  const peek = page.getByTestId('event-chrono-following-event-global-peek');

  await arrow.hover({ force: true });
  await expect(peek).toBeVisible({ timeout: 2000 });

  // The very first render of a fresh peek lands at its un-measured
  // default (below, unbounded -- app.css's own `var(--peek-max-height,
  // none)` fallback, ArrowNav.razor's own header comment) for one
  // instant, THEN OnAfterRenderAsync's own reader.js measurement lands
  // and (if needed) flips/caps it -- poll rather than assert once,
  // matching the SAME "measurement is a real async round trip" allowance
  // ExplorerPopover's own verse-anchoring tests already need.
  await expect.poll(async () => {
    const box = await peek.boundingBox();
    return box === null ? null : box.y >= 0 && box.y + box.height <= 480;
  }, { timeout: 2000 }).toBe(true);

  const box = await peek.boundingBox();
  expect(box, 'the peek must have a real, measurable box').not.toBeNull();
  expect(box!.y, 'the peek\'s own top edge must never be clipped above the viewport').toBeGreaterThanOrEqual(0);
  expect(box!.y + box!.height, 'the peek\'s own bottom edge must never be clipped below the viewport').toBeLessThanOrEqual(480);
});

test('PEEK-4: moving the pointer from the arrow, across the gap, into the box does not dismiss the peek -- the reveal control inside remains genuinely clickable', async ({ page }) => {
  await openEventPopover(page, 'rob_crippled_woman_sabbath');
  const arrow = page.getByTestId('event-following-event-jesus-ministry');
  const peek = page.getByTestId('event-following-event-jesus-ministry-peek');
  const moreBtn = page.getByTestId('event-following-event-jesus-ministry-peek-more');

  const arrowBox = await arrow.boundingBox();
  expect(arrowBox, 'the arrow must have a real box for this gesture to mean anything').not.toBeNull();
  await page.mouse.move(arrowBox!.x + arrowBox!.width / 2, arrowBox!.y + arrowBox!.height / 2);
  await expect(peek).toBeVisible({ timeout: 2000 });
  await expect(moreBtn, 'rob_dedication_feast resolves well over one verse -- more must be offered').toBeVisible();

  const moreBox = await moreBtn.boundingBox();
  expect(moreBox).not.toBeNull();

  // A real, MULTI-STEP transit -- never a teleport -- from the arrow's
  // own center, across the small visual gap between the wrapper and the
  // peek (app.css's own .4rem margin), to the "more" control itself. If
  // PEEK-TRUNC-1's own grace-period corridor were missing or broken, the
  // transient pointerleave the wrapper fires mid-gap (ArrowNav.razor's
  // own header comment has the full "why a transient leave/enter pair
  // fires here" story) would hide the whole peek partway through this
  // move, before it ever reaches the button.
  await page.mouse.move(moreBox!.x + moreBox!.width / 2, moreBox!.y + moreBox!.height / 2, { steps: 12 });
  await expect(peek, 'the peek must survive the transit through the gap').toBeVisible();

  // Genuinely clickable, not just "still technically in the DOM" --
  // proves this is a real, interactive arrival.
  await page.mouse.down();
  await page.mouse.up();
  await expect(peek.locator('.popover-arrow-peek-verse')).toHaveCount(3); // Default 1 + Step 2
  await expect(peek, 'clicking more inside the box must not itself dismiss the peek').toBeVisible();
});

// ---------------------------------------------------------------------
// TITLE-WRAP-1 (owner report, 2026-08-24, verbatim: "i don't like that
// arrow titles are getting cut off with elipses... we need to find a way
// to have a nice presentation while showing a relatively full title.").
// CONTRACT.md's own TITLE-WRAP-1 note has the full, binding contract.
// ---------------------------------------------------------------------

test('TITLE-2: a long event name renders via the two-line clamp (never a single-line ellipsis) and the fixed grid holds; the peek header always carries the full name', async ({ page }) => {
  // rob_elijah_puzzle's own PRIOR (rob_transfiguration, "The
  // Transfiguration", 19 chars) and FOLLOWING (rob_demoniac_boy, "Jesus
  // heals a demoniac boy the disciples could not heal", 57 chars) are
  // real neighbors on the SAME jesus-ministry narrative leg, one clearly
  // short and one clearly long -- opening ITS popover renders both arrows
  // side by side in one row, so the grid-alignment comparison below is
  // between two REAL rows on the SAME live page, not a synthetic fixture.
  const positions = await api.narrativeEventPositions('rob_elijah_puzzle');
  const narrativePos = positions.narrative.find((p: any) => p.narrative_id === 'jesus-ministry');
  expect(narrativePos?.prior?.id).toBe('rob_transfiguration');
  expect(narrativePos?.following?.id).toBe('rob_demoniac_boy');
  const shortLabel: string = narrativePos.prior.label;
  const longLabel: string = narrativePos.following.label;
  expect(shortLabel.length, 'this test needs a genuinely short neighbor label').toBeLessThanOrEqual(24);
  expect(longLabel.length, 'this test needs a genuinely long neighbor label').toBeGreaterThan(50);

  await openEventPopover(page, 'rob_elijah_puzzle');
  const shortNameLabel = page.getByTestId('event-prior-event-jesus-ministry').locator('.popover-event-nav-label');
  const longNameLabel = page.getByTestId('event-following-event-jesus-ministry').locator('.popover-event-nav-label');
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
  const shortRole = page.getByTestId('event-prior-label-jesus-ministry');
  const longRole = page.getByTestId('event-following-label-jesus-ministry');
  const shortRoleBox = await shortRole.boundingBox();
  const longRoleBox = await longRole.boundingBox();
  expect(shortRoleBox).not.toBeNull();
  expect(longRoleBox).not.toBeNull();
  expect(Math.abs(shortRoleBox!.y - longRoleBox!.y), 'the PRIOR/FOLLOWING role captions must not shift out of alignment').toBeLessThanOrEqual(3);

  // The peek header always carries the FULL name -- "the complete name is
  // one dwell away, always" -- dwelling the LONG-named arrow specifically,
  // since that is the case a bare button label could never fully show.
  const longArrow = page.getByTestId('event-following-event-jesus-ministry');
  await longArrow.hover({ force: true });
  const peekTitle = page.getByTestId('event-following-event-jesus-ministry-peek-title');
  await expect(peekTitle).toBeVisible({ timeout: 2000 });
  await expect(peekTitle).toHaveText(longLabel);
});
