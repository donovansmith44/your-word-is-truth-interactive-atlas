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

    /// Batch R requirement 4 (the verse-text section's own compact view):
    /// the passage's own already-known text, exposed publicly so
    /// VerseTextSectionProvider (Explore/PopoverSectionProviders.cs) can
    /// read it without a second network round trip -- the SAME text
    /// <see cref="BodyAsync"/> already rendered pre-Batch-R, now surfaced
    /// as a plain property since BodyAsync itself is no longer the
    /// popover's own rendering path for this node kind (see the registry's
    /// own doc comment for why BodyAsync survives regardless, as a fallback).
    public string Text => _text;

    // Every chip below is derivable from the sref string alone. Batch R
    // requirement 3: cross-references are no longer a CONDITIONAL chip here
    // (Batch G1's own original behavior) -- they render INLINE now via the
    // registry's own CrossRefsSection (Explore/PopoverSections.cs), which
    // does its own presence-checking fetch (reusing XrefsAsync below) rather
    // than this method eagerly fetching just to decide a chip's presence --
    // see CONTRACT.md's own amendment for this batch.
    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
    {
        var (book, chapter, verse) = CanonRef.ParseVerse(CanonRef.FirstVerseOf(_sref));
        IReadOnlyList<Exploration> list = new[]
        {
            new Exploration("Explore geo-temporally", "popover-chip-map", new ExplorationTarget.ShowMiniMap(_sref)),
            new Exploration("Read in context", "popover-chip-context",
                new ExplorationTarget.NavigateReader(book, chapter, verse)),
            new Exploration("About this book", "popover-chip-book",
                new ExplorationTarget.Push(new AuthorNode(book))),
        };
        return Task.FromResult(list);
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
