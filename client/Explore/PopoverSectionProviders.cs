using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Rendering;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// M-D3 (owner rulings U4/B3): the chapter's own metadata-and-context card
/// -- "when you're reading a chapter, you're in its focus. you can focus
/// further by clicking chapter heading and you get metadata and context...
/// container title, position in book, edge summary -- what the graph knows
/// ABOUT the chapter" -- NEVER the chapter's own verse text (B3, the
/// standing "first verse" bug; see <see cref="ChapterNode"/>'s own doc
/// comment for the fuller history). Every fact below is read straight off
/// the SAME <c>ChapterOut</c> the reading view itself already fetched
/// (<see cref="ChapterNode.AlreadyLoaded"/>, reused via
/// <see cref="ChapterNode.Load"/> -- zero new network cost for the common
/// "open the heading of the chapter you're currently reading" case).
/// Headings/places are deduplicated (a multi-witness container, or M-D1's
/// own chapter-boundary continuation, can anchor more than one verse in
/// this same chapter) and individually explorable via the SAME
/// <see cref="IPopoverSectionContext.PushAsync"/> every other section-native
/// row in this file already uses -- "outward connections," not a dead-end
/// summary. Cross-references are a plain, non-explorable total (summing
/// each verse's own already-on-the-wire XrefCount, never a fetch of its
/// own) -- there is no single node a chapter-wide xref COUNT could push to.
/// </summary>
public sealed class ChapterCardSection : IPopoverSectionProvider
{
    // M-D3 fix round -- a real, live-caught bug, not a style preference:
    // both lists below were rendered fully unbounded, and a long acrostic
    // psalm (PSA.119, 22 Hebrew-letter sections -- CONTAINERS IN THIS
    // CHAPTER alone runs to 22 rows) makes this card tall enough to cover
    // chapter-head's OWN screen position while open. Since U4/B3 also
    // opens this SAME card on hover (matching XSCRIPT-1's own "hover and
    // click open the same popover" rule), that self-overlap means a
    // genuine click gesture -- which always hovers the target FIRST -- can
    // never actually land on chapter-head again once the hover-opened card
    // already covers it: reader.spec.ts's own READ-2c property test caught
    // this as an unrecoverable, indefinitely-retrying click specifically
    // whenever it happened to sample PSA.119. A hard cap bounds the card's
    // own height for every chapter, not just the pathological one found --
    // a plain, honest "+N more" line (not yet the full RevealControls
    // interactive mechanic U2 gives cross-refs/catechism elsewhere in this
    // same batch) rather than silently dropping the rest; widening this to
    // a real reveal control is a disclosed, deliberate follow-up, not
    // pretended-finished here.
    private const int ListCap = 8;

    public bool AppliesTo(IExplorable node) => node.Kind == "Chapter";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not ChapterNode chapterNode)
        {
            return null;
        }

        ChapterOut chapter;
        try
        {
            chapter = await chapterNode.Load(api);
        }
        catch (Exception)
        {
            return null;
        }

        var headings = chapter.Verses
            .Where(v => v.Heading is not null)
            .Select(v => v.Heading!)
            .GroupBy(h => h.EventId)
            .Select(g => g.First())
            .ToList();
        var places = chapter.Verses
            .SelectMany(v => v.Places)
            .GroupBy(p => p.Id)
            .Select(g => g.First())
            .ToList();
        var xrefTotal = chapter.Verses.Sum(v => v.XrefCount);
        var verseCount = chapter.Verses.Count;
        var positionText = chapterNode.TotalChapters is int total ? $"Chapter {chapterNode.Chapter} of {total}" : $"Chapter {chapterNode.Chapter}";

        RenderFragment body = builder =>
        {
            var seq = 0;

            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "popover-meta");
            builder.AddAttribute(seq++, "data-testid", "chapter-card-position");
            builder.AddContent(seq++, positionText);
            builder.CloseElement();

            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "popover-meta");
            builder.AddAttribute(seq++, "data-testid", "chapter-card-verse-count");
            builder.AddContent(seq++, $"{verseCount} verse{(verseCount == 1 ? "" : "s")}.");
            builder.CloseElement();

            if (headings.Count > 0)
            {
                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "catechism-section-heading");
                builder.AddAttribute(seq++, "data-testid", "chapter-card-headings-heading");
                builder.AddContent(seq++, "CONTAINERS IN THIS CHAPTER");
                builder.CloseElement();

                builder.OpenElement(seq++, "div");
                builder.AddAttribute(seq++, "class", "popover-chapter-card-list");
                builder.AddAttribute(seq++, "data-testid", "chapter-card-headings");
                foreach (var h in headings.Take(ListCap))
                {
                    var eventId = h.EventId; // local copies -- captured per-row by the onclick closure below
                    var title = h.Title;
                    var headingKind = h.Kind; // fix round 1 (S-1a/Q-1a): passed to EventNode's own knownKind below -- see that constructor param's own doc comment
                    builder.OpenElement(seq++, "button");
                    builder.AddAttribute(seq++, "type", "button");
                    builder.AddAttribute(seq++, "class", "popover-event-row popover-event-row-button explorable");
                    builder.AddAttribute(seq++, "data-testid", $"chapter-card-heading-{eventId}");
                    builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new EventNode(eventId, title, headingKind))));
                    builder.AddContent(seq++, title);
                    builder.CloseElement();
                }
                builder.CloseElement();

                if (headings.Count > ListCap)
                {
                    builder.OpenElement(seq++, "p");
                    builder.AddAttribute(seq++, "class", "popover-meta");
                    builder.AddAttribute(seq++, "data-testid", "chapter-card-headings-more");
                    builder.AddContent(seq++, $"+ {headings.Count - ListCap} more container{(headings.Count - ListCap == 1 ? "" : "s")} in this chapter.");
                    builder.CloseElement();
                }
            }

            if (places.Count > 0)
            {
                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "catechism-section-heading");
                builder.AddAttribute(seq++, "data-testid", "chapter-card-places-heading");
                builder.AddContent(seq++, "PLACES MENTIONED");
                builder.CloseElement();

                builder.OpenElement(seq++, "div");
                builder.AddAttribute(seq++, "class", "popover-chapter-card-list");
                builder.AddAttribute(seq++, "data-testid", "chapter-card-places");
                foreach (var p in places.Take(ListCap))
                {
                    var placeId = p.Id; // local copies -- captured per-row by the onclick closure below
                    var placeName = p.Name;
                    builder.OpenElement(seq++, "button");
                    builder.AddAttribute(seq++, "type", "button");
                    builder.AddAttribute(seq++, "class", "popover-event-row popover-event-row-button explorable");
                    builder.AddAttribute(seq++, "data-testid", $"chapter-card-place-{placeId}");
                    builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new PlaceNode(placeId, placeName))));
                    builder.AddContent(seq++, placeName);
                    builder.CloseElement();
                }
                builder.CloseElement();

                if (places.Count > ListCap)
                {
                    builder.OpenElement(seq++, "p");
                    builder.AddAttribute(seq++, "class", "popover-meta");
                    builder.AddAttribute(seq++, "data-testid", "chapter-card-places-more");
                    builder.AddContent(seq++, $"+ {places.Count - ListCap} more place{(places.Count - ListCap == 1 ? "" : "s")} mentioned in this chapter.");
                    builder.CloseElement();
                }
            }

            if (xrefTotal > 0)
            {
                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "popover-meta");
                builder.AddAttribute(seq++, "data-testid", "chapter-card-xref-total");
                builder.AddContent(seq++, $"{xrefTotal} cross-reference{(xrefTotal == 1 ? "" : "s")} in this chapter.");
                builder.CloseElement();
            }
        };
        return new PopoverSection("chapter-card", body);
    }
}

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
///
/// Batch M-D4 ("the recursive reader," decision 3, "name links everywhere...
/// wire data needs: if any surface's data path lacks spans, EXTEND that
/// fetch's DTO, disclosed, never a parallel path"): NEITHER VerseNode's own
/// <c>GET /api/verse/{vref}</c> (<see cref="VerseDetail"/> carries no
/// Places/Persons at all) NOR PassageNode's own pre-known <c>Text</c> (a
/// flat, already-concatenated string, no per-verse breakdown) can feed
/// <see cref="PlaceMentions"/> -- so this provider ALSO fetches the focal
/// range's own chapter (<c>GET /api/chapter/{cref}</c>, the SAME
/// LRU-cached, already-carries-Places/Persons/XrefCount endpoint
/// MiniReaderExpand itself fetches on expand -- no server change, no
/// graph-types touch, the client-side DTO extension the brief's own words
/// anticipate) and slices out the focal verses, WITH their real mention
/// data, as <see cref="Components.VerseTextSection.FocalVerses"/>. Fail-soft
/// (house pattern): wrapped in its own try/catch, independent of the
/// pre-existing compactText fetch immediately below, which stays exactly as
/// it was -- a genuinely CHEAP safety net (VerseNode.DetailAsync is
/// memoized, already re-fetched by CrossRefsSection/CatechismSeamSection/
/// VerseEventMembershipSection/VerseParallelsSection on the SAME node this
/// batch open, so calling it again here is a cache hit, never a second
/// round trip; PassageNode.Text is a zero-cost property read) -- so a
/// failed chapter fetch degrades to VerseTextSection's own pre-M-D4 plain
/// text, never a broken or blank focal section.
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

        // See this class's own doc comment -- fail-soft, independent of the
        // compactText fetch above; empty (never thrown past this point)
        // just means VerseTextSection's own markup falls back to compactText.
        List<VerseOut> focalVerses;
        try
        {
            var chapterOut = await api.Chapter(book, chapter);
            focalVerses = chapterOut.Verses.Where(cv => cv.Verse >= focalFrom && cv.Verse <= focalTo).ToList();
        }
        catch (Exception)
        {
            focalVerses = new List<VerseOut>();
        }

        RenderFragment fragment = builder =>
        {
            builder.OpenComponent<Components.VerseTextSection>(0);
            builder.AddAttribute(1, "Book", book);
            builder.AddAttribute(2, "Chapter", chapter);
            builder.AddAttribute(3, "FocalFromVerse", focalFrom);
            builder.AddAttribute(4, "FocalToVerse", focalTo);
            builder.AddAttribute(5, "CompactText", compactText);
            builder.AddAttribute(6, "FocalVerses", (IReadOnlyList<VerseOut>)focalVerses);
            builder.AddAttribute(7, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("verse-text", fragment);
    }
}

