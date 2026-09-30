using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.Rendering;

namespace BibleAtlas.Client.Explore;

public sealed class ChapterCardSection : IPopoverSectionProvider
{
    // Hard-capped: an unbounded list here can grow tall enough to cover chapter-head's
    // own on-screen position while the card is open (e.g. PSA.119's 22 acrostic sections),
    // and since this card also opens on hover, that self-overlap can make a click gesture
    // (which hovers its target first) permanently unable to land on chapter-head again.
    private const int ListCap = 8;

    public bool AppliesTo(IExplorable node) => node.Kind == "Chapter";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not ChapterNode chapterNode)
        {
            return null;
        }

        Chapter chapter;
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
                    var eventId = h.EventId;
                    var title = h.Title;
                    var headingKind = h.Kind;
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
                    var placeId = p.Id;
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

public sealed class YearFrontierSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Year";

    public Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx) =>
        node is YearNode year ? year.ResolveFrontierAsync(api, ctx) : Task.FromResult<PopoverSection?>(null);
}

public sealed class VerseTextSectionProvider : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind is "Verse" or "Passage";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        string book;
        int chapter, focalFrom, focalTo;
        string compactText;
        IReadOnlyList<string> textProvenance = Array.Empty<string>();

        switch (node)
        {
            case VerseNode v:
                var (vBook, vChapter, vVerse) = CanonRef.ParseVerse(v.Title);
                book = vBook;
                chapter = vChapter;
                focalFrom = focalTo = vVerse;
                try
                {
                    var vDetail = await v.DetailAsync(api);
                    compactText = vDetail.Text;
                    textProvenance = new[] { ProvenanceResolver.NormalizeId(vDetail.Provenance) };
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
                var dash = p.Title.LastIndexOf('-');
                focalTo = dash >= 0 && int.TryParse(p.Title[(dash + 1)..], out var toVerse) ? toVerse : focalFrom;
                compactText = p.Text;
                break;

            default:
                return null;
        }

        IReadOnlyList<Verse> focalVerses;
        try
        {
            var chapterText = await api.Chapter(book, chapter);
            focalVerses = chapterText.Verses.Where(cv => cv.Number >= focalFrom && cv.Number <= focalTo).ToList();
        }
        catch (Exception)
        {
            focalVerses = new List<Verse>();
        }

        var registry = await FrontierProvenance.RegistryOrNull(api);

        RenderFragment fragment = builder =>
        {
            var seq = 0;
            builder.OpenComponent<Components.VerseTextSection>(seq++);
            builder.AddAttribute(seq++, "Book", book);
            builder.AddAttribute(seq++, "Chapter", chapter);
            builder.AddAttribute(seq++, "FocalFromVerse", focalFrom);
            builder.AddAttribute(seq++, "FocalToVerse", focalTo);
            builder.AddAttribute(seq++, "CompactText", compactText);
            builder.AddAttribute(seq++, "FocalVerses", (IReadOnlyList<Verse>)focalVerses);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();

            seq = FrontierProvenance.Affordance(
                builder, seq, textProvenance, registry, "verse-text-provenance",
                "Source for this verse's text", Components.ProvenanceAffordance.RowRegister);
        };
        return new PopoverSection("verse-text", fragment);
    }
}

internal static class FrontierProvenance
{
    // Fail-soft on the fetch, fail-loud on the resolution: a registry that could not be
    // fetched must not take a whole frontier section down with it, so this returns null
    // and callers render their content anyway. An id the registry does NOT contain is a
    // different fact (a piece of data with no source) and ProvenanceAffordance says so
    // out loud rather than treating it the same as a fetch failure.
    internal static async Task<SourcesDocument?> RegistryOrNull(AtlasClient api)
    {
        try
        {
            return await api.Sources();
        }
        catch (Exception)
        {
            return null;
        }
    }

    // The affordance is a sibling of the heading element, not a child of it: nesting the "?"
    // inside the heading <p> makes its text node part of the heading's accessible/text content
    // (e.g. `toHaveText('SIMILAR ACCOUNTS')` would read "SIMILAR ACCOUNTS?"). The
    // .popover-section-head wrapper (app.css) puts the two back on one line via `display: inline`
    // on the heading.
    internal static int Heading(
        RenderTreeBuilder builder,
        int seq,
        string text,
        string headingTestId,
        IReadOnlyList<string>? provenance,
        SourcesDocument? sources,
        string provenanceTestId,
        string buttonLabel)
    {
        builder.OpenElement(seq++, "div");
        builder.AddAttribute(seq++, "class", "popover-section-head");
        builder.OpenElement(seq++, "p");
        builder.AddAttribute(seq++, "class", "catechism-section-heading");
        builder.AddAttribute(seq++, "data-testid", headingTestId);
        builder.AddContent(seq++, text);
        builder.CloseElement();
        seq = Affordance(builder, seq, provenance, sources, provenanceTestId, buttonLabel, Components.ProvenanceAffordance.HeaderRegister);
        builder.CloseElement();
        return seq;
    }

