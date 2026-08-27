using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch CORP-1 (R2): one Kretzmann Popular Commentary unit, reached by
/// clicking an explorable row in the Kretzmann chapter browser
/// (Pages/Kretzmann.razor). <see cref="Title"/> is the unit's own pericope
/// heading -- known synchronously at construction (the caller already has
/// it from the "commented-on-by" edge page's own <c>NodeRefDto.Label</c>,
/// which IS the node's <c>card()</c> label for a CommentaryItem --
/// graph-types/src/node.rs) -- the same "never a fetch for the header"
/// discipline every <see cref="IExplorable"/> already follows.
///
/// Constructor takes ONLY (id, heading) -- no <see cref="IExplorableClient"/>
/// dependency -- the SAME minimal shape <c>CatechismNode</c>/<c>PersonNode</c>
/// already establish, deliberately: <see cref="ExplorationDescriptor.Reconstruct"/>
/// (the G2 serialization seam a saved exploration trail/tray selection both
/// ride) only ever threads a plain <c>AtlasClient</c>, never the generic
/// graph client -- a node whose own constructor NEEDS
/// <see cref="IExplorableClient"/> cannot be reconstructed from a saved
/// descriptor at all. (A first draft of this class held one for a "Read in
/// context" chip; retired once this constraint surfaced -- see
/// batch-corp1-report.md's own self-review.)
///
/// Batch CORP-1b (owner authorization, resolving CORP-1's own disclosed
/// NEEDS_CONTEXT gap): the unit's own PROSE now rides the generic node
/// card's additive <c>description</c> field, server-side
/// (<c>atlas_graph::legacy::node_description</c>'s widened match -- the
/// SAME seam ENT-1a built for Place/Person/PeopleGroup, reused rather than
/// a new endpoint/wire type) -- reached client-side via
/// <see cref="CommentaryItemProseSection"/> (Explore/PopoverSectionProviders.cs),
/// the SAME "a registered provider reads <c>ctx.Graph</c>, this node's own
/// constructor stays fetch-free" shape <c>PersonCardAndMentionsSection</c>/
/// <c>PersonNode</c> already establish, for the identical reconstruction
/// reason above.
/// </summary>
public sealed class CommentaryItemNode : IExplorable
{
    private readonly string _id;

    public CommentaryItemNode(string id, string heading)
    {
        _id = id;
        Title = string.IsNullOrWhiteSpace(heading) ? "Commentary" : heading;
    }

    /// The graph's own wire node id (e.g. "CommentaryItem:kretzmann/0.1.0")
    /// -- also this descriptor's own reconstruction Key
    /// (<see cref="ExplorationDescriptor"/>), and what
    /// <see cref="CommentaryItemProseSection"/> threads into its own
    /// <c>Card()</c> call.
    public string Id => _id;

    public string Title { get; }
    public string Kind => "CommentaryItem";

    // No chips this batch -- the SAME "nothing extra to offer" shape
    // CatechismNode/PersonNode already establish for a node kind with no
    // onward traversal of its own.
    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api) =>
        Task.FromResult<IReadOnlyList<Exploration>>(Array.Empty<Exploration>());

    // Batch CORP-1b: PopoverSectionRegistry now claims Kind == "CommentaryItem"
    // (CommentaryItemProseSection), so ExplorerPopover.LoadCurrent never
    // calls this in practice -- the SAME "registry supersedes BodyAsync
    // entirely" shape PersonNode/CatechismNode already establish. Defensive
    // fallback only.
    public Task<RenderFragment> BodyAsync(AtlasClient api) => Task.FromResult<RenderFragment>(_ => { });
}