/// <summary>
/// Batch R requirement 3(b), rebuilt Batch F2 (6-ARCH + requirement 6): the
/// existing <c>GET /api/xrefs/{sref}</c> list, rendered INLINE (no
/// <c>popover-chip-xrefs</c> toggle press -- see VerseNode/PassageNode's
/// own <c>ExploreAsync</c> comments for the retired chip) for BOTH Verse
/// (unconditional fetch, same as before) and Passage (conditional -- absent
/// whenever the span has zero cross-references). Each entry is explorable
/// via the SHARED <c>PassageList</c> component (6-ARCH: "same underlying
/// data structure as the hover menu... reuse the bits that we have") --
/// sequential verses within one target's own span render as ONE passage
/// entry with its own FULL text (fetched via the existing chapter/LRU-cache
/// mechanism, <see cref="CanonRef.TargetSpan"/>/<see cref="Explore.PassageListVerse"/>,
/// rather than <see cref="CrossRefOut.Preview"/>'s own first-verse-only
/// text), never N separate verse rows.
///
/// Requirement 6 (truncation): capped at 3 entries when xrefs is the ONLY
/// context section present, 2 when any OTHER context section (Batch F's
/// own catechism seam today; any future provider automatically) also
/// resolved -- <see cref="IPopoverSectionContext.OtherContextSectionCount"/>
/// is read INSIDE the render fragment (at render time, after every sibling
/// provider has already resolved), never captured during this method's own
/// concurrent <c>ResolveAsync</c> call. A down-arrow (<c>xrefs-more</c>)
/// reveals the rest; fewer entries than the cap means no arrow at all
/// (conditional presence, per <c>PassageList.razor</c>'s own rule).
///
/// Batch M-D2 (owner's cross-reference superscript directive, ENTRY-POINT
/// PARAMETER -- CAP RECONCILIATION, owner decree: "do not silently break
/// F2... a parameter on the one abstraction, NOT a second interface"):
/// <see cref="IPopoverSectionContext.XrefEntryPoint"/>, read the SAME way
/// (render time, inside this closure) as <see cref="IPopoverSectionContext.OtherContextSectionCount"/>
/// immediately above, OVERRIDES the F2 ternary entirely when true -- 3
/// initial entries UNCONDITIONALLY (the owner's own words, "shows 3
/// explorable verses to start"), regardless of whether catechism or any
/// other context section is also present. F2's own 3-vs-2 rule governs the
/// GENERAL (non-entry-point) popover UNCHANGED -- this is the reconciled
/// parameter, not a fork: one `Cap` expression, one component, one section
/// provider. Both branches' own "3" reads
/// <see cref="EdgeSectionRegistry.Cites"/>'s own <c>InitialClamp</c> -- one
/// constant, not two coincidentally-equal literals.
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

        // Resolve each target's own FULL member-verse text. Same-chapter
        // targets (the overwhelming majority) fetch their whole chapter via
        // the existing LRU-cached AtlasClient.Chapter (several targets
        // sharing a chapter cost exactly one fetch); a cross-chapter/book
        // target (CanonRef.TargetSpan returns null -- rare, see its own doc
        // comment) falls back to its own first-verse preview text, the
        // pre-existing behavior for that edge case.
        var spans = xrefs.Select(x => (Xref: x, Span: CanonRef.TargetSpan(x.Target))).ToList();
        var chapterKeys = spans.Where(s => s.Span is not null).Select(s => (s.Span!.Value.Book, s.Span.Value.Chapter)).Distinct().ToList();
        var chapters = new Dictionary<(string, int), ChapterOut>();
        try
        {
            var fetched = await Task.WhenAll(chapterKeys.Select(k => api.Chapter(k.Item1, k.Item2)));
            foreach (var (key, chapter) in chapterKeys.Zip(fetched))
            {
                chapters[key] = chapter;
            }
        }
        catch (Exception)
        {
            // graceful degrade -- every target below falls back to its own preview text
        }

        var units = new List<PassageSourceUnit>();
        foreach (var (x, span) in spans)
        {
            if (span is { } s && chapters.TryGetValue((s.Book, s.Chapter), out var chapter))
            {
                var verses = new List<PassageListVerse>();
                for (var v = s.FromVerse; v <= s.ToVerse; v++)
                {
                    // M-D4 fix round 1 (R-M1): the whole VerseOut row, not
                    // just its own .Text -- Places/Persons were already
                    // sitting in this SAME already-fetched chapter, unread.
                    var cv = chapter.Verses.FirstOrDefault(cv => cv.Verse == v);
                    if (cv is not null)
                    {
                        verses.Add(new PassageListVerse($"{s.Book}.{s.Chapter}.{v}", cv.Text, Places: cv.Places, Persons: cv.Persons, WordsOfChrist: cv.WordsOfChrist));
                    }
                }
                if (verses.Count > 0)
                {
                    units.Add(new PassageSourceUnit(verses));
                    continue;
                }
            }
            units.Add(new PassageSourceUnit(new[] { new PassageListVerse(CanonRef.FirstVerseOf(x.Target), x.Preview) }));
        }

        RenderFragment body = builder =>
        {
            var seq = 0;

            // P3 (owner, verbatim, 2026-08-23: "Also the xrefs block has no
            // title. just give it a generic Cross References title") -- the
            // SAME shared small-caps eyebrow every other section title in
            // this popover platform uses (THE SMALL CATECHISM/EVENT/
            // PARALLEL ACCOUNTS/PARALLELS precedent), not a new one.
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "catechism-section-heading");
            builder.AddAttribute(seq++, "data-testid", "xrefs-section-heading");
            builder.AddContent(seq++, "Cross References");
            builder.CloseElement();

            builder.OpenComponent<Components.PassageList>(seq++);
            builder.AddAttribute(seq++, "Units", (IReadOnlyList<PassageSourceUnit>)units);
            builder.AddAttribute(seq++, "RefTestIdPrefix", "xref-item");
            builder.AddAttribute(seq++, "Cap", ctx.XrefEntryPoint ? EdgeSectionRegistry.Cites.InitialClamp : (ctx.OtherContextSectionCount > 0 ? 2 : EdgeSectionRegistry.Cites.InitialClamp));
            builder.AddAttribute(seq++, "MoreTestId", "xrefs-more");
            builder.AddAttribute(seq++, "CollapseTestId", "xrefs-collapse");
            builder.AddAttribute(seq++, "RevealNoun", "cross-references");
            // A real, live-caught regression (reader.spec.ts READ-3, found by
            // the full pre-existing suite): restores the pre-Batch-F2 click
            // contract -- every xref-item pushes a VerseNode at the target's
            // own FIRST verse, regardless of whether its preview text spans
            // more than one verse (~25% of real targets do). See
            // PassageList.razor's own ExploreAsVerse doc comment.
            builder.AddAttribute(seq++, "ExploreAsVerse", true);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            // G2-m2: the ONE PassageList consumer this batch wires for
            // Ctrl/Cmd-click -- see that parameter's own doc comment.
            builder.AddAttribute(seq++, "OnToggleSelect", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.ToggleSelectAsync(n)));
            builder.CloseComponent();
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
/// (curated) display name -- "The First Commandment", "What Does Baptism
/// Give or Profit?" -- clicking one pushes a <see cref="CatechismNode"/> (the SAME
/// drill-in <see cref="IPopoverSectionContext.PushAsync"/> mechanism every
/// other section-native explorable row already uses). Conditional presence:
/// a verse/passage citing nothing shows no section at all (no placeholder
/// text), same rule <see cref="CrossRefsSection"/> already follows.
///
/// Batch F2 requirement 4 ("verse -&gt; catechism lookup now returns
/// question-level hits"): a row whose own <see cref="CatechismRefDto.Question"/>
/// is present reads "&lt;Item&gt; — &lt;Question title&gt;" (e.g. "The First
/// Commandment — God the Holy Trinity"); a bare item-level hit (no
/// question, Luther's own embedded citation) keeps the plain item name,
/// unchanged since Batch F. The SAME item can legitimately appear more than
/// once (via two different questions, or a question plus the bare
/// citation) -- <c>catechism-item-{ID}</c> stays the testid for the FIRST
/// occurrence of a given id (so every existing single-occurrence assertion,
/// e.g. Baptism's own items, is untouched); a SECOND+ occurrence of the
/// SAME id gets a numbered suffix (<c>catechism-item-{ID}--q2</c>,
/// <c>--q3</c>, ...) so every row still has its own unique, addressable
/// testid rather than colliding.
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

            // M-D3 (U2/U6, owner: "Catechism defaults to 2 shown + U2
            // mechanics"): CatechismList.razor owns the actual rendering +
            // reveal state (a genuine component instance, unlike this
            // stateless provider) -- see that component's own doc comment.
            builder.OpenComponent<Components.CatechismList>(seq++);
            builder.AddAttribute(seq++, "Items", items);
            builder.AddAttribute(seq++, "Cap", CatechismDefaultCap);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("catechism", body);
    }

    // Owner, verbatim (progress.md, U6): "Catechism defaults to 2 shown."
    private const int CatechismDefaultCap = 2;
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
/// truncated preview) and explorable: clicking one pushes a fresh
/// <see cref="VerseNode"/> (or <see cref="PassageNode"/> for a grouped
/// passage entry), so onward navigation keeps working exactly as it does
/// everywhere else in this app (verse -&gt; catechism -&gt; proof verse -&gt; its
/// OWN cross-references -&gt; ..., requirement 4 verbatim). Conditional
/// presence: absent for an item with zero curated proof verses.
///
/// Batch F2, 6-ARCH: rebuilt on the shared <see cref="Components.PassageList"/>
/// -- sequential proof verses now display as ONE passage entry, never N
/// separate rows (a real, common case since a range citation like "EXO.20.5-6"
/// expands to individual verses on the wire). <c>CatechismItemDetail.Verses</c>
/// is split into contiguous same-<see cref="CatechismProofVerseDto.Question"/>
/// runs FIRST (item-level verses -- question null, Luther's own embedded
/// citation, always listed first -- then each question's own verses, in
/// curated order) -- each run becomes its own <see cref="PassageSourceUnit"/>,
/// so passage-grouping never silently spans two different questions' worth
/// of proof text, and each block's own question title renders as a caption
/// (requirement 4's own "if cheap, highlight/deep-link the question
/// context"). No cap -- items rarely have enough proof verses to need one.
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

        // M-D4 fix round 1 (R-M1, review Important-1): unlike CrossRefsSection/
        // VerseTextResolver, this provider's own source (CatechismProofVerseDto,
        // curated citation data) never carried Places/Persons at all -- it is
        // not ChapterOut-sourced the way every OTHER PassageListVerse producer
        // already is. Fail-soft, own try/catch (independent of the DetailAsync
        // fetch above, the house pattern): fetches each proof verse's own
        // chapter (GET /api/chapter/{cref}, LRU-cached -- the SAME endpoint
        // VerseTextSectionProvider's own compact-focus extension already
        // established this exact pattern for) and reads Places/Persons off
        // the matching VerseOut row. A failed/missing lookup just leaves that
        // verse's own mention data absent -- MentionText.razor already treats
        // an empty pair as "resolved, attests nothing," the same honest
        // degrade every other mention-aware surface in this app has.
        var mentionData = new Dictionary<string, VerseOut>();
        try
        {
            var chapterKeys = detail.Verses.Select(v => CanonRef.ParseVerse(v.Vref)).Select(p => (p.Book, p.Chapter)).Distinct().ToList();
            var fetched = await Task.WhenAll(chapterKeys.Select(k => api.Chapter(k.Book, k.Chapter)));
            foreach (var (key, chapterOut) in chapterKeys.Zip(fetched))
            {
                foreach (var cv in chapterOut.Verses)
                {
                    mentionData[$"{key.Book}.{key.Chapter}.{cv.Verse}"] = cv;
                }
            }
        }
        catch (Exception)
        {
            // fail soft -- proof verses still render, just without mention links this time
        }

        var units = new List<PassageSourceUnit>();
        List<PassageListVerse>? currentGroup = null;
        string? currentQuestion = null;
        foreach (var v in detail.Verses)
        {
            if (currentGroup is null || v.Question != currentQuestion)
            {
                if (currentGroup is not null)
                {
                    units.Add(new PassageSourceUnit(currentGroup, currentQuestion));
                }
                currentGroup = new List<PassageListVerse>();
                currentQuestion = v.Question;
            }
            var mention = mentionData.GetValueOrDefault(v.Vref);
            currentGroup.Add(new PassageListVerse(v.Vref, v.Text, Places: mention?.Places, Persons: mention?.Persons, WordsOfChrist: mention?.WordsOfChrist));
        }
        if (currentGroup is not null)
        {
            units.Add(new PassageSourceUnit(currentGroup, currentQuestion));
        }

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "catechism-section-heading");
            builder.AddAttribute(seq++, "data-testid", "catechism-section-heading");
            builder.AddContent(seq++, "THE SCRIPTURES");
            builder.CloseElement();

            builder.OpenComponent<Components.PassageList>(seq++);
            builder.AddAttribute(seq++, "Units", (IReadOnlyList<PassageSourceUnit>)units);
            builder.AddAttribute(seq++, "RefTestIdPrefix", "catechism-verse");
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("catechism-scriptures", body);
    }
}

