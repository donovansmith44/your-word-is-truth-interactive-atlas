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
