// Fix round (Q-2 CRITICAL, controller ruling Q-C2, KRETZMANN-13): the
// original ship of this batch fetched EVERY commentary item's own prose
// (Kretzmann.razor's own Task.WhenAll over the whole chapter) the instant a
// chapter's listing resolved -- reintroducing KRETZ-SCALE-1's own retired
// per-verse fan-out through a different endpoint (Card() instead of the
// commented-on-by edges page), with its own guard test silently narrowed to
// assert only the OLD url shape stayed at zero, never that the NEW one was
// bounded. "Never do anything like this" (controller ruling). This module
// is the real, disclosed client-side mitigation:
//
// KRETZ-PROSE-SCALE (parked, disclosed -- see Kretzmann.razor's own header):
// the REAL fix is a future, additive server change (widen the chapter-scoped
// LISTING response to carry each item's own prose, collapsing this whole
// mechanism back to KRETZ-SCALE-1's original ONE-request shape). Zero server
// changes is this batch's own standing machine rule, so that ticket stays
// parked; what ships here instead is a genuine, load-bearing mitigation, not
// a cosmetic one: a commentary item's own prose is requested only once its
// own row APPROACHES the viewport (IntersectionObserver, a generous
// rootMargin so the fetch starts slightly ahead of scroll, not exactly at
// the fold) -- never for the whole chapter on page load. Kretzmann.razor's
// own OnItemNearViewport additionally bounds ACTUAL CONCURRENT fetches to a
// SemaphoreSlim(8) gate; this module only decides WHEN a fetch is
// requested, never how many run at once.
let _observer = null;

export function observeProseItems(dotnetRef, selector) {
    unobserveProseItems();

    if (!('IntersectionObserver' in window)) {
        // No IntersectionObserver support in this environment -- fail
        // toward CORRECTNESS, not silence: request every item right away
        // rather than leaving prose permanently unfetched. This is
        // deliberately the SAME "degrade to the old eager behavior, never
        // to nothing" choice this app already makes elsewhere for optional
        // browser capabilities.
        document.querySelectorAll(selector).forEach(el => {
            dotnetRef.invokeMethodAsync('OnItemNearViewport', el.dataset.kretzmannItemId);
        });
        return;
    }

    _observer = new IntersectionObserver((entries) => {
        for (const entry of entries) {
            if (entry.isIntersecting) {
                // One-shot: this item's own prose fetch is requested at
                // most once (Kretzmann.razor's own _proseRequested set is
                // the authoritative guard; unobserving here just avoids a
                // redundant callback if the row crosses the rootMargin
                // boundary again before Blazor re-renders it with
                // data-loaded).
                _observer.unobserve(entry.target);
                dotnetRef.invokeMethodAsync('OnItemNearViewport', entry.target.dataset.kretzmannItemId);
            }
        }
    }, { rootMargin: '800px 0px', threshold: 0 });

    document.querySelectorAll(selector).forEach(el => _observer.observe(el));
}

export function unobserveProseItems() {
    if (_observer) {
        _observer.disconnect();
        _observer = null;
    }
}