/// <summary>
/// Batch R requirement 3 named this registry slot THE SEAM for a future
/// Theographic place description (same treatment as
/// <see cref="CatechismSeamSection"/>, one registry slot earlier -- PLACE's
/// own ordering: description, dates, blurb, events) and left it a no-op
/// stub pending that content. Batch E3 fills it -- with requirement 2's
/// quiet provenance note, not the originally-envisioned description (owner
/// bug report 2026-08-20, root cause: a map label showing the Theographic
/// canonical name where the KJV text uses a different word entirely). "The
/// canonical/Theographic name appears at most ONCE, quietly, inside the
/// place popover as provenance" -- decisive display (map/hover-card/
/// popover title all show the SAME resolved KJV name, per NAME-1/the
/// scene's own `display_name`) never loses the ORIGINAL name entirely; it
/// just moves it here, one quiet line, non-interactive, immediately under
/// the title. Conditional presence, same idiom every other section here
/// follows: <see cref="PlaceDetail.CanonicalName"/> is `null` whenever this
/// place's displayed name already IS its canonical name (server-decided --
/// <c>resolve_display_name_and_canonical</c> only ever returns `Some` when
/// a curated KJV alias is the reason the two differ, never for a curated
/// period-history rename, e.g. Luz/Bethel, which is itself already
/// KJV-accurate for its own era and has nothing to disclose) -- this
/// provider trusts that server-side decision rather than re-deriving it
/// from a string comparison against <see cref="IExplorable.Title"/>
/// client-side (the "no client-side rename map" rule applies to this
/// comparison too, not just the alias resolution itself). A future Batch P
/// place description, if authored, would need its own registry slot.
/// </summary>
public sealed class PlaceDescriptionSection : IPopoverSectionProvider
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
            detail = await place.DetailAsync(api); // memoized -- shared with PlaceDatesSection/PlaceBlurbSection/PlaceEventsSection
        }
        catch (Exception)
        {
            return null;
        }

        if (detail.CanonicalName is not { } canonical)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "popover-meta");
            builder.AddAttribute(seq++, "data-testid", "popover-place-canonical-name");
            builder.AddContent(seq++, $"Known in modern atlases as {canonical}.");
            builder.CloseElement();
        };
        return new PopoverSection("place-description", body);
    }
}

/// <summary>
/// Batch R requirement 3, rebuilt Batch F2 requirement 6b (user direction
/// 2026-08-20: "on the established/destroyed buttons just display
/// verses/passages how we do on every other hover menu... rather than the
/// stupid buttons i have to click to see"): established/destroyed, with
/// their supporting verses/passages rendered INLINE, immediately -- no
/// click-to-reveal step. The date LABEL itself is no longer a button that
/// gates the verses behind a YearNode push (that "reveal button" role is
/// retired, per the requirement's own wording) -- it is now a plain,
/// non-interactive instrument-face line, same house treatment
/// PlaceCard.razor's own established/destroyed line already uses one hop
/// further OUT. The verses/passages themselves are what stay explorable
/// ("the refs themselves stay explorable entries (click a ref -&gt; its
/// verse node, as everywhere)"), rendered via the SHARED
/// <see cref="Components.PassageList"/> component (6-ARCH), capped at 2
/// passage entries per date (est and dest each -- "the place popover always
/// has sibling sections," so this cap is unconditional, not context-
/// dependent the way requirement 6's xref cap is) with the same down-arrow
/// reveal/up-arrow snap-back language. Conditional presence: absent when
/// this place has neither claim curated.
/// </summary>
public sealed class PlaceDatesSection : IPopoverSectionProvider
{
    private const int SupportingVersesCap = 2;

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

        // Both dates' own supporting verses resolve concurrently (independent
        // fetches, same "never serialize" rule this app follows throughout).
        var establishedVersesTask = established is { } est ? VerseTextResolver.ResolveAsync(api, est.Verses) : Task.FromResult(new List<PassageListVerse>());
        var destroyedVersesTask = destroyed is { } dest ? VerseTextResolver.ResolveAsync(api, dest.Verses) : Task.FromResult(new List<PassageListVerse>());
        await Task.WhenAll(establishedVersesTask, destroyedVersesTask);

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "div");
            builder.AddAttribute(seq++, "class", "popover-place-dates");
            if (established is { } est)
            {
                RenderDateRow(builder, ref seq, "established", "Established", est, establishedVersesTask.Result, ctx);
            }
            if (destroyed is { } dest)
            {
                RenderDateRow(builder, ref seq, "destroyed", "Destroyed", dest, destroyedVersesTask.Result, ctx);
            }
            builder.CloseElement();
        };
        return new PopoverSection("place-dates", body);
    }

    private static void RenderDateRow(
        RenderTreeBuilder builder, ref int seq, string testidSuffix, string label, DateClaimOut claim, List<PassageListVerse> verses, IPopoverSectionContext ctx)
    {
        var dateText = YearText.FormatClaim(claim.When.FromYear, claim.When.ToYear, claim.Note);

        builder.OpenElement(seq++, "div");
        builder.AddAttribute(seq++, "class", "popover-place-date");
        builder.AddAttribute(seq++, "data-testid", $"popover-place-date-{testidSuffix}");

        builder.OpenElement(seq++, "span");
        builder.AddAttribute(seq++, "class", "popover-place-date-label");
        builder.AddContent(seq++, label);
        builder.CloseElement();

        builder.OpenElement(seq++, "span");
        builder.AddAttribute(seq++, "class", "popover-place-date-value");
        builder.AddContent(seq++, dateText);
        builder.CloseElement();
        builder.CloseElement();

        if (verses.Count > 0)
        {
            var units = new PassageSourceUnit[] { new(verses) };
            builder.OpenComponent<Components.PassageList>(seq++);
            builder.AddAttribute(seq++, "Units", (IReadOnlyList<PassageSourceUnit>)units);
            builder.AddAttribute(seq++, "RefTestIdPrefix", $"popover-place-date-{testidSuffix}-verse");
            builder.AddAttribute(seq++, "Cap", SupportingVersesCap);
            builder.AddAttribute(seq++, "MoreTestId", $"popover-place-date-{testidSuffix}-more");
            builder.AddAttribute(seq++, "CollapseTestId", $"popover-place-date-{testidSuffix}-collapse");
            builder.AddAttribute(seq++, "RevealNoun", "verses");
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        }
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
/// M-D1 requirement 4 (TRUNCATION AUDIT): the row list itself is now
/// CAPPED (<see cref="Components.PlaceEventsList"/>, cap 10, down-arrow
/// reveal) -- a real, live-verified gap this batch found and fixed
/// (Jerusalem alone real-carries 236 located-at events, unbounded by any
/// time window, previously rendered with no cap or disclosure at all).
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
        // M-D1 requirement 4 (TRUNCATION AUDIT): a real, live-verified gap
        // -- this list previously had NO cap at all (Jerusalem alone: 236
        // located-at events across the whole atlas). Delegated to a real
        // component (PlaceEventsList.razor) rather than a hand-built
        // RenderFragment here, mirroring VerseTextSectionProvider's own
        // established "a provider instance is shared/static and cannot own
        // per-popover expand state; a component can" precedent.
        RenderFragment body = builder =>
        {
            builder.OpenComponent<Components.PlaceEventsList>(0);
            builder.AddAttribute(1, "PlaceId", place.PlaceId);
            builder.AddAttribute(2, "PlaceName", placeName);
            builder.AddAttribute(3, "Events", events);
            builder.AddAttribute(4, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            // G2-m2: closes this section's own named gap in the parked report.
            builder.AddAttribute(5, "OnToggleSelect", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.ToggleSelectAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("place-events", body);
    }
}

/// <summary>
/// Batch PERI-1 (PRESENTATION CATEGORY LAW -- owner, verbatim: "NUN is not
/// an event. fix this error and others like it"): the exhaustive,
/// drift-failing kind-&gt;heading mapping <see cref="VerseEventMembershipSection"/>/
/// <see cref="VersePassageMembershipSection"/> both read, rather than each
/// hand-rolling its own ternary/ternary-of-a-ternary (the pre-PERI-1 shape
/// this class replaces, which silently defaulted an unrecognized kind to
/// "EVENT" instead of failing loud). Pinned directly by
/// client.Tests/Explore/EventMembershipHeadingTests.cs -- "event" -&gt;
/// "EVENT", "general" -&gt; "PASSAGE" (this project's own pre-existing
/// PASSAGE noun, reused per the register law -- see CONTRACT.md's own
/// PRESENTATION CATEGORY LAW section for the full cross-surface rule this
/// mapping is one instance of), anything else throws
/// <see cref="NotSupportedException"/> rather than defaulting -- the
/// "cheapest honest structural check" the batch brief's own conformance
/// corollary asks for.
/// </summary>
public static class EventMembershipHeading
{
    public static string For(string kind) => kind switch
    {
        "event" => "EVENT",
        "general" => "PASSAGE",
        _ => throw new NotSupportedException($"EventMembershipHeading.For: unrecognized Event::kind '{kind}'."),
    };
}

/// <summary>
/// Batch T requirement 3 ("verse popover: event membership replaces
/// prev/next"): the VERSE popover's own "EVENT" section -- one row per
/// EVENT-kind PASSAGE citing this verse (the pre-existing
/// <see cref="VerseDetail.Events"/> list, already fetched by
/// <see cref="VerseTextSectionProvider"/>/<see cref="CrossRefsSection"/>,
/// zero new network cost), each a small-caps, explorable title that pushes
/// a fresh <see cref="EventNode"/>. REPLACES Batch N's own verse-level
/// PRIOR/FOLLOWING sections (the owner, verbatim: "rather than putting the
/// next/previous event on every verse, add titles of events that
/// correspond to passages") -- traversal (PRIOR/FOLLOWING) now lives
/// entirely on the EVENT node itself, reached by clicking one of these
/// rows. Conditional presence: absent for a verse touching zero titled
/// events (the overwhelming majority of verses outside Gospels/Acts/the
/// curated narratives).
///
/// Batch PERI-1 (PRESENTATION CATEGORY LAW): SPLIT by
/// <see cref="VerseEventDto.Kind"/> -- this section now renders ONLY
/// `kind == "event"` rows, under the unchanged "EVENT" heading/testid/row
/// shape; a `kind == "general"` PASSAGE (a Psalm acrostic stanza, an
/// epistle outline pericope -- the owner's own PSA.119.105/GAL.1.8 repros)
/// renders instead under <see cref="VersePassageMembershipSection"/>, a
/// sibling section immediately below in the registry, never under this
/// one. The two sections share <see cref="RenderRows"/> (below) --
/// identical row markup/testid/click behavior either way, only the
/// grouping heading/PopoverSection testid differs.
/// </summary>
public sealed class VerseEventMembershipSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Verse";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not VerseNode v)
        {
            return null;
        }

        List<VerseEventDto> events;
        try
        {
            events = (await v.DetailAsync(api)).Events; // memoized -- shares VerseTextSectionProvider's own fetch
        }
        catch (Exception)
        {
            return null;
        }

        var dated = events.Where(e => e.Kind == "event").ToList();
        if (dated.Count == 0)
        {
            return null;
        }

        return new PopoverSection("event-membership", RenderRows("event", dated, ctx));
    }

    /// Shared by <see cref="VersePassageMembershipSection"/> below -- ONE
    /// row-rendering implementation for both sibling sections (identical
    /// button/testid/click/quiet-styling shape either way; only the
    /// heading text and the PopoverSection's own testid differ, both
    /// derived from <paramref name="kind"/> via
    /// <see cref="EventMembershipHeading"/> above).
    internal static RenderFragment RenderRows(string kind, IReadOnlyList<VerseEventDto> events, IPopoverSectionContext ctx) => builder =>
    {
        var seq = 0;
        builder.OpenElement(seq++, "p");
        builder.AddAttribute(seq++, "class", "catechism-section-heading"); // the SAME house small-caps eyebrow every section-registry heading shares (CATECH-1/NARRATIVE-1/DELTA-1) -- not a fifth copy
        builder.AddAttribute(seq++, "data-testid", "event-section-heading");
        builder.AddContent(seq++, EventMembershipHeading.For(kind));
        builder.CloseElement();

        foreach (var e in events)
        {
            var id = e.Id; // local copies -- captured per-row by the onclick closure below
            var label = e.Label;
            var rowKind = e.Kind; // fix round 1 (S-1a/Q-1a): passed to EventNode's own knownKind below -- see that constructor param's own doc comment
            // Batch HOTFIX-4 requirement 6 (AFFORDANCE HONESTY): a
            // general-kind event is NOT part of time traversal (req 2)
            // -- its own row here must not look like a dated event's
            // (which DOES traverse, after req 1). `.explorable-quiet`
            // REPLACES `.explorable` (never both) -- same class,
            // everywhere a non-traversable node's own identity renders,
            // per that class's own app.css comment. Unchanged by PERI-1
            // (this section's own events are now single-kind, but the
            // per-row check stays -- cheaper than threading `kind` through
            // a second parameter, and correct either way).
            var explorableClass = e.Kind == "general" ? "explorable-quiet" : "explorable";
            builder.OpenElement(seq++, "button");
            builder.AddAttribute(seq++, "type", "button");
            builder.AddAttribute(seq++, "class", $"popover-event-row popover-event-row-button {explorableClass}");
            builder.AddAttribute(seq++, "data-testid", $"verse-event-{id}");
            builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new EventNode(id, label, rowKind))));
            builder.AddContent(seq++, label);
            builder.CloseElement();
        }
    };
}

