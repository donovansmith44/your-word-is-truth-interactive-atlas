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
                    var text = chapter.Verses.FirstOrDefault(cv => cv.Verse == v)?.Text;
                    if (text is not null)
                    {
                        verses.Add(new PassageListVerse($"{s.Book}.{s.Chapter}.{v}", text));
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
            builder.OpenComponent<Components.PassageList>(0);
            builder.AddAttribute(1, "Units", (IReadOnlyList<PassageSourceUnit>)units);
            builder.AddAttribute(2, "RefTestIdPrefix", "xref-item");
            builder.AddAttribute(3, "Cap", ctx.OtherContextSectionCount > 0 ? 2 : 3);
            builder.AddAttribute(4, "MoreTestId", "xrefs-more");
            builder.AddAttribute(5, "CollapseTestId", "xrefs-collapse");
            builder.AddAttribute(6, "RevealNoun", "cross-references");
            // A real, live-caught regression (reader.spec.ts READ-3, found by
            // the full pre-existing suite): restores the pre-Batch-F2 click
            // contract -- every xref-item pushes a VerseNode at the target's
            // own FIRST verse, regardless of whether its preview text spans
            // more than one verse (~25% of real targets do). See
            // PassageList.razor's own ExploreAsVerse doc comment.
            builder.AddAttribute(7, "ExploreAsVerse", true);
            builder.AddAttribute(8, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
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
/// (curated) display name -- "The First Commandment", "Baptism — Part the
/// Fourth" -- clicking one pushes a <see cref="CatechismNode"/> (the SAME
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

            builder.OpenElement(seq++, "div");
            builder.AddAttribute(seq++, "class", "popover-catechism-list");
            var occurrences = new Dictionary<string, int>();
            foreach (var it in items)
            {
                var id = it.Id; // local copies -- captured per-row by the onclick closure below
                var name = it.Name;
                var count = occurrences[id] = occurrences.GetValueOrDefault(id) + 1;
                var testid = count == 1 ? $"catechism-item-{id}" : $"catechism-item-{id}--q{count}";
                var label = it.Question is { } q ? $"{name} — {q}" : name;

                builder.OpenElement(seq++, "button");
                builder.AddAttribute(seq++, "type", "button");
                builder.AddAttribute(seq++, "class", "popover-catechism-item explorable");
                builder.AddAttribute(seq++, "data-testid", testid);
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new CatechismNode(id, name))));
                builder.AddContent(seq++, label);
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
            currentGroup.Add(new PassageListVerse(v.Vref, v.Text));
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
