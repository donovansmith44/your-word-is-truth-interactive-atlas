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

// Batch H (view-state round-trip). setScrollY is the restore half -- plain
// window-level scroll, covering BOTH the standalone reader (the whole
// document scrolls) and the split-view reader pane (app.css's own
// .split-pane-reader is a normal in-flow flex child, not its own
// overflow:auto container, so the document itself is still what scrolls
// there too -- no separate code path needed for either).
export function setScrollY(y) {
    window.scrollTo(0, y);
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
let _scrollCleanup = null;

export function watchScroll(dotnetRef) {
    if (_scrollCleanup) {
        _scrollCleanup();
    }

    let ticking = false;
    const onScroll = () => {
        if (ticking) {
            return;
        }
        ticking = true;
        requestAnimationFrame(() => {
            ticking = false;
            dotnetRef.invokeMethodAsync('OnScroll', window.scrollY);
        });
    };
    window.addEventListener('scroll', onScroll, { passive: true });

    _scrollCleanup = () => {
        window.removeEventListener('scroll', onScroll);
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
let _navCenterCleanup = null;

export function watchChapterNavCenter() {
    if (_navCenterCleanup) {
        _navCenterCleanup();
    }

    const page = document.querySelector('[data-testid="reader-root"]');
    if (!page) {
        return;
    }

    const recompute = () => {
        const top = window.innerHeight / 2 - page.getBoundingClientRect().top;
        page.style.setProperty('--chapter-nav-top', `${top}px`);
    };

    recompute();
    window.addEventListener('scroll', recompute, { passive: true });
    window.addEventListener('resize', recompute);

    _navCenterCleanup = () => {
        window.removeEventListener('scroll', recompute);
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
// continuous tracker), so the popover can center itself within whichever
// PANE it opened from rather than the full viewport.
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