/// <summary>
/// Batch PERI-1 (PRESENTATION CATEGORY LAW): the VERSE popover's own
/// "PASSAGE" section -- <see cref="VerseEventMembershipSection"/>'s own
/// sibling, header comment above, restricted to `kind == "general"` rows
/// (a dateless pericope/literary-structure PASSAGE) -- NEVER rendered
/// under the "EVENT" heading. Conditional presence: absent for a verse
/// touching zero general-kind passages (the overwhelming majority --
/// PSA.119.105/GAL.1.8 are the owner's own two named exceptions).
/// </summary>
public sealed class VersePassageMembershipSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Verse";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not VerseNode v)
        {
            return null;
        }

        List<VerseEventDto> events;
        try
        {
            events = (await v.DetailAsync(api)).Events; // memoized -- shares VerseEventMembershipSection's/VerseTextSectionProvider's own fetch
        }
        catch (Exception)
        {
            return null;
        }

        var general = events.Where(e => e.Kind == "general").ToList();
        if (general.Count == 0)
        {
            return null;
        }

        return new PopoverSection("passage-membership", VerseEventMembershipSection.RenderRows("general", general, ctx));
    }
}

/// <summary>
/// Batch T requirement 4: an <see cref="EventNode"/>'s own date + place(s) --
/// "title + traditional date + place(s) (each explorable -- place opens the
/// place node; the date line quiet, with ref_note provenance on hover or a
/// quiet note)," the brief verbatim. ONE section (not two) since both are
/// small, header-adjacent facts about the SAME subject, rendered together
/// immediately below the popover's own title.
///
/// Batch T2 (general-kind PASSAGEs): a `Kind == "general"` passage has no
/// defensible date (<see cref="EventDetail.When"/> is `null`, never the
/// server's own internal placeholder -- see that DTO's doc comment) and,
/// by construction, no places either -- "do not fabricate a date/place"
/// extends to this section: the date line renders ONLY when
/// <c>detail.When</c> is present, and the whole section is ABSENT
/// (returns `null`, same conditional-presence idiom every other
/// zero-content section in this file already follows -- e.g.
/// <see cref="VerseEventMembershipSection"/> above) when there is neither
/// a date NOR any places to show.
/// </summary>
/// <summary>
/// M-D3 owner ruling ("MORNING ADDRESS", 2026-08-23), decision 6 (U1): an
/// EVENT node's own FOCUS section -- date, place(s), AND (owner UI spec,
/// near-verbatim, progress.md: "immediately below focus, left arrow w/
/// PRIOR event name, right arrow w/ FOLLOWING (narrative); explorable;
/// hover = normal focus+frontier; those foci truncated to ONE VERSE") the
/// narrative prior/following traversal -- now rendered INSIDE this one
/// section (a compact flanking row per qualifying narrative), not as two
/// separate "PRIOR EVENT"/"FOLLOWING EVENT" registry sections underneath
/// PARALLEL ACCOUNTS. RETIRES Batch N/T's own EventPriorSection/
/// EventFollowingSection/NarrativeDirectionSection -- their per-narrative
/// fetch + same-name disambiguation logic lives on here (see
/// <c>occurrences</c> below); their own big-headed, multi-verse-list
/// rendering does not ("one-verse foci": only the adjacent event's OWN
/// first vref across its VerseGroups is ever resolved, never the whole
/// group). LEFT/RIGHT, never "prior in time" -- this is strictly
/// NARRATIVE (succession-relation) order, the doubly-linked-list the owner
/// named; the GLOBAL CHRONOLOGICAL adjacency
/// (<see cref="EventChronologySection"/>, below, its own separate,
/// headed "CHRONOLOGY" block) is out of U1's own stated scope ("never
/// chronological").
/// "hover = normal focus+frontier": true of the ARROW ROW itself (the
/// ink-wash `.explorable` state every explorable element in this popover
/// platform gets) -- TRAV-1 (controller decision 4) layers a bespoke
/// DWELL-hover on top, entirely inside <see cref="Components.ArrowNav"/>
/// (a sustained hover reveals a transient verse-text peek; a quick pass
/// stays exactly this "normal focus+frontier" ink-wash and nothing more).
/// </summary>
/// <summary>
/// M-D4 fix round 1/P4 (owner, live off demo36, verbatim: "we straight up
/// should not have [the verse text]. you get that when you traverse.") --
/// retires U1's own "those foci truncated to ONE VERSE" caption entirely
/// (no count, no content preview of ANY kind in the arrow affordance
/// itself; the click is what YIELDS the event, not what the affordance
/// previews). In its place, each arrow now carries a static small-caps
/// role caption beneath it -- "PRIOR EVENT"/"FOLLOWING EVENT" -- naming
/// the DIRECTION only, never the destination's own content. The event
/// NAME (inside the button, unchanged) is the only per-event text left;
/// <see cref="Components.ArrowNav"/>'s own doc comment has the rest.
/// </summary>
/// <summary>
/// CHRONO-MERGE-1 (owner NOD 2026-08-24: "put chronology up top... nix the
/// narrative thing from hover menu"; POPOVER-LAW-1's own first
/// application): RETIRES the narrative prior/following nav this section
/// used to fold in (M-D3/U1, immediately above) WHOLE -- controller
/// measurement found 72% of narrative rows byte-identical to the SAME
/// event's own global-timeline row (<see cref="EventChronologySection"/>),
/// so a dedicated per-narrative nav duplicated the Chronology block's own
/// arrows for the overwhelming majority of events; the 28% that genuinely
/// differ now surface as <see cref="EventChronologySection"/>'s own
/// divergence-only story-thread line instead (that class's own doc
/// comment has the full rule) -- ONE traversal block survives, not two.
/// This section is left holding exactly what its own name always said:
/// an EVENT's date + place(s), nothing else. `RenderArrowNav` below stays
/// -- <see cref="EventChronologySection"/>'s own arrows (block AND, new
/// this batch, inline story-thread legs) are its one remaining caller.
/// </summary>
public sealed class EventDateAndPlacesSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Event";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not EventNode ev)
        {
            return null;
        }

        EventDetail detail;
        try
        {
            detail = await ev.DetailAsync(api); // memoized -- shared with EventWitnessesSection and EventChronologySection's own map-focus-sync read
        }
        catch (Exception)
        {
            return null; // CHRONO-MERGE-1: date/places is this section's ONLY content now -- a failed fetch leaves nothing to fail soft INTO
        }

        if (detail.When is null && detail.Places.Count == 0)
        {
            return null; // conditional presence: nothing this section can honestly show
        }

        RenderFragment body = builder =>
        {
            var seq = 0;

            if (detail.When is { } when)
            {
                var dateText = YearText.FormatRange(when.FromYear, when.ToYear);
                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "popover-meta event-date-line");
                builder.AddAttribute(seq++, "data-testid", "event-date");
                if (detail.RefNote is { } refNote)
                {
                    builder.AddAttribute(seq++, "title", refNote); // quiet, hover-revealed provenance -- a native tooltip, no extra affordance/click needed
                }
                builder.AddContent(seq++, dateText);
                builder.CloseElement();
            }

            if (detail.Places.Count > 0)
            {
                builder.OpenElement(seq++, "div");
                builder.AddAttribute(seq++, "class", "popover-event-places");
                builder.AddAttribute(seq++, "data-testid", "event-places");
                foreach (var p in detail.Places)
                {
                    var placeId = p.Id; // local copies -- captured per-row by the onclick closure below
                    var placeName = p.Name;
                    builder.OpenElement(seq++, "button");
                    builder.AddAttribute(seq++, "type", "button");
                    builder.AddAttribute(seq++, "class", "popover-event-place explorable");
                    builder.AddAttribute(seq++, "data-testid", $"event-place-{placeId}");
                    builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new PlaceNode(placeId, placeName))));
                    builder.AddContent(seq++, placeName);
                    builder.CloseElement();
                }
                builder.CloseElement();
            }
        };
        return new PopoverSection("event-date-places", body);
    }

    // Opens a real <see cref="Components.ArrowNav"/> component instance
    // (never raw elements built by hand here anymore) from this
    // RenderTreeBuilder body, the SAME "open a real component from
    // imperative RenderTreeBuilder code" pattern this file already
    // established for <see cref="Components.PassageList"/>. TRAV-1
    // (controller decision 3, "same arrow-traversal component"): this is
    // that reuse, realized -- see ArrowNav.razor's own doc comment for why
    // the rendering moved out of a static helper into a genuine component
    // (decision 4's dwell-hover peek needs per-arrow state that survives
    // across renders, which a RenderFragment closure cannot hold).
    //
    // CHRONO-MERGE-1: this section's own former narrative-nav loop (the
    // "called twice per row" caller this comment used to name) is retired
    // -- <see cref="EventChronologySection"/> is now the ONLY caller,
    // twice for its own single GLOBAL block row (block mode, unchanged)
    // and again, up to twice per diverging narrative, for the story-thread
    // line's own inline leg affordances (<paramref name="inline"/> true,
    // <paramref name="inlinePrefixText"/> the sentence-level word a
    // following-direction leg needs -- see ArrowNav.razor's own doc
    // comment for the full Inline-mode story). Both trailing parameters
    // default to the block-mode shape so neither existing call site needed
    // to change.
    internal static void RenderArrowNav(RenderTreeBuilder builder, ref int seq, IPopoverSectionContext ctx, string direction, string eventTestIdPrefix, string roleTestIdPrefix, string idSuffix, NarrativeAdjacentEventDto? adjacent, string glyph, bool inline = false, string? inlinePrefixText = null)
    {
        builder.OpenComponent<Components.ArrowNav>(seq++);
        builder.AddAttribute(seq++, "Direction", direction);
        builder.AddAttribute(seq++, "EventTestIdPrefix", eventTestIdPrefix);
        builder.AddAttribute(seq++, "RoleTestIdPrefix", roleTestIdPrefix);
        builder.AddAttribute(seq++, "IdSuffix", idSuffix);
        builder.AddAttribute(seq++, "Adjacent", adjacent);
        builder.AddAttribute(seq++, "Glyph", glyph);
        builder.AddAttribute(seq++, "Inline", inline);
        builder.AddAttribute(seq++, "InlinePrefixText", inlinePrefixText);
        builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
        builder.CloseComponent();
    }
}

