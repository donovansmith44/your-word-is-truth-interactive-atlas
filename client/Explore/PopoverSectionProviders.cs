using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Rendering;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch R requirement 3(a)/4: the verse/passage's OWN text, with the
/// expand-into-a-scrollable-mini-reader affordance requirement 4 asks for.
/// Applies to Verse and Passage nodes; resolves the (book, chapter, focal
/// verse range, compact text) tuple synchronously-ish (VerseNode's own text
/// needs ONE memoized fetch, reused rather than duplicated -- see below) and
/// hands the actual rendering to a real component (<c>VerseTextSection</c>,
/// client/Components/) rather than a hand-built RenderFragment: unlike every
/// OTHER section here, this one owns real interactive STATE (expanded/
/// collapsed, a lazily-fetched chapter, a scroll target) that a closure-based
/// fragment has no clean way to hold.
/// </summary>
public sealed class VerseTextSectionProvider : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind is "Verse" or "Passage";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        string book;
        int chapter, focalFrom, focalTo;
        string compactText;

        switch (node)
        {
            case VerseNode v:
                var (vBook, vChapter, vVerse) = CanonRef.ParseVerse(v.Title); // Title IS the vref
                book = vBook;
                chapter = vChapter;
                focalFrom = focalTo = vVerse;
                try
                {
                    compactText = (await v.DetailAsync(api)).Text; // memoized -- CrossRefsSection shares this exact fetch, never a second one
                }
                catch (Exception)
                {
                    compactText = "";
                }
                break;

            case PassageNode p:
                var (pBook, pChapter, pFromVerse) = CanonRef.ParseVerse(CanonRef.FirstVerseOf(p.Title));
                book = pBook;
                chapter = pChapter;
                focalFrom = pFromVerse;
                // A passage's own span is same-chapter by construction (READ-5's
                // own shift-click mechanic; PassageNode never crosses a chapter
                // boundary) -- the LAST verse number is the tail of "GEN.12.1-4"'s
                // own "1-4" segment.
                var dash = p.Title.LastIndexOf('-');
                focalTo = dash >= 0 && int.TryParse(p.Title[(dash + 1)..], out var toVerse) ? toVerse : focalFrom;
                compactText = p.Text; // already known, no fetch (PassageNode's own doc comment)
                break;

            default:
                return null;
        }

        RenderFragment fragment = builder =>
        {
            builder.OpenComponent<Components.VerseTextSection>(0);
            builder.AddAttribute(1, "Book", book);
            builder.AddAttribute(2, "Chapter", chapter);
            builder.AddAttribute(3, "FocalFromVerse", focalFrom);
            builder.AddAttribute(4, "FocalToVerse", focalTo);
            builder.AddAttribute(5, "CompactText", compactText);
            builder.CloseComponent();
        };
        return new PopoverSection("verse-text", fragment);
    }
}

/// <summary>
/// Batch R requirement 3(b): the existing <c>GET /api/xrefs/{sref}</c> list,
/// now rendered INLINE (no <c>popover-chip-xrefs</c> toggle press -- see
/// VerseNode/PassageNode's own <c>ExploreAsync</c> comments for the retired
/// chip) for BOTH Verse (unconditional fetch, same as before) and Passage
/// (conditional -- absent whenever the span has zero cross-references, same
/// "conditional presence" this whole batch is built around). Each entry is
/// explorable (<c>.explorable</c>, per the batch brief's own "each entry
/// explorable" wording) -- clicking one pushes a fresh VerseNode for its
/// target, same FollowXref behavior this app has always had for a
/// cross-reference.
/// </summary>
public sealed class CrossRefsSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind is "Verse" or "Passage";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        List<CrossRefOut> xrefs;
        try
        {
            xrefs = node switch
            {
                VerseNode v => (await v.DetailAsync(api)).CrossRefs, // memoized -- shares VerseTextSectionProvider's own fetch
                PassageNode p => await p.XrefsAsync(api), // memoized -- its own dedicated cache
                _ => new List<CrossRefOut>(),
            };
        }
        catch (Exception)
        {
            return null; // fail soft -- same graceful-degradation policy every other lazy fetch in this app follows
        }

        if (xrefs.Count == 0)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = 0;
            // Reuses the pre-existing .popover-xref-list wrapper (app.css) --
            // its own flex-column + gap is what gives multiple xref items
            // their vertical rhythm; without SOME wrapper here, each
            // .popover-xref-item button (a block-level flex container itself,
            // for its OWN target/preview layout) would still stack correctly
            // but with no gap between rows at all.
            builder.OpenElement(seq++, "div");
            builder.AddAttribute(seq++, "class", "popover-xref-list");
            foreach (var x in xrefs)
            {
                var target = x.Target; // local copy captured per-row by the onclick closure below
                var preview = x.Preview;
                builder.OpenElement(seq++, "button");
                builder.AddAttribute(seq++, "type", "button");
                builder.AddAttribute(seq++, "class", "popover-xref-item explorable");
                builder.AddAttribute(seq++, "data-testid", $"xref-item-{target}");
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new VerseNode(CanonRef.FirstVerseOf(target)))));

                builder.OpenElement(seq++, "span");
                builder.AddAttribute(seq++, "class", "popover-xref-target");
                builder.AddContent(seq++, target);
                builder.CloseElement();

                builder.OpenElement(seq++, "span");
                builder.AddAttribute(seq++, "class", "popover-xref-preview");
                builder.AddContent(seq++, preview);
                builder.CloseElement();

                builder.CloseElement();
            }
            builder.CloseElement();
        };
        return new PopoverSection("xrefs", body);
    }
}

