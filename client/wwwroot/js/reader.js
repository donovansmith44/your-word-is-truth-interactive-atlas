// Scrolls a verse line into view for the reader's #v{n} anchor contract
// (CONTRACT.md: "/read/{BOOK}/{chapter}#v{n} -- verse anchor"). Kept as its
// own tiny module -- mirroring map.js's import-once pattern
// (client/MapInterop.cs) -- rather than relying on Blazor's own
// fragment-navigation heuristics, so the exact scroll target and timing are
// deterministic and independent of framework version behavior.
export function scrollToVerse(n) {
    const el = document.getElementById('v' + n);
    if (el) {
        el.scrollIntoView({ block: 'center' });
    }
}

// Batch R requirement 4 (expandable popover + in-context chapter reading):
// scrolls the mini-reader's own focal verse row into view once its chapter
// has actually rendered -- called by VerseTextSection.razor (client/
// Components/) on expand, by a random per-instance DOM id (never the reader
// page's own bare "v{n}", which this popover's mini-reader could easily
// collide with -- see that component's own comment). `block: 'nearest'`
// (not 'center', unlike scrollToVerse above) -- the mini-reader is a small,
// already-bounded overflow:auto region (app.css's own .popover-reader), not
// the whole viewport; 'nearest' scrolls the LEAST amount needed to bring the
// focal row fully into that region, which for a verse already near the top
// of a freshly-expanded, freshly-fetched chapter is often already true (a
// true no-op scroll) rather than always re-centering it. No smooth-scroll
// animation (an implicit, one-time instant jump, same "no unnecessary
// motion" restraint every OTHER non-orchestrated-moment interaction in this
// app already follows -- design-direction.md's own Motion section) --
// nothing here needs its own prefers-reduced-motion guard as a result.
export function scrollFocalRowIntoView(domId) {
    const el = document.getElementById(domId);
    if (el) {
        el.scrollIntoView({ block: 'nearest' });
    }
}

// Fix round 1 (Task 15 finding): Reader.razor tracks whether Shift is
// currently held (_shiftHeld) via plain @onkeydown/@onkeyup bindings, so
// that ExplorerPopover's own click-outside-to-close backdrop can go
// pointer-events:none for exactly as long as Shift is down (letting a
// shift-click's second click reach the verse-num button underneath it --
// see Reader.razor's own comments for the full story). Blazor has no
// binding for either window.blur or document.visibilitychange, though, and
// neither keydown NOR keyup ever reaches this page at all if Shift is
// released while this tab/window isn't the focused one (alt-tab to a
// different application, or switch to a different browser tab) -- without
// this, _shiftHeld would stay stuck true and the backdrop would stay
// permanently non-interactive (silently breaking click-outside-to-close)
// until some LATER, unrelated Shift press+release cycle happened to clear
// it. Both listeners call back into the SAME dotnetRef method
// (ResetShiftHeld) -- resetting on either signal is always safe, per that
// method's own comment.
//
// Module-scoped (not a class/closure returned to the caller) because
// exactly one Reader.razor instance is ever mounted at a time in this
// single-page app; watchShiftRelease replaces any prior listener pair
// first so calling it twice (e.g. a future hot-reload) can't double-wire.
let _shiftReleaseCleanup = null;

export function watchShiftRelease(dotnetRef) {
    if (_shiftReleaseCleanup) {
        _shiftReleaseCleanup();
    }

    const reset = () => dotnetRef.invokeMethodAsync('ResetShiftHeld');
    window.addEventListener('blur', reset);
    document.addEventListener('visibilitychange', reset);

    _shiftReleaseCleanup = () => {
        window.removeEventListener('blur', reset);
        document.removeEventListener('visibilitychange', reset);
        _shiftReleaseCleanup = null;
    };
}

export function unwatchShiftRelease() {
    if (_shiftReleaseCleanup) {
        _shiftReleaseCleanup();
    }
}