/// <summary>
/// M-D3/U6 (extracted from the former inline body of
/// <see cref="EventWitnessesSection"/>, unchanged logic): resolves a
/// (possibly FILTERED) witness list into <see cref="PassageSourceUnit"/>s
/// with real KJV text, one unit per witness -- the shared step BOTH an
/// EVENT node's own "PARALLEL ACCOUNTS" (<see cref="EventWitnessesSection"/>,
/// every witness) and a VERSE node's own "PARALLELS"
/// (<see cref="VerseParallelsSection"/>, EVERY OTHER witness -- excluding
/// the one the current verse itself belongs to) need identically -- "one
/// component/behavior, parameterized -- never two implementations," the
/// SAME discipline U2's own RevealControls.razor follows for the reveal
/// mechanic, applied here to witness-resolution.
///
/// M-D4 fix round 1/P5 (owner, verbatim, of EventWitnessesSection's own
/// "PARALLEL ACCOUNTS": "we're wasting real estate... it's obvious where
/// they're coming from already"): units carry NO caption now. This
/// resolver used to book-display-name every unit (an extra
/// <see cref="AtlasClient.Books"/> read) on the reasoning that the
/// caption was "genuinely load-bearing" for EventWitnessesSection
/// specifically -- retired outright, since PassageList.razor's own
/// ref-label (e.g. "MRK.6.1-6") already names the book; a caption line
/// under it was always a second header for the same one fact.
/// <see cref="VerseParallelsSection"/>'s own O5 ruling (below) had
/// already reached the identical conclusion for "PARALLELS" one fix
/// round earlier via a per-call-site null-out -- this makes it true at
/// the SOURCE instead, so both consumers agree by construction, not by
/// two separate call sites each remembering to null a field.
/// </summary>
file static class WitnessUnitsResolver
{
    public static async Task<List<PassageSourceUnit>> ResolveAsync(AtlasClient api, IReadOnlyList<EventWitnessDto> witnesses)
    {
        var units = witnesses.Select(w =>
        {
            // Batch HOTFIX-4 requirement 7: GroupCount carries each
            // VerseGroup's own TRUE total (server-side `take(20)` cap,
            // scene::verse_groups_for) so a truncated witness group shows
            // the "+N more" signal instead of silently ending at 20.
            var verses = w.VerseGroups.SelectMany(g => g.Verses.Select(v => new PassageListVerse(v, "", g.Count))).ToList();
            return new PassageSourceUnit(verses);
        }).ToList();

        // The witnesses' own verse TEXT isn't on VerseGroup (ids only, same
        // as every other VerseGroup on this wire) -- resolve it the SAME way
        // every other section in this file resolves a curated verse list
        // (VerseTextResolver, the existing chapter fetch + LRU cache), then
        // re-pair it back onto each unit's own PassageListVerse list.
        List<PassageListVerse> resolvedFlat;
        try
        {
            var allVrefs = witnesses.SelectMany(w => w.VerseGroups.SelectMany(g => g.Verses)).ToList();
            resolvedFlat = await VerseTextResolver.ResolveAsync(api, allVrefs);
        }
        catch (Exception)
        {
            resolvedFlat = new List<PassageListVerse>();
        }
        // GroupBy + first-wins (not a raw ToDictionary) -- defensive against a
        // duplicate Vref across two witnesses, which server-side validation
        // (validate::run's own overlap check) already prevents for real
        // compiled data, but this is client code reading a network response,
        // not something to assume well-formed a second time.
        //
        // M-D4 fix round 1 (R-M1): keyed by the WHOLE resolved PassageListVerse
        // now, not just its own .Text -- VerseTextResolver.ResolveAsync
        // already carries Places/Persons (the same already-fetched ChapterOut
        // row), so re-pairing below must not silently re-drop them the way a
        // text-only dictionary would.
        var resolvedByVref = resolvedFlat.GroupBy(v => v.Vref).ToDictionary(g => g.Key, g => g.First());
        return units.Select(u => new PassageSourceUnit(
            // v.GroupCount carried through from the FIRST construction
            // above -- resolving text must never silently drop it.
            u.Verses.Select(v =>
            {
                var resolved = resolvedByVref.GetValueOrDefault(v.Vref);
                return new PassageListVerse(v.Vref, resolved?.Text ?? "", v.GroupCount, resolved?.Places, resolved?.Persons, resolved?.WordsOfChrist);
            }).ToList())).ToList(); // no Caption -- P5, see this class's own doc comment
    }
}

/// <summary>
/// Batch T requirement 4: "PARALLEL ACCOUNTS -- one short passage per
/// witness (Gospel name + passage ref), each clamped to 2 verses with
/// per-passage expand/collapse... Single-witness events show the one
/// passage (no 'parallel' framing when n=1 -- conditional presence)." One
/// <see cref="Components.PassageList"/> unit per witness (never merged --
/// PASSAGE-1's own "a source unit never blurs into its neighbor's," applied
/// here across Gospels). "Gospel name + passage ref" falls out of
/// PassageList's own ref-label ALONE (e.g. "MRK.6.1-6" already names the
/// book) -- <see cref="WitnessUnitsResolver"/>'s own doc comment has the
/// M-D4 fix round 1/P5 history of why a separate book-name Caption
/// retired instead of spelling the same book out a second time
/// underneath it. <see cref="Components.PassageList.ClampVerses"/>
/// (batch-n2 req 2, generalized here to every consumer of the shared
/// component) is what realizes "clamped to 2 verses... per-passage
/// expand/collapse."
/// </summary>
public sealed class EventWitnessesSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Event";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not EventNode ev)
        {
            return null;
        }

        EventDetail detail;
        try
        {
            detail = await ev.DetailAsync(api);
        }
        catch (Exception)
        {
            return null;
        }

        if (detail.Witnesses.Count == 0)
        {
            return null; // defensive -- scene::witnesses_for always synthesizes >=1 server-side, but never assume a network response is well-formed
        }

        List<PassageSourceUnit> units;
        try
        {
            units = await WitnessUnitsResolver.ResolveAsync(api, detail.Witnesses);
        }
        catch (Exception)
        {
            return null;
        }

        var multi = units.Count > 1;

        RenderFragment body = builder =>
        {
            var seq = 0;
            if (multi)
            {
                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "catechism-section-heading");
                builder.AddAttribute(seq++, "data-testid", "event-section-heading");
                builder.AddContent(seq++, "PARALLEL ACCOUNTS");
                builder.CloseElement();
            }

            builder.OpenComponent<Components.PassageList>(seq++);
            builder.AddAttribute(seq++, "Units", (IReadOnlyList<PassageSourceUnit>)units);
            builder.AddAttribute(seq++, "RefTestIdPrefix", "event-witness");
            builder.AddAttribute(seq++, "ClampVerses", 2);
            // M-D1 requirement 3 (span-not-echo, owner live report #4,
            // verbatim: "it also is completely redundant to just show the
            // verses associated with a container in the container's hover
            // box. we should just see the passage span."): a SINGLE-witness
            // container (the overwhelming majority -- every event this
            // batch does not curate parallel accounts for) shows its own
            // span line only, never an enumerated verse-list echo -- !multi
            // is exactly "exactly one witness" here (units.Count <= 1 by
            // construction once `detail.Witnesses.Count == 0` already
            // returned null above). A MULTI-witness event's own PARALLEL
            // ACCOUNTS list is UNCHANGED (every witness keeps its clamped,
            // expandable preview text -- "PARALLEL ACCOUNTS from OTHER
            // witnesses keep their clamped expandable passage text," the
            // requirement's own explicit distinction; the Crucifixion's
            // four-Gospel case stays exactly as rich as before).
            builder.AddAttribute(seq++, "SpanOnly", !multi);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection(multi ? "event-witnesses" : "event-witness", body);
    }
}

