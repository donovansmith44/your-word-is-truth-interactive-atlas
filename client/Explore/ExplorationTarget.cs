namespace BibleAtlas.Client.Explore;

/// <summary>
/// One labelled continuation a node offers from <see cref="IExplorable.ExploreAsync"/>,
/// rendered by <c>ExplorerPopover</c> as a button carrying the fixed CONTRACT
/// testid <see cref="ChipTestId"/> (one of <c>popover-chip-xrefs</c>,
/// <c>popover-chip-map</c>, <c>popover-chip-book</c>, <c>popover-chip-context</c> --
/// the same four ids are reused across node kinds for the conceptually-same
/// action, e.g. <c>popover-chip-map</c> is <see cref="EventNode"/>'s own
/// "Show on the map" chip, target <see cref="ExplorationTarget.NavigateWorld"/>.
/// O1 (owner live-preview correction, 2026-08-23) retired the OTHER chip that
/// used to share this testid -- VerseNode/PassageNode's "Explore
/// geo-temporally", target ShowMiniMap, reveal-a-mini-map-in-place -- along
/// with the ShowMiniMap case itself and MiniWorld.razor, its only renderer;
/// see ExplorerPopover.razor's own header comment for the fuller story).
/// </summary>
public sealed record Exploration(string Label, string ChipTestId, ExplorationTarget Target);

/// <summary>
/// Closed hierarchy of what activating an <see cref="Exploration"/> chip
/// actually does. <c>ExplorerPopover</c> pattern-matches on the concrete
/// case:
/// <list type="bullet">
/// <item><see cref="Push"/> -- go deeper in the SAME popover (grows its breadcrumb stack).</item>
/// <item><see cref="NavigateWorld"/> -- leave for <c>/world?{Query}</c>. <see cref="NavigateWorld.Query"/>
/// is the EXACT query-string suffix, already escaped, ready to append verbatim after
/// <c>/world?</c> -- either a scripture ref (<c>ref=GEN.1</c>) or a time window
/// (<c>from=-1445&amp;to=-1405</c>), matching the two shapes <c>/world</c> itself understands.</item>
/// <item><see cref="NavigateReader"/> -- leave for <c>/read/{Book}/{Chapter}</c>, optionally
/// anchored at a specific verse (<c>#v{n}</c>).</item>
/// </list>
/// </summary>
public abstract record ExplorationTarget
{
    public sealed record Push(IExplorable Next) : ExplorationTarget;
    public sealed record NavigateWorld(string Query) : ExplorationTarget;
    public sealed record NavigateReader(string Book, int Chapter, int? Verse) : ExplorationTarget;
}
