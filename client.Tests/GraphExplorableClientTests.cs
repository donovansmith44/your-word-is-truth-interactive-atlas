using System.Net;
using System.Net.Http;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace BibleAtlas.Client.Tests;

/// <summary>
/// Batch M-D2 (P7 closure, "the client gains the generic contract... implemented
/// by ONE concrete client class over /api/node/{id} + /api/node/{id}/edges"):
/// direct proof of <see cref="GraphExplorableClient"/>'s own URL construction
/// and wire deserialization, in isolation -- no live server needed. This
/// project has no dedicated unit tests for <see cref="AtlasClient"/> itself
/// (its own HTTP behavior is exercised end to end by the Playwright suite,
/// per every prior batch report's own "no new C# unit-test work needed"
/// note) -- this file is the ONE deliberate exception for the new generic
/// seam, precisely because it has no live in-app caller yet this batch (see
/// CONTRACT.md's own M-D2 strangler-inventory note: the interface/
/// implementation are real, DI-wired, and this batch's own proof of
/// correctness, but the popover's own xrefs section stays on its existing
/// bespoke fetch -- disclosed, not silently assumed correct). Real sample
/// JSON shaped exactly like atlas-server's own `graph_handlers` output,
/// field names cross-checked against `server/atlas-server/tests/graph_api.rs`'s
/// own live assertions against the real running server.
/// </summary>
public class GraphExplorableClientTests
{
    private sealed class StubHandler : HttpMessageHandler
    {
        public Uri? LastRequestUri;
        public string ResponseBody = "{}";

        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
        {
            LastRequestUri = request.RequestUri;
            var response = new HttpResponseMessage(HttpStatusCode.OK) { Content = new StringContent(ResponseBody, Encoding.UTF8, "application/json") };
            return Task.FromResult(response);
        }
    }

    private static (GraphExplorableClient Client, StubHandler Handler) MakeClient()
    {
        var handler = new StubHandler();
        var http = new HttpClient(handler) { BaseAddress = new Uri("http://localhost:8000/") };
        return (new GraphExplorableClient(http), handler);
    }

    [Fact]
    public async Task Card_RequestsTheGenericNodeEndpoint_AndDeserializesTheRealWireShape()
    {
        var (client, handler) = MakeClient();
        handler.ResponseBody = """
            {"id":"text-unit:JHN.3.16","kind":"TextUnit","label":"JHN.3.16","provenance":"kjv","edge_summary":[{"kind":"cites","count":178}],"version":"abc123"}
            """;

        var card = await client.Card("text-unit:JHN.3.16");

        // Node ids can carry a colon (the "kind:raw" wire form) -- verify the
        // REQUEST round-trips to the same unescaped path this id names,
        // rather than asserting one specific percent-encoding of it.
        Assert.Equal("/api/node/text-unit:JHN.3.16", Uri.UnescapeDataString(handler.LastRequestUri!.AbsolutePath));
        Assert.Equal("text-unit:JHN.3.16", card.Id);
        Assert.Equal("TextUnit", card.Kind);
        Assert.Equal("JHN.3.16", card.Label);
        Assert.Equal("kjv", card.Provenance);
        Assert.Equal("abc123", card.Version);
        Assert.Single(card.EdgeSummary);
        Assert.Equal("cites", card.EdgeSummary[0].Kind);
        Assert.Equal(178, card.EdgeSummary[0].Count);
    }

    [Fact]
    public async Task Edges_BuildsKindAndLimitQueryParams_AndOmitsCursorWhenNull()
    {
        var (client, handler) = MakeClient();
        handler.ResponseBody = """
            {"kind":"cites","entries":[{"edge":"e1","node":{"id":"text-unit:ROM.3.23","kind":"TextUnit","label":"ROM.3.23"}}],"next":null,"version":"abc123"}
            """;

        var page = await client.Edges("text-unit:JHN.3.16", new EdgeKindId("cites"), cursor: null, limit: 5);

        Assert.Equal("/api/node/text-unit:JHN.3.16/edges", Uri.UnescapeDataString(handler.LastRequestUri!.AbsolutePath));
        Assert.Contains("kind=cites", handler.LastRequestUri.Query);
        Assert.Contains("limit=5", handler.LastRequestUri.Query);
        Assert.DoesNotContain("cursor=", handler.LastRequestUri.Query);
        Assert.Equal("cites", page.Kind);
        Assert.Single(page.Entries);
        Assert.Equal("e1", page.Entries[0].Edge);
        Assert.Equal("text-unit:ROM.3.23", page.Entries[0].Node.Id);
        Assert.Equal("TextUnit", page.Entries[0].Node.Kind);
        Assert.Null(page.Next);
    }

    [Fact]
    public async Task Edges_IncludesCursor_WhenProvided()
    {
        var (client, handler) = MakeClient();
        handler.ResponseBody = """{"kind":"cites","entries":[],"next":7,"version":"abc123"}""";

        var page = await client.Edges("text-unit:JHN.3.16", new EdgeKindId("cites"), cursor: 3, limit: 1);

        Assert.Contains("cursor=3", handler.LastRequestUri!.Query);
        Assert.Equal(7, page.Next);
    }

    [Fact]
    public async Task Reading_RequestsTheTextWindowEndpoint_AndDeserializesTheRealWireShape()
    {
        var (client, handler) = MakeClient();
        handler.ResponseBody = """
            {"units":[{"ref":"JHN.3.16","text":"For God so loved the world..."}],"next":"JHN.3.17","version":"abc123"}
            """;

        var window = await client.Reading("JHN.3.16", 1, "onward");

        Assert.Equal("/api/text", Uri.UnescapeDataString(handler.LastRequestUri!.AbsolutePath));
        Assert.Contains("ref=JHN.3.16", handler.LastRequestUri.Query);
        Assert.Contains("n=1", handler.LastRequestUri.Query);
        Assert.Contains("dir=onward", handler.LastRequestUri.Query);
        Assert.Single(window.Units);
        Assert.Equal("JHN.3.16", window.Units[0].Ref);
        Assert.Equal("JHN.3.17", window.Next);
    }

    [Fact]
    public async Task Reading_DefaultsDirectionToOnward()
    {
        var (client, handler) = MakeClient();
        handler.ResponseBody = """{"units":[],"next":null,"version":"abc123"}""";

        await client.Reading("GEN.1.1", 3);

        Assert.Contains("dir=onward", handler.LastRequestUri!.Query);
    }
}
