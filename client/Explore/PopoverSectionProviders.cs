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
            builder.AddAttribute(3, "Cap", ctx.XrefEntryPoint ? EdgeSectionRegistry.Cites.InitialClamp : (ctx.OtherContextSectionCount > 0 ? 2 : EdgeSectionRegistry.Cites.InitialClamp));
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
            builder.AddAttribute(1, "PlaceName", placeName);
            builder.AddAttribute(2, "Events", events);
            builder.AddAttribute(3, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("place-events", body);
    }
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

        if (events.Count == 0)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "catechism-section-heading"); // the SAME house small-caps eyebrow every section-registry heading shares (CATECH-1/NARRATIVE-1/DELTA-1) -- not a fifth copy
            builder.AddAttribute(seq++, "data-testid", "event-section-heading");
            builder.AddContent(seq++, "EVENT");
            builder.CloseElement();

            foreach (var e in events)
            {
                var id = e.Id; // local copies -- captured per-row by the onclick closure below
                var label = e.Label;
                // Batch HOTFIX-4 requirement 6 (AFFORDANCE HONESTY): a
                // general-kind event is NOT part of time traversal (req 2)
                // -- its own row here must not look like a dated event's
                // (which DOES traverse, after req 1). `.explorable-quiet`
                // REPLACES `.explorable` (never both) -- same class,
                // everywhere a non-traversable node's own identity renders,
                // per that class's own app.css comment.
                var explorableClass = e.Kind == "general" ? "explorable-quiet" : "explorable";
                builder.OpenElement(seq++, "button");
                builder.AddAttribute(seq++, "type", "button");
                builder.AddAttribute(seq++, "class", $"popover-event-row popover-event-row-button {explorableClass}");
                builder.AddAttribute(seq++, "data-testid", $"verse-event-{id}");
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new EventNode(id, label))));
                builder.AddContent(seq++, label);
                builder.CloseElement();
            }
        };
        return new PopoverSection("event-membership", body);
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
            detail = await ev.DetailAsync(api); // memoized -- shared with EventWitnessesSection and the PRIOR/FOLLOWING providers' own map-focus-sync read
        }
        catch (Exception)
        {
            return null;
        }

        if (detail.When is null && detail.Places.Count == 0)
        {
            return null; // general-kind passage with nothing this section can honestly show
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
}

