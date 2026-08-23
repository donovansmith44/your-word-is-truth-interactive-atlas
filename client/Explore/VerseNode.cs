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
    private VerseDetail? _cached;

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
            new Exploration("Explore geo-temporally", "popover-chip-map", new ExplorationTarget.ShowMiniMap(_vref)),
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
    public async Task<VerseDetail> DetailAsync(AtlasClient api) => _cached ??= await api.Verse(_vref);
}
