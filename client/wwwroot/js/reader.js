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