/// <summary>
/// Batch T requirement 4: "PARALLEL ACCOUNTS -- one short passage per
/// witness (Gospel name + passage ref), each clamped to 2 verses with
/// per-passage expand/collapse... Single-witness events show the one
/// passage (no 'parallel' framing when n=1 -- conditional presence)." One
/// <see cref="Components.PassageList"/> unit per witness (never merged --
/// PASSAGE-1's own "a source unit never blurs into its neighbor's," applied
/// here across Gospels), each captioned with its own book's display name
/// (resolved via the already-cached <see cref="AtlasClient.Books"/> roster,
/// no extra fetch beyond the first page load) -- "Gospel name + passage
/// ref" falls out of PassageList's own existing Caption + auto-rendered
/// Span, no new rendering needed. <see cref="Components.PassageList.ClampVerses"/>
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
        List<BookTocEntry> books;
        try
        {
            detail = await ev.DetailAsync(api);
            books = await api.Books(); // fetched once, cached forever -- see AtlasClient's own doc comment
        }
        catch (Exception)
        {
            return null;
        }

        if (detail.Witnesses.Count == 0)
        {
            return null; // defensive -- scene::witnesses_for always synthesizes >=1 server-side, but never assume a network response is well-formed
        }

        var units = detail.Witnesses.Select(w =>
        {
            var bookName = books.FirstOrDefault(b => b.Code == w.Book)?.Name ?? w.Book;
            // Batch HOTFIX-4 requirement 7: GroupCount carries each
            // VerseGroup's own TRUE total (server-side `take(20)` cap,
            // scene::verse_groups_for) so a truncated witness group shows
            // the "+N more" signal instead of silently ending at 20.
            var verses = w.VerseGroups.SelectMany(g => g.Verses.Select(v => new PassageListVerse(v, "", g.Count))).ToList();
            return new PassageSourceUnit(verses, bookName);
        }).ToList();

        // The witnesses' own verse TEXT isn't on VerseGroup (ids only, same
        // as every other VerseGroup on this wire) -- resolve it the SAME way
        // every other section in this file resolves a curated verse list
        // (VerseTextResolver, the existing chapter fetch + LRU cache), then
        // re-pair it back onto each unit's own PassageListVerse list.
        List<PassageListVerse> resolvedFlat;
        try
        {
            var allVrefs = detail.Witnesses.SelectMany(w => w.VerseGroups.SelectMany(g => g.Verses)).ToList();
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
        var textByVref = resolvedFlat.GroupBy(v => v.Vref).ToDictionary(g => g.Key, g => g.First().Text);
        units = units.Select(u => new PassageSourceUnit(
            // v.GroupCount carried through from the FIRST construction
            // above -- resolving text must never silently drop it.
            u.Verses.Select(v => new PassageListVerse(v.Vref, textByVref.GetValueOrDefault(v.Vref, ""), v.GroupCount)).ToList(),
            u.Caption)).ToList();

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
/// Batch N: shared rendering for the PRIOR EVENT / FOLLOWING EVENT sections
/// (<see cref="EventPriorSection"/>/<see cref="EventFollowingSection"/>
/// below) -- both are the exact same shape (one block per qualifying
/// narrative position: a house small-caps heading, an explorable
/// event-traversal row, that adjacent event's own verses via the shared
/// passage-list component), differing only in which side of
/// <see cref="NarrativePositionDto"/> they read and their own house
/// label/testid vocabulary ("prior"/"PRIOR EVENT" vs
/// "following"/"FOLLOWING EVENT") -- ONE resolver, two thin callers, never
/// two copies of the same render loop. Batch T requirement 2/4: retargeted
/// from Verse/NarrativeEvent onto the new EVENT node (see
/// <see cref="EventPriorSection.AppliesTo"/>) -- the resolver itself
/// (`positions_for_events`, server-side) needed no change; recursion falls
/// out of an EventNode's own traversal row pushing ANOTHER EventNode, the
/// exact same registry re-run every other push/pop already triggers.
/// </summary>
file static class NarrativeDirectionSection
{
    public static async Task<PopoverSection?> ResolveAsync(
        IExplorable node,
        AtlasClient api,
        IPopoverSectionContext ctx,
        Func<NarrativePositionDto, NarrativeAdjacentEventDto?> pick,
        string testid,
        string sectionLabel,
        string eventTestIdPrefix,
        string verseTestIdPrefix)
    {
        if (node is not INarrativeAware aware)
        {
            return null;
        }

        IReadOnlyList<NarrativePositionDto> positions;
        try
        {
            // Batch HOTFIX-4: `.Narrative` only -- this resolver renders the
            // per-narrative PRIOR/FOLLOWING sections; the global-timeline
            // half (`.Timeline`) has its OWN, separate shared resolver
            // (EventTimelineDirectionSection, below) with its own quiet
            // "IN TIME" wording, per requirement 1's own "narrative rows
            // render as today (unchanged)... the global-timeline adjacency
            // renders as a quiet, clearly distinct" instruction.
            positions = (await aware.NarrativePositionsAsync(api)).Narrative;
        }
        catch (Exception)
        {
            return null; // fail soft -- same graceful-degradation policy every other lazy fetch in this app follows
        }

        var entries = positions
            .Select(p => (Position: p, Adjacent: pick(p)))
            .Where(e => e.Adjacent is not null)
            .Select(e => (e.Position, Adjacent: e.Adjacent!))
            .ToList();

        if (entries.Count == 0)
        {
            return null; // conditional presence: no qualifying narrative in this direction -> no section, not a disabled stub
        }

        List<PassageListVerse>[] resolved;
        try
        {
            // Batch HOTFIX-4 requirement 7: ResolveGroupsAsync (not the bare
            // ResolveAsync) so a truncated adjacent-event VerseGroup carries
            // its own GroupCount through to the "+N more" signal.
            resolved = await Task.WhenAll(entries.Select(e => VerseTextResolver.ResolveGroupsAsync(api, e.Adjacent.VerseGroups)));
        }
        catch (Exception)
        {
            resolved = entries.Select(_ => new List<PassageListVerse>()).ToArray();
        }

        // Requirement 2's own "at a narrative boundary... name the
        // narrative" -- extended one step further (a disclosed, real-data-
        // driven decision, see the batch report): a single qualifying
        // entry needs no name at all ("PRIOR EVENT" bare); >1 entries are
        // each named by their own narrative UNLESS two entries share one
        // narrative NAME (a real case in the compiled data -- EXO.12.37 is
        // cited by two different events, both legs of the exodus
        // narrative), in which case the CURRENT event's own label is also
        // appended so the two stay distinguishable.
        var nameCounts = entries.Count > 1
            ? entries.GroupBy(e => e.Position.NarrativeName).ToDictionary(g => g.Key, g => g.Count())
            : new Dictionary<string, int>();

        RenderFragment body = builder =>
        {
            // Testid disambiguation for the rare case two entries share one
            // narrative id (same shape as CatechismSeamSection's own
            // catechism-item-{id}/--q2 numbered suffix): first occurrence of
            // a narrative id keeps the bare `{prefix}-{narrativeId}` testid;
            // later occurrences get a numbered `--2`/`--3` suffix. Declared
            // FRESH INSIDE this closure (not in ResolveAsync's own outer
            // scope) -- a real, live-caught bug fixed before this shipped:
            // a RenderFragment delegate is not guaranteed to run exactly
            // once (Blazor re-invokes the SAME stored delegate on every
            // later re-render of this popover, e.g. an unrelated
            // OnAfterRenderAsync-triggered StateHasChanged), so a dictionary
            // captured BY REFERENCE from the enclosing method would keep
            // accumulating counts across those re-renders -- observed live
            // as a SINGLE entry's own testid drifting from bare
            // `event-following-event-exodus` (Batch N's own original bug
            // report predates Batch T's testid rename, `narrative-` ->
            // `event-`, but the exact same class of bug/fix) to `--2`, `--3`, ... on
            // successive renders, with no second entry ever existing.
            // CatechismSeamSection's own `occurrences` (a few sections up)
            // and PassageList.razor's own `Visible` getter both already get
            // this right (freshly declared at render/access time) -- this
            // fix matches that established, correct precedent exactly.
            var occurrences = new Dictionary<string, int>();
            var seq = 0;
            for (var i = 0; i < entries.Count; i++)
            {
                var (position, adjacent) = entries[i];
                var verses = resolved[i];

                var heading = entries.Count <= 1
                    ? sectionLabel
                    : nameCounts[position.NarrativeName] > 1
                        ? $"{sectionLabel} — {position.NarrativeName} ({position.EventLabel})"
                        : $"{sectionLabel} — {position.NarrativeName}";

                builder.OpenElement(seq++, "p");
                // Batch T: shares the SAME testid/class every other section
                // eyebrow in this popover platform does (catechism-section-
                // heading -- "THE SMALL CATECHISM"/"THE SCRIPTURES"/"EVENT"/
                // "PARALLEL ACCOUNTS" all already reuse this one identical
                // rule, distinguished only by their own rendered TEXT, never
                // a fresh testid per heading) -- retired the Batch-N-era
                // narrative-section-heading name, which was pixel-identical
                // CSS under a fourth name.
                builder.AddAttribute(seq++, "class", "catechism-section-heading");
                builder.AddAttribute(seq++, "data-testid", "event-section-heading");
                builder.AddContent(seq++, heading);
                builder.CloseElement();

                var narrativeId = position.NarrativeId; // local copies -- captured per-row by the onclick closure below
                var eventId = adjacent.Id;
                var eventLabel = adjacent.Label;

                var count = occurrences[narrativeId] = occurrences.GetValueOrDefault(narrativeId) + 1;
                var idSuffix = count == 1 ? narrativeId : $"{narrativeId}--{count}";

                builder.OpenElement(seq++, "button");
                builder.AddAttribute(seq++, "type", "button");
                builder.AddAttribute(seq++, "class", "popover-event-row popover-event-row-button explorable");
                builder.AddAttribute(seq++, "data-testid", $"{eventTestIdPrefix}-{idSuffix}");
                builder.AddAttribute(seq++, "aria-label", $"{sectionLabel}: {eventLabel}");
                // Batch T: pushes the new EventNode (Kind == "Event") -- not
                // narrative-locked the way the retired NarrativeEventNode was
                // (that lock existed only to keep a Verse-reached traversal
                // from silently branching into an unrelated narrative the
                // SAME event also happened to touch; an EventNode reached
                // from ANY of its own several entry points -- a heading, a
                // verse's own EVENT row, or this very traversal -- always
                // shows the FULL, unfiltered position list, recursing
                // through every qualifying narrative it belongs to).
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () =>
                    ctx.PushAsync(new EventNode(eventId, eventLabel))));
                builder.OpenElement(seq++, "span");
                builder.AddAttribute(seq++, "class", "popover-event-label");
                builder.AddContent(seq++, eventLabel);
                builder.CloseElement();
                builder.CloseElement();

                if (verses.Count > 0)
                {
                    var units = new PassageSourceUnit[] { new(verses) };
                    builder.OpenComponent<Components.PassageList>(seq++);
                    builder.AddAttribute(seq++, "Units", (IReadOnlyList<PassageSourceUnit>)units);
                    builder.AddAttribute(seq++, "RefTestIdPrefix", $"{verseTestIdPrefix}-{idSuffix}");
                    builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
                    builder.CloseComponent();
                }
            }
        };
        return new PopoverSection(testid, body);
    }
}