    internal static int Affordance(
        RenderTreeBuilder builder,
        int seq,
        IReadOnlyList<string>? provenance,
        SourcesDocument? sources,
        string testId,
        string buttonLabel,
        string register)
    {
        builder.OpenComponent<Components.ProvenanceAffordance>(seq++);
        builder.AddAttribute(seq++, "Provenance", provenance);
        builder.AddAttribute(seq++, "TestId", testId);
        builder.AddAttribute(seq++, "ButtonLabel", buttonLabel);
        builder.AddAttribute(seq++, "Register", register);
        builder.AddAttribute(seq++, "Sources", sources);
        builder.CloseComponent();
        return seq;
    }

    // Deliberately does not filter out blank/whitespace provenance: a blank here means
    // real data has no attribution, and that must reach the affordance rather than be
    // silently treated as "no provenance at all" (which renders no affordance).
    internal static IReadOnlyList<string> Distinct(IEnumerable<string> rowProvenances) =>
        rowProvenances.Select(ProvenanceResolver.NormalizeId).Distinct().ToList();
}

public sealed class CrossRefsSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind is "Verse" or "Passage";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        IReadOnlyList<CrossRef> xrefs;
        IReadOnlyList<string> xrefProvenance = Array.Empty<string>();
        try
        {
            switch (node)
            {
                case VerseNode v:
                {
                    var detail = await v.DetailAsync(api);
                    xrefs = detail.CrossRefs;
                    xrefProvenance = detail.CrossRefsProvenance;
                    break;
                }
                case PassageNode p:
                    xrefs = await p.XrefsAsync(api);
                    xrefProvenance = FrontierProvenance.Distinct(xrefs.SelectMany(x => x.Provenance));
                    break;
                default:
                    xrefs = new List<CrossRef>();
                    break;
            }
        }
        catch (Exception)
        {
            return null;
        }

        if (xrefs.Count == 0)
        {
            return null;
        }

        // Only the first InitialClamp targets are fetched eagerly (xrefs here is uncapped --
        // e.g. GEN.1.1 carries 61 -- and the popover only ever shows a few before "reveal
        // more"); targets beyond that keep their full identity and are resolved lazily via
        // ResolveUnits on first reveal, never narrowed to a single-verse preview.
        var spans = xrefs.Select(x => (Xref: x, Span: CanonRef.TargetSpan(x.Target))).ToList();
        var eagerSpans = spans.Take(EdgeSectionRegistry.Cites.InitialClamp).ToList();
        var lazySpans = spans.Skip(EdgeSectionRegistry.Cites.InitialClamp).ToList();
        var units = await ResolveUnits(api, eagerSpans);
        var registry = await FrontierProvenance.RegistryOrNull(api);

        RenderFragment body = builder =>
        {
            var seq = 0;

            seq = FrontierProvenance.Heading(
                builder, seq, "Cross References", "xrefs-section-heading",
                xrefProvenance, registry, "xrefs-provenance", "Sources for these cross references");

            builder.OpenComponent<Components.PassageList>(seq++);
            builder.AddAttribute(seq++, "Units", (IReadOnlyList<PassageSourceUnit>)units);
            builder.AddAttribute(seq++, "TrueTotal", xrefs.Count);
            builder.AddAttribute(seq++, "ResolveRemainingAsync", (Func<Task<IReadOnlyList<PassageSourceUnit>>>)(async () => await ResolveUnits(api, lazySpans)));
            builder.AddAttribute(seq++, "RefTestIdPrefix", "xref-item");
            builder.AddAttribute(seq++, "Cap", ctx.XrefEntryPoint ? EdgeSectionRegistry.Cites.InitialClamp : (ctx.OtherContextSectionCount > 0 ? 2 : EdgeSectionRegistry.Cites.InitialClamp));
            builder.AddAttribute(seq++, "MoreTestId", "xrefs-more");
            builder.AddAttribute(seq++, "CollapseTestId", "xrefs-collapse");
            builder.AddAttribute(seq++, "RevealNoun", "cross-references");
            builder.AddAttribute(seq++, "ClampVerses", Components.PassageList.StandardVerseClamp);
            builder.AddAttribute(seq++, "ExploreAsVerse", true);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.AddAttribute(seq++, "OnToggleSelect", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.ToggleSelectAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("xrefs", body);
    }

    private static async Task<List<PassageSourceUnit>> ResolveUnits(AtlasClient api, List<(CrossRef Xref, (string Book, int Chapter, int FromVerse, int ToVerse)? Span)> targets)
    {
        var chapterKeys = targets.Where(t => t.Span is not null).Select(t => (t.Span!.Value.Book, t.Span.Value.Chapter)).Distinct().ToList();
        var chapters = new Dictionary<(string, int), Chapter>();
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
        }

        var units = new List<PassageSourceUnit>();
        foreach (var (x, span) in targets)
        {
            if (span is { } s && chapters.TryGetValue((s.Book, s.Chapter), out var chapter))
            {
                var verses = new List<PassageListVerse>();
                for (var v = s.FromVerse; v <= s.ToVerse; v++)
                {
                    var cv = chapter.Verses.FirstOrDefault(cv => cv.Number == v);
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
        return units;
    }
}

public sealed class CatechismSeamSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind is "Verse" or "Passage";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        IReadOnlyList<CatechismRef> items;
        IReadOnlyList<string> catechismProvenance = Array.Empty<string>();
        try
        {
            switch (node)
            {
                case VerseNode v:
                {
                    var detail = await v.DetailAsync(api);
                    items = detail.Catechism;
                    catechismProvenance = detail.CatechismProvenance;
                    break;
                }
                case PassageNode p:
                    items = await p.CatechismAsync(api);
                    catechismProvenance = FrontierProvenance.Distinct(items.SelectMany(i => i.Provenance));
                    break;
                default:
                    items = new List<CatechismRef>();
                    break;
            }
        }
        catch (Exception)
        {
            return null;
        }

        if (items.Count == 0)
        {
            return null;
        }

        var registry = await FrontierProvenance.RegistryOrNull(api);

        RenderFragment body = builder =>
        {
            var seq = 0;
            seq = FrontierProvenance.Heading(
                builder, seq, "THE SMALL CATECHISM", "catechism-section-heading",
                catechismProvenance, registry, "catechism-provenance", "Sources for this catechism mapping");

            builder.OpenComponent<Components.CatechismList>(seq++);
            builder.AddAttribute(seq++, "Items", items);
            builder.AddAttribute(seq++, "Cap", CatechismDefaultCap);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("catechism", body);
    }

    private const int CatechismDefaultCap = 2;
}

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

public sealed class CatechismTextSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Catechism";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not CatechismNode item)
        {
            return null;
        }

        CatechismItem detail;
        try
        {
            detail = await item.DetailAsync(api);
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

public sealed class CatechismExplanationSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Catechism";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not CatechismNode item)
        {
            return null;
        }

        CatechismItem detail;
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

public sealed class CatechismWhereWrittenSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Catechism";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not CatechismNode item)
        {
            return null;
        }

        CatechismItem detail;
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

public sealed class CatechismScripturesSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Catechism";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not CatechismNode item)
        {
            return null;
        }

        CatechismItem detail;
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

        var mentionData = new Dictionary<string, Verse>();
        try
        {
            var chapterKeys = detail.Verses.Select(v => CanonRef.ParseVerse(v.Vref)).Select(p => (p.Book, p.Chapter)).Distinct().ToList();
            var fetched = await Task.WhenAll(chapterKeys.Select(k => api.Chapter(k.Book, k.Chapter)));
            foreach (var (key, chapterText) in chapterKeys.Zip(fetched))
            {
                foreach (var cv in chapterText.Verses)
                {
                    mentionData[$"{key.Book}.{key.Chapter}.{cv.Number}"] = cv;
                }
            }
        }
        catch (Exception)
        {
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

public sealed class PlaceDescriptionSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Place";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not PlaceNode place)
        {
            return null;
        }

        PlacePage detail;
        try
        {
            detail = await place.DetailAsync(api);
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

        IReadOnlyList<PlaceDate> dates;
        try
        {
            dates = await place.DatesAsync(api);
        }
        catch (Exception)
        {
            return null;
        }

        if (dates.Count == 0)
        {
            return null;
        }

        var versesOfEach = await Task.WhenAll(dates.Select(date => VerseTextResolver.ResolveAsync(api, date.Verses)));

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "div");
            builder.AddAttribute(seq++, "class", "popover-place-dates");
            foreach (var (date, verses) in dates.Zip(versesOfEach))
            {
                RenderDateRow(builder, ref seq, date, verses, ctx);
            }
            builder.CloseElement();
        };
        return new PopoverSection("place-dates", body);
    }

    private static void RenderDateRow(RenderTreeBuilder builder, ref int seq, PlaceDate date, List<PassageListVerse> verses, IPopoverSectionContext ctx)
    {
        var testidSuffix = date.Label.ToLowerInvariant();

        builder.OpenElement(seq++, "div");
        builder.AddAttribute(seq++, "class", "popover-place-date");
        builder.AddAttribute(seq++, "data-testid", $"popover-place-date-{testidSuffix}");

        builder.OpenElement(seq++, "span");
        builder.AddAttribute(seq++, "class", "popover-place-date-label");
        builder.AddContent(seq++, date.Label);
        builder.CloseElement();

        builder.OpenElement(seq++, "span");
        builder.AddAttribute(seq++, "class", "popover-place-date-value");
        builder.AddContent(seq++, date.Claim.Label);
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

public sealed class PlaceBlurbSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Place";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not PlaceNode place)
        {
            return null;
        }

        PlacePage detail;
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

public sealed class PlaceEventsSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Place";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not PlaceNode place)
        {
            return null;
        }

        PlacePage detail;
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
            builder.OpenComponent<Components.PlaceEventsList>(0);
            builder.AddAttribute(1, "PlaceId", place.PlaceId);
            builder.AddAttribute(2, "PlaceName", placeName);
            builder.AddAttribute(3, "Events", events);
            builder.AddAttribute(4, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.AddAttribute(5, "OnToggleSelect", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.ToggleSelectAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("place-events", body);
    }
}

public static class EventMembershipHeading
{
    public static string For(EventKind kind) => kind switch
    {
        EventKind.Event => "EVENT",
        EventKind.General => "PASSAGE",
        _ => throw new NotSupportedException($"EventMembershipHeading.For: unrecognized Event::kind '{kind}'."),
    };
}

public sealed class VerseEventMembershipSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Verse";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not VerseNode v)
        {
            return null;
        }

        IReadOnlyList<VerseEvent> events;
        try
        {
            events = (await v.DetailAsync(api)).Events;
        }
        catch (Exception)
        {
            return null;
        }

        var dated = events.Where(e => e.Kind == EventKind.Event).ToList();
        if (dated.Count == 0)
        {
            return null;
        }

        return new PopoverSection("event-membership", RenderRows(EventKind.Event, dated, ctx, await FrontierProvenance.RegistryOrNull(api)));
    }

    internal static RenderFragment RenderRows(EventKind kind, IReadOnlyList<VerseEvent> events, IPopoverSectionContext ctx, SourcesDocument? registry) => builder =>
    {
        var seq = 0;
        seq = FrontierProvenance.Heading(
            builder, seq,
            EventMembershipHeading.For(kind),
            "event-section-heading",
            FrontierProvenance.Distinct(events.Select(e => e.Provenance)),
            registry,
            "event-membership-provenance-" + kind.WireName(),
            "Sources for these " + kind.WireName() + " rows");

        foreach (var e in events)
        {
            var id = e.Id;
            var label = e.Label;
            var rowKind = e.Kind;
            // .explorable-quiet replaces .explorable (never both): a general-kind event
            // is not part of time traversal, so its row must not look traversable.
            var explorableClass = e.Kind == EventKind.General ? "explorable-quiet" : "explorable";
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

public sealed class VersePassageMembershipSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Verse";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not VerseNode v)
        {
            return null;
        }

        IReadOnlyList<VerseEvent> events;
        try
        {
            events = (await v.DetailAsync(api)).Events;
        }
        catch (Exception)
        {
            return null;
        }

        var general = events.Where(e => e.Kind == EventKind.General).ToList();
        if (general.Count == 0)
        {
            return null;
        }

        return new PopoverSection("passage-membership", VerseEventMembershipSection.RenderRows(EventKind.General, general, ctx, await FrontierProvenance.RegistryOrNull(api)));
    }
}

public sealed class EventProvenanceSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Event";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not EventNode ev)
        {
            return null;
        }

        EventPage detail;
        try
        {
            detail = await ev.DetailAsync(api);
        }
        catch (Exception)
        {
            return null;
        }

        var registry = await FrontierProvenance.RegistryOrNull(api);
        RenderFragment body = builder =>
        {
            FrontierProvenance.Affordance(
                builder, 0, new[] { ProvenanceResolver.NormalizeId(detail.Provenance) }, registry, "event-provenance",
                "Source for this event", Components.ProvenanceAffordance.RowRegister);
        };
        return new PopoverSection("event-provenance", body);
    }
}