// Batch CORPREAD-1a (SPLIT-SCROLL-1): app.css's own .split-view (that
// rule's own header comment has the full pinned-pane design and its live
// root-cause diagnosis) makes .split-pane-reader/.split-pane-host a REAL
// overflow-y:auto scroll container of their own whenever Reader is genuinely
// hosting a split -- the "whole document scrolls either way" assumption
// setScrollY/watchScroll/watchChapterNavCenter used to document and rely on
// (Batch H) no longer holds there. This is the ONE place "which element is
// the reader's real scroll target right now" gets decided, from a REAL,
// CURRENT layout fact (a computed style), never a guessed/hand-copied class
// name -- so it stays correct even if a future batch renames or restructures
// the split-pane classes, and degrades safely (falls back to window) if
// nothing along the chain is actually a scroll container, which is exactly
// what "standalone reader, whole document scrolls" IS. Starts from
// `[data-testid="reader-root"]` itself (not one of its ancestors) so it
// covers BOTH roles this element can play: HOST (the element itself is the
// overflow:auto container -- .split-pane-reader/.split-pane-host, app.css)
// and GUEST (a WRAPPER one level up -- .split-pane-guest -- is the real
// container instead, e.g. Reader mounted under Sources/Kretzmann/Concord;
// see .split-pane-guest's own app.css comment). Stops at document.body --
// nothing above that is ever a meaningful scroll boundary for this app.
function findReaderScrollContainer() {
    let node = document.querySelector('[data-testid="reader-root"]');
    while (node && node !== document.body) {
        const style = getComputedStyle(node);
        if (style.overflowY === 'auto' || style.overflowY === 'scroll') {
            return node;
        }
        node = node.parentElement;
    }
    return null; // no real overflow container found -- window/document is genuinely the scroller (standalone)
}

// Batch H (view-state round-trip). setScrollY is the restore half.
// Batch CORPREAD-1a: rebound (see findReaderScrollContainer's own header
// comment) -- a real internal scroll container gets its OWN scrollTop set;
// its absence (standalone) falls through to the original plain
// window-level scroll, UNCHANGED for that case.
export function setScrollY(y) {
    const container = findReaderScrollContainer();
    if (container) {
        container.scrollTop = y;
    } else {
        window.scrollTo(0, y);
    }
}

// watchScroll/unwatchScroll -- the CAPTURE half, and NOT a plain "read
// window.scrollY once in DisposeAsync" the way it might look like it should
// be: confirmed live (a real failing round-trip test, not a guess) that
// Blazor's own router resets the window's scroll position to (0,0) as part
// of committing a navigation to a new page -- BEFORE the outgoing
// component's own DisposeAsync gets a chance to run, so a dispose-time
// `getScrollY()` read reliably captures 0, not wherever the page actually
// was. Continuously reporting the scroll position INTO ViewStateService
// instead (throttled to one call per animation frame, same "cheap, no
// missed final position" trade-off a scroll listener normally makes)
// sidesteps the ordering question entirely: by the time ANYTHING reads
// ViewStateService.Reader.ScrollY -- regardless of exactly when Blazor's
// own reset fires relative to disposal -- the last real scroll position is
// already sitting there, written well before navigation ever started.
// Same module-scoped-single-cleanup shape as watchShiftRelease above (this
// app never mounts more than one Reader.razor instance at a time).
//
// Batch CORPREAD-1a (SPLIT-SCROLL-1): rebound to
// findReaderScrollContainer()'s own result -- REPORTS that container's own
// scrollTop, not window.scrollY, whenever one is found. This FUNCTION is
// idempotent/self-cleaning (re-resolves the target fresh on every call,
// exactly like watchChapterNavCenter below does for its own DOM query) --
// but being idempotent only means a REPEAT call is safe and correct; it
// does not, by itself, cause one. FIX ROUND 1 (review S-1, CRITICAL): an
// earlier draft of this comment claimed the re-resolution alone kept this
// "rebinding correctly rather than staying latched onto a target that may
// no longer be the real scroller" -- true of this function, false of the
// SYSTEM: Reader.razor called this ONLY from `OnAfterRenderAsync`'s
// `firstRender` branch, so nothing ever actually issued that later call --
// a split opened via a hatch click, or a fresh `?split=world` load hitting
// the SAME SupplyParameterFromQuery timing quirk watchChapterNavCenter's
// own comment describes, left this permanently bound to `window` even
// once `.reader-page` became a real scroll container, silently breaking
// the VIEWSTATE-1 round-trip in split (window scroll is capped near the
// header's own height there post-SPLIT-SCROLL-1, so the continuously-
// reported position collapsed to ≈0). Reader.razor now calls this from the
// SAME every-render block that already calls watchChapterNavCenter, for
// the identical self-healing reason -- see that call site's own comment
// for the full fix. Standalone (no container found) is BYTE-IDENTICAL to
// the pre-CORPREAD-1a behavior -- window.scrollY, unchanged.
let _scrollCleanup = null;