/// <summary>
/// Batch N requirement 2, retargeted Batch T requirement 4: "PRIOR EVENT" --
/// present only when the current node's own narrative position(s)
/// (<see cref="INarrativeAware"/>) include at least one with a
/// <see cref="NarrativePositionDto.Prior"/>. Applies to Event ONLY now
/// (Batch T requirement 3 retires this from Verse -- "rather than putting
/// the next/previous event on every verse... traversal lives on event
/// nodes," the owner verbatim) -- recursion (an EventNode's own further
/// prior, "recursively, arbitrarily far") falls out of THIS SAME
/// `AppliesTo` clause matching the traversed EventNode too, not a second
/// mechanism. Deliberately still NOT Passage (unchanged scope note from
/// Batch N: a shift-click passage span's own per-verse narrative
/// membership is genuinely ambiguous in a way a single curated event never
/// is).
/// </summary>
public sealed class EventPriorSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Event";

    public Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx) =>
        NarrativeDirectionSection.ResolveAsync(node, api, ctx, p => p.Prior, "event-prior", "PRIOR EVENT", "event-prior-event", "event-prior-verse");
}

/// <summary>
/// Batch N requirement 2, retargeted Batch T requirement 4: "FOLLOWING
/// EVENT" -- the mirror image of <see cref="EventPriorSection"/> (same
/// applicability, same shared resolver, reading
/// <see cref="NarrativePositionDto.Following"/> instead). Registered
/// immediately after it (PopoverSections.cs) so PRIOR always renders above
/// FOLLOWING, matching the narrative's own forward reading order.
/// </summary>
public sealed class EventFollowingSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Event";

    public Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx) =>
        NarrativeDirectionSection.ResolveAsync(node, api, ctx, p => p.Following, "event-following", "FOLLOWING EVENT", "event-following-event", "event-following-verse");
}

