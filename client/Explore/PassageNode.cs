using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// A verse range within a single chapter (READ-5/Task 15's shift-click
/// passage selection). The reader already has every verse's text on hand
/// (it just rendered the chapter to let the user shift-click it), so unlike
/// VerseNode/ChapterNode this node needs no AtlasClient fetch of its own for
/// its body -- the caller hands over the already-known sref and concatenated
/// text directly at construction, avoiding a second network round-trip for
/// content already in memory. Reader.razor owns the actual shift-click
/// wiring (Task 15).
///
/// The map chip is <see cref="ExplorationTarget.ShowMiniMap"/> (Task 15) --
/// "chips: map/context like VerseNode" per the brief, i.e. reveals the
/// <c>mini-map</c> IN PLACE the same way VerseNode's own map chip does,
/// rather than Task 14's placeholder <c>NavigateWorld</c> (which would have
/// left the popover entirely, breaking READ-5's own "chip -&gt; popover-chip-
/// map -&gt; mini-map, all within the SAME still-open passage popover" flow).
/// </summary>
public sealed class PassageNode : IExplorable
{
    private readonly string _sref;
    private readonly string _text;
    private List<CrossRefOut>? _cachedXrefs;

    public PassageNode(string sref, string text)
    {
        _sref = sref;
        _text = text;
    }

    public string Title => _sref;
    public string Kind => "Passage";

    // Batch G1 requirement 2 ("passage context -- passages give xrefs, not
    // just geo"): unlike every OTHER chip on every node type in this file
    // (all derivable from the ref string alone -- see VerseNode.ExploreAsync's
    // own comment on that general rule), the "Cross-references" chip here
    // is CONDITIONAL: it appears only when GET /api/xrefs/{sref} actually
    // returns >=1 target (the brief's own words, verbatim). That can only be
    // known by actually fetching -- a deliberate, documented exception to
    // the "no fetch needed to decide which explorations exist" rule, scoped
    // to this one node/chip. Fails soft (no chip, not an exception bubbling
    // out of the popover) on a network error, same graceful-degradation
    // policy every other lazy fetch in this app already follows.
    public async Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        var (book, chapter, verse) = CanonRef.ParseVerse(CanonRef.FirstVerseOf(_sref));
        var list = new List<Exploration>
        {
            new Exploration("Explore geo-temporally", "popover-chip-map", new ExplorationTarget.ShowMiniMap(_sref)),
            new Exploration("Read in context", "popover-chip-context",
                new ExplorationTarget.NavigateReader(book, chapter, verse)),
            new Exploration("About this book", "popover-chip-book",
                new ExplorationTarget.Push(new AuthorNode(book))),
        };

        try
        {
            var xrefs = await XrefsAsync(api);
            if (xrefs.Count > 0)
            {
                // Target/Push(this) is the same "chip's real behavior is
                // ExplorerPopover's own special-cased popover-chip-xrefs
                // handling, never generically dispatched" placeholder
                // VerseNode's own xrefs chip uses -- see its comment.
                list.Add(new Exploration("Cross-references", "popover-chip-xrefs", new ExplorationTarget.Push(this)));
            }
        }
        catch (Exception)
        {
        }

        return list;
    }

    // Public + memoized (mirrors VerseNode.DetailAsync's own reasoning
    // exactly) so ExploreAsync's own presence check above and
    // ExplorerPopover's later popover-chip-xrefs click share ONE fetch, not
    // two -- the chip would otherwise re-request the exact same data it
    // just used to decide whether to render itself at all.
    public async Task<List<CrossRefOut>> XrefsAsync(AtlasClient api) => _cachedXrefs ??= await api.Xrefs(_sref);

    public Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-verse-text");
            builder.AddContent(2, _text);
            builder.CloseElement();
        };
        return Task.FromResult(fragment);
    }
}