export function watchScroll(dotnetRef) {
    if (_scrollCleanup) {
        _scrollCleanup();
    }

    const container = findReaderScrollContainer();
    const target = container || window;

    let ticking = false;
    const onScroll = () => {
        if (ticking) {
            return;
        }
        ticking = true;
        requestAnimationFrame(() => {
            ticking = false;
            const y = container ? container.scrollTop : window.scrollY;
            dotnetRef.invokeMethodAsync('OnScroll', y);
        });
    };
    target.addEventListener('scroll', onScroll, { passive: true });

    _scrollCleanup = () => {
        target.removeEventListener('scroll', onScroll);
        _scrollCleanup = null;
    };
}

export function unwatchScroll() {
    if (_scrollCleanup) {
        _scrollCleanup();
    }
}

// Batch R requirement 6 ("chapter nav always visible, middle-aligned...
// even as i scroll"): .reader-prev/.reader-next are `position:fixed` but
// live INSIDE `.reader-page`, which Batch H's own `contain: layout`
// (load-bearing for split view's own z-index containment -- see app.css's
// own comment on that rule) deliberately makes their CSS containing block
// instead of the viewport -- correct and necessary for LEFT/RIGHT (confines
// them to the reader PANE's own width in split view, never the full
// window) but means any `top` value (a plain percentage OR a `vh` unit --
// confirmed live, both) is measured from `.reader-page`'s OWN top edge,
// which scrolls WITH the document, not from the viewport: a static value
// centers correctly only at scroll position 0 and drifts (or vanishes
// entirely) on a long chapter once actually scrolled -- the exact
// regression this function exists to prevent.
//
// Fix: a CSS custom property, `--chapter-nav-top`, set on `.reader-page`
// itself and read by app.css's own `.reader-prev`/`.reader-next` rules,
// recomputed from `.reader-page`'s own REAL, CURRENT
// `getBoundingClientRect().top` (which already reflects wherever the page
// currently sits relative to the viewport, whatever that offset is) so the
// two buttons land at the viewport's own actual vertical center regardless
// of scroll position, while their own left/right values keep resolving
// against `.reader-page` exactly as before (untouched, still correctly
// pane-confined in split view). app.css's own fallback (`var(--chapter-nav-
// top, 50vh)`) covers the brief instant before this listener's first
// `recompute()` call, or if JS somehow never wires up at all -- centered
// at scroll position 0, same as every other CSS-only value would be, never
// unstyled/undefined.
//
// HOTFIX-3 (user report 2026-08-21, near-verbatim: "the next/previous
// chapter buttons shouldn't be redrawn on every scroll. they should be
// fixed in place like on bible.com"): recompute() used to be
// rAF-THROTTLED off the scroll/resize listener (`ticking` flag +
// `requestAnimationFrame(() => { ticking = false; recompute(); })`, the
// same shape watchScroll above still correctly uses) -- ROOT-CAUSED, not
// assumed, with a rAF-timestamped getBoundingClientRect trace during a
// scripted scroll (both a smooth multi-frame scroll and discrete
// mouse.wheel bursts, standalone AND split): every real scroll input
// (`mouse.wheel`, the closest proxy to an actual user gesture Playwright
// has) produced a ONE-FRAME position "teleport" of reader-next sized to
// EXACTLY that gesture's own scroll delta (measured: a 300px wheel notch
// -> the button jumped a clean 300px, then snapped back the very next
// frame), reproducing on every single discrete scroll input, 100% hit
// rate, IDENTICAL in both panes. Mechanism: the BROWSER paints the page's
// own scrolled content the SAME frame the `scroll` event fires, but the
// old code deferred its OWN compensating write one MORE
// `requestAnimationFrame` tick beyond that event -- so for exactly one
// rendered frame, the content had already moved but `--chapter-nav-top`
// still held the pre-scroll value, and (because `.reader-page`'s own
// `contain: layout`, Batch H, is these buttons' containing block, not the
// viewport) a stale compensation makes them briefly render as if pinned to
// a fixed point ON THE PAGE rather than the viewport -- exactly the "isn't
// really fixed" symptom reported. A MutationObserver on the whole nav
// subtree recorded ZERO mutations throughout every trace, and a stamped
// node-identity check confirmed reader-next/reader-prev are NEVER
// recreated -- both suspects the batch brief raised going in
// (view-state-sync-triggered Blazor re-render; a second, undocumented
// split-only containing-block ancestor) are RULED OUT by this evidence,
// not merely unconfirmed: OnScroll (Reader.razor) never calls
// StateHasChanged (unchanged by this fix, still true), and the one
// containing-block ancestor that exists (`.reader-page`'s `contain:
// layout`) is the SAME element in both panes, producing the identical
// glitch in both -- there is no second, split-specific offender.
//
// Fix: recompute() now runs SYNCHRONOUSLY, directly as the scroll/resize
// listener itself -- no `requestAnimationFrame` deferral, no `ticking`
// gate. This removes the exact one-extra-frame gap the trace isolated,
// landing the write inside the SAME task the browser is already using to
// apply the scroll before that frame paints (confirmed: the identical
// trace technique against this fix shows zero position deviation across
// every sampled frame, both panes, including on MAT.26 -- heading-dense,
// this app's own jank test bed). Trade-off, disclosed: this can now run
// more than once within a single frame if the browser fires multiple
// `scroll` events before that frame's paint (unlike the throttled
// version, which guaranteed at most once) -- accepted because the work
// itself is trivial (one `getBoundingClientRect` + one custom-property
// write, for exactly one element), nothing like the render-tree cost a
// throttle exists to protect against elsewhere in this app; watchScroll
// above is UNCHANGED and keeps its own rAF throttle, since invokeMethodAsync
// crossing the JS/.NET boundary is real, non-trivial work worth batching.
//
// Batch CORPREAD-1a (SPLIT-SCROLL-1): rebound to
// findReaderScrollContainer()'s own result, same as watchScroll above.
//
// THE FORMULA ITSELF ALSO CHANGES, in one specific, real case -- caught
// live (a real Playwright NAV-3 failure, not a guess) after an EARLIER
// draft of this fix shipped with the formula genuinely unchanged and the
// reasoning "page.rect.top is invariant under its own internal scroll."
// That reasoning is correct for `page.getBoundingClientRect()` itself, but
// WRONG about what it implies for `reader-prev`/`reader-next`: `.reader-page`'s
// own pre-existing `contain: layout` (app.css) makes IT the CSS containing
// block for its `position: fixed` descendants -- and a containing block's
// `top` offset for such a descendant resolves against that block's OWN
// PADDING-BOX COORDINATE SPACE, which -- once `.reader-page` is genuinely
// its OWN scroll container (HOST role in split, this batch's own new
// case) -- is the SCROLLED CONTENT space, not the visible slice of it.
// Confirmed live: `--chapter-nav-top` correctly held steady across an
// internal `.reader-page` scroll (exactly as the invariant-rect reasoning
// predicted), while the BUTTON'S OWN real screen position drifted by the
// exact scrolled amount -- `contain: layout` does not exempt a fixed
// descendant from its own containing block's internal scroll (this is the
// SAME relationship an ordinary `position: absolute` child of a
// `position: relative; overflow: auto` ancestor already has -- `contain:
// layout` only supplies the CONTAINING BLOCK, it does not additionally
// make the descendant immune to that block's own scrolling, which is a
// materially different (and, before this fix, silently unverified)
// property).
//
// THE FIX: when `page` IS its own found container (the HOST-in-split
// case, the ONLY case where this distinction is live), add `page.scrollTop`
// back into the computed offset -- compensating exactly for the amount
// the content coordinate space has shifted, so the VISUAL (viewport)
// position stays put regardless of how far the pane has scrolled
// internally. The other two cases are genuinely unaffected and need no
// compensation: STANDALONE (container is null -- `page.rect.top` moves
// with the whole-document scroll exactly as it always has, formula
// unchanged) and GUEST-mounted (the found container is an ANCESTOR of
// `page`, not `page` itself -- e.g. `.split-pane-guest` when Reader hosts
// under Sources/Kretzmann/Concord -- `.reader-page` itself never scrolls
// its OWN content there, so `page.rect.top` already moves correctly as
// THAT ancestor scrolls, the same way it always has for any scrolling
// ancestor).
//
// Re-resolved on EVERY call (not cached across calls), matching this
// function's own pre-existing idempotent/self-cleaning discipline
// (_navCenterCleanup at the top of every invocation) -- a later call
// after this same Reader.razor instance's own split state changed (open
// -> closed, or vice versa) rebinds to whichever target/formula is
// ACTUALLY correct now, never a stale one from before the transition.
// Disclosed, narrow trade-off: while genuinely split-hosting, this binds
// to the reader's own internal container ONLY, not window -- .split-view's
// own header comment (app.css) explains why WINDOW no longer scrolls at
// all in that mode (header + split-view together are exactly one viewport
// tall by construction), so there is no missed window-scroll case left to
// widen for -- unlike an earlier draft of this same reasoning, before that
// layout fix landed.
let _navCenterCleanup = null;

