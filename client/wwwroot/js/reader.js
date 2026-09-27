export function scrollToVerse(n) {
    const el = document.getElementById('v' + n);
    if (el) {
        el.scrollIntoView({ block: 'center' });
    }
}

// Uses a per-instance random DOM id, not the reader page's own "v{n}" anchor, so a popover's
// mini-reader never collides with it. `block: 'nearest'` because the mini-reader is a small,
// bounded overflow region, not the whole viewport.
export function scrollFocalRowIntoView(domId) {
    const el = document.getElementById(domId);
    if (el) {
        el.scrollIntoView({ block: 'nearest' });
    }
}

// Shift-held tracking has no Blazor binding for window.blur/visibilitychange, and neither
// keydown nor keyup fires if Shift is released while this tab isn't focused -- without this,
// _shiftHeld would stay stuck true. Both listeners reset the same state; resetting on either is safe.
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

// Split view makes .split-pane-reader/.split-pane-host a real overflow-y:auto container of its
// own, so window scroll is no longer a safe assumption. Walks up from the root testid to find
// the real scrolling ancestor, falling back to window/document if none is found.
function findScrollContainer(rootTestId) {
    let node = document.querySelector(`[data-testid="${rootTestId}"]`);
    while (node && node !== document.body) {
        const style = getComputedStyle(node);
        if (style.overflowY === 'auto' || style.overflowY === 'scroll') {
            return node;
        }
        node = node.parentElement;
    }
    return null;
}

function findReaderScrollContainer() {
    return findScrollContainer('reader-root');
}

export function scrollToTop(rootTestId) {
    const container = findScrollContainer(rootTestId);
    if (container) {
        container.scrollTop = 0;
    } else {
        window.scrollTo(0, 0);
    }
}

export function setScrollY(y) {
    const container = findReaderScrollContainer();
    if (container) {
        container.scrollTop = y;
    } else {
        window.scrollTo(0, y);
    }
}

// Reports scroll position continuously into ViewStateService rather than reading it once at
// dispose time: Blazor's router resets window scroll to (0,0) as part of committing a navigation,
// BEFORE the outgoing component's DisposeAsync runs, so a dispose-time read would reliably see 0.
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

// Viewport-clamped, not the element's raw rect: a pane can be an ordinary in-flow box taller
// than one screen, so its raw rect height is the whole scrollable content, not what's on screen.
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

// Unlike getPaneRect, returns unclamped coordinates: a verse can legitimately sit just off-screen
// for a frame during scroll-into-view, and clamping here would make the anchor read as
// "at the edge" before it truly is.
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

// Keyed off a real element reference, not a CSS selector, because multiple ArrowNav instances
// can be mounted at once and share a class. Returns the nearest enclosing .popover's bounds
// (not the viewport) since that's the real clipping boundary the peek can spill past.
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

// The browser computes pointerenter/pointerleave for a listening ancestor by comparing against
// the transition's old hit-test target; once that target is removed from the DOM mid-gesture,
// the comparison silently reports "never contained," so a leave is never fired. This re-verifies
// via a real geometric query instead of trusting that tracking. Checked separately from the
// wrapper because the peek is position:absolute and renders outside the wrapper's own box.
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

// setPointerCapture keeps subsequent pointermove/pointerup targeting the divider even once the
// cursor travels beyond its narrow hit area -- without it, Blazor's pointerleave would silently
// stop tracking the drag the instant the cursor left that strip.
export function capturePointer(el, pointerId) {
    if (el && typeof el.setPointerCapture === 'function') {
        el.setPointerCapture(pointerId);
    }
}

// offsetHeight, not a clamped/viewport rect: this element is in normal flow, so a scrolled page
// would clamp `top` to 0 and under-report height by the scroll offset.
export function getElementHeight(selector) {
    const el = document.querySelector(selector);
    return el ? el.offsetHeight : 0;
}
