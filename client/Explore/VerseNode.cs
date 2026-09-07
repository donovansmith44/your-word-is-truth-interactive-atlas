using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// A single verse (READ-2/READ-3). Title is exactly the vref (READ-2
/// asserts this literally), which doubles as the AtlasClient.Verse key
/// used both here and by ExplorerPopover's own cross-ref-expansion special
/// case (Current.Title IS the vref for any node on the stack of this type).
///
/// Batch T requirement 3 ("verse popover: event membership replaces
/// prev/next"): no longer implements `INarrativeAware` -- Batch N's own
/// verse-level chronological PRIOR/FOLLOWING is retired (it lives entirely
/// on the EVENT node now, reached via this node's own new "EVENT" section,
/// `VerseEventMembershipSection` in PopoverSectionProviders.cs, which reads
/// the pre-existing `DetailAsync().Events` this class already fetched --
/// no new field, no new fetch).
/// </summary>
public sealed class VerseNode : IExplorable
{
    private readonly string _vref;
    private readonly AsyncMemo<VerseDetail> _detail = new();

    /// <summary>
    /// Batch M-D2 (owner's cross-reference superscript directive):
    /// <c>true</c> exactly when this node was pushed by way of a verse's own
    /// xref superscript cluster/many-marker (Reader.razor's
    /// <c>OpenVerseXrefEntry</c>) rather than the ordinary verse line/
    /// verse-num click. A PARAMETER on the one abstraction, never a second
    /// node type or a parallel popover (owner decree, batch-x-brief.md
    /// verbatim: "a parallel popover implementation is a defect") -- reads
    /// through <see cref="Explore.IPopoverSectionContext.XrefEntryPoint"/>
    /// to change exactly two things about the SAME popover machinery: the
    /// xrefs section's own initial cap (3, unconditionally, vs. F2's
    /// existing 2-when-mixed-context rule) and its LEADING position among
    /// sections (both in <c>ExplorerPopover.razor</c>/
    /// <c>PopoverSectionProviders.CrossRefsSection</c>). Default <c>false</c>
    /// -- every pre-existing call site (<c>PassageList.Explore</c>,
    /// <c>Reader.razor</c>'s own plain <c>OpenVerse</c>, every provider that
    /// pushes a fresh <c>VerseNode</c> onward) is byte-for-byte unchanged.
    /// </summary>
    public bool XrefEntryPoint { get; }

    public VerseNode(string vref, bool xrefEntryPoint = false)
    {
        _vref = vref;
        XrefEntryPoint = xrefEntryPoint;
    }

    public string Title => _vref;
    public string Kind => "Verse";

    // Every chip below is derivable from the vref string alone -- no fetch
    // needed to decide WHICH explorations exist. Batch R requirement 3:
    // cross-references are no longer one of these chips -- they render
    // INLINE now, unconditionally offered, via the registry's own
    // CrossRefsSection (Explore/PopoverSections.cs) -- "no extra button
    // press" -- so the old popover-chip-xrefs toggle entry is gone (see
    // CONTRACT.md's own amendment for this batch).
    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        var (book, chapter, verse) = CanonRef.ParseVerse(_vref);
        IReadOnlyList<Exploration> list = new[]
        {
            // O1 (owner live-preview correction, 2026-08-23): "Explore
            // geo-temporally" (popover-chip-map, ExplorationTarget.ShowMiniMap)
            // removed -- "it's not serving us right now." Dead-code law took
            // the whole mechanism with it (ShowMiniMap itself and
            // MiniWorld.razor, its only renderer): see ExplorerPopover.razor's
            // own header comment.
            new Exploration("About this book", "popover-chip-book", new ExplorationTarget.Push(new AuthorNode(book))),
            new Exploration("Read in context", "popover-chip-context", new ExplorationTarget.NavigateReader(book, chapter, verse)),
        };
        return Task.FromResult(list);
    }

    public async Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        var detail = await DetailAsync(api);
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-verse-text");
            builder.AddContent(2, detail.Text);
            builder.CloseElement();
        };
        return fragment;
    }

    // Public + memoized so ExplorerPopover's popover-chip-xrefs handling can
    // read THIS node's own CrossRefs (fix round 1 finding: it previously
    // issued its own independent AtlasClient.Verse(Current.Title) call on
    // every expand, on top of the one BodyAsync already makes -- 2-4x
    // fetches per popover with no actual freshness benefit, since a NEW
    // VerseNode instance is pushed per hop anyway, so its cache is exactly
    // as fresh as a re-fetch would be). Idempotent: the underlying HTTP
    // call only ever happens once per node instance regardless of how many
    // times BodyAsync/this are each called.
    //
    // PERF-3 (owner: verse-click frontier <100ms; PHASE 0 waterfall,
    // measured live): that "only ever happens once" claim was FALSE under
    // the concurrency ExplorerPopover.LoadCurrent actually uses. Six
    // section providers applicable to a Verse (VerseTextSectionProvider,
    // CrossRefsSection, CatechismSeamSection, VerseEventMembershipSection,
    // VersePassageMembershipSection, VerseParallelsSection) each call THIS
    // method, and LoadCurrent fires all of them via ONE Task.WhenAll --
    // every provider's own ResolveAsync runs synchronously up to its first
    // await BEFORE the next provider even starts (Task.WhenAll builds its
    // task list via a synchronous `.Select(...).ToList()`). A
    // value-memoizing `_cached ??= await ...` checks the CACHE before the
    // FIRST call's own await has resolved it -- every one of the six sees
    // `_cached` still null and fires its OWN independent `GET
    // /api/verse/{vref}`. Confirmed live: GEN.1.1 fired SIX concurrent,
    // identical requests to this exact endpoint (see the PHASE 0 waterfall
    // in batch-perf3-report.md). Fixed via AsyncMemo&lt;T&gt; (Explore/AsyncMemo.cs)
    // -- caches the in-flight TASK itself (assigned synchronously, before
    // any await resolves, so a concurrent caller shares it rather than
    // starting a new request) AND resets on fault (fix round 1, review
    // Q-2 -- see AsyncMemo's own doc comment for why a plain task-cache
    // alone regressed the old "a transient failure self-heals on the next
    // call" property).
    public Task<VerseDetail> DetailAsync(AtlasClient api) => _detail.Get(() => api.Verse(_vref));
}