export function watchChapterNavCenter() {
    if (_navCenterCleanup) {
        _navCenterCleanup();
    }

    const page = document.querySelector('[data-testid="reader-root"]');
    if (!page) {
        return;
    }

    const container = findReaderScrollContainer();
    const target = container || window;
    const selfScrolls = container === page;

    const recompute = () => {
        const selfScrollTop = selfScrolls ? page.scrollTop : 0;
        const top = window.innerHeight / 2 - page.getBoundingClientRect().top + selfScrollTop;
        page.style.setProperty('--chapter-nav-top', `${top}px`);
    };

    recompute();
    target.addEventListener('scroll', recompute, { passive: true });
    window.addEventListener('resize', recompute);

    _navCenterCleanup = () => {
        target.removeEventListener('scroll', recompute);
        window.removeEventListener('resize', recompute);
        _navCenterCleanup = null;
    };
}

export function unwatchChapterNavCenter() {
    if (_navCenterCleanup) {
        _navCenterCleanup();
    }
}

// Batch F2 requirement 6d ("if i am exploring anything on either side of
// the split screen, the hover windows ought not be smack dab in the center
// of the screen, but on the side of the screen where the hover exploration
// originated"): the CURRENTLY VISIBLE portion of `selector`'s own element,
// clamped to the viewport on every side -- called ONCE, at popover-open
// time (ExplorerPopover.razor's own OnAfterRenderAsync, mirroring
// CardPlacement's proven "measure once, on open" snapshot discipline, not a
// continuous tracker), so the popover PANEL can center itself within
// whichever PANE it opened from rather than the full viewport. O6
// (2026-08-23): the BACKDROP no longer consumes this at all -- its own
// one-shot-then-stale snapshot, left uncorrected across subsequent
// scrolling, was the real cause of the owner's own reported bug; see
// ExplorerPopover.razor's own header comment and app.css's own
// .popover-backdrop comment for the fuller story. This function's own
// PANEL-only contract is otherwise byte-for-byte unchanged.
//
// Viewport-clamped, not the element's own raw getBoundingClientRect(): the
// reader pane (.split-pane-reader) is an ordinary in-flow box that can be
// far taller than one screen (a long chapter), so its own raw rect's height
// is the WHOLE scrollable content's height, not what's actually on screen
// right now -- using that directly would center the popover somewhere in
// the middle of off-screen content. Clamping to [0, innerWidth]/
// [0, innerHeight] on every edge gives "the visible slice of this pane,
// right now" instead, which is always a SUBSET of the pane's own real box
// -- so anything positioned within it is automatically still "within the
// pane" too, just additionally guaranteed on-screen. The atlas pane
// (.split-pane-atlas) is `position: sticky` and always exactly one viewport
// tall, so clamping is a no-op for it in practice -- the same function
// works correctly for both without a pane-specific branch.
export function getPaneRect(selector) {
    const el = document.querySelector(selector);
    if (!el) {
        return null;
    }

    const r = el.getBoundingClientRect();
    const left = Math.max(r.left, 0);
    const top = Math.max(r.top, 0);
    const right = Math.min(r.right, window.innerWidth);
    const bottom = Math.min(r.bottom, window.innerHeight);
    return { left, top, width: Math.max(right - left, 0), height: Math.max(bottom - top, 0) };
}