public sealed class EventDateAndPlacesSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Event";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not EventNode ev)
        {
            return null;
        }

        EventPage detail;
        TimeRange? when;
        try
        {
            detail = await ev.DetailAsync(api);
            when = (await ev.CardAsync(api)).Event?.When;
        }
        catch (Exception)
        {
            return null;
        }

        if (when is null && detail.Places.Count == 0)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = 0;

            if (when is not null)
            {
                builder.OpenComponent<Components.FrontierMetadataRow>(seq++);
                builder.AddAttribute(seq++, "TestId", "event-time");
                builder.AddAttribute(seq++, "ChildContent", (RenderFragment)(valueBuilder =>
                {
                    var vseq = 0;
                    valueBuilder.OpenElement(vseq++, "button");
                    valueBuilder.AddAttribute(vseq++, "type", "button");
                    valueBuilder.AddAttribute(vseq++, "class", "popover-frontier-metadata-value explorable");
                    valueBuilder.AddAttribute(vseq++, "data-testid", "event-time-value");
                    if (detail.RefNote is { } refNote)
                    {
                        valueBuilder.AddAttribute(vseq++, "title", refNote);
                    }
                    valueBuilder.AddAttribute(vseq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new YearNode(when))));
                    valueBuilder.AddContent(vseq++, when.Label);
                    valueBuilder.CloseElement();

                    foreach (var p in detail.Places)
                    {
                        var placeId = p.Id;
                        var placeName = p.Name;
                        var query = MapFocusHatch.Query(placeId, when);
                        valueBuilder.OpenElement(vseq++, "button");
                        valueBuilder.AddAttribute(vseq++, "type", "button");
                        valueBuilder.AddAttribute(vseq++, "class", "popover-frontier-metadata-value explorable");
                        valueBuilder.AddAttribute(vseq++, "data-testid", $"event-place-{placeId}");
                        valueBuilder.AddAttribute(vseq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.NavigateWorldAsync(query)));
                        valueBuilder.AddContent(vseq++, placeName);
                        valueBuilder.CloseElement();
                    }
                }));
                builder.CloseComponent();
            }
        };
        return new PopoverSection("event-date-places", body);
    }

    internal static void RenderArrowNav(RenderTreeBuilder builder, ref int seq, IPopoverSectionContext ctx, string direction, string eventTestIdPrefix, string roleTestIdPrefix, string idSuffix, NarrativeAdjacentEvent? adjacent, string glyph, bool inline = false, string? inlinePrefixText = null)
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