/// <summary>
/// M-D3/U6, owner verbatim order (progress.md): "Header / Verse (focus) /
/// Event / Parallels / Small Catechism / cross references LAST." A VERSE
/// (or PASSAGE) node's own quick peek at OTHER witnesses of an event it
/// belongs to -- "reading John 12, glance and see Matthew/Mark/Luke also
/// witnessed this," without a click into the EVENT node first. Reuses
/// <see cref="WitnessUnitsResolver"/> (the SAME resolve step
/// <see cref="EventWitnessesSection"/> uses), just fed a FILTERED witness
/// list -- every witness EXCEPT the one the current verse itself already
/// belongs to (there is no reason to preview a verse's own text back to
/// itself; the verse-text section above already shows it). Conditional
/// presence, doubly: absent when the verse cites no titled event at all
/// (the SAME `.Events` VerseEventMembershipSection already reads, memoized,
/// zero extra fetch), and absent per-event when that event turns out to
/// have no OTHER witness once the current one is excluded (a real
/// EventDetail fetch is still needed per event to know its own full
/// witness list -- VerseEventDto's own membership-list shape carries no
/// sibling-witness information). A verse touching >1 QUALIFYING event
/// (genuinely rare, but not impossible) renders one block per event, named
/// by that event's own label when there is more than one -- the SAME
/// "single entry needs no name, multiple entries each get named" rule
/// EventDateAndPlacesSection's own narrative nav (U1) already establishes.
///
/// O5 (owner live-preview correction, 2026-08-23, "parallels has double
/// headers... 1Ki.3.1-15 and 1 kings right below it... get rid of the
/// second header"): unlike <see cref="EventWitnessesSection"/>'s own
/// "PARALLEL ACCOUNTS" (which keeps <see cref="WitnessUnitsResolver"/>'s
/// own book-name Caption -- genuinely load-bearing there, telling several
/// Gospels apart at a glance), this section strips it to null before
/// handing units to <see cref="Components.PassageList"/> -- one header
/// (the ref-label, which already carries the book CODE) per entry, not two.
/// </summary>
public sealed class VerseParallelsSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind is "Verse" or "Passage";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        List<VerseEventDto> events;
        string ownVref;
        switch (node)
        {
            case VerseNode v:
                events = (await v.DetailAsync(api)).Events; // memoized -- shares VerseTextSectionProvider's/CrossRefsSection's own fetch
                ownVref = v.Title; // VerseNode's own doc comment: Title IS the vref
                break;
            case PassageNode p:
                // A passage's own event membership is read at its FIRST
                // verse -- the same "first verse anchors the locus"
                // convention CrossRefsSection/VersePersonsSection already
                // establish for a passage's own onward identity.
                ownVref = CanonRef.FirstVerseOf(p.Title);
                try
                {
                    events = (await api.Verse(ownVref)).Events;
                }
                catch (Exception)
                {
                    return null;
                }
                break;
            default:
                return null;
        }

        if (events.Count == 0)
        {
            return null; // conditional presence -- most verses cite no titled event at all
        }

        // One real fetch per candidate event (EventDetail carries the FULL
        // witness list; VerseEventDto's own slim membership shape does not)
        // -- run concurrently, same "don't serialize independent fetches"
        // discipline NarrativeDirectionSection's own retired resolver and
        // EventDateAndPlacesSection's own one-verse-focus resolve already
        // followed.
        EventDetail?[] details;
        try
        {
            details = await Task.WhenAll(events.Select(async e =>
            {
                try
                {
                    return await new EventNode(e.Id, e.Label).DetailAsync(api);
                }
                catch (Exception)
                {
                    return null;
                }
            }));
        }
        catch (Exception)
        {
            return null; // fail soft -- same graceful-degradation policy every other lazy fetch in this app follows
        }

        var qualifying = new List<(string Label, List<EventWitnessDto> OtherWitnesses)>();
        foreach (var (e, detail) in events.Zip(details))
        {
            if (detail is null)
            {
                continue;
            }
            var others = detail.Witnesses.Where(w => !w.VerseGroups.Any(g => g.Verses.Contains(ownVref))).ToList();
            if (others.Count > 0)
            {
                qualifying.Add((e.Label, others));
            }
        }

        if (qualifying.Count == 0)
        {
            return null; // conditional presence -- no cited event has any OTHER witness once this verse's own is excluded
        }

        List<List<PassageSourceUnit>> unitsPerEvent;
        try
        {
            unitsPerEvent = new List<List<PassageSourceUnit>>();
            foreach (var (_, others) in qualifying)
            {
                var units = await WitnessUnitsResolver.ResolveAsync(api, others);
                // O5 (owner live-preview correction, 2026-08-23, verbatim:
                // "parallels has double headers. for instance we have
                // 1Ki.3.1-15 and 1 kings right below it when focused on
                // 2ch.1.2. Get rid of the second header"): originally fixed
                // here with a per-call-site null-out of WitnessUnitsResolver's
                // own book-name Caption (PassageList.razor's own ref-label,
                // "1Ki.3.1-15," already carries the book CODE, so spelling
                // the same book out a second time right below it read as two
                // headers on one entry, not one). M-D4 fix round 1/P5
                // (owner: "we're wasting real estate... it's obvious where
                // they're coming from already") reached the SAME conclusion
                // for EventWitnessesSection's own "PARALLEL ACCOUNTS" -- once
                // BOTH consumers agreed no caption belongs on screen, the
                // null-out moved to the SOURCE (WitnessUnitsResolver's own
                // doc comment has that history); units arrive already
                // caption-free here, no per-call-site projection needed.
                unitsPerEvent.Add(units);
            }
        }
        catch (Exception)
        {
            return null;
        }

        var multiEvent = qualifying.Count > 1;

        RenderFragment body = builder =>
        {
            var seq = 0;
            for (var i = 0; i < qualifying.Count; i++)
            {
                var (label, _) = qualifying[i];
                var units = unitsPerEvent[i];

                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "catechism-section-heading");
                builder.AddAttribute(seq++, "data-testid", "event-section-heading");
                builder.AddContent(seq++, multiEvent ? $"PARALLELS — {label}" : "PARALLELS");
                builder.CloseElement();

                builder.OpenComponent<Components.PassageList>(seq++);
                builder.AddAttribute(seq++, "Units", (IReadOnlyList<PassageSourceUnit>)units);
                builder.AddAttribute(seq++, "RefTestIdPrefix", multiEvent ? $"verse-parallel-{Slugify(label)}" : "verse-parallel");
                builder.AddAttribute(seq++, "ClampVerses", 2);
                // Always the full clamped preview (never SpanOnly) -- unlike
                // EventWitnessesSection's own single-witness case, THIS
                // section only ever renders when there genuinely IS an
                // other witness to preview; "just the span, no text" would
                // defeat the section's own purpose (a quick peek at what
                // the parallel account actually says).
                builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
                builder.CloseComponent();
            }
        };
        return new PopoverSection("parallels", body);
    }

    // Same slug shape VersePersonsSection's own Slug helper already
    // establishes for a display label -> stable DOM-safe testid fragment.
    private static string Slugify(string label)
    {
        var chars = label.ToLowerInvariant().Select(c => char.IsLetterOrDigit(c) ? c : '-').ToArray();
        var slug = new string(chars);
        while (slug.Contains("--"))
        {
            slug = slug.Replace("--", "-");
        }
        return slug.Trim('-');
    }
}