// M-D3 (R3, the superscript rework): "the popover anchors OVER THE VERSE
// (not pane-centered), ALWAYS VISIBLE, never cut off by other UI." Unlike
// getPaneRect above (which only ever needs the target's own clamped rect --
// the popover that reads it centers ON that rect and is deliberately
// BOUNDED to it), a verse-anchored popover keeps its own ORDINARY preferred
// size (same as the default, viewport-centered case) and must still never
// spill off-screen regardless of where in the chapter the anchor verse
// sits -- top-of-chapter, bottom-of-chapter, or (split view) hugging either
// pane's own edge. That needs the viewport's own dimensions alongside the
// verse's rect, which getPaneRect's own return shape doesn't carry (its
// only consumer, PaneRectStyle, computes purely from the rect + a fixed
// margin already known in CSS) -- returned here instead of added there so
// getPaneRect's own existing contract/callers stay untouched.
//
// left/top are UNCLAMPED viewport-relative coordinates (getBoundingClientRect
// itself, not getPaneRect's own "visible slice" clamp -- a verse can
// legitimately sit just above/below the visible area for a frame during
// scroll-into-view, and ExplorerPopover.razor's own clamping arithmetic
// below needs the TRUE position to grow the popover away from the correct
// edge, not a pre-clamped one that could already read as "at the edge"
// when it truly isn't yet).
export function getVerseAnchorRect(selector) {
    const el = document.querySelector(selector);
    if (!el) {
        return null;
    }

    const r = el.getBoundingClientRect();
    return {
        left: r.left, top: r.top, width: r.width, height: r.height,
        viewportWidth: window.innerWidth, viewportHeight: window.innerHeight,
    };
}