file static class WitnessUnitsResolver
{
    public static async Task<List<PassageSourceUnit>> ResolveAsync(AtlasClient api, IReadOnlyList<EventWitness> witnesses)
    {
        Versification? canon = null;
        try
        {
            canon = Versification.From(await api.Books());
        }
        catch (Exception)
        {
        }

        var units = witnesses.Select(w =>
        {
            var verses = PassageBlockBuilder.FlattenWitness(w);
            return new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: canon);
        }).ToList();

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
        // GroupBy + first-wins, not a raw ToDictionary: defensive against a duplicate Vref
        // across two witnesses in the network response (which ToDictionary would throw on).
        var resolvedByVref = resolvedFlat.GroupBy(v => v.Vref).ToDictionary(g => g.Key, g => g.First());
        return units.Select(u => new PassageSourceUnit(
            u.Verses.Select(v =>
            {
                var resolved = resolvedByVref.GetValueOrDefault(v.Vref);
                return new PassageListVerse(v.Vref, resolved?.Text ?? "", v.GroupCount, resolved?.Places, resolved?.Persons, resolved?.WordsOfChrist);
            }).ToList(), CoalesceAcrossChapters: u.CoalesceAcrossChapters, Canon: u.Canon)).ToList();
    }
}

public sealed class EventWitnessesSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Event";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not EventNode ev)
        {
            return null;
        }

        EventPage detail;
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
            return null;
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
        var registry = await FrontierProvenance.RegistryOrNull(api);

        RenderFragment body = builder =>
        {
            var seq = 0;
            if (multi)
            {
                seq = FrontierProvenance.Heading(
                    builder, seq, "PARALLEL ACCOUNTS", "event-section-heading",
                    detail.WitnessesProvenance ?? [], registry, "event-witnesses-provenance",
                    "Sources for these parallel accounts");
            }

            builder.OpenComponent<Components.PassageList>(seq++);
            builder.AddAttribute(seq++, "Units", (IReadOnlyList<PassageSourceUnit>)units);
            builder.AddAttribute(seq++, "RefTestIdPrefix", "event-witness");
            builder.AddAttribute(seq++, "ClampVerses", Components.PassageList.StandardVerseClamp);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection(multi ? "event-witnesses" : "event-witness", body);
    }
}