/// <summary>
/// Batch R requirement 3(c): THE SEAM for Batch F's own "THE SMALL
/// CATECHISM" provider -- registered at exactly the position requirement 3
/// asks for (between cross-references and Explore), renders NOTHING today
/// (conditional presence: no catechism content exists yet in this app at
/// all). Batch F either replaces this class's own <c>ResolveAsync</c> body
/// with real content, or registers a new provider at this same
/// <see cref="PopoverSectionRegistry"/> position and removes this
/// placeholder -- either way, no other file changes. Deliberately scoped to
/// VERSE only (per requirement 3's own "VERSE node sections" list) -- a
/// passage's own catechism relevance, if any, is a Batch F design decision,
/// not this seam's to anticipate.
/// </summary>
public sealed class CatechismSeamSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Verse";

    public Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx) =>
        Task.FromResult<PopoverSection?>(null);
}

/// <summary>
/// Batch R requirement 3: THE SEAM for Batch P's own Theographic place
/// description -- same treatment as <see cref="CatechismSeamSection"/>, one
/// registry slot earlier (PLACE's own ordering: description, dates, blurb,
/// events).
/// </summary>
public sealed class PlaceDescriptionSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Place";

    public Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx) =>
        Task.FromResult<PopoverSection?>(null);
}

/// <summary>
/// Batch R requirement 3: established/destroyed, "with their supporting
/// verse refs explorable" -- existing Batch E data (<c>PlaceHistoryOut</c>'s
/// own <c>established</c>/<c>destroyed</c>), now surfaced in the POPOVER
/// itself (PlaceCard's own <c>place-card-date-established</c>/
/// <c>-destroyed</c> affordances already reached this same content one hop
/// further out -- this is the content those buttons drill INTO, unchanged;
/// see CONTRACT.md's own new testids for this popover-native rendering).
/// Conditional presence: absent when this place has neither claim curated.
/// </summary>
public sealed class PlaceDatesSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Place";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not PlaceNode place)
        {
            return null;
        }

        PlaceDetail detail;
        try
        {
            detail = await place.DetailAsync(api); // memoized -- shared with PlaceBlurbSection/PlaceEventsSection
        }
        catch (Exception)
        {
            return null;
        }

        var established = detail.History?.Established;
        var destroyed = detail.History?.Destroyed;
        if (established is null && destroyed is null)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "div");
            builder.AddAttribute(seq++, "class", "popover-place-dates");
            if (established is { } est)
            {
                RenderDateRow(builder, ref seq, "established", "Established", est, ctx);
            }
            if (destroyed is { } dest)
            {
                RenderDateRow(builder, ref seq, "destroyed", "Destroyed", dest, ctx);
            }
            builder.CloseElement();
        };
        return new PopoverSection("place-dates", body);
    }

    // Named wrapper, not an inline lambda -- Razor/RenderTreeBuilder's own
    // @onclick attribute-value machinery can't reliably close over a `ref`
    // parameter, and this keeps the established/destroyed rows byte-for-byte
    // identical rather than two near-duplicated inline blocks.
    private static void RenderDateRow(RenderTreeBuilder builder, ref int seq, string testidSuffix, string label, DateClaimOut claim, IPopoverSectionContext ctx)
    {
        var dateText = YearText.FormatClaim(claim.When.FromYear, claim.When.ToYear, claim.Note);
        var when = claim.When;
        var verses = claim.Verses;
        var note = claim.Note;

        builder.OpenElement(seq++, "button");
        builder.AddAttribute(seq++, "type", "button");
        builder.AddAttribute(seq++, "class", "popover-place-date explorable");
        builder.AddAttribute(seq++, "data-testid", $"popover-place-date-{testidSuffix}");
        builder.AddAttribute(seq++, "aria-label", $"Explore {label.ToLowerInvariant()} date: {dateText}");
        builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new YearNode(label, when, verses, note))));

        builder.OpenElement(seq++, "span");
        builder.AddAttribute(seq++, "class", "popover-place-date-label");
        builder.AddContent(seq++, label);
        builder.CloseElement();

        builder.OpenElement(seq++, "span");
        builder.AddAttribute(seq++, "class", "popover-place-date-value");
        builder.AddContent(seq++, dateText);
        builder.CloseElement();

        builder.CloseElement();
    }
}