// PEEK-TRUNC-1 (arrow-peek clipping defect, owner report 2026-08-24: "menus
// appearing on hover from arrow hover are getting cut off. needs to be
// truncated to an expandable menu limit one verse."): keyed off a real
// ElementReference, not a CSS selector -- ArrowNav can have up to FOUR
// simultaneous instances mounted on one popover (prior/following x
// narrative/chronology), and none of their own wrapping elements carries a
// unique id -- a selector naming their shared CSS class would only ever
// resolve document.querySelector's own FIRST match, silently measuring the
// wrong instance's anchor for every side but one. Same "pass the real
// element, not a selector" fix capturePointer (below) already established
// for SplitDivider's own analogous multi-instance concern -- this function
// is the read-side equivalent of that write-side precedent.
//
// F1 fix round (reviewer live-repro, real bug -- not a hypothetical): the
// first cut of this function returned window.innerWidth/innerHeight
// alongside the wrapper's own rect, and ArrowNav.razor budgeted its own
// flip/max-height math against THAT -- but the peek's own true clipping
// boundary is never the viewport; it is the nearest `.popover` ancestor
// (app.css: position:fixed, max-height:calc(100vh - 4rem), overflow-y:
// auto), which is routinely SHORTER than the viewport (vertically centered,
// never taller than 100vh-4rem) and clips every descendant once it exceeds
// that box -- including a position:absolute one like the peek, since
// overflow clipping is a PAINT-time ancestor relationship, independent of a
// descendant's own positioning scheme. Measuring the viewport instead of
// this real boundary is exactly why the owner's own original defect
// ("cut off... needs to be truncated") could resurface once expanded: the
// reviewer's own live repro clicked `all` on a many-verse peek and watched
// it spill ~269px past the popover's own bottom edge while the peek's own
// internal scrollbar (app.css's own overflow-y:auto) never engaged --
// its budget had come from the wrong frame entirely, so it never thought
// it was out of room. Now returns the wrapper's own rect PLUS the
// enclosing .popover's own top/bottom (viewport-relative coordinates,
// the SAME space getBoundingClientRect already uses, so ArrowNav.razor's
// own arithmetic needs no unit conversion) -- falling back to the
// viewport's own bounds only if no .popover ancestor is found at all
// (never true in practice, this component's only rendering context IS
// inside one, but a safe, honest degrade rather than a crash if that
// assumption is ever wrong).
export function getElementRect(el) {
    if (!el) {
        return null;
    }

    const r = el.getBoundingClientRect();
    const popover = el.closest('.popover');
    const clip = popover ? popover.getBoundingClientRect() : null;
    return {
        left: r.left, top: r.top, width: r.width, height: r.height,
        popoverTop: clip ? clip.top : 0,
        popoverBottom: clip ? clip.bottom : window.innerHeight,
    };
}