// A mention (a verse referencing this event without narrating it) is deliberately
// distinct from a witness/account (EventWitnessesSection's "PARALLEL ACCOUNTS") --
// rendering a mention as an account would misrepresent it as a parallel narration.
public sealed class EventMentionsSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Event";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not EventNode ev)
        {
            return null;
        }

        EventPage detail;
        try
        {
            detail = await ev.DetailAsync(api);
        }
        catch (Exception)
        {
            return null;
        }

        var mentions = detail.MentionedIn ?? [];
        if (mentions.Count == 0)
        {
            return null;
        }

        var refs = mentions.Select(v => new Components.RefsList.RefDescriptor(v, (IExplorable)new VerseNode(v))).ToList();

        var registry = await FrontierProvenance.RegistryOrNull(api);

        RenderFragment body = builder =>
        {
            var seq = 0;
            seq = FrontierProvenance.Heading(
                builder, seq, "MENTIONED IN", "event-section-heading",
                detail.MentionsProvenance ?? [], registry, "event-mentions-provenance",
                "Sources for these mentions");

            builder.OpenComponent<Components.RefsList>(seq++);
            builder.AddAttribute(seq++, "Refs", (IReadOnlyList<Components.RefsList.RefDescriptor>)refs);
            builder.AddAttribute(seq++, "TestIdPrefix", "event-mentioned-in");
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("event-mentions", body);
    }
}

public sealed class EventAnaloguesSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Event";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not EventNode ev)
        {
            return null;
        }

        EventPage detail;
        try
        {
            detail = await ev.DetailAsync(api);
        }
        catch (Exception)
        {
            return null;
        }

        var analogues = detail.Analogues ?? [];
        if (analogues.Count == 0)
        {
            return null;
        }

        var refs = analogues
            .Select(a => new Components.RefsList.RefDescriptor(a.Title, (IExplorable)new EventNode(a.Id, a.Title, EventKind.Event), a.Id))
            .ToList();

        var registry = await FrontierProvenance.RegistryOrNull(api);
        var analogueProvenance = FrontierProvenance.Distinct(analogues.Select(a => a.Provenance));

        RenderFragment body = builder =>
        {
            var seq = 0;
            seq = FrontierProvenance.Heading(
                builder, seq, "SIMILAR ACCOUNTS", "event-section-heading",
                analogueProvenance, registry, "event-analogues-provenance",
                "Sources for these similar accounts");

            builder.OpenComponent<Components.RefsList>(seq++);
            builder.AddAttribute(seq++, "Refs", (IReadOnlyList<Components.RefsList.RefDescriptor>)refs);
            builder.AddAttribute(seq++, "TestIdPrefix", "event-analogues");
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("event-analogues", body);
    }
}