/// <summary>
/// Batch R requirement 3: the place's own period blurb, resolved against the
/// SAME window <see cref="PlaceNode"/> was constructed with (see that
/// class's own constructor comment) -- same BLURB-1 resolution
/// (CONTRACT.md) PlaceCard's own <c>place-card-blurb</c> already uses.
/// Conditional presence: absent whenever the API returned none for this
/// window (no window at all, or a window matching neither an "era" nor a
/// "broad" curated range).
/// </summary>
public sealed class PlaceBlurbSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Place";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not PlaceNode place)
        {
            return null;
        }

        PlaceDetail detail;
        try
        {
            detail = await place.DetailAsync(api);
        }
        catch (Exception)
        {
            return null;
        }

        var blurb = detail.History?.Blurb;
        if (blurb is null)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-place-blurb");
            builder.AddAttribute(2, "data-testid", "popover-place-blurb");
            builder.AddContent(3, blurb);
            builder.CloseElement();
        };
        return new PopoverSection("place-blurb", body);
    }
}

/// <summary>
/// Batch R requirement 3: "events (verse groups)" -- this place's own event
/// rows, each pushing a fresh <see cref="TimeAndPlaceNode"/> on click (the
/// SAME drill-in <see cref="PlaceNode"/>'s own retired-as-primary
/// <c>BodyAsync</c>/<c>OnSelectEvent</c> mechanism already offered -- see
/// that node's own doc comment for why BodyAsync survives, unused, as a
/// defensive fallback). Reuses the existing <c>place-event-{id}</c> testid
/// (already shipped, simply undocumented in CONTRACT.md before this batch --
/// see this batch's own CONTRACT amendment). Conditional presence: absent
/// for a place with zero recorded events (impossible for a REAL lit/quiet
/// place opened via its own card -- SCENE-2/QUIET-1 -- but a place explored
/// some other way, e.g. a future search feature, could still have none).
/// </summary>
public sealed class PlaceEventsSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Place";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not PlaceNode place)
        {
            return null;
        }

        PlaceDetail detail;
        try
        {
            detail = await place.DetailAsync(api);
        }
        catch (Exception)
        {
            return null;
        }

        if (detail.Events.Count == 0)
        {
            return null;
        }

        var placeName = place.Title;
        var events = detail.Events;
        RenderFragment body = builder =>
        {
            var seq = 0;
            foreach (var e in events)
            {
                var ev = e; // local copy -- captured per-row by the onclick closure below
                builder.OpenElement(seq++, "button");
                builder.AddAttribute(seq++, "type", "button");
                builder.AddAttribute(seq++, "class", "popover-event-row popover-event-row-button explorable");
                builder.AddAttribute(seq++, "data-testid", $"place-event-{ev.Id}");
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () =>
                    ctx.PushAsync(new TimeAndPlaceNode(placeName, ev.When, ev.Label, ev.VerseGroups))));

                builder.OpenElement(seq++, "span");
                builder.AddAttribute(seq++, "class", "popover-event-label");
                builder.AddContent(seq++, ev.Label);
                builder.CloseElement();

                builder.OpenElement(seq++, "span");
                builder.AddAttribute(seq++, "class", "popover-event-years");
                builder.AddContent(seq++, YearText.FormatRange(ev.When.FromYear, ev.When.ToYear));
                builder.CloseElement();

                builder.CloseElement();
            }
        };
        return new PopoverSection("place-events", body);
    }
}