/// <summary>
/// Batch HOTFIX-4 requirement 1 ("generalize the ONE resolver -- traversal
/// by time for every dated event"): shared rendering for the new "PRIOR IN
/// TIME" / "FOLLOWING IN TIME" sections (<see cref="EventTimelinePriorSection"/>/
/// <see cref="EventTimelineFollowingSection"/> below) -- the GLOBAL
/// chronological adjacency, independent of narrative membership, alongside
/// (never instead of) the narrative-scoped rows <see cref="NarrativeDirectionSection"/>
/// already renders. Structurally simpler than that sibling resolver: the
/// global timeline has exactly ONE row in each direction (never "one block
/// per qualifying narrative"), so there is no name-collision/disambiguation
/// logic to duplicate -- reuses the SAME underlying mechanism
/// (<see cref="Components.PassageList"/>, the event-traversal row shape,
/// PASSAGE-1's own click contract) without forcing an ill-fitting
/// multi-entry abstraction onto a single-entry case. "Quiet, clearly
/// distinct" (requirement 1 verbatim) from the narrative rows: the SAME
/// shared `catechism-section-heading`/`event-section-heading` eyebrow
/// class/testid every section heading in this popover platform already
/// uses, PLUS a second, self-contained modifier class
/// (`event-timeline-heading`, app.css) that softens the text to `--ink-soft`
/// -- and "IN TIME" wording, never "EVENT", so the two are never
/// mistakable for each other even at a glance.
/// </summary>
file static class EventTimelineDirectionSection
{
    public static async Task<PopoverSection?> ResolveAsync(
        IExplorable node,
        AtlasClient api,
        IPopoverSectionContext ctx,
        Func<TimelinePositionDto, NarrativeAdjacentEventDto?> pick,
        string testid,
        string sectionLabel,
        string rowTestId,
        string verseTestIdPrefix)
    {
        if (node is not INarrativeAware aware)
        {
            return null;
        }

        TimelinePositionDto? timeline;
        try
        {
            timeline = (await aware.NarrativePositionsAsync(api)).Timeline;
        }
        catch (Exception)
        {
            return null; // fail soft -- same graceful-degradation policy every other lazy fetch in this app follows
        }

        // Requirement 2: `Timeline` is null (the wire key OMITTED) for a
        // general-kind or unknown event -- "NOT part of time traversal,"
        // resolved here by simple absence, never a disabled stub.
        if (timeline is null)
        {
            return null;
        }

        var adjacent = pick(timeline);
        if (adjacent is null)
        {
            // Conditional presence at the atlas's own TRUE first/last dated
            // event only (requirement 1 verbatim) -- everywhere else, a
            // dated event always has both directions, so a user can always
            // keep walking.
            return null;
        }

        List<PassageListVerse> verses;
        try
        {
            // Batch HOTFIX-4 requirement 7: same reason as
            // NarrativeDirectionSection's own identical call, above.
            verses = await VerseTextResolver.ResolveGroupsAsync(api, adjacent.VerseGroups);
        }
        catch (Exception)
        {
            verses = new List<PassageListVerse>();
        }

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "catechism-section-heading event-timeline-heading");
            builder.AddAttribute(seq++, "data-testid", "event-section-heading");
            builder.AddContent(seq++, sectionLabel);
            builder.CloseElement();

            var eventId = adjacent.Id; // local copies -- captured by the onclick closure below
            var eventLabel = adjacent.Label;

            builder.OpenElement(seq++, "button");
            builder.AddAttribute(seq++, "type", "button");
            builder.AddAttribute(seq++, "class", "popover-event-row popover-event-row-button explorable");
            builder.AddAttribute(seq++, "data-testid", rowTestId);
            builder.AddAttribute(seq++, "aria-label", $"{sectionLabel}: {eventLabel}");
            // Same ONE-RULE traversal as the narrative rows: pushes a fresh
            // EventNode, re-anchoring the popover -- recursion falls out of
            // THIS SAME AppliesTo clause matching the traversed node too.
            builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new EventNode(eventId, eventLabel))));
            builder.OpenElement(seq++, "span");
            builder.AddAttribute(seq++, "class", "popover-event-label");
            builder.AddContent(seq++, eventLabel);
            builder.CloseElement();
            builder.CloseElement();

            if (verses.Count > 0)
            {
                var units = new PassageSourceUnit[] { new(verses) };
                builder.OpenComponent<Components.PassageList>(seq++);
                builder.AddAttribute(seq++, "Units", (IReadOnlyList<PassageSourceUnit>)units);
                builder.AddAttribute(seq++, "RefTestIdPrefix", verseTestIdPrefix);
                builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
                builder.CloseComponent();
            }
        };
        return new PopoverSection(testid, body);
    }
}

