using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.Rendering;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

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

        var chapter = await chapterNode.Load(api);

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
                    builder.OpenElement(seq++, "button");
                    builder.AddAttribute(seq++, "type", "button");
                    builder.AddAttribute(seq++, "class", "popover-event-row popover-event-row-button explorable");
                    builder.AddAttribute(seq++, "data-testid", $"chapter-card-heading-{eventId}");
                    builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new PopoverOpening.Legacy(new EventNode(eventId, title)), EdgeKind.Attests)));
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
                    var place = new PopoverOpening.Explore(new NodePosition(p.Node));
                    builder.OpenElement(seq++, "button");
                    builder.AddAttribute(seq++, "type", "button");
                    builder.AddAttribute(seq++, "class", "popover-event-row popover-event-row-button explorable");
                    builder.AddAttribute(seq++, "data-testid", $"chapter-card-place-{placeId}");
                    builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(place, EdgeKind.Mentions)));
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
                var vDetail = await v.DetailAsync(api);
                compactText = vDetail.Text;
                textProvenance = new[] { ProvenanceResolver.NormalizeId(vDetail.Provenance) };
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

        var focalVerses = (await api.ChapterText(book, chapter)).Between(focalFrom, focalTo);

        var registry = await FrontierProvenance.Registry(api);

        RenderFragment fragment = builder =>
        {
            var seq = 0;
            builder.OpenComponent<Components.VerseTextSection>(seq++);
            builder.AddAttribute(seq++, "Book", book);
            builder.AddAttribute(seq++, "Chapter", chapter);
            builder.AddAttribute(seq++, "FocalFromVerse", focalFrom);
            builder.AddAttribute(seq++, "FocalToVerse", focalTo);
            builder.AddAttribute(seq++, "CompactText", compactText);
            builder.AddAttribute(seq++, "FocalVerses", focalVerses);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<PopoverOpening>(ctx, opening => ctx.PushAsync(opening, EdgeKind.Mentions)));
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
    internal static async Task<SourcesDocument?> Registry(AtlasClient api) =>
        (await default(Request).Fetch(() => api.Sources())).Match<SourcesDocument?>(arrived: registry => registry, failed: () => null, superseded: () => null);

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
    private const int XrefsShown = 3;

    public bool AppliesTo(IExplorable node) => node.Kind is "Verse" or "Passage";

    public async Task<PopoverSection?> ResolveAsync(IExplorable node, AtlasClient api, IPopoverSectionContext ctx)
    {
        IReadOnlyList<CrossRef> xrefs;
        IReadOnlyList<string> xrefProvenance = Array.Empty<string>();
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

        if (xrefs.Count == 0)
        {
            return null;
        }

        // Only the first InitialClamp targets are fetched eagerly (xrefs here is uncapped --
        // e.g. GEN.1.1 carries 61 -- and the popover only ever shows a few before "reveal
        // more"); targets beyond that keep their full identity and are resolved lazily via
        // ResolveUnits on first reveal, never narrowed to a single-verse preview.
        var spans = xrefs.Select(x => (Xref: x, Span: CanonRef.TargetSpan(x.Target))).ToList();
        var eagerSpans = spans.Take(XrefsShown).ToList();
        var lazySpans = spans.Skip(XrefsShown).ToList();
        var units = await ResolveUnits(api, eagerSpans);
        var registry = await FrontierProvenance.Registry(api);

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
            builder.AddAttribute(seq++, "Cap", ctx.XrefEntryPoint ? XrefsShown : (ctx.OtherContextSectionCount > 0 ? 2 : XrefsShown));
            builder.AddAttribute(seq++, "MoreTestId", "xrefs-more");
            builder.AddAttribute(seq++, "CollapseTestId", "xrefs-collapse");
            builder.AddAttribute(seq++, "RevealNoun", "cross-references");
            builder.AddAttribute(seq++, "ClampVerses", Components.PassageList.StandardVerseClamp);
            builder.AddAttribute(seq++, "ExploreAsVerse", true);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<PopoverOpening>(ctx, opening => ctx.PushAsync(opening, EdgeKind.Cites)));
            builder.AddAttribute(seq++, "OnToggleSelect", EventCallback.Factory.Create<NodeRef>(ctx, node => ctx.ToggleSelectAsync(node)));
            builder.CloseComponent();
        };
        return new PopoverSection("xrefs", body);
    }

    private static async Task<List<PassageSourceUnit>> ResolveUnits(AtlasClient api, List<(CrossRef Xref, (string Book, int Chapter, int FromVerse, int ToVerse)? Span)> targets)
    {
        var chapterKeys = targets.Where(t => t.Span is not null).Select(t => (t.Span!.Value.Book, t.Span.Value.Chapter)).Distinct().ToList();
        var chapters = new Dictionary<(string, int), ChapterText>();
        var fetched = await Task.WhenAll(chapterKeys.Select(k => api.ChapterText(k.Item1, k.Item2)));
        foreach (var (key, chapter) in chapterKeys.Zip(fetched))
        {
            chapters[key] = chapter;
        }

        var units = new List<PassageSourceUnit>();
        foreach (var (x, span) in targets)
        {
            if (span is { } s && chapters.TryGetValue((s.Book, s.Chapter), out var chapter))
            {
                var verses = chapter.Between(s.FromVerse, s.ToVerse).Select(PassageListVerse.Of).ToList();
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

        if (items.Count == 0)
        {
            return null;
        }

        var registry = await FrontierProvenance.Registry(api);

        RenderFragment body = builder =>
        {
            var seq = 0;
            seq = FrontierProvenance.Heading(
                builder, seq, "THE SMALL CATECHISM", "catechism-section-heading",
                catechismProvenance, registry, "catechism-provenance", "Sources for this catechism mapping");

            builder.OpenComponent<Components.CatechismList>(seq++);
            builder.AddAttribute(seq++, "Items", items);
            builder.AddAttribute(seq++, "Cap", CatechismDefaultCap);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<PopoverOpening>(ctx, opening => ctx.PushAsync(opening, EdgeKind.CatechismLink)));
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

        var detail = await item.DetailAsync(api);

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

        var detail = await item.DetailAsync(api);

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

        var detail = await item.DetailAsync(api);

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

        var detail = await item.DetailAsync(api);

        if (detail.Verses.Count == 0)
        {
            return null;
        }

        var servedText = new Dictionary<string, TextUnit>();
        var chapterKeys = detail.Verses.Select(v => CanonRef.ParseVerse(v.Vref)).Select(p => (p.Book, p.Chapter)).Distinct().ToList();
        var fetched = await Task.WhenAll(chapterKeys.Select(k => api.ChapterText(k.Book, k.Chapter)));
        foreach (var unit in fetched.SelectMany(chapterText => chapterText.Units))
        {
            servedText[unit.Ref] = unit;
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
            currentGroup.Add(servedText.TryGetValue(v.Vref, out var unit) ? PassageListVerse.Of(unit) : new PassageListVerse(v.Vref, v.Text));
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
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<PopoverOpening>(ctx, opening => ctx.PushAsync(opening, EdgeKind.CatechismLink)));
            builder.CloseComponent();
        };
        return new PopoverSection("catechism-scriptures", body);
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

        var events = (await v.DetailAsync(api)).Events;

        var dated = events.Where(e => e.Kind == EventKind.Event).ToList();
        if (dated.Count == 0)
        {
            return null;
        }

        return new PopoverSection("event-membership", RenderRows(EventKind.Event, dated, ctx, await FrontierProvenance.Registry(api)));
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
            // .explorable-quiet replaces .explorable (never both): a general-kind event
            // is not part of time traversal, so its row must not look traversable.
            var explorableClass = e.Kind == EventKind.General ? "explorable-quiet" : "explorable";
            builder.OpenElement(seq++, "button");
            builder.AddAttribute(seq++, "type", "button");
            builder.AddAttribute(seq++, "class", $"popover-event-row popover-event-row-button {explorableClass}");
            builder.AddAttribute(seq++, "data-testid", $"verse-event-{id}");
            builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new PopoverOpening.Legacy(new EventNode(id, label)), EdgeKind.Attests)));
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

        var events = (await v.DetailAsync(api)).Events;

        var general = events.Where(e => e.Kind == EventKind.General).ToList();
        if (general.Count == 0)
        {
            return null;
        }

        return new PopoverSection("passage-membership", VerseEventMembershipSection.RenderRows(EventKind.General, general, ctx, await FrontierProvenance.Registry(api)));
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

        var detail = await ev.DetailAsync(api);

        var registry = await FrontierProvenance.Registry(api);
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
        detail = await ev.DetailAsync(api);
        when = (await ev.CardAsync(api)).Event?.When;

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
                    valueBuilder.AddAttribute(vseq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new PopoverOpening.Legacy(new YearNode(when, ev.Identity)), EdgeKind.DatedBy)));
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

    internal static void RenderArrowNav(RenderTreeBuilder builder, ref int seq, IPopoverSectionContext ctx, NarrativeArrow direction, string eventTestIdPrefix, string roleTestIdPrefix, string idSuffix, NarrativeAdjacentEvent? adjacent, string glyph, bool inline = false, string? inlinePrefixText = null)
    {
        builder.OpenComponent<Components.ArrowNav>(seq++);
        builder.AddAttribute(seq++, "Direction", direction.Name);
        builder.AddAttribute(seq++, "EventTestIdPrefix", eventTestIdPrefix);
        builder.AddAttribute(seq++, "RoleTestIdPrefix", roleTestIdPrefix);
        builder.AddAttribute(seq++, "IdSuffix", idSuffix);
        builder.AddAttribute(seq++, "Adjacent", adjacent);
        builder.AddAttribute(seq++, "Glyph", glyph);
        builder.AddAttribute(seq++, "Inline", inline);
        builder.AddAttribute(seq++, "InlinePrefixText", inlinePrefixText);
        builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<PopoverOpening>(ctx, opening => ctx.PushAsync(opening, direction.Via)));
        builder.CloseComponent();
    }
}