/// <summary>
/// TRAV-1 (controller decisions 2+3, owner verbatim, progress.md: "the
/// prior and time and following in time basically get condensed into one
/// Chronological block with the arrow traversal that is separate from the
/// narrative block"). RETIRES Batch HOTFIX-4's own
/// EventTimelineDirectionSection/EventTimelinePriorSection/
/// EventTimelineFollowingSection whole (the former "PRIOR IN TIME"/
/// "FOLLOWING IN TIME" sections -- two separate headed rows, each with a
/// full verse-text preview inline via <see cref="Components.PassageList"/>,
/// per HOTFIX-4 requirement 1): this ONE section replaces both. "Traverse
/// them in essentially the same way" (the owner's own words) is realized
/// structurally, not just in spirit -- the SAME arrow-traversal component
/// the Narrative nav uses (<see cref="Components.ArrowNav"/>, opened via
/// <see cref="EventDateAndPlacesSection.RenderArrowNav"/>), reused here for
/// the GLOBAL chronological adjacency instead of narrative-leg adjacency.
/// P4's own "name only, no verse text in the arrow itself" rule is
/// satisfied by <c>ArrowNav</c> itself now (unchanged, not re-proven here)
/// -- the verse text this section's own predecessor used to show inline is
/// now decision 4's dwell-hover PEEK instead, never inline.
///
/// Two differences from the Narrative rows this reuses the SAME rendering
/// for: (1) exactly ONE row, always (the global timeline has one
/// prior/following pair, never "one block per qualifying narrative" --
/// no name-collision disambiguation needed, so `IdSuffix` is the fixed
/// literal `"global"`, never a real narrative id); (2) a quiet
/// "CHRONOLOGY" eyebrow heading names the block (the SAME shared
/// `catechism-section-heading`/`event-timeline-heading` classes the
/// retired "IN TIME" sections already used for their own headings) -- the
/// Narrative nav itself renders headerless, immediately below focus, by
/// M-D3/U1's own established design (unchanged by this batch); this
/// section is a brand-new, separate block, so it announces its own
/// identity, matching the owner's own "we have two sections: Narrative and
/// Chronology."
///
/// Conditional presence: renders whenever this event genuinely has a
/// `Timeline` position at all (i.e. it is dated -- `Timeline` is the wire
/// key OMITTED, not null, for a general-kind/unknown event, HOTFIX-4
/// requirement 2, unchanged) -- INCLUDING the degenerate case where BOTH
/// `Prior` and `Following` are independently absent (the atlas's own true
/// first-AND-last dated event, or the rare single-dated-event atlas):
/// `GraphService.temporal_neighbors` (service.rs) still returns `Some`
/// there (seeded from the chronology's own order, not merely from row
/// presence -- see that field's own doc comment), so this section
/// honestly renders a real Chronology position with two empty-placeholder
/// arrows, never silently omitting the block just because this one event
/// happens to have no neighbor on either side.
///
/// CHRONO-MERGE-1 (owner NOD 2026-08-24 -- the design question: "is there
/// a meaningful disjunction between chronology and narrative order? It's
/// looking like they're basically the same and we can nix the narrative
/// thing from hover menu and put chronology up top"; the nod, verbatim:
/// "yes I agree but just don't clutter with story line where story
/// doesn't exist"; POPOVER-LAW-1's own first application -- "we only pull
/// in anything if there's something non-redundant to pull"): this block
/// is now Batch T's ENTIRE surviving traversal surface -- the narrative
/// nav <see cref="EventDateAndPlacesSection"/> used to fold in (M-D3/U1)
/// is retired whole (that class's own doc comment), and this section
/// MOVES UP to occupy the vacated top position (<see cref="PopoverSectionRegistry"/>'s
/// own doc comment has the registration-order change) -- "Chronology,
/// always on top," the owner's own words.
///
/// A controller sweep of all 255 real narrative events found 72% of rows
/// byte-identical to this SAME event's own global-timeline row (a
/// dedicated per-narrative nav was pure duplication for those), 28%
/// genuinely diverging (always because a DIFFERENT narrative's own event
/// interleaves chronologically between two of THIS narrative's legs --
/// df_adullam's own following is a real, worked example: `david-flight`'s
/// next leg is df_keilah, but the global-timeline next event is
/// 1ch_ziklag_warriors, a Chronicles genealogy entry that happens to fall
/// between them in time). The STORY-THREAD line below is what survives of
/// the narrative nav -- ONLY for that 28%, "just don't clutter with story
/// line where story doesn't exist."
///
/// THE DIVERGENCE TEST (<see cref="Diverges"/>): a client-side id
/// comparison, per direction, of a `<see cref="NarrativePositionDto"/>`
/// row's own Prior/Following against THIS SAME event's `Timeline`
/// Prior/Following (both halves of the identical, already-memoized
/// `NarrativePositionsAsync` fetch -- one network call, zero new cost). A
/// null/absent narrative-side leg is the narrative's own first/last leg
/// -- a chain END, never a divergence (nothing to show for that
/// direction, the brief's own words) -- so `Diverges` short-circuits
/// false there regardless of what the timeline says. An event belonging
/// to >1 narrative (a real but, per the controller's own live sweep,
/// currently EMPTY case -- zero of the 255) renders one line per
/// DIVERGING narrative, stacked, the same "guard the rare case, common
/// case stays simple" discipline <see cref="EventDateAndPlacesSection"/>'s
/// own retired multi-narrative handling already established.
///
/// DISPLAY: one `.popover-meta` line per diverging narrative (the SAME
/// quiet-provenance register <see cref="PlaceDescriptionSection"/>'s own
/// "Known in modern atlases as..." line already established) reading
/// "in &lt;narrative&gt;: " followed by ONLY the diverging direction(s) --
/// `&lt;- &lt;prior leg&gt;` alone, `next -&gt; &lt;leg&gt;` alone, or both
/// joined by " &middot; " when BOTH directions diverge (pw_jerusalem_entry
/// is a real, worked example of the dual case: passion-week's own prior
/// AND following both differ from the global timeline's). The leg name(s)
/// are <see cref="Components.ArrowNav"/>'s own new `Inline` rendering mode
/// (<see cref="EventDateAndPlacesSection.RenderArrowNav"/>) -- the SAME
/// click-commits/dwell-peeks affordance the block arrows above give,
/// reusing the identical dwell timer/peek-fetch/placement-measurement
/// machinery (never re-derived) -- so a diverging leg is exactly as
/// traversable as a chronological one, just named inline rather than in
/// its own arrow button.
/// </summary>
public sealed class EventChronologySection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Event";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not INarrativeAware aware)
        {
            return null;
        }

        NarrativeEventPositionsResult positions;
        try
        {
            positions = await aware.NarrativePositionsAsync(api); // ONE fetch -- .Narrative feeds the story-thread line below, .Timeline the block arrows, memoized on the node instance either way
        }
        catch (Exception)
        {
            return null; // fail soft -- same graceful-degradation policy every other lazy fetch in this app follows
        }

        var timeline = positions.Timeline;
        if (timeline is null)
        {
            return null; // general-kind or unknown event -- NOT part of time traversal, this class's own doc comment
        }

        // CHRONO-MERGE-1: which of this event's own narrative membership(s),
        // if any, genuinely diverge from the SAME event's global-timeline
        // row -- see this class's own "THE DIVERGENCE TEST" doc paragraph.
        // A non-narrative event (positions.Narrative empty) or a
        // fully-agreeing one filters down to zero rows here, same as each
        // other -- both correctly render NO story-thread line at all,
        // never a hollow wrapper.
        var storyThreadRows = positions.Narrative
            .Where(p => Diverges(p.Prior, timeline.Prior) || Diverges(p.Following, timeline.Following))
            .ToList();

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "catechism-section-heading event-timeline-heading");
            builder.AddAttribute(seq++, "data-testid", "event-chronology-heading");
            builder.AddContent(seq++, "CHRONOLOGY");
            builder.CloseElement();

            builder.OpenElement(seq++, "div");
            builder.AddAttribute(seq++, "class", "popover-event-nav-list");
            builder.AddAttribute(seq++, "data-testid", "event-chronology");

            builder.OpenElement(seq++, "div");
            builder.AddAttribute(seq++, "class", "popover-event-nav-row");
            builder.AddAttribute(seq++, "data-testid", "event-chronology-row");

            // Arrows live in their OWN inner flex row -- same reason
            // EventDateAndPlacesSection's own (now-retired) narrative rows
            // kept this one level of nesting (space-between across exactly
            // these two items, nothing else sharing that flex line).
            builder.OpenElement(seq++, "div");
            builder.AddAttribute(seq++, "class", "popover-event-nav-arrows");
            EventDateAndPlacesSection.RenderArrowNav(builder, ref seq, ctx, "prior", "event-chrono-prior-event", "event-chrono-prior-label", "global", timeline.Prior, "◂");
            EventDateAndPlacesSection.RenderArrowNav(builder, ref seq, ctx, "following", "event-chrono-following-event", "event-chrono-following-label", "global", timeline.Following, "▸");
            builder.CloseElement(); // .popover-event-nav-arrows

            builder.CloseElement(); // .popover-event-nav-row
            builder.CloseElement(); // .popover-event-nav-list

            if (storyThreadRows.Count > 0)
            {
                builder.OpenElement(seq++, "div");
                builder.AddAttribute(seq++, "class", "popover-story-thread");
                builder.AddAttribute(seq++, "data-testid", "event-story-thread");
                foreach (var row in storyThreadRows)
                {
                    var priorDiverges = Diverges(row.Prior, timeline.Prior);
                    var followingDiverges = Diverges(row.Following, timeline.Following);

                    builder.OpenElement(seq++, "p");
                    builder.AddAttribute(seq++, "class", "popover-meta");
                    builder.AddAttribute(seq++, "data-testid", $"event-story-thread-{row.NarrativeId}");
                    builder.AddContent(seq++, $"in {row.NarrativeName}: ");

                    if (priorDiverges)
                    {
                        EventDateAndPlacesSection.RenderArrowNav(builder, ref seq, ctx, "prior", "event-story-thread-prior-event", "event-story-thread-prior-label", row.NarrativeId, row.Prior, "←", inline: true);
                    }
                    if (priorDiverges && followingDiverges)
                    {
                        builder.AddContent(seq++, " · ");
                    }
                    if (followingDiverges)
                    {
                        EventDateAndPlacesSection.RenderArrowNav(builder, ref seq, ctx, "following", "event-story-thread-following-event", "event-story-thread-following-label", row.NarrativeId, row.Following, "→", inline: true, inlinePrefixText: "next ");
                    }

                    builder.CloseElement(); // p.popover-meta (event-story-thread-{narrativeId})
                }
                builder.CloseElement(); // .popover-story-thread
            }
        };
        return new PopoverSection("event-chronology", body);
    }

    // A narrative-side leg that is null is that narrative's OWN first/last
    // leg -- a chain end there, never a divergence (the brief's own words:
    // "nothing to show for that direction") -- so this short-circuits
    // false before ever comparing ids. Otherwise a plain id comparison:
    // the SAME event both sides name is agreement (false); a different id,
    // OR the timeline having no leg at all in that direction while the
    // narrative genuinely does (the atlas's own true first/last dated
    // event, vanishingly rare in practice), both read as divergence (true)
    // -- "the divergence test is an id comparison client-side," the
    // brief's own words, generalized to both of its null-handling edges.
    private static bool Diverges(NarrativeAdjacentEventDto? narrativeLeg, NarrativeAdjacentEventDto? timelineLeg)
        => narrativeLeg is not null && narrativeLeg.Id != timelineLeg?.Id;
}

/// <summary>
/// Batch M requirement 4: a <see cref="PolityDeltaNode"/>'s own event text
/// -- the "event title + years" content beneath the popover's own Title
/// header (which already carries "{polity}, {from} -&gt; {to}"). A plain
/// <c>.popover-meta</c> line (the same quiet-secondary-text treatment
/// <c>TimeAndPlaceNode</c>/<c>NarrativeEventTextSection</c> already use for
/// an analogous "what is this node, in prose" line), present ONLY when
/// <see cref="PolityDeltaNode.EventText"/> is non-null -- the minimal-
/// popover case (an honestly uneventful boundary) renders NOTHING here,
/// conditional presence, never a placeholder like "No event recorded."
/// </summary>
public sealed class PolityDeltaEventSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "PolityDelta";

    public Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not PolityDeltaNode delta || delta.EventText is not { } eventText)
        {
            return Task.FromResult<PopoverSection?>(null);
        }

        RenderFragment body = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-meta");
            builder.AddContent(2, eventText);
            builder.CloseElement();
        };
        return Task.FromResult<PopoverSection?>(new PopoverSection("polity-delta-event", body));
    }
}

/// <summary>
/// Batch M requirement 4: "THE SCRIPTURES" -- the delta's own curated
/// verses, via the SAME shared <see cref="Components.PassageList"/>
/// component every other verse list in this app renders through
/// (PASSAGE-1) -- grouped, truncation-free (no cap asked for, same as
/// CATECH-1's own THE SCRIPTURES and NARRATIVE-1's own event-text
/// section), each entry independently expandable. Conditional presence:
/// absent when <see cref="PolityDeltaNode.Verses"/> is empty (a delta MAY
/// be grounded only in Church-traditional history, with no single verse to
/// pinpoint it -- see the batch report's own delta-coverage table) OR when
/// none of the curated refs actually resolve (a graceful-degrade floor
/// every other verse-list section in this file already shares).
/// </summary>
public sealed class PolityDeltaScripturesSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "PolityDelta";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not PolityDeltaNode delta || delta.Verses.Count == 0)
        {
            return null;
        }

        List<PassageListVerse> verses;
        try
        {
            verses = await VerseTextResolver.ResolveAsync(api, delta.Verses);
        }
        catch (Exception)
        {
            verses = new List<PassageListVerse>();
        }
        if (verses.Count == 0)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "catechism-section-heading"); // the SAME "THE SCRIPTURES" small-caps eyebrow treatment CATECH-1/NARRATIVE-1 already establish -- one shared testid/class for one house convention, not a fourth copy
            builder.AddAttribute(seq++, "data-testid", "catechism-section-heading");
            builder.AddContent(seq++, "THE SCRIPTURES");
            builder.CloseElement();

            var units = new PassageSourceUnit[] { new(verses) };
            builder.OpenComponent<Components.PassageList>(seq++);
            builder.AddAttribute(seq++, "Units", (IReadOnlyList<PassageSourceUnit>)units);
            builder.AddAttribute(seq++, "RefTestIdPrefix", "polity-delta-verse");
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("polity-delta-scriptures", body);
    }
}

