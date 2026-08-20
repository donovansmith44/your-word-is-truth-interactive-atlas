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
/// Batch F ("the small catechism"): fills Batch R's own seam (requirement
/// 3(c)) with real content -- "THE SMALL CATECHISM" section for VERSE
/// AND PASSAGE popovers (the seam's own doc comment scoped this to Verse
/// only; the batch brief's own requirement 4 heading, "VERSE/PASSAGE
/// popover," and requirement 3's "span/passage selections aggregate citing
/// items the way xrefs already aggregate" both extend it to Passage too --
/// same <c>AppliesTo</c> shape <see cref="CrossRefsSection"/> already uses).
/// Same registry SLOT, same class NAME, per the task reviewer's own live
/// verification that this batch needs only this one provider's
/// <c>ResolveAsync</c> body replaced -- no other registry/ExplorerPopover
/// change.
///
/// Lists citing items as explorable entries named by the item's own
/// (curated) display name -- "The First Commandment", "Baptism — Part the
/// Fourth" -- clicking one pushes a <see cref="CatechismNode"/> (the SAME
/// drill-in <see cref="IPopoverSectionContext.PushAsync"/> mechanism every
/// other section-native explorable row already uses). Conditional presence:
/// a verse/passage citing nothing shows no section at all (no placeholder
/// text), same rule <see cref="CrossRefsSection"/> already follows.
/// </summary>
public sealed class CatechismSeamSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind is "Verse" or "Passage";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        List<CatechismRefDto> items;
        try
        {
            items = node switch
            {
                VerseNode v => (await v.DetailAsync(api)).Catechism, // memoized -- shares VerseTextSectionProvider's/CrossRefsSection's own fetch
                PassageNode p => await p.CatechismAsync(api), // memoized -- its own dedicated cache, mirrors XrefsAsync
                _ => new List<CatechismRefDto>(),
            };
        }
        catch (Exception)
        {
            return null; // fail soft -- same graceful-degradation policy every other lazy fetch in this app follows
        }

        if (items.Count == 0)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "catechism-section-heading");
            builder.AddAttribute(seq++, "data-testid", "catechism-section-heading");
            builder.AddContent(seq++, "THE SMALL CATECHISM");
            builder.CloseElement();

            builder.OpenElement(seq++, "div");
            builder.AddAttribute(seq++, "class", "popover-catechism-list");
            foreach (var it in items)
            {
                var id = it.Id; // local copies -- captured per-row by the onclick closure below
                var name = it.Name;
                builder.OpenElement(seq++, "button");
                builder.AddAttribute(seq++, "type", "button");
                builder.AddAttribute(seq++, "class", "popover-catechism-item explorable");
                builder.AddAttribute(seq++, "data-testid", $"catechism-item-{id}");
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new CatechismNode(id, name))));
                builder.AddContent(seq++, name);
                builder.CloseElement();
            }
            builder.CloseElement();
        };
        return new PopoverSection("catechism", body);
    }
}

/// <summary>
/// Batch F: shared rendering helper for every "Catechism" node-kind section
/// below -- an optional small-caps title (Luther's own verbatim heading, or
/// this app's own invented section chrome like "THE SCRIPTURES") followed by
/// the body text, split on a literal blank line ("\n\n") into one
/// &lt;p&gt; per paragraph. The blank-line split matters specifically for
/// Lord's-Prayer/Confession items, whose curated `explanation` deliberately
/// concatenates TWO of Luther's own answers (e.g. "What does this
/// mean?" + "How is this done?") under one heading -- see
/// data/curated/catechism.toml's own header comment -- rendering them as
/// two separate paragraphs preserves that structure visually rather than
/// running them together as one wall of text.
/// </summary>
file static class CatechismSectionRendering
{
    public static void TitledParagraphs(RenderTreeBuilder builder, ref int seq, string? title, string bodyClass, string body)
    {
        if (title is not null)
        {
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "catechism-section-heading");
            builder.AddAttribute(seq++, "data-testid", "catechism-section-heading");
            builder.AddContent(seq++, title);
            builder.CloseElement();
        }

        foreach (var para in body.Split("\n\n", StringSplitOptions.RemoveEmptyEntries))
        {
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", bodyClass);
            builder.AddContent(seq++, para);
            builder.CloseElement();
        }
    }
}

/// <summary>
/// Batch F: a <see cref="CatechismNode"/>'s own primary-source TEXT --
/// the commandment/creed-article/Lord's-Prayer-petition wording itself,
/// unlabeled (no heading precedes it -- requirement 4 lists it first,
/// bare: "sections = the item's text"). Conditional presence: Baptism/
/// Confession/Sacrament-of-the-Altar items have no separate prompt distinct
/// from their own Q&amp;A (<c>CatechismItemDetail.Text</c> is null there --
/// see that record's own doc comment), so this section is simply absent for
/// them -- "no Explore/map section... conditional presence applies to
/// affordances too" extends here too, one layer further in.
/// </summary>
public sealed class CatechismTextSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Catechism";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not CatechismNode item)
        {
            return null;
        }

        CatechismItemDetail detail;
        try
        {
            detail = await item.DetailAsync(api); // memoized -- shared with the three sibling providers below
        }
        catch (Exception)
        {
            return null;
        }

        if (detail.Text is not { } text)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-catechism-text");
            builder.AddContent(2, text);
            builder.CloseElement();
        };
        return new PopoverSection("catechism-text", body);
    }
}

