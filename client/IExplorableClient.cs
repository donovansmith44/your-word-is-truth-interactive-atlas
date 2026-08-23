namespace BibleAtlas.Client;

/// <summary>
/// Batch P fix round 1 (R-P1, closing the M-D2 Minor-2 follow-through the
/// batch-p-brief.md's own requirement 3 named -- "tightening its stringly
/// params toward the typed sketch as you consume it" -- once a real caller
/// existed to tighten it FOR): a NARROW wrapper around one edge-kind label
/// string (e.g. <c>"mentions"</c>, <c>"mentioned-in"</c>, <c>"cites"</c>),
/// deliberately carrying NO implicit conversion to/from <c>string</c> in
/// either direction. That absence is the entire point: M-D2 Minor-2's own
/// named risk was an <c>id</c>/<c>kind</c> ARGUMENT-ORDER transposition at
/// an <see cref="IExplorableClient.Edges"/> call site (both were plain
/// <c>string</c>, so the compiler could never catch a swap). An implicit
/// string conversion would silently let ANY string -- including a
/// mis-ordered node id -- become an <c>EdgeKindId</c> again, reopening the
/// exact hole this type exists to close. The only lawful way to construct
/// one is the explicit constructor, `new EdgeKindId("...")` -- confined,
/// by convention, to the THREE literal-authoring sites in
/// <c>EdgeSectionRegistry.cs</c> (`Cites`/`Mentions`/`MentionedIn`); every
/// other call site consumes an already-typed constant
/// (`EdgeSectionRegistry.Mentions.EdgeKind`, ...), never types a new
/// literal of its own.
///
/// SCOPE, disclosed (per the controller's own R-P1 ruling): `id` (still
/// plain <c>string</c>) and `cursor` (still plain <c>int?</c>) are
/// deliberately NOT tightened this round -- the design spec's own
/// <c>AnyNodeId</c>/<c>Cursor</c> typed sketch completion stays queued for
/// the polish pass. `kind` alone was named by M-D2 Minor-2 and is the one
/// tightened here.
/// </summary>
public readonly record struct EdgeKindId(string Value);

/// <summary>
/// Batch M-D2 -- P7's REVERSIBILITY-BY-INTERFACE closure for the "CLIENT
/// ACCESS" seam (design spec §2's own seam inventory: "GAP: the Blazor
/// client still calls bespoke endpoints via a concrete AtlasClient. CLOSES
/// AT M-D"; §8's own C# sketch). Every subsystem this app already has is
/// consumed only through an interface it does not own (GraphQuery on the
/// server, IPopoverSectionProvider client-side, ...); this is that same law
/// applied to the generic graph query surface itself: THREE methods, mapping
/// 1:1 onto the design spec's own two generic endpoints plus the
/// text-window endpoint (§5/§6) -- <c>GET /api/node/{id}</c>,
/// <c>GET /api/node/{id}/edges?kind=&amp;cursor=&amp;limit=</c>,
/// <c>GET /api/text?ref=&amp;n=&amp;dir=</c>.
///
/// DISCLOSED SIMPLIFICATION from the design spec's own literal C# sketch
/// (which types <c>id</c>/<c>kind</c>/<c>cursor</c>/<c>dir</c> as
/// <c>AnyNodeId</c>/<c>EdgeKind</c>/<c>Cursor</c>/<c>Direction</c>), NARROWED
/// (Batch P fix round 1, R-P1): <see cref="Edges"/>'s own
/// <c>kind</c> parameter is now typed (<see cref="EdgeKindId"/>) -- the
/// batch-p-brief.md's own requirement 3 named this specific tightening
/// ("toward the typed sketch, as you consume it") once a real caller
/// existed, per M-D2 Minor-2's own "worth revisiting once a first real
/// caller lands." <c>id</c> and <c>cursor</c> remain plain
/// <c>string</c>/<c>int?</c> this round -- this client still represents
/// every REF as a plain string everywhere it appears
/// (<c>VerseNode.Title</c> IS the vref; <c>CanonRef</c> parses strings; no
/// typed ref wrapper exists anywhere in <c>client/</c> today), and the
/// wire itself is already string-keyed for ids
/// (<c>graph_wire::encode_node_id</c> -- e.g. <c>"text-unit:JHN.3.16"</c>).
/// The design spec's own full <c>AnyNodeId</c>/<c>Cursor</c>/
/// <c>Direction</c> typed-sketch completion remains queued for the polish
/// pass, disclosed rather than silently finished here in one sweep --
/// "client stays thin" still cuts against inventing a parallel ref
/// hierarchy wholesale in one round. The CONTRACT (three methods, one
/// concrete implementation, no business logic) is what P7 asks to be
/// reversible; the exact parameter TYPES are this seam's own
/// implementation detail, disclosed here and in CONTRACT.md's own M-D2/
/// PERSONS-1 notes.
///
/// ONE concrete implementation ships this batch (<see cref="GraphExplorableClient"/>);
/// per P7's own doc comment, "where only one exists, the interface is still
/// the only consumption path" -- callers depend on this interface, never
/// the concrete type, at every wiring site.
/// </summary>
public interface IExplorableClient
{
    /// <c>GET /api/node/{id}</c> -- a node's card (id/kind/label/provenance)
    /// plus its edge-summary (kind -&gt; true count, every inhabited kind).
    Task<NodeCardDto> Card(string id);

    /// <c>GET /api/node/{id}/edges?kind=&amp;cursor=&amp;limit=</c> -- one
    /// page of one edge kind. <paramref name="kind"/> is typed
    /// (<see cref="EdgeKindId"/>, Batch P fix round 1, R-P1) -- see that
    /// struct's own doc comment for why. <paramref name="cursor"/> is the
    /// opaque offset a prior page's own <c>Next</c> returned (<c>null</c>
    /// for the first page); <paramref name="limit"/> mirrors the server's
    /// own default (20, capped at 200 server-side).
    Task<EdgePageDto> Edges(string id, EdgeKindId kind, int? cursor = null, int limit = 20);

    /// <c>GET /api/text?ref=&amp;n=&amp;dir=</c> -- a window of <c>{ref,
    /// text}</c> units along one corpus's reading spine, starting at
    /// <paramref name="fromRef"/> (a dot-ref, e.g. <c>"JHN.3.16"</c>).
    /// <paramref name="dir"/> is <c>"onward"</c> (default) or
    /// <c>"backward"</c>.
    Task<TextWindowDto> Reading(string fromRef, int n, string dir = "onward");
}
