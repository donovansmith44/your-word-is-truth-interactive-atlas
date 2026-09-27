let _observer = null;

export function observeProseItems(dotnetRef, selector) {
    unobserveProseItems();

    if (!('IntersectionObserver' in window)) {
        // No IntersectionObserver: fetch everything eagerly rather than leaving prose unfetched forever.
        document.querySelectorAll(selector).forEach(el => {
            dotnetRef.invokeMethodAsync('OnItemNearViewport', el.dataset.kretzmannItemId);
        });
        return;
    }

    _observer = new IntersectionObserver((entries) => {
        for (const entry of entries) {
            if (entry.isIntersecting) {
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
