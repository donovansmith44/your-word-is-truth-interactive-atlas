import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch M-D4 ("the recursive reader") -- the owner's own defect report,
// near-verbatim (2026-08-23): "the hovering over place names is not
// universal but only seems to work if im just looking at read the whole
// chapter mini scrollable menus. additionally, the whole thing isnt
// recursively defined. the Bible I get when I click read the whole chapter
// is of a fundamentally different kind - location hovering works. verses
// are not clickable (therefore explorations hit deadends. no-go)."
//
// Controller decision 4, the LAW this file exists to prove: "New Playwright
// coverage asserts the affordance SET of a verse line is identical across
// surfaces: for a sampled verse rendered (a) in the main reader and (b)
// inside the popover mini-reader -- same clickability (focus push), same
// name links present, same superscript behavior (per current flag state).
// Assert by behavior, not by implementation details."
//
// Every scenario below anchors on REAL, already-verified data -- the SAME
// "read real data, don't hardcode an invented example" discipline every
// other reader-*.spec.ts file in this suite already follows:
//   - GEN.1.3 -> 2CO.4.6 (votes=81) is the SAME cross-reference
//     popover-sections.spec.ts's own READER-1 test already relies on to
//     reach a DIFFERENT-chapter mini-reader; 2CO.4.4 (inside that target
//     chapter) is real, curated data confirmed (via a direct /api/chapter
//     query while drafting this file) to carry BOTH a literal, case-correct
//     "God" person mention (persons: [{id: "god_1324", name: "God"}], text
//     "...who is the image of God, should shine...") and xref_count=35
//     (>0, so a superscript renders), giving one real verse that exercises
//     every affordance the parity law names at once. M-D4 fix round 1, P1
//     (owner correction): the marker's own glyph is now an ORDINAL letter
//     among 2CO.4's own xref-bearing verses, not a function of THIS
//     verse's own count -- RECURSE-1 below asserts the main reader and the
//     mini-reader render the SAME letter for verse 4 (reading it back off
//     the main reader rather than hardcoding a predicted ordinal), which is
//     both simpler and a more direct proof of decision 4's own parity law
//     than independently recomputing the expected letter here would be.
//   - JHN.1.1 -> GEN.1.1 (votes=337, the TOP-voted entry for JHN.1.1 --
//     "In the beginning was the Word"/"In the beginning God created") lands
//     in GEN.1, whose own verse 1 anchors a real pericope heading
//     ("theo-1", "Creation of all things" -- reader-headings.spec.ts's own
//     GEN.6 test already establishes this app's own heading-anchor
//     discipline; GEN.1.1 is this suite's simplest real example of one).