// PEEK-TRUNC-1 fix round (real, live-caught bug -- Playwright-reproduced
// AND root-caused, not guessed): clicking RevealControls' own `less`
// inside ArrowNav's peek can remove the EXACT element the pointer is
// resting on (Shown returns to Default, `less` itself -- and, moving back
// under Step, one or more verses -- disappear from the DOM in the same
// render that handled the click). Confirmed via an isolated repro
// (DIAG-A/B/C/D, batch report has the full matrix): a bare hover-then-
// leave dismisses correctly every time; hover-then-`more`(adds content,
// nothing removed)-then-leave ALSO dismisses correctly; hover-then-`more`-
// then-`less` (removes content under the pointer) leaves the peek stuck
// open through a subsequent, genuinely-away `pointermove` -- UNTIL one
// more, unrelated hover transition over a still-live element happens
// somewhere in the subtree, after which leaving works again. Root cause:
// the browser computes pointerenter/pointerleave for a listening ancestor
// by comparing the transition's own OLD hit-test target against that
// ancestor -- once that OLD target is DETACHED (removed, no parent chain
// left to walk), the comparison silently reads "was never contained" for
// every ancestor, so no leave is ever computed relative to it; the very
// next VALID transition (a live element re-entered) gives the browser a
// non-detached "old" reference again, which is why that one fix works.
// ArrowNav.razor's own OnWrapperPointerDown/OnPeekShownChanged use this
// function to independently re-verify, via a real geometric query (never
// trusting the browser's own possibly-corrupted tracking) whether the
// pointer's last known position is still inside the wrapper OR the peek
// box -- x/y are viewport (clientX/clientY) coordinates, matching
// PointerEventArgs' own ClientX/ClientY. Checks the peek box SEPARATELY
// from the wrapper's own rect (not just the wrapper's) because the peek
// is `position:absolute` and renders OUTSIDE the wrapper's own normal-flow
// box (app.css's own comment on .popover-event-nav-side has the fuller
// story) -- getBoundingClientRect() on the wrapper alone would never
// include it.
export function isPointInsideEither(el, x, y) {
    if (!el) {
        return false;
    }

    const within = (r) => x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
    if (within(el.getBoundingClientRect())) {
        return true;
    }

    const peek = el.querySelector('.popover-arrow-peek');
    return peek ? within(peek.getBoundingClientRect()) : false;
}

