using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>A whole chapter (spec node table row "Passage/Chapter/Book").
///
/// M-D3 (owner rulings U4/B3): "when you're reading a chapter, you're in
/// its focus. you can focus further by clicking chapter heading and you get
/// metadata and context. it doesn't make sense to just return the whole
/// chapter" -- restated after a controller misread ("yielding the chapter
/// focus" was wrong and superseded): the READING VIEW itself already IS the
/// chapter's own focus (Reader.razor rendering every verse, unchanged); the
/// heading is a FURTHER-focus affordance onto the chapter AS A THING THE
/// GRAPH KNOWS ABOUT -- container title, position in book, and an honest,
/// graph-sourced summary of what touches it -- NEVER the chapter's own
/// verse text, NEVER just its first verse (B3, the standing bug this
/// rewrite fixes: <see cref="BodyAsync"/> used to preview
/// <c>chapter.Verses[0].Text</c>, exactly "the stupid first verse is the
/// visible thing" the owner's own live report named). The real card content
/// is <see cref="PopoverSectionProviders.ChapterCardSection"/> (this node
/// gets its OWN registry slot, like Verse/Passage/Place/Event/Catechism/
/// PolityDelta/Person before it -- see that section's own doc comment);
/// <see cref="BodyAsync"/> below survives only as the same "defensive,
/// never-called-in-practice fallback" every other now-registry-driven node
/// kind keeps (PlaceNode's own doc comment names the precedent).
/// </summary>
public sealed class ChapterNode : IExplorable
{
    private readonly string _book;
    private readonly int _chapter;
    private readonly AsyncMemo<ChapterOut> _loaded = new();

    /// <summary>Position in book ("Chapter N of M") -- Reader.razor's own
    /// already-loaded TOC entry count for this book, passed straight
    /// through (no fetch of its own); null if unavailable, in which case
    /// <see cref="PopoverSectionProviders.ChapterCardSection"/> renders the
    /// bare chapter number only, conditional presence, never a fabricated
    /// "of ?" placeholder.</summary>
    public int? TotalChapters { get; }

    /// <summary>Reader.razor's own already-fetched chapter, when this node
    /// is opened FOR the chapter currently on screen (the overwhelming
    /// majority of real opens) -- <see cref="Load"/> reuses it instead of a
    /// redundant re-fetch. Left null for a heading reached any other way.</summary>
    public ChapterOut? AlreadyLoaded { get; init; }

    public string Book => _book;
    public int Chapter => _chapter;

    public ChapterNode(string book, int chapter, int? totalChapters = null)
    {
        _book = book;
        _chapter = chapter;
        TotalChapters = totalChapters;
    }

    public string Title => $"{_book}.{_chapter}";
    public string Kind => "Chapter";

    // HATCH-DELIVERABLE-1 (2026-09-07, deliverability audit -- see
    // EventNode.ExploreAsync's own header comment for the law this fix
    // round applies): "popover-chip-map" below is UNCONDITIONAL, unlike
    // EventNode's (now Places-gated) or AuthorNode's (already WritePlace-
    // gated) own map chips -- the resulting `/world?ref=...` scene resolves
    // server-side against this BOOK's own WritePlace (BookMetaDto), which
    // AuthorNode.ExploreAsync already proves can be absent for a real book
    // (its own `meta.WritePlace is null` guard). DISCLOSED, NOT FIXED this
    // round: checking it here would need this SAME BookMeta fetch
    // (`api.Verse($"{_book}.1.1")`, AuthorNode's own `Load`) that
    // ChapterNode.ExploreAsync does not otherwise make (it is synchronous
    // today) -- the ticket's own instruction is explicit ("disclose any
    // chip where deliverability can't be known client-side without extra
    // fetches -- do NOT add fetches for this"). A future ticket that
    // threads BookMeta through ChapterNode/BookNode (or resolves this
    // server-side, mirroring `/api/scene`'s own emptiness) can close this;
    // not built here.
    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        IReadOnlyList<Exploration> list = new[]
        {
            new Exploration("Show on /world", "popover-chip-map",
                new ExplorationTarget.NavigateWorld($"ref={Uri.EscapeDataString(Title)}")),
            new Exploration("Read in context", "popover-chip-context",
                new ExplorationTarget.NavigateReader(_book, _chapter, null)),
            new Exploration("About this book", "popover-chip-book",
                new ExplorationTarget.Push(new AuthorNode(_book))),
        };
        return Task.FromResult(list);
    }

    /// Defensive fallback ONLY -- unreachable while
    /// <see cref="PopoverSectionProviders.ChapterCardSection"/> is
    /// registered (it claims every "Chapter"-kind node, so ExplorerPopover
    /// never calls this). Deliberately minimal and text-only (no verse
    /// echo, no fetch beyond what Load already does) -- if this DOES ever
    /// render (a future caller that renders a node's body outside a real
    /// ExplorerPopover, the same caveat PlaceNode's own fallback carries),
    /// it must still never violate B3's own law.
    public async Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        var chapter = await Load(api);
        var count = chapter.Verses.Count;
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-meta");
            builder.AddContent(2, $"{count} verse{(count == 1 ? "" : "s")}.");
            builder.CloseElement();
        };
        return fragment;
    }

    // PERF-3 re-review fix round 2: migrated to AsyncMemo alongside its
    // siblings -- "cheap insurance," the re-review's own words, not a live
    // bug (this method's only caller today is ChapterCardSection, the sole
    // registered "Chapter"-kind provider; BodyAsync's own call is the
    // documented dead fallback above, unreachable while that provider is
    // registered) -- but the whole point of AsyncMemo is that the old
    // `??=`-style idiom dies completely in this file, not "mostly."
    // AlreadyLoaded's own short-circuit (Reader.razor's already-fetched
    // chapter, reused with zero new fetch) is preserved exactly -- a plain
    // Task.FromResult wrapping it costs nothing extra and still lets
    // AsyncMemo cache/dedupe/reset it the same way as a real fetch.
    public Task<ChapterOut> Load(AtlasClient api) => _loaded.Get(() => AlreadyLoaded is { } already ? Task.FromResult(already) : api.Chapter(_book, _chapter));
}
