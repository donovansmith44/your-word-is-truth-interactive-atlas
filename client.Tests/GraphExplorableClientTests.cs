using System.Net;
using System.Net.Http;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Tests;

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

        Assert.Equal("/api/node/text-unit:JHN.3.16", Uri.UnescapeDataString(handler.LastRequestUri!.AbsolutePath));
        Assert.Equal("text-unit:JHN.3.16", card.Id);
        Assert.Equal(NodeKind.TextUnit, card.Kind);
        Assert.Equal("JHN.3.16", card.Label);
        Assert.Equal("kjv", card.Provenance);
        Assert.Equal("abc123", card.Version);
        Assert.Single(card.EdgeSummary);
        Assert.Equal(EdgeKind.Cites, card.EdgeSummary[0].Kind);
        Assert.Equal(178, card.EdgeSummary[0].Count);
    }

    [Fact]
    public async Task Edges_BuildsKindAndLimitQueryParams_AndOmitsCursorWhenNull()
    {
        var (client, handler) = MakeClient();
        handler.ResponseBody = """
            {"kind":"cites","entries":[{"edge":{"id":"e1","kind":"cites","label":"JHN.3.16 · Cites · ROM.3.23"},"neighbour":{"position":"node","node":{"id":"text-unit:ROM.3.23","kind":"TextUnit","label":"ROM.3.23"}}}],"next":null,"version":"abc123"}
            """;

        var page = await client.Edges("text-unit:JHN.3.16", EdgeKind.Cites, cursor: null, limit: 5);

        Assert.Equal("/api/node/text-unit:JHN.3.16/edges", Uri.UnescapeDataString(handler.LastRequestUri!.AbsolutePath));
        Assert.Equal("?kind=cites&limit=5", handler.LastRequestUri.Query);
        Assert.Equal(EdgeKind.Cites, page.Kind);
        Assert.Single(page.Entries);
        Assert.Equal(new EdgeRef(id: "e1", kind: EdgeKind.Cites, label: "JHN.3.16 · Cites · ROM.3.23"), page.Entries[0].Edge);
        Assert.Equal(new NodePosition(new NodeRef(id: "text-unit:ROM.3.23", kind: NodeKind.TextUnit, label: "ROM.3.23")), page.Entries[0].Neighbour);
        Assert.Null(page.Next);
    }

    [Fact]
    public async Task Edges_ReadsAnEdgeNeighbour_AsTheEdgeItself()
    {
        var (client, handler) = MakeClient();
        handler.ResponseBody = """
            {"kind":"justifies","entries":[{"edge":{"id":"JustifiedBy:00aa","kind":"justified-by","label":"A dating · Justified by · Solomon crowned"},"neighbour":{"position":"edge","edge":{"id":"DatedBy:00ff","kind":"dated-by","label":"A dating"}}}],"next":null,"version":"abc123"}
            """;

        var page = await client.Edges("Anchor:solomon-crowned", EdgeKind.Justifies, cursor: null, limit: 5);

        Assert.Equal(new EdgePosition(new EdgeRef(id: "DatedBy:00ff", kind: EdgeKind.DatedBy, label: "A dating")), page.Entries[0].Neighbour);
    }

    [Fact]
    public async Task Edges_IncludesCursor_WhenProvided()
    {
        var (client, handler) = MakeClient();
        handler.ResponseBody = """{"kind":"cites","entries":[],"next":7,"version":"abc123"}""";

        var page = await client.Edges("text-unit:JHN.3.16", EdgeKind.Cites, cursor: 3, limit: 1);

        Assert.Contains("cursor=3", handler.LastRequestUri!.Query);
        Assert.Equal(7, page.Next);
    }

    [Fact]
    public async Task Edges_asks_for_the_one_default_page_size_when_no_limit_is_given()
    {
        // Arrange
        var (client, handler) = MakeClient();
        handler.ResponseBody = """{"kind":"cites","entries":[],"next":null,"version":"abc123"}""";

        // Act
        await client.Edges("text-unit:JHN.3.16", EdgeKind.Cites);

        // Assert
        Assert.Equal($"?kind=cites&limit={IExplorableClient.DefaultPageSize}", handler.LastRequestUri!.Query);
    }

    [Fact]
    public async Task Elements_asks_the_element_read_for_every_id_at_once_and_answers_its_elements_in_order()
    {
        // Arrange
        var (client, handler) = MakeClient();
        handler.ResponseBody = """
            {"elements":[
              {"element":"node","node":{"id":"text-unit:EXO.14.21","kind":"TextUnit","label":"Exodus 14:21","provenance":"kjv","edge_summary":[],"version":"abc123"}},
              {"element":"edge","edge":{"id":"Attests:00aa","kind":"attested-in","label":"The Red Sea parted · Attested in · Exodus 14:21","subject":{"position":"node","node":{"id":"Event:red_sea","kind":"Event","label":"The Red Sea parted"}},"object":{"position":"node","node":{"id":"text-unit:EXO.14.21","kind":"TextUnit","label":"Exodus 14:21"}},"provenance":"kjv","edge_summary":[]}},
              {"element":"missing","id":"Event:nowhere"}],"version":"abc123"}
            """;

        // Act
        var elements = await client.Elements(["text-unit:EXO.14.21", "Attests:00aa", "Event:nowhere"]);

        // Assert
        Assert.Equal(
            ("/api/elements", "?ids=text-unit:EXO.14.21,Attests:00aa,Event:nowhere", WholeValue.Of(new Element[]
            {
                new NodeElement(new NodeRecord(book: null, catechism: null, description: null, edgeSummary: [], @event: null, id: "text-unit:EXO.14.21", kind: NodeKind.TextUnit, label: "Exodus 14:21", person: null, place: null, era: null, map: null, polity: null, provenance: "kjv", version: "abc123")),
                new EdgeElement(new EdgeRecord(edgeSummary: [], id: "Attests:00aa", kind: EdgeKind.AttestedIn, label: "The Red Sea parted · Attested in · Exodus 14:21", narrative: null,
                    @object: new NodePosition(new NodeRef(id: "text-unit:EXO.14.21", kind: NodeKind.TextUnit, label: "Exodus 14:21")), parentage: null, provenance: "kjv",
                    subject: new NodePosition(new NodeRef(id: "Event:red_sea", kind: NodeKind.Event, label: "The Red Sea parted")), votes: null)),
                new MissingElement("Event:nowhere"),
            })),
            (handler.LastRequestUri!.AbsolutePath, Uri.UnescapeDataString(handler.LastRequestUri.Query), WholeValue.Of(elements.ToArray())));
    }

    [Fact]
    public async Task Elements_escapes_each_id_and_keeps_the_separator_between_them()
    {
        // Arrange
        var (client, handler) = MakeClient();
        handler.ResponseBody = """{"elements":[],"version":"abc123"}""";

        // Act
        await client.Elements(["a,b", "c d"]);

        // Assert
        Assert.Equal("?ids=a%2Cb,c%20d", handler.LastRequestUri!.Query);
    }

    [Fact]
    public async Task Reading_RequestsTheTextWindowEndpoint_AndDeserializesTheRealWireShape()
    {
        var (client, handler) = MakeClient();
        handler.ResponseBody = """
            {"units":[{"ref":"JHN.3.16","text":"For God so loved the world..."}],"next":"JHN.3.17","version":"abc123"}
            """;

        var window = await client.Reading("JHN.3.16", 1, WindowDir.Onward);

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