test.describe('M-D4: the recursive reader', () => {
  // -----------------------------------------------------------------------
  // RECURSE-1: the affordance SET (clickable line that PUSHES rather than
  // dead-ending, in-text mention link, xref superscript) is identical for
  // the SAME real verse whether it renders in the main reader or inside the
  // popover's own mini-reader.
  // -----------------------------------------------------------------------
  test('RECURSE-1: a verse carries the identical affordance set in the main reader and inside the popover mini-reader', async ({ page }) => {
    // (a) The main reader, direct: /read/2CO/4, verse 4.
    await page.goto('/read/2CO/4');
    const mainLine = page.getByTestId('verse-line-4');
    await expect(mainLine).toBeVisible();

    const mainMention = page.getByTestId('verse-mention-person-4-god_1324');
    await expect(mainMention, 'main reader: the literal "God" mention must render as a clickable span').toBeVisible();
    await expect(mainMention).toHaveText('God');

    // P1: the glyph is a per-chapter ORDINAL letter now (not a function of
    // this verse's own count) -- read the main reader's own rendered value
    // rather than predicting it, then assert the mini-reader matches it
    // exactly below (the real parity claim, decision 4).
    const mainXref = page.getByTestId('verse-xref-marker-4');
    await expect(mainXref, 'main reader: xref_count=35 (>0) must render a single ordinal letter').toHaveText(/^[a-z]$/);
    const mainGlyph = await mainXref.textContent();

    // Clickability -- opens THIS verse's own popover (keyboard activation,
    // MENTION-1's own documented hazard: a coordinate click on the line can
    // land on its own nested mention span instead).
    await mainLine.focus();
    await page.keyboard.press('Enter');
    await expect(page.getByTestId('popover-title')).toHaveText('2CO.4.4');
    await page.getByTestId('popover-close').click();

    // (b) The SAME verse, reached recursively: a DIFFERENT chapter's own
    // verse-line -> a real cross-reference -> the resulting popover's own
    // mini-reader, expanded. GEN.1.3 -> 2CO.4.6 is the same real, stable
    // pair popover-sections.spec.ts's own READER-1 test already exercises.
    await page.goto('/read/GEN/1');
    await page.getByTestId('verse-line-3').focus();
    await page.keyboard.press('Enter');
    await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.3');

    const xrefEntry = page.getByTestId('xref-item-2CO.4.6');
    await expect(xrefEntry).toBeVisible();
    await xrefEntry.click();
    await expect(page.getByTestId('popover-title')).toHaveText('2CO.4.6');

    // Before this batch, MiniReaderExpand.razor rendered verse 4's own row
    // with NO clickability, NO xref marker at all -- the owner's own
    // "verses are not clickable... no-go."
    await page.getByTestId('popover-verse-expand').click();
    await expect(page.getByTestId('popover-verse-reader')).toBeVisible();

    const miniRow = page.getByTestId('popover-reader-verse-4');
    await expect(miniRow).toBeVisible();

    const miniMention = page.getByTestId('popover-reader-mention-person-4-god_1324');
    await expect(miniMention, 'mini-reader: the SAME "God" mention must render as a clickable span').toBeVisible();
    await expect(miniMention).toHaveText('God');

    // P1/decision 4: chapter-scoped, not container-scoped -- the SAME
    // letter the main reader rendered above for this exact verse, proven
    // by direct comparison rather than each surface merely matching its
    // OWN independently-plausible-looking glyph.
    const miniXref = page.getByTestId('popover-reader-xref-marker-4');
    await expect(miniXref, 'mini-reader: the SAME ordinal letter as the main reader').toHaveText(mainGlyph!);

    // Clickability parity -- the row itself pushes a fresh VerseNode onto
    // the SAME popover's own stack (decision 2: "reuse the popover's
    // EXISTING push stack"), never a dead end and never a second,
    // independent popover mount.
    await miniRow.focus();
    await page.keyboard.press('Enter');
    await expect(page.getByTestId('popover-title')).toHaveText('2CO.4.4');
    await expect(page.getByTestId('popover-breadcrumb-back'), 'a PUSH, not a replace -- the stack now runs GEN.1.3 -> 2CO.4.6 -> 2CO.4.4').toBeVisible();
  });

  // -----------------------------------------------------------------------
  // RECURSE-2: the mini-reader's own xref superscript click COMMITS the
  // SAME xrefs-leading entry-point view the main reader's own click does --
  // not merely present-but-inert.
  // -----------------------------------------------------------------------
  test('RECURSE-2: the mini-reader\'s own xref superscript pushes the SAME xrefs-leading entry point as the main reader\'s', async ({ page }) => {
    await page.goto('/read/GEN/1');
    await page.getByTestId('verse-line-3').focus();
    await page.keyboard.press('Enter');
    await page.getByTestId('xref-item-2CO.4.6').click();
    await page.getByTestId('popover-verse-expand').click();
    await expect(page.getByTestId('popover-verse-reader')).toBeVisible();

    const marker = page.getByTestId('popover-reader-xref-marker-4');
    await marker.focus();
    await page.keyboard.press('Enter');
    await expect(page.getByTestId('popover-title')).toHaveText('2CO.4.4');

    // VerseNode.XrefEntryPoint=true -> CrossRefsSection leads (registry
    // order otherwise puts xrefs LAST, per M-D3/U6's own owner-specified
    // order) -- the SAME entry-point behavior Reader.razor's own
    // OpenVerseXrefEntryPersistent gives, reached this time via
    // MiniReaderExpand's own OnXrefClick wiring instead.
    const sections = page.locator('[data-testid^="popover-section-"]');
    await expect(sections.first()).toHaveAttribute('data-testid', 'popover-section-xrefs');
  });

  // -----------------------------------------------------------------------
  // RECURSE-3: pericope headings interleave inside the mini-reader too --
  // absent there entirely before this batch (decision 1, "heading
  // interleaving where applicable").
  // -----------------------------------------------------------------------
  test('RECURSE-3: a pericope heading interleaves inside the popover mini-reader, same as the main reader', async ({ page }) => {
    const gen1 = await api.chapter('GEN.1');
    const v1Heading = gen1.verses.find((v: any) => v.verse === 1).heading;
    expect(v1Heading, 'GEN.1.1 must anchor a real heading for this test to mean anything').toBeTruthy();

    // The main reader's own rendering -- unchanged since batch-t, the
    // reference behavior this test compares the mini-reader against.
    await page.goto('/read/GEN/1');
    await expect(page.getByTestId(`pericope-heading-${v1Heading.event_id}`)).toHaveText(v1Heading.title);

    // Reach GEN.1's own mini-reader from a DIFFERENT starting chapter, via
    // a real, top-voted cross-reference (JHN.1.1 -> GEN.1.1).
    await page.goto('/read/JHN/1');
    await page.getByTestId('verse-line-1').focus();
    await page.keyboard.press('Enter');
    await expect(page.getByTestId('popover-title')).toHaveText('JHN.1.1');

    const xrefEntry = page.getByTestId('xref-item-GEN.1.1');
    await expect(xrefEntry).toBeVisible();
    await xrefEntry.click();
    await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.1');

    await page.getByTestId('popover-verse-expand').click();
    await expect(page.getByTestId('popover-verse-reader')).toBeVisible();

    const miniHeading = page.getByTestId(`pericope-heading-${v1Heading.event_id}`);
    await expect(miniHeading, 'the mini-reader never rendered ANY heading before this batch').toBeVisible();
    await expect(miniHeading).toHaveText(v1Heading.title);

    // Renders immediately above verse 1's own row, same as the main reader.
    const headingBox = await miniHeading.boundingBox();
    const v1Box = await page.getByTestId('popover-reader-verse-1').boundingBox();
    expect(headingBox && v1Box && headingBox.y).toBeLessThan(v1Box!.y);

    // Explorable, same ONE-RULE treatment -- pushes the SAME EventNode the
    // main reader's own heading click opens.
    await miniHeading.focus();
    await page.keyboard.press('Enter');
    await expect(page.getByTestId('popover-title')).toHaveText(v1Heading.title);
    await expect(page.getByTestId('popover-breadcrumb-back')).toBeVisible();
  });

  // -----------------------------------------------------------------------
  // RECURSE-4 (decision 3, "name links everywhere... render wherever verse
  // text renders"): the verse/passage popover's own COMPACT focus preview
  // (VerseTextSection.razor, "one-verse foci") -- previously bare, unscanned
  // text with no mention links AT ALL -- carries the SAME in-text mention
  // link too, not just the two chapter-body reading flows RECURSE-1 covers.
  // -----------------------------------------------------------------------
  test('RECURSE-4: the verse popover\'s own compact focus text carries an in-text mention link too', async ({ page }) => {
    await page.goto('/read/2CO/4');
    await page.getByTestId('verse-line-4').focus();
    await page.keyboard.press('Enter');
    await expect(page.getByTestId('popover-title')).toHaveText('2CO.4.4');

    // Compact, never expanded, on purpose -- this chapter IS the current
    // reader chapter, so popover-verse-expand is unconditionally absent
    // here (READER-1's own established chapter-aware-suppression rule);
    // this test is specifically about the COMPACT view's own text.
    await expect(page.getByTestId('popover-verse-expand')).toHaveCount(0);
    await expect(page.getByTestId('popover-verse-reader')).toHaveCount(0);

    const mention = page.getByTestId('popover-verse-mention-person-4-god_1324');
    await expect(mention).toBeVisible();
    await expect(mention).toHaveText('God');

    await mention.click();
    await expect(page.getByTestId('popover-title')).toHaveText('God');
    await expect(page.getByTestId('popover-breadcrumb-back'), 'a PUSH onto the SAME stack, not a dead end').toBeVisible();
  });
});
