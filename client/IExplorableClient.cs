namespace BibleAtlas.Client;

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
/// <c>AnyNodeId</c>/<c>EdgeKind</c>/<c>Cursor</c>/<c>Direction</c>): this
/// client already represents every ref as a plain STRING everywhere it
/// appears (<c>VerseNode.Title</c> IS the vref; <c>CanonRef</c> parses
/// strings; no typed ref wrapper exists anywhere in <c>client/</c> today),
/// and the wire itself is already string-keyed (<c>graph_wire::encode_node_id</c>/
/// <c>EdgeKind::label()</c> — e.g. <c>"text-unit:JHN.3.16"</c>/<c>"cites"</c>).
/// Introducing a parallel typed id/kind hierarchy here would be a SECOND
/// representation of the same thing this client already has one of --
/// "client stays thin" cuts against inventing that ceremony, not for it. The
/// CONTRACT (three methods, one concrete implementation, no business logic)
/// is what P7 asks to be reversible; the exact parameter TYPES are an
/// implementation detail of this one seam, disclosed here and in
/// CONTRACT.md's own M-D2 note.
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
    /// page of one edge kind. <paramref name="cursor"/> is the opaque offset
    /// a prior page's own <c>Next</c> returned (<c>null</c> for the first
    /// page); <paramref name="limit"/> mirrors the server's own default (20,
    /// capped at 200 server-side).
    Task<EdgePageDto> Edges(string id, string kind, int? cursor = null, int limit = 20);

    /// <c>GET /api/text?ref=&amp;n=&amp;dir=</c> -- a window of <c>{ref,
    /// text}</c> units along one corpus's reading spine, starting at
    /// <paramref name="fromRef"/> (a dot-ref, e.g. <c>"JHN.3.16"</c>).
    /// <paramref name="dir"/> is <c>"onward"</c> (default) or
    /// <c>"backward"</c>.
    Task<TextWindowDto> Reading(string fromRef, int n, string dir = "onward");
}