public sealed class VerseParallelsSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind is "Verse" or "Passage";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        IReadOnlyList<VerseEvent> events;
        string ownVref;
        switch (node)
        {
            case VerseNode v:
                events = (await v.DetailAsync(api)).Events;
                ownVref = v.Title;
                break;
            case PassageNode p:
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
            return null;
        }

        EventPage?[] details;
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
            return null;
        }

        var qualifying = new List<(string Label, List<EventWitness> OtherWitnesses)>();
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
            return null;
        }

        List<List<PassageSourceUnit>> unitsPerEvent;
        try
        {
            // Task.WhenAll preserves input order, so unitsPerEvent[i] lines up with
            // qualifying[i] in the render fragment below.
            unitsPerEvent = (await Task.WhenAll(qualifying.Select(q => WitnessUnitsResolver.ResolveAsync(api, q.OtherWitnesses)))).ToList();
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
                builder.AddAttribute(seq++, "ClampVerses", Components.PassageList.StandardVerseClamp);
                builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
                builder.CloseComponent();
            }
        };
        return new PopoverSection("parallels", body);
    }

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

public sealed class EventChronologySection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Event";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not INarrativeAware aware)
        {
            return null;
        }

        NarrativeEventPositions positions;
        try
        {
            positions = await aware.NarrativePositionsAsync(api);
        }
        catch (Exception)
        {
            return null;
        }

        var timeline = positions.Timeline;
        if (timeline is null)
        {
            return null;
        }

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

            builder.OpenElement(seq++, "div");
            builder.AddAttribute(seq++, "class", "popover-event-nav-arrows");
            EventDateAndPlacesSection.RenderArrowNav(builder, ref seq, ctx, "prior", "event-chrono-prior-event", "event-chrono-prior-label", "global", timeline.Prior, "◂");
            EventDateAndPlacesSection.RenderArrowNav(builder, ref seq, ctx, "following", "event-chrono-following-event", "event-chrono-following-label", "global", timeline.Following, "▸");
            builder.CloseElement();

            builder.CloseElement();
            builder.CloseElement();

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

                    builder.CloseElement();
                }
                builder.CloseElement();
            }
        };
        return new PopoverSection("event-chronology", body);
    }

    // A null narrative-side leg is that narrative's own chain end, never a divergence,
    // so this short-circuits false before comparing ids.
    private static bool Diverges(NarrativeAdjacentEvent? narrativeLeg, NarrativeAdjacentEvent? timelineLeg)
        => narrativeLeg is not null && narrativeLeg.Id != timelineLeg?.Id;
}

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
            builder.AddAttribute(seq++, "class", "catechism-section-heading");
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

// Unregistered from PopoverSectionRegistry.Providers (nothing constructs this class
// today), but deliberately kept rather than deleted -- do not remove as dead code.
public sealed class VersePersonsSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind is "Verse" or "Passage";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        string wireId;
        switch (node)
        {
            case VerseNode v:
                wireId = NodeIds.Of(NodeKind.TextUnit, v.Title);
                break;
            case PassageNode p:
                wireId = NodeIds.Of(NodeKind.TextUnit, CanonRef.FirstVerseOf(p.Title));
                break;
            default:
                return null;
        }

        EdgePage page;
        try
        {
            page = await ctx.Graph.Edges(wireId, EdgeSectionRegistry.Mentions.EdgeKind, cursor: null, limit: EdgeSectionRegistry.Mentions.InitialClamp);
        }
        catch (Exception)
        {
            return null;
        }

        var persons = page.Entries.Where(e => e.Node.Kind == PositionKind.Person).ToList();
        if (persons.Count == 0)
        {
            return null;
        }

        var mayHaveMore = page.Next is not null;
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
                var id = entry.Node.Id;
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
        NodeCard card;
        EdgePage page;
        try
        {
            var cardTask = person.CardAsync(() => ctx.Graph.Card(person.PersonId));
            var pageTask = ctx.Graph.Edges(person.PersonId, spec.EdgeKind, cursor: null, limit: spec.InitialClamp);
            await Task.WhenAll(cardTask, pageTask);
            card = cardTask.Result;
            page = pageTask.Result;
        }
        catch (Exception)
        {
            return null;
        }

        var total = card.EdgeSummary.FirstOrDefault(s => s.Kind == spec.EdgeKind)?.Count ?? page.Entries.Count;

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "details");
            builder.AddAttribute(seq++, "class", "person-mentions-disclosure");
            builder.AddAttribute(seq++, "data-testid", "person-mentions-disclosure");
            builder.OpenElement(seq++, "summary");
            builder.AddAttribute(seq++, "class", "catechism-section-heading");
            builder.AddAttribute(seq++, "data-testid", "person-mentions-heading");
            builder.AddContent(seq++, $"MENTIONED IN SCRIPTURE ({total})");
            builder.CloseElement();
            builder.OpenComponent<Components.PersonMentionsList>(seq++);
            builder.AddAttribute(seq++, "PersonId", person.PersonId);
            builder.AddAttribute(seq++, "Provenance", card.Provenance);
            builder.AddAttribute(seq++, "InitialEntries", page.Entries);
            builder.AddAttribute(seq++, "InitialNext", page.Next);
            builder.AddAttribute(seq++, "TotalCount", total);
            builder.AddAttribute(seq++, "ShowHeading", false);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
            builder.CloseElement();
        };
        return new PopoverSection("person-mentions", body);
    }
}