// M-D3/U5+B2 (split-view drag-resize divider, Components/SplitDivider.razor):
// setPointerCapture is a real DOM element method with no Blazor-side
// equivalent (PointerEventArgs carries PointerId as plain data, not a
// capturable handle) -- SplitDivider's own OnPointerDown calls this once,
// at drag start, with its own @ref ElementReference and the fired event's
// PointerId, so every subsequent pointermove/pointerup keeps targeting
// the divider itself for the rest of the gesture even once the cursor
// travels beyond its own (deliberately narrow, 13px) hit area -- e.g. a
// fast drag toward one pane's own text/map content. Without this, the
// gesture would silently stop tracking (Blazor's own pointerleave firing on
// the divider) the instant the cursor left that narrow strip, which for a
// horizontal-only drag intended to travel far in the X direction is the
// common case, not an edge case. TimeSlider.razor's own drag mechanic gets
// away with no equivalent because its own draggable surface (.slider-track)
// is the FULL travel range already -- a single-purpose narrow strip like
// this divider has no equally-wide native target to rely on instead.
export function capturePointer(el, pointerId) {
    if (el && typeof el.setPointerCapture === 'function') {
        el.setPointerCapture(pointerId);
    }
}

// Batch CORPREAD-1a fix round 1 (review S-6, IMPORTANT -- "--app-header-height
// is measured with a viewport-CLAMPING helper, so a scrolled hatch-open
// under-measures it"): `CompositionSplit.razor`'s own `.split-view`
// height:calc() (`app.css`'s own header comment on that rule) needs
// `.app-header`'s TRUE, intrinsic rendered height -- getPaneRect above
// deliberately clamps its own rect to [0, innerWidth]/[0, innerHeight] (the
// CORRECT behavior for its own original purpose, bounding a pane-anchored
// popover to "the visible slice of this pane, right now"), which is WRONG
// for measuring an element's own height: `.app-header` is in NORMAL FLOW
// (`.header-parchment` sets no `position`), so opening a split from a
// standalone reader already scrolled down by even a small amount clamps
// `top` to 0 and silently under-reports `bottom - top` by exactly that
// scroll offset -- confirmed live (the review's own repro): at scrollY≈20,
// a real 54.375px header measured as ≈34px, making `.split-view` ~20px
// TALLER than the remaining viewport, reintroducing (for that session) the
// exact "header-sized sliver stranded below the fold" failure class
// iteration 1 of the SPLIT-SCROLL-1 fix was rejected for. `offsetHeight` is
// the CSS layout height of the element's own border box -- entirely
// independent of scroll position or viewport clipping, exactly what a
// `calc(100vh - ...)` sizing computation needs. `el.offsetHeight` is `0`
// for a `display:none`/detached element, matching this call site's own
// existing "0/negative measurement -> fall back to the CSS default"
// fail-soft handling (CompositionSplit.razor's own `if (h.Height > 0)`
// still applies to whatever this returns).
export function getElementHeight(selector) {
    const el = document.querySelector(selector);
    return el ? el.offsetHeight : 0;
}