/// <summary>
/// Batch HOTFIX-4 requirement 1: "PRIOR IN TIME" -- the global-timeline
/// counterpart to <see cref="EventPriorSection"/>, present whenever THIS
/// event has a chronologically-prior dated event ANYWHERE in the atlas
/// (independent of narrative membership -- the whole point of this
/// requirement: "every DATED event traverses," not just narrative
/// members). Absent only at the atlas's own true first dated event.
/// </summary>
public sealed class EventTimelinePriorSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Event";

    public Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx) =>
        EventTimelineDirectionSection.ResolveAsync(node, api, ctx, t => t.Prior, "event-prior-timeline", "PRIOR IN TIME", "event-prior-event-timeline", "event-prior-verse-timeline");
}

/// <summary>
/// Batch HOTFIX-4 requirement 1: "FOLLOWING IN TIME" -- the mirror image of
/// <see cref="EventTimelinePriorSection"/>. Registered immediately after it
/// (PopoverSections.cs), and both after EventFollowingSection, so the
/// narrative rows always render above the timeline rows -- narrative
/// primacy preserved, requirement 1 verbatim.
/// </summary>
public sealed class EventTimelineFollowingSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Event";

    public Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx) =>
        EventTimelineDirectionSection.ResolveAsync(node, api, ctx, t => t.Following, "event-following-timeline", "FOLLOWING IN TIME", "event-following-event-timeline", "event-following-verse-timeline");
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
