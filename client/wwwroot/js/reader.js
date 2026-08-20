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
// recomputed continuously (scroll + resize, rAF-throttled, same shape as
// watchScroll above) from `.reader-page`'s own REAL, CURRENT
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
let _navCenterCleanup = null;

export function watchChapterNavCenter() {
    if (_navCenterCleanup) {
        _navCenterCleanup();
    }

    const page = document.querySelector('[data-testid="reader-root"]');
    if (!page) {
        return;
    }

    let ticking = false;
    const recompute = () => {
        const top = window.innerHeight / 2 - page.getBoundingClientRect().top;
        page.style.setProperty('--chapter-nav-top', `${top}px`);
    };
    const onScrollOrResize = () => {
        if (ticking) {
            return;
        }
        ticking = true;
        requestAnimationFrame(() => {
            ticking = false;
            recompute();
        });
    };

    recompute();
    window.addEventListener('scroll', onScrollOrResize, { passive: true });
    window.addEventListener('resize', onScrollOrResize);

    _navCenterCleanup = () => {
        window.removeEventListener('scroll', onScrollOrResize);
        window.removeEventListener('resize', onScrollOrResize);
        _navCenterCleanup = null;
    };
}

export function unwatchChapterNavCenter() {
    if (_navCenterCleanup) {
        _navCenterCleanup();
    }
}