public sealed class CommentaryItemProseSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "CommentaryItem";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not CommentaryItemNode item)
        {
            return null;
        }

        NodeCard card;
        try
        {
            card = await ctx.Graph.Card(item.Id);
        }
        catch (Exception)
        {
            return null;
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

public sealed class CatechismInConcordSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Catechism";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not CatechismNode item)
        {
            return null;
        }

        List<NodeRef> units;
        try
        {
            // The whole frontier, not the first page: an item's catechism-link edges are
            // mostly its proof verses (the First Commandment alone has 200+), and the
            // Concord paragraphs sit after them.
            units = (await CatechismLinks.AllTargetsAsync(ctx.Graph, NodeIds.Of(NodeKind.CatechismItem, item.Id)))
                .Where(n => n.Kind == PositionKind.TextUnit && NodeIds.LocalPart(n).StartsWith("BoC ", StringComparison.Ordinal))
                .ToList();
        }
        catch (Exception)
        {
            return null;
        }

        if (units.Count == 0)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "catechism-section-heading");
            builder.AddAttribute(seq++, "data-testid", "catechism-in-concord-heading");
            builder.AddContent(seq++, $"IN THE BOOK OF CONCORD ({units.Count})");
            builder.CloseElement();

            builder.OpenComponent<Components.ConcordUnitList>(seq++);
            builder.AddAttribute(seq++, "Items", (IReadOnlyList<NodeRef>)units);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("catechism-in-concord", body);
    }
}

public sealed class ConcordSmallCatechismSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "ConcordUnit";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not ConcordUnitNode unit)
        {
            return null;
        }

        IReadOnlyList<CatechismRef> items;
        try
        {
            items = (await CatechismLinks.AllTargetsAsync(ctx.Graph, unit.NodeId))
                .Where(n => n.Kind == PositionKind.CatechismItem)
                .Select(n => new CatechismRef(id: NodeIds.LocalPart(n), name: n.Label, provenance: [], question: null))
                .ToList();
        }
        catch (Exception)
        {
            return null;
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
            builder.AddAttribute(seq++, "data-testid", "concord-small-catechism-heading");
            builder.AddContent(seq++, $"THE SMALL CATECHISM ({items.Count})");
            builder.CloseElement();

            builder.OpenComponent<Components.CatechismList>(seq++);
            builder.AddAttribute(seq++, "Items", (IReadOnlyList<CatechismRef>)items);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<IExplorable>(ctx, n => ctx.PushAsync(n)));
            builder.CloseComponent();
        };
        return new PopoverSection("concord-small-catechism", body);
    }
}

// Registering ANY section for Kind == "ConcordUnit" (see ConcordSmallCatechismSection)
// makes sections replace ConcordUnitNode.BodyAsync entirely rather than supplement it,
// so this section carries the paragraph's own text or it would silently disappear.
public sealed class ConcordUnitTextSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "ConcordUnit";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not ConcordUnitNode unit)
        {
            return null;
        }

        string text;
        try
        {
            text = await unit.TextAsync(ctx.Graph);
        }
        catch (Exception)
        {
            return null;
        }

        if (string.IsNullOrWhiteSpace(text))
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-concord-text");
            builder.AddAttribute(2, "data-testid", "concord-unit-text");
            builder.AddContent(3, text);
            builder.CloseElement();
        };
        return new PopoverSection("concord-unit-text", body);
    }
}

internal static class CatechismLinks
{
    private const int PageSize = 200;

    public static async Task<List<NodeRef>> AllTargetsAsync(IExplorableClient graph, string nodeId)
    {
        var targets = new List<NodeRef>();
        int? cursor = null;
        do
        {
            var page = await graph.Edges(nodeId, EdgeKind.CatechismLink, cursor: cursor, limit: PageSize);
            targets.AddRange(page.Entries.Select(e => e.Node));
            cursor = page.Next;
        }
        while (cursor is not null);
        return targets;
    }
}

file static class PersonSectionRendering
{
    public static int Heading(RenderTreeBuilder builder, int seq, string text, string testid)
    {
        builder.OpenElement(seq++, "p");
        builder.AddAttribute(seq++, "class", "catechism-section-heading");
        builder.AddAttribute(seq++, "data-testid", testid);
        builder.AddContent(seq++, text);
        builder.CloseElement();
        return seq;
    }

    public static int Chips(RenderTreeBuilder builder, int seq, string testidPrefix, IEnumerable<(string Id, string Label, IExplorable Node)> chips, IPopoverSectionContext ctx)
    {
        builder.OpenElement(seq++, "div");
        builder.AddAttribute(seq++, "class", "popover-catechism-list person-chips");
        foreach (var (id, label, node) in chips)
        {
            var target = node;
            builder.OpenElement(seq++, "button");
            builder.AddAttribute(seq++, "type", "button");
            builder.AddAttribute(seq++, "class", "popover-catechism-item explorable");
            builder.AddAttribute(seq++, "data-testid", $"{testidPrefix}-{id.Replace(':', '-').Replace('.', '-')}");
            builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create<MouseEventArgs>(ctx, () => ctx.PushAsync(target)));
            builder.AddContent(seq++, label);
            builder.CloseElement();
        }
        builder.CloseElement();
        return seq;
    }
}