/// <summary>
/// Batch M requirement 4: "grounding note (ref_note, quiet)" -- the
/// curator's own citation for this delta (which source(s) were actually
/// consulted, or an honest "tradition only" disclosure), rendered plainly
/// and quietly (the SAME <c>.popover-meta</c> treatment as
/// <see cref="PolityDeltaEventSection"/> above -- both are secondary,
/// quiet-register text, not a second style). Conditional presence: absent
/// when <see cref="PolityDeltaNode.RefNote"/> is null (the minimal-popover
/// case).
/// </summary>
public sealed class PolityDeltaGroundingSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "PolityDelta";

    public Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not PolityDeltaNode delta || delta.RefNote is not { } refNote)
        {
            return Task.FromResult<PopoverSection?>(null);
        }

        RenderFragment body = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-meta");
            builder.AddContent(2, refNote);
            builder.CloseElement();
        };
        return Task.FromResult<PopoverSection?>(new PopoverSection("polity-delta-grounding", body));
    }
}

/// <summary>
/// O4 (owner live-preview correction, 2026-08-23: "remove persons from
/// hover menus for now") UNREGISTERED this class from
/// <see cref="PopoverSectionRegistry.Providers"/> -- it is never
/// constructed anywhere today, so nothing below actually runs. Kept intact
/// rather than deleted per the ruling's own explicit words ("machinery
/// retained"), a deliberate, disclosed exception to this codebase's usual
/// dead-code law; see <see cref="PopoverSectionRegistry"/>'s own comment
/// for the fuller story and reader-persons.spec.ts's own header comment for
/// how its test coverage moved to the surviving in-text-mention entry path
/// (M-D3/U5) instead. The doc comment below describes this class's own
/// pre-O4 behavior, unchanged, for whenever it is re-registered.
///
/// Batch P (the extensibility proof; M-D2 ruling): the VERSE/PASSAGE
/// popover's own PERSONS section -- mentioned persons, conditional
/// presence. THE FIRST <see cref="IExplorableClient"/> CONSUMER: reads the
/// generic <c>mentions</c> edge (<see cref="IPopoverSectionContext.Graph"/>)
/// directly, never <c>AtlasClient</c>/<c>VerseDetail</c> -- "NO parallel
/// client data path," the brief's own words, verbatim. A verse's own
/// `mentions` frontier carries BOTH Place and Person entities under the
/// SAME edge kind (server-side: one `mentions` relation, `PlaceOrPerson`-
/// typed rows -- `graph_types::edge::Mentions`); this section filters to
/// <c>Kind == "Person"</c> client-side (design doc §7's own CHAIN
/// HOMOGENEITY law: "a frontier SECTION renders entries of ONE kind-shape"
/// -- a mixed Place+Person list would violate it, so filtering here, not
/// interleaving, is what the law itself requires).
///
/// Fetch scope, disclosed: ONE page at <see cref="EdgeSectionRegistry.Mentions"/>'s
/// own <c>InitialClamp</c> (50) -- a real verse's own total mentions
/// (places+persons combined) is always small in the real compiled data;
/// spot-checked, no verse comes remotely close. If the true total somehow
/// exceeds this one page (<c>page.Next is not null</c>), a plain,
/// non-interactive "+ more mentions" line discloses it honestly (design
/// doc §7's own "+N more" law: a visible signal with the true count) --
/// no second fetch is wired for this direction, since the realistic case
/// this would ever fire is effectively zero and a person's own SEPARATE
/// "mentioned-in" direction (<see cref="PersonCardAndMentionsSection"/>
/// below) is where genuine large-count pagination actually matters.
/// </summary>
public sealed class VersePersonsSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind is "Verse" or "Passage";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        string wireId;
        switch (node)
        {
            case VerseNode v:
                wireId = $"text-unit:{v.Title}"; // v.Title IS the vref (VerseNode's own doc comment)
                break;
            case PassageNode p:
                // A passage's own mentions render at its FIRST verse -- the
                // same "first verse anchors the locus" convention this
                // file's own CrossRefsSection/PassageList.ExploreAsVerse
                // already establish for a passage's own onward identity.
                wireId = $"text-unit:{CanonRef.FirstVerseOf(p.Title)}";
                break;
            default:
                return null;
        }

        EdgePageDto page;
        try
        {
            page = await ctx.Graph.Edges(wireId, EdgeSectionRegistry.Mentions.EdgeKind, cursor: null, limit: EdgeSectionRegistry.Mentions.InitialClamp);
        }
        catch (Exception)
        {
            return null; // fail soft -- same graceful-degradation policy every other lazy fetch in this app follows
        }

        var persons = page.Entries.Where(e => e.Node.Kind == "Person").ToList();
        if (persons.Count == 0)
        {
            return null; // conditional presence -- most verses mention no person at all
        }

        var mayHaveMore = page.Next is not null; // see this class's own doc comment
        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "catechism-section-heading");
            builder.AddAttribute(seq++, "data-testid", "persons-section-heading");
            builder.AddContent(seq++, "PERSONS");
            builder.CloseElement();

            foreach (var entry in persons)
            {
                var id = entry.Node.Id; // local copies -- captured per-row by the onclick closure below
                var label = entry.Node.Label;
                builder.OpenElement(seq++, "button");
                builder.AddAttribute(seq++, "type", "button");
                builder.AddAttribute(seq++, "class", "popover-event-row popover-event-row-button explorable");
                builder.AddAttribute(seq++, "data-testid", $"verse-person-{Slug(label)}");
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new PersonNode(id, label))));
                builder.AddContent(seq++, label);
                builder.CloseElement();
            }

            if (mayHaveMore)
            {
                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "popover-meta");
                builder.AddAttribute(seq++, "data-testid", "persons-section-more");
                builder.AddContent(seq++, "+ more mentions in this verse");
                builder.CloseElement();
            }
        };
        return new PopoverSection("persons", body);
    }

    // A stable, DOM-safe testid fragment from a person's own display name
    // ("Simon Peter" -> "simon-peter") -- names are never guaranteed
    // ASCII-simple (this app's own broader KJV-name vocabulary isn't), so
    // this narrows to the same safe alphanumeric-plus-dash shape
    // `EventNode`/`PlaceNode` ids already are, rather than assuming Theographic's
    // own personLookup-derived label is always dash-safe.
    private static string Slug(string label)
    {
        var chars = label.ToLowerInvariant().Select(c => char.IsLetterOrDigit(c) ? c : '-').ToArray();
        var slug = new string(chars);
        while (slug.Contains("--"))
        {
            slug = slug.Replace("--", "-");
        }
        return slug.Trim('-');
    }
}

/// <summary>
/// Batch P (the extensibility proof): the PERSON popover's own card +
/// "mentioned-in" frontier -- BOTH via <see cref="IExplorableClient"/>
/// (<see cref="IPopoverSectionContext.Graph"/>), the generic contract, zero
/// bespoke endpoints (the batch's own thesis). "Card" here means exactly
/// what the generic <c>GET /api/node/{id}</c> wire shape carries -- label
/// (already the popover's own title, via <see cref="PersonNode.Title"/>)
/// and <c>Provenance</c> (a genuinely new field this batch is the first to
/// render anywhere in this app, disclosed) -- richer payload facts
/// (gender/birth_year/death_year/also_called) ride the SERVER-side graph
/// payload but are NOT projected onto the generic card wire this batch
/// (<c>NodeCardOut.Label</c> is one string, matching every OTHER kind's
/// own card label precedent exactly -- Place's canonical name carries no
/// lat/lon, Event's label carries no date -- see graph_types::node::card's
/// own match arms); a future batch's own decision to widen the generic
/// card wire is real, disclosed follow-up, not attempted here.
/// "Mentioned in Scripture" (every mention, canon order) is
/// <c>mentioned-in</c>'s own edges page, handed to
/// <see cref="Components.PersonMentionsList"/> (the one component in this
/// batch that owns real per-popover fetch state, for the SAME
/// "a provider instance is shared/static, a component can hold state"
/// reason <c>VerseTextSectionProvider</c>/<c>PlaceEventsList</c> already
/// establish).
/// </summary>
public sealed class PersonCardAndMentionsSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Person";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not PersonNode person)
        {
            return null;
        }

        var spec = EdgeSectionRegistry.MentionedIn;
        NodeCardDto card;
        EdgePageDto page;
        try
        {
            var cardTask = ctx.Graph.Card(person.PersonId);
            var pageTask = ctx.Graph.Edges(person.PersonId, spec.EdgeKind, cursor: null, limit: spec.InitialClamp);
            await Task.WhenAll(cardTask, pageTask);
            card = cardTask.Result;
            page = pageTask.Result;
        }
        catch (Exception)
        {
            return null; // fail soft -- same graceful-degradation policy every other lazy fetch in this app follows
        }

        // spec.EdgeKind is EdgeKindId (Batch P fix round 1, R-P1); card.EdgeSummary[].Kind
        // is a plain string straight off the wire (EdgeSummaryEntryDto, unvalidated JSON) --
        // .Value is the one, explicit, disclosed crossing point between the two.
        var total = card.EdgeSummary.FirstOrDefault(s => s.Kind == spec.EdgeKind.Value)?.Count ?? page.Entries.Count;

        RenderFragment body = builder =>
        {
            builder.OpenComponent<Components.PersonMentionsList>(0);
            builder.AddAttribute(1, "PersonId", person.PersonId);
            builder.AddAttribute(2, "Provenance", card.Provenance);
            builder.AddAttribute(3, "InitialEntries", page.Entries);
            builder.AddAttribute(4, "InitialNext", page.Next);
            builder.AddAttribute(5, "TotalCount", total);
            builder.AddAttribute(6, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("person-mentions", body);
    }
}

/// <summary>
/// Batch CORP-1b (owner authorization, resolving CORP-1's own disclosed
/// NEEDS_CONTEXT gap): a Kretzmann CommentaryItem's own real prose, fetched
/// through the generic graph client's <c>Card()</c> call -- the SAME
/// additive <c>description</c> field ENT-1a built for Place/Person/
/// PeopleGroup (server: <c>atlas_graph::legacy::node_description</c>'s
/// widened match), reused rather than a new endpoint. Conditional presence:
/// a unit with no resolvable description (never true for a real compiled
/// CommentaryItem, but the wire's own <c>skip_serializing_if</c> makes this
/// honestly possible) renders no section at all, matching this app's
/// standing "nothing registered / nothing to show = no placeholder" rule.
/// ANY embedded verse text inside the prose (Kretzmann's own inline quotes,
/// KRETZ-1's lemma-excision design: "inline verse-fragment quotes inside
/// the prose are content, kept verbatim") is already part of this same
/// plain-text string -- there is no separately-marked-up sub-span to route
/// through a second render path, so the ONE render rule here is simply:
/// this text, verbatim, in one paragraph (the same shape <c>VerseNode.
/// BodyAsync</c> already uses for its own verse text).
/// </summary>
public sealed class CommentaryItemProseSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "CommentaryItem";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not CommentaryItemNode item)
        {
            return null;
        }

        NodeCardDto card;
        try
        {
            card = await ctx.Graph.Card(item.Id);
        }
        catch (Exception)
        {
            return null; // fail soft -- same graceful-degradation policy every other lazy fetch in this app follows
        }

        if (string.IsNullOrWhiteSpace(card.Description))
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-commentary-text");
            builder.AddContent(2, card.Description);
            builder.CloseElement();
        };
        return new PopoverSection("commentary-text", body);
    }
}
