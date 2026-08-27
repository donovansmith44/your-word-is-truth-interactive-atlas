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
/// context" chip; retired once this constraint surfaced -- see the batch
/// report's own self-review.)
///
/// DISCLOSED GAP (batch-corp1-report.md has the full investigation): no
/// existing server query exposes a CommentaryItem's own PROSE (its `text`
/// field) to the client. The generic node card computes ONLY the heading as
/// its label (graph-types/src/node.rs's own <c>card()</c>: <c>NodePayload::
/// CommentaryItem { heading, .. } => heading...</c> -- the `text` field is
/// never read there); <c>/api/text</c> serves only the "bible"/"concord"
/// corpora (server: `graph_handlers.rs`, CORP-2a); there is no bespoke
/// `/api/commentary/item/{id}` the way `CatechismItem` has its own
/// `/api/catechism/item/{id}` (`AtlasClient.CatechismItem`). This node
/// therefore renders the heading only -- it does NOT fabricate placeholder
/// prose. Closing this gap for real needs a bespoke commentary-detail server
/// route (mirroring <c>CatechismItemDetail</c>), which this batch's own
/// machine rules bar it from adding (ZERO server/Rust changes) -- see the
/// report's own NEEDS_CONTEXT naming of the exact missing endpoint.
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
    /// (<see cref="ExplorationDescriptor"/>).
    public string Id => _id;

    public string Title { get; }
    public string Kind => "CommentaryItem";

    // No chips this batch -- the SAME "nothing extra to offer" shape
    // CatechismNode/PersonNode already establish for a node kind with no
    // registered popover section provider either.
    public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api) =>
        Task.FromResult<IReadOnlyList<Exploration>>(Array.Empty<Exploration>());

    // No popover section provider claims Kind == "CommentaryItem" -- this
    // fallback renders directly, the same shape VerseNode's own BodyAsync
    // establishes. See this class's own header for why there is no prose to
    // show beyond the heading already carried in the popover's own title.
    public Task<RenderFragment> BodyAsync(AtlasClient api)
    {
        RenderFragment fragment = builder =>
        {
            builder.OpenElement(0, "p");
            builder.AddAttribute(1, "class", "popover-commentary-note");
            builder.AddContent(2, "Kretzmann, Popular Commentary of the Bible.");
            builder.CloseElement();
        };
        return Task.FromResult(fragment);
    }
}
