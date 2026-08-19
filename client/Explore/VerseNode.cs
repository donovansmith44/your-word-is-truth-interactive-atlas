using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// A single verse (READ-2/READ-3). Title is exactly the vref (READ-2
/// asserts this literally), which doubles as the AtlasClient.Verse key
/// used both here and by ExplorerPopover's own cross-ref-expansion special
/// case (Current.Title IS the vref for any node on the stack of this type).
/// </summary>
public sealed class VerseNode : IExplorable
{
    private readonly string _vref;
    private VerseDetail? _cached;

    public VerseNode(string vref) => _vref = vref;

    public string Title => _vref;
    public string Kind => "Verse";

    // Every chip below is derivable from the vref string alone -- no fetch
    // needed to decide WHICH explorations exist; only a chip's eventual
    // content (e.g. the cross-ref list itself) needs the network, and that
    // happens lazily (ExplorerPopover's popover-chip-xrefs handling), not here.
    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        var (book, chapter, verse) = CanonRef.ParseVerse(_vref);
        IReadOnlyList<Exploration> list = new[]
        {
            // ExplorerPopover special-cases this exact ChipTestId to expand an
            // inline xref-item-{TARGET} list fetched fresh from
            // VerseDetail.CrossRefs, rather than dispatching through Target --
            // see its own comment. Target still needs a value to satisfy the
            // record shape; Push(this) is the closest honest placeholder (if
            // ever generically dispatched, "explore this verse's
            // cross-references" degrades to "stay on this verse" rather than
            // crashing) and is never actually reached in practice.
            new Exploration("Cross-references", "popover-chip-xrefs", new ExplorationTarget.Push(this)),
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