/// <summary>
/// Batch F: a <see cref="CatechismNode"/>'s own explanation -- Luther's OWN
/// verbatim heading (<c>CatechismItemDetail.ExplanationHeading</c>, "What
/// does this mean?" for the overwhelming majority of items, a distinct real
/// question for Baptism/Confession/Sacrament-of-the-Altar items) as the
/// section's own small-caps title, per requirement 4 verbatim. Always
/// present -- every curated item has a non-empty explanation.
/// </summary>
public sealed class CatechismExplanationSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Catechism";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not CatechismNode item)
        {
            return null;
        }

        CatechismItemDetail detail;
        try
        {
            detail = await item.DetailAsync(api);
        }
        catch (Exception)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = 0;
            CatechismSectionRendering.TitledParagraphs(builder, ref seq, detail.ExplanationHeading, "popover-catechism-explanation", detail.Explanation);
        };
        return new PopoverSection("catechism-explanation", body);
    }
}

/// <summary>
/// Batch F: a <see cref="CatechismNode"/>'s own "Where is this written?"
/// proof text -- present only for the items where Luther's own text poses
/// that exact question (Baptism Part the First/Second/Fourth, the
/// Sacrament of the Altar's institution words); conditional presence,
/// absent otherwise (e.g. Baptism Part the Third's own Titus citation is
/// embedded inline in its explanation instead -- see that item's own
/// curated `ref_note`).
/// </summary>
public sealed class CatechismWhereWrittenSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Catechism";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not CatechismNode item)
        {
            return null;
        }

        CatechismItemDetail detail;
        try
        {
            detail = await item.DetailAsync(api);
        }
        catch (Exception)
        {
            return null;
        }

        if (detail.WhereWritten is not { } whereWritten)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = 0;
            CatechismSectionRendering.TitledParagraphs(builder, ref seq, "Where is this written?", "popover-catechism-where-written", whereWritten);
        };
        return new PopoverSection("catechism-where-written", body);
    }
}

/// <summary>
/// Batch F: "THE SCRIPTURES" -- a <see cref="CatechismNode"/>'s own proof
/// verses, each rendered with its OWN FULL KJV text (house rendering, not a
/// truncated preview -- <c>CatechismItemDetail.Verses</c>'s own doc
/// comment) and explorable: clicking one pushes a fresh <see cref="VerseNode"/>,
/// so onward navigation keeps working exactly as it does everywhere else in
/// this app (verse -&gt; catechism -&gt; proof verse -&gt; its OWN cross-references
/// -&gt; ..., requirement 4 verbatim) with no bespoke code here -- a plain
/// VerseNode already carries every section (text/xrefs/its own catechism
/// citations, if any) uniformly regardless of how it was reached.
/// Conditional presence: absent for an item with zero curated proof verses
/// (most items -- Luther's Small Catechism embeds few explicit citations;
/// see this batch's own report for the full disclosure).
/// </summary>
public sealed class CatechismScripturesSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Catechism";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not CatechismNode item)
        {
            return null;
        }

        CatechismItemDetail detail;
        try
        {
            detail = await item.DetailAsync(api);
        }
        catch (Exception)
        {
            return null;
        }

        if (detail.Verses.Count == 0)
        {
            return null;
        }

        var verses = detail.Verses;
        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "catechism-section-heading");
            builder.AddAttribute(seq++, "data-testid", "catechism-section-heading");
            builder.AddContent(seq++, "THE SCRIPTURES");
            builder.CloseElement();

            builder.OpenElement(seq++, "div");
            builder.AddAttribute(seq++, "class", "popover-catechism-verse-list");
            foreach (var v in verses)
            {
                var vref = v.Vref; // local copies -- captured per-row by the onclick closure below
                var text = v.Text;
                builder.OpenElement(seq++, "button");
                builder.AddAttribute(seq++, "type", "button");
                builder.AddAttribute(seq++, "class", "popover-catechism-verse explorable");
                builder.AddAttribute(seq++, "data-testid", $"catechism-verse-{vref}");
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new VerseNode(vref))));

                builder.OpenElement(seq++, "span");
                builder.AddAttribute(seq++, "class", "popover-catechism-verse-ref");
                builder.AddContent(seq++, vref);
                builder.CloseElement();

                builder.OpenElement(seq++, "span");
                builder.AddAttribute(seq++, "class", "popover-catechism-verse-text");
                builder.AddContent(seq++, text);
                builder.CloseElement();

                builder.CloseElement();
            }
            builder.CloseElement();
        };
        return new PopoverSection("catechism-scriptures", body);
    }
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