public sealed record NarrativeArrow(string Name, EdgeKind Via)
{
    public static readonly NarrativeArrow Prior = new("prior", EdgeKind.PrecedesIn);
    public static readonly NarrativeArrow Following = new("following", EdgeKind.FollowsIn);
}

file static class WitnessUnitsResolver
{
    public static async Task<List<PassageSourceUnit>> ResolveAsync(AtlasClient api, IReadOnlyList<EventAccount> accounts)
    {
        var verses = await Task.WhenAll(accounts.Select(account => VerseTextResolver.ResolveSpansAsync(api, account.Runs)));
        return accounts.Zip(verses).Select(PassageSourceUnit (pair) => new AccountSourceUnit(pair.Second, pair.First)).ToList();
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
        IReadOnlyList<EventAccount> accounts;
        detail = await ev.DetailAsync(api);
        accounts = await EventAccounts.ReadAsync(ctx.Graph, ev.EventId);

        if (accounts.Count == 0)
        {
            return null;
        }

        var units = await WitnessUnitsResolver.ResolveAsync(api, accounts);

        var multi = units.Count > 1;
        var registry = await FrontierProvenance.Registry(api);

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
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<PopoverOpening>(ctx, opening => ctx.PushAsync(opening, EdgeKind.AttestedIn)));
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

        var detail = await ev.DetailAsync(api);

        var mentions = detail.MentionedIn ?? [];
        if (mentions.Count == 0)
        {
            return null;
        }

        var refs = mentions.Select(v => new Components.RefsList.RefDescriptor(v, new PopoverOpening.Legacy(new VerseNode(v)))).ToList();

        var registry = await FrontierProvenance.Registry(api);

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
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<PopoverOpening>(ctx, opening => ctx.PushAsync(opening, EdgeKind.MentionedIn)));
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

        var detail = await ev.DetailAsync(api);

        var analogues = detail.Analogues ?? [];
        if (analogues.Count == 0)
        {
            return null;
        }

        var refs = analogues
            .Select(a => new Components.RefsList.RefDescriptor(a.Title, new PopoverOpening.Legacy(new EventNode(a.Id, a.Title)), a.Id))
            .ToList();

        var registry = await FrontierProvenance.Registry(api);
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
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<PopoverOpening>(ctx, opening => ctx.PushAsync(opening, EdgeKind.AnalogousTo)));
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
                events = (await api.Verse(ownVref)).Events;
                break;
            default:
                return null;
        }

        if (events.Count == 0)
        {
            return null;
        }

        var own = CanonRef.BibleRefOf(ownVref);
        var accountsOfEach = await Task.WhenAll(events.Select(async e =>
        {
            return await EventAccounts.ReadAsync(ctx.Graph, e.Id);
        }));
        var qualifying = events.Zip(accountsOfEach)
            .Select(pair => (pair.First.Label, OtherAccounts: pair.Second.Where(account => !account.Reads(own)).ToList()))
            .Where(q => q.OtherAccounts.Count > 0)
            .ToList();

        if (qualifying.Count == 0)
        {
            return null;
        }

        var unitsPerEvent = await Task.WhenAll(qualifying.Select(q => WitnessUnitsResolver.ResolveAsync(api, q.OtherAccounts)));

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
                builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<PopoverOpening>(ctx, opening => ctx.PushAsync(opening, EdgeKind.Parallel)));
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

        var positions = await aware.NarrativePositionsAsync(api);

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
            EventDateAndPlacesSection.RenderArrowNav(builder, ref seq, ctx, NarrativeArrow.Prior, "event-chrono-prior-event", "event-chrono-prior-label", "global", timeline.Prior, "◂");
            EventDateAndPlacesSection.RenderArrowNav(builder, ref seq, ctx, NarrativeArrow.Following, "event-chrono-following-event", "event-chrono-following-label", "global", timeline.Following, "▸");
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
                        EventDateAndPlacesSection.RenderArrowNav(builder, ref seq, ctx, NarrativeArrow.Prior, "event-story-thread-prior-event", "event-story-thread-prior-label", row.NarrativeId, row.Prior, "←", inline: true);
                    }
                    if (priorDiverges && followingDiverges)
                    {
                        builder.AddContent(seq++, " · ");
                    }
                    if (followingDiverges)
                    {
                        EventDateAndPlacesSection.RenderArrowNav(builder, ref seq, ctx, NarrativeArrow.Following, "event-story-thread-following-event", "event-story-thread-following-label", row.NarrativeId, row.Following, "→", inline: true, inlinePrefixText: "next ");
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

        var verses = await VerseTextResolver.ResolveAsync(api, delta.Verses);
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
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<PopoverOpening>(ctx, opening => ctx.PushAsync(opening, EdgeKind.JustifiedBy)));
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

        var mentions = await Paging.First(ctx.Graph, wireId, EdgeKind.Mentions);
        var persons = mentions.Items.Nodes().Where(n => n.Kind == NodeKind.Person).ToList();
        if (persons.Count == 0)
        {
            return null;
        }

        var mayHaveMore = mentions.Next is not null;
        RenderFragment body = builder =>
        {
            var seq = 0;
            builder.OpenElement(seq++, "p");
            builder.AddAttribute(seq++, "class", "catechism-section-heading");
            builder.AddAttribute(seq++, "data-testid", "persons-section-heading");
            builder.AddContent(seq++, "PERSONS");
            builder.CloseElement();

            foreach (var person in persons)
            {
                var id = person.Id;
                var label = person.Label;
                builder.OpenElement(seq++, "button");
                builder.AddAttribute(seq++, "type", "button");
                builder.AddAttribute(seq++, "class", "popover-event-row popover-event-row-button explorable");
                builder.AddAttribute(seq++, "data-testid", $"verse-person-{Slug(label)}");
                builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create(ctx, () => ctx.PushAsync(new PopoverOpening.Legacy(new PersonNode(id, label)), EdgeKind.Mentions)));
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

        var cardTask = person.CardAsync(() => ctx.Graph.Card(person.PersonId));
        var mentionsTask = Paging.Window(ctx.Graph, person.PersonId, EdgeKind.MentionedIn);
        var (card, mentions) = (await cardTask, await mentionsTask);
        var total = card.EdgeSummary.FirstOrDefault(s => s.Kind == EdgeKind.MentionedIn)?.Count ?? mentions.Shown.Count();

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
            builder.AddAttribute(seq++, "Provenance", card.Provenance);
            builder.AddAttribute(seq++, "Mentions", mentions);
            builder.AddAttribute(seq++, "TotalCount", total);
            builder.AddAttribute(seq++, "ShowHeading", false);
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<PopoverOpening>(ctx, opening => ctx.PushAsync(opening, EdgeKind.MentionedIn)));
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

        var card = await ctx.Graph.Card(item.Identity.Id);

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
        units = (await CatechismLinks.AllTargetsAsync(ctx.Graph, NodeIds.Of(NodeKind.CatechismItem, item.Id)))
            .Where(n => n.Kind == NodeKind.TextUnit && NodeIds.LocalPart(n).StartsWith("BoC ", StringComparison.Ordinal))
            .ToList();

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
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<PopoverOpening>(ctx, opening => ctx.PushAsync(opening, EdgeKind.CatechismLink)));
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
        items = (await CatechismLinks.AllTargetsAsync(ctx.Graph, unit.NodeId))
            .Where(n => n.Kind == NodeKind.CatechismItem)
            .Select(n => new CatechismRef(id: NodeIds.LocalPart(n), name: n.Label, provenance: [], question: null))
            .ToList();

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
            builder.AddAttribute(seq++, "OnExplore", EventCallback.Factory.Create<PopoverOpening>(ctx, opening => ctx.PushAsync(opening, EdgeKind.CatechismLink)));
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

        var text = await unit.TextAsync(ctx.Graph);

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
    public static async Task<List<NodeRef>> AllTargetsAsync(IExplorableClient graph, string nodeId) =>
        (await Paging.Whole(graph, nodeId, EdgeKind.CatechismLink)).Nodes().ToList();
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

    public static int Chips(RenderTreeBuilder builder, int seq, string testidPrefix, IEnumerable<(string Id, string Label, IExplorable Node)> chips, IPopoverSectionContext ctx, EdgeKind via)
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
            builder.AddAttribute(seq++, "onclick", EventCallback.Factory.Create<MouseEventArgs>(ctx, () => ctx.PushAsync(new PopoverOpening.Legacy(target), via)));
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

        PersonLife? life = (await person.CardAsync(() => ctx.Graph.Card(person.PersonId))).Person;

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
                seq = PersonSectionRendering.Chips(builder, seq, "person-eternal-ground", life.EternalGrounds.Select(g => (g, g, (IExplorable)new VerseNode(g))), ctx, EdgeKind.MentionedIn);
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

        var events = (await Paging.Whole(ctx.Graph, person.PersonId, EdgeKind.ParticipatesIn)).Nodes().ToList();

        if (events.Count == 0)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = PersonSectionRendering.Heading(builder, 0, $"EVENTS ({events.Count})", "person-events-heading");
            PersonSectionRendering.Chips(builder, seq, "person-event", events.Select(e => (NodeIds.LocalPart(e), e.Label, (IExplorable)new EventNode(NodeIds.LocalPart(e), e.Label))), ctx, EdgeKind.ParticipatesIn);
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

        List<EdgeEntry> parents, children;
        List<NodeRef> spouses, brethren;
        var siblings = new List<NodeRef>();
        parents = (await Paging.Whole(ctx.Graph, person.PersonId, EdgeKind.ChildOf)).ToList();
        spouses = (await Paging.Whole(ctx.Graph, person.PersonId, EdgeKind.SpouseOf)).Nodes().ToList();
        children = (await Paging.Whole(ctx.Graph, person.PersonId, EdgeKind.ParentOf)).ToList();
        brethren = (await Paging.Whole(ctx.Graph, person.PersonId, EdgeKind.BrethrenOf)).Nodes().ToList();
        foreach (var parent in parents.Where(p => Kinship.MakesSiblings(Kinship.Of(p))).Nodes())
        {
            var theirs = (await Paging.Whole(ctx.Graph, parent.Id, EdgeKind.ParentOf)).Where(e => Kinship.MakesSiblings(Kinship.Of(e))).Nodes();
            foreach (var s in theirs)
            {
                if (!PositionIdentity.Comparer.Equals(s, person.Identity) && !siblings.Contains(s, PositionIdentity.Comparer))
                {
                    siblings.Add(s);
                }
            }
        }

        var groups = Kinship.Groups(parents, spouses, children, siblings, brethren);
        if (groups.Count == 0)
        {
            return null;
        }

        RenderFragment body = builder =>
        {
            var seq = PersonSectionRendering.Heading(builder, 0, "FAMILY", "person-family-heading");
            foreach (var group in groups)
            {
                builder.OpenElement(seq++, "p");
                builder.AddAttribute(seq++, "class", "person-family-label");
                builder.AddAttribute(seq++, "data-testid", $"person-family-{group.TestId}");
                builder.AddContent(seq++, group.Heading);
                builder.CloseElement();
                seq = PersonSectionRendering.Chips(builder, seq, $"person-{group.TestId}", group.People.Select(p => (NodeIds.LocalPart(p), p.Label, (IExplorable)new PersonNode(p.Id, p.Label))), ctx, group.Via);
            }
        };
        return new PopoverSection("person-family", body);
    }
}