public sealed class PersonLifeSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Person";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not PersonNode person)
        {
            return null;
        }

        PersonLife? life;
        try
        {
            life = (await person.CardAsync(() => ctx.Graph.Card(person.PersonId))).Person;
        }
        catch (Exception)
        {
            return null;
        }

        if (life is null)
        {
            return null;
        }

        if (LineOf(life) is not { } line)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = PersonSectionRendering.Heading(builder, 0, person.Title == "Jesus" ? "EARTHLY LIFE" : "LIFE", "person-life-heading");
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "person-life-line");
            builder.AddAttribute(seq++, "data-testid", "person-life");
            builder.AddContent(seq++, line);
            builder.CloseElement();
            if (life.Eternal && life.EternalGrounds.Count > 0)
            {
                seq = PersonSectionRendering.Chips(builder, seq, "person-eternal-ground", life.EternalGrounds.Select(g => (g, g, (IExplorable)new VerseNode(g))), ctx);
            }
        };
        return new PopoverSection("person-life", body);
    }

    public static string? LineOf(PersonLife life)
    {
        if (life.Eternal)
        {
            return "Eternal";
        }

        if (life.Birth is not null || life.Death is not null)
        {
            var parts = new List<string>();
            if (life.Birth is { } born) parts.Add(PersonNode.Born(born));
            if (life.Death is { } died) parts.Add(PersonNode.Died(died));
            return string.Join(" \u00b7 ", parts);
        }

        return life.First is { } first && life.Last is { } last
            ? PersonNode.MentionedAcross(first, last)
            : null;
    }
}

public sealed class PersonEventsSection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Person";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not PersonNode person)
        {
            return null;
        }

        List<NodeRef> events;
        try
        {
            events = (await ctx.Graph.Edges(person.PersonId, EdgeKind.ParticipatesIn, cursor: null, limit: 200)).Entries.Select(e => e.Node).ToList();
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
            var seq = PersonSectionRendering.Heading(builder, 0, $"EVENTS ({events.Count})", "person-events-heading");
            PersonSectionRendering.Chips(builder, seq, "person-event", events.Select(e => (NodeIds.LocalPart(e), e.Label, (IExplorable)new EventNode(NodeIds.LocalPart(e), e.Label))), ctx);
        };
        return new PopoverSection("person-events", body);
    }
}

public sealed class PersonFamilySection : IPopoverSectionProvider
{
    public bool AppliesTo(IExplorable node) => node.Kind == "Person";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        if (node is not PersonNode person)
        {
            return null;
        }

        List<NodeRef> parents, partners, children;
        var siblings = new List<NodeRef>();
        try
        {
            parents = (await ctx.Graph.Edges(person.PersonId, EdgeKind.ChildOf, cursor: null, limit: 200)).Entries.Select(e => e.Node).ToList();
            partners = (await ctx.Graph.Edges(person.PersonId, EdgeKind.PartnerOf, cursor: null, limit: 200)).Entries.Select(e => e.Node).ToList();
            children = (await ctx.Graph.Edges(person.PersonId, EdgeKind.ParentOf, cursor: null, limit: 200)).Entries.Select(e => e.Node).ToList();
            foreach (var parent in parents)
            {
                var theirs = (await ctx.Graph.Edges(parent.Id, EdgeKind.ParentOf, cursor: null, limit: 200)).Entries.Select(e => e.Node);
                foreach (var s in theirs)
                {
                    if (s.Id != person.PersonId && siblings.All(x => x.Id != s.Id))
                    {
                        siblings.Add(s);
                    }
                }
            }
        }
        catch (Exception)
        {
            return null;
        }

        if (parents.Count + partners.Count + children.Count + siblings.Count == 0)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = PersonSectionRendering.Heading(builder, 0, "FAMILY", "person-family-heading");
            foreach (var (label, testid, people) in new[] { ("Parents", "parents", parents), ("Partners", "partners", partners), ("Children", "children", children), ("Siblings", "siblings", siblings) })
            {
                if (people.Count == 0)
                {
                    continue;
                }

                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "person-family-label");
                builder.AddAttribute(seq++, "data-testid", $"person-family-{testid}");
                builder.AddContent(seq++, $"{label} ({people.Count})");
                builder.CloseElement();
                seq = PersonSectionRendering.Chips(builder, seq, $"person-{testid}", people.Select(p => (NodeIds.LocalPart(p), p.Label, (IExplorable)new PersonNode(p.Id, p.Label))), ctx);
            }
        };
        return new PopoverSection("person-family", body);
    }
}
