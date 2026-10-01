using System.Net;
using System.Net.Http;
using System.Text;
using System.Threading;
using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Primitives;

namespace BibleAtlas.Client.Tests;

public sealed class AtlasClientTests
{
    private const string BaseAddress = "http://localhost:8000/";

    private sealed class StubHandler(params string[] responses) : HttpMessageHandler
    {
        private readonly Queue<string> _responses = new(responses);

        public readonly List<Uri> RequestedUris = [];

        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
        {
            RequestedUris.Add(request.RequestUri!);
            var body = _responses.Dequeue();
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK) { Content = new StringContent(body, Encoding.UTF8, "application/json") });
        }
    }

    private static (AtlasClient Client, StubHandler Handler) MakeClient(params string[] responses)
    {
        var handler = new StubHandler(responses);
        var http = new HttpClient(handler) { BaseAddress = new Uri(BaseAddress) };
        return (new AtlasClient(http), handler);
    }

    private sealed class FakeConfiguration(string? apiBase) : IConfiguration
    {
        public string? this[string key]
        {
            get => key == "ApiBase" ? apiBase : null;
            set => throw new NotSupportedException();
        }

        public IEnumerable<IConfigurationSection> GetChildren() => throw new NotSupportedException();
        public IChangeToken GetReloadToken() => throw new NotSupportedException();
        public IConfigurationSection GetSection(string key) => throw new NotSupportedException();
    }

    private sealed class FakeHostEnvironment(string baseAddress) : IWebAssemblyHostEnvironment
    {
        public string Environment => "Production";
        public string BaseAddress { get; } = baseAddress;
    }

    [Theory]
    [InlineData(null, "http://host-environment.example/")]
    [InlineData("", "http://host-environment.example/")]
    [InlineData("   ", "http://host-environment.example/")]
    public void ResolveBaseAddress_falls_back_to_the_host_environment_when_ApiBase_is_blank(string? apiBase, string expected)
    {
        // Arrange
        var configuration = new FakeConfiguration(apiBase);
        var hostEnvironment = new FakeHostEnvironment("http://host-environment.example/");
        // Act
        var resolved = AtlasClient.ResolveBaseAddress(configuration, hostEnvironment);
        // Assert
        Assert.Equal(new Uri(expected), resolved);
    }

    [Fact]
    public void ResolveBaseAddress_prefers_a_configured_ApiBase_over_the_host_environment()
    {
        // Arrange
        var configuration = new FakeConfiguration("http://configured.example");
        var hostEnvironment = new FakeHostEnvironment("http://host-environment.example/");
        // Act
        var resolved = AtlasClient.ResolveBaseAddress(configuration, hostEnvironment);
        // Assert
        Assert.Equal(new Uri("http://configured.example/"), resolved);
    }

    [Fact]
    public void ResolveBaseAddress_appends_a_trailing_slash_a_configured_ApiBase_lacks()
    {
        // Arrange
        var configuration = new FakeConfiguration("http://configured.example/api");
        var hostEnvironment = new FakeHostEnvironment("http://host-environment.example/");
        // Act
        var resolved = AtlasClient.ResolveBaseAddress(configuration, hostEnvironment);
        // Assert
        Assert.Equal(new Uri("http://configured.example/api/"), resolved);
    }

    [Fact]
    public void ResolveBaseAddress_does_not_double_the_trailing_slash_a_configured_ApiBase_already_carries()
    {
        // Arrange
        var configuration = new FakeConfiguration("http://configured.example/");
        var hostEnvironment = new FakeHostEnvironment("http://host-environment.example/");
        // Act
        var resolved = AtlasClient.ResolveBaseAddress(configuration, hostEnvironment);
        // Assert
        Assert.Equal(new Uri("http://configured.example/"), resolved);
    }

    [Fact]
    public async Task SceneTime_requests_the_time_scoped_scene_endpoint_and_caches_it_by_from_and_to()
    {
        // Arrange
        var (client, handler) = MakeClient("""{"places":[],"events":[],"narratives":[],"arrows":[],"polities":[],"landmarks":[],"version":"v"}""");
        // Act
        var first = await client.SceneTime(-100, 100);
        var second = await client.SceneTime(-100, 100);
        // Assert
        Assert.Equal("/api/scene", handler.RequestedUris[0].AbsolutePath);
        Assert.Equal("?from=-100&to=100", handler.RequestedUris[0].Query);
        Assert.Same(first, second);
        Assert.Single(handler.RequestedUris);
    }

    [Fact]
    public async Task SceneTime_treats_a_different_window_as_a_different_cache_entry()
    {
        // Arrange
        var (client, handler) = MakeClient(
            """{"places":[],"events":[],"narratives":[],"arrows":[],"polities":[],"landmarks":[],"version":"a"}""",
            """{"places":[],"events":[],"narratives":[],"arrows":[],"polities":[],"landmarks":[],"version":"b"}""");
        // Act
        await client.SceneTime(-100, 100);
        await client.SceneTime(-200, 200);
        // Assert
        Assert.Equal(2, handler.RequestedUris.Count);
        Assert.Equal("?from=-200&to=200", handler.RequestedUris[1].Query);
    }

    [Fact]
    public async Task SceneScripture_requests_the_scripture_scoped_scene_endpoint_and_caches_it_by_ref()
    {
        // Arrange
        var (client, handler) = MakeClient("""{"places":[],"events":[],"narratives":[],"arrows":[],"polities":[],"landmarks":[],"version":"v"}""");
        // Act
        var first = await client.SceneScripture("JHN.3.16");
        var second = await client.SceneScripture("JHN.3.16");
        // Assert
        Assert.Equal("/api/scene/scripture", handler.RequestedUris[0].AbsolutePath);
        Assert.Equal("?ref=JHN.3.16", handler.RequestedUris[0].Query);
        Assert.Same(first, second);
        Assert.Single(handler.RequestedUris);
    }

    [Fact]
    public async Task SceneScripture_treats_a_different_ref_as_a_different_cache_entry()
    {
        // Arrange
        var (client, handler) = MakeClient(
            """{"places":[],"events":[],"narratives":[],"arrows":[],"polities":[],"landmarks":[],"version":"a"}""",
            """{"places":[],"events":[],"narratives":[],"arrows":[],"polities":[],"landmarks":[],"version":"b"}""");
        // Act
        await client.SceneScripture("JHN.3.16");
        await client.SceneScripture("GEN.1.1");
        // Assert
        Assert.Equal(2, handler.RequestedUris.Count);
        Assert.Equal("?ref=GEN.1.1", handler.RequestedUris[1].Query);
    }

    [Fact]
    public async Task Books_requests_the_books_endpoint()
    {
        // Arrange
        var (client, handler) = MakeClient("[]");
        // Act
        await client.Books();
        // Assert
        Assert.Equal("/api/books", handler.RequestedUris[0].AbsolutePath);
    }

    [Fact]
    public async Task Eras_requests_the_eras_endpoint()
    {
        // Arrange
        var (client, handler) = MakeClient("[]");
        // Act
        await client.Eras();
        // Assert
        Assert.Equal("/api/eras", handler.RequestedUris[0].AbsolutePath);
    }

    [Fact]
    public async Task Chapter_requests_the_book_and_chapter_and_caches_it_by_both()
    {
        // Arrange
        var (client, handler) = MakeClient("""{"book":"John","chapter":3,"ref":"JHN.3","verses":[]}""");
        // Act
        var first = await client.Chapter("John", 3);
        var second = await client.Chapter("John", 3);
        // Assert
        Assert.Equal("/api/chapter/John.3", handler.RequestedUris[0].AbsolutePath);
        Assert.Same(first, second);
        Assert.Single(handler.RequestedUris);
    }

    [Fact]
    public async Task Chapter_treats_a_different_chapter_of_the_same_book_as_a_different_cache_entry()
    {
        // Arrange
        var (client, handler) = MakeClient(
            """{"book":"John","chapter":3,"ref":"JHN.3","verses":[]}""",
            """{"book":"John","chapter":4,"ref":"JHN.4","verses":[]}""");
        // Act
        await client.Chapter("John", 3);
        await client.Chapter("John", 4);
        // Assert
        Assert.Equal(2, handler.RequestedUris.Count);
        Assert.Equal("/api/chapter/John.4", handler.RequestedUris[1].AbsolutePath);
    }

    [Fact]
    public async Task Verse_requests_the_verse_endpoint()
    {
        // Arrange
        var (client, handler) = MakeClient("{}");
        // Act
        await client.Verse("JHN.3.16");
        // Assert
        Assert.Equal("/api/verse/JHN.3.16", handler.RequestedUris[0].AbsolutePath);
    }

    [Fact]
    public async Task KretzmannChapter_requests_the_kretzmann_chapter_endpoint_uncached()
    {
        // Arrange
        var (client, handler) = MakeClient("{}", "{}");
        // Act
        await client.KretzmannChapter("John", 3);
        await client.KretzmannChapter("John", 3);
        // Assert
        Assert.All(handler.RequestedUris, uri => Assert.Equal("/api/kretzmann/chapter/John.3", uri.AbsolutePath));
        Assert.Equal(2, handler.RequestedUris.Count);
    }

    [Fact]
    public async Task Xrefs_requests_the_xrefs_endpoint_and_caches_it_by_ref()
    {
        // Arrange
        var (client, handler) = MakeClient("[]");
        // Act
        var first = await client.Xrefs("JHN.3.16");
        var second = await client.Xrefs("JHN.3.16");
        // Assert
        Assert.Equal("/api/xrefs/JHN.3.16", handler.RequestedUris[0].AbsolutePath);
        Assert.Same(first, second);
        Assert.Single(handler.RequestedUris);
    }

    [Fact]
    public async Task Xrefs_treats_a_different_ref_as_a_different_cache_entry()
    {
        // Arrange
        var (client, handler) = MakeClient("[]", "[]");
        // Act
        await client.Xrefs("JHN.3.16");
        await client.Xrefs("GEN.1.1");
        // Assert
        Assert.Equal(2, handler.RequestedUris.Count);
        Assert.Equal("/api/xrefs/GEN.1.1", handler.RequestedUris[1].AbsolutePath);
    }

    [Fact]
    public async Task Catechism_requests_the_catechism_span_endpoint_and_caches_it_by_ref()
    {
        // Arrange
        var (client, handler) = MakeClient("[]");
        // Act
        var first = await client.Catechism("JHN.3.16");
        var second = await client.Catechism("JHN.3.16");
        // Assert
        Assert.Equal("/api/catechism/JHN.3.16", handler.RequestedUris[0].AbsolutePath);
        Assert.Same(first, second);
        Assert.Single(handler.RequestedUris);
    }

    [Fact]
    public async Task Catechism_treats_a_different_ref_as_a_different_cache_entry()
    {
        // Arrange
        var (client, handler) = MakeClient("[]", "[]");
        // Act
        await client.Catechism("JHN.3.16");
        await client.Catechism("GEN.1.1");
        // Assert
        Assert.Equal(2, handler.RequestedUris.Count);
        Assert.Equal("/api/catechism/GEN.1.1", handler.RequestedUris[1].AbsolutePath);
    }

    [Fact]
    public async Task CatechismItem_requests_the_catechism_item_endpoint()
    {
        // Arrange
        var (client, handler) = MakeClient("{}");
        // Act
        await client.CatechismItem("commandment-1");
        // Assert
        Assert.Equal("/api/catechism/item/commandment-1", handler.RequestedUris[0].AbsolutePath);
    }

    [Fact]
    public async Task Narratives_requests_the_narratives_endpoint()
    {
        // Arrange
        var (client, handler) = MakeClient("[]");
        // Act
        await client.Narratives();
        // Assert
        Assert.Equal("/api/narratives", handler.RequestedUris[0].AbsolutePath);
    }

    [Fact]
    public async Task NarrativeEventPositions_requests_the_narrative_event_endpoint()
    {
        // Arrange
        var (client, handler) = MakeClient("{}");
        // Act
        await client.NarrativeEventPositions("Event:ab_ur");
        // Assert
        Assert.Equal("/api/narrative/event/Event:ab_ur", Uri.UnescapeDataString(handler.RequestedUris[0].AbsolutePath));
    }

    [Fact]
    public async Task Event_requests_the_event_endpoint()
    {
        // Arrange
        var (client, handler) = MakeClient("{}");
        // Act
        await client.Event("Event:ab_ur");
        // Assert
        Assert.Equal("/api/event/Event:ab_ur", Uri.UnescapeDataString(handler.RequestedUris[0].AbsolutePath));
    }

    [Fact]
    public async Task Polities_requests_the_polities_endpoint_and_caches_it_by_from_and_to()
    {
        // Arrange
        var (client, handler) = MakeClient("""{"eras":[],"version":"v"}""");
        // Act
        var first = await client.Polities(-100, 100);
        var second = await client.Polities(-100, 100);
        // Assert
        Assert.Equal("/api/polities", handler.RequestedUris[0].AbsolutePath);
        Assert.Equal("?from=-100&to=100", handler.RequestedUris[0].Query);
        Assert.Same(first, second);
        Assert.Single(handler.RequestedUris);
    }

    [Fact]
    public async Task Polities_treats_a_different_window_as_a_different_cache_entry()
    {
        // Arrange
        var (client, handler) = MakeClient("""{"eras":[],"version":"a"}""", """{"eras":[],"version":"b"}""");
        // Act
        await client.Polities(-100, 100);
        await client.Polities(-200, 200);
        // Assert
        Assert.Equal(2, handler.RequestedUris.Count);
        Assert.Equal("?from=-200&to=200", handler.RequestedUris[1].Query);
    }

    [Fact]
    public async Task Landmarks_requests_the_landmarks_endpoint()
    {
        // Arrange
        var (client, handler) = MakeClient("[]");
        // Act
        await client.Landmarks();
        // Assert
        Assert.Equal("/api/landmarks", handler.RequestedUris[0].AbsolutePath);
    }

    [Fact]
    public async Task LandMask_requests_the_land_mask_endpoint()
    {
        // Arrange
        var (client, handler) = MakeClient("""{"rings":[]}""");
        // Act
        await client.LandMask();
        // Assert
        Assert.Equal("/api/land-mask", handler.RequestedUris[0].AbsolutePath);
    }

    [Fact]
    public async Task Sources_requests_the_sources_endpoint()
    {
        // Arrange
        var (client, handler) = MakeClient("{}");
        // Act
        await client.Sources();
        // Assert
        Assert.Equal("/api/sources", handler.RequestedUris[0].AbsolutePath);
    }

    [Fact]
    public async Task NodeRecord_requests_the_generic_node_endpoint()
    {
        // Arrange
        var (client, handler) = MakeClient("""{"id":"text-unit:JHN.3.16","kind":"TextUnit","label":"JHN.3.16","provenance":"kjv","edge_summary":[],"version":"v"}""");
        // Act
        await client.NodeRecord("text-unit:JHN.3.16");
        // Assert
        Assert.Equal("/api/node/text-unit:JHN.3.16", Uri.UnescapeDataString(handler.RequestedUris[0].AbsolutePath));
    }

    [Fact]
    public async Task Contents_requests_the_contents_endpoint_for_the_given_corpus_and_caches_it_by_corpus()
    {
        // Arrange
        var (client, handler) = MakeClient("""{"corpus":"bible","roots":[],"version":"v"}""");
        // Act
        var first = await client.Contents(Corpus.Bible);
        var second = await client.Contents(Corpus.Bible);
        // Assert
        Assert.Equal("/api/contents/bible", handler.RequestedUris[0].AbsolutePath);
        Assert.Same(first, second);
        Assert.Single(handler.RequestedUris);
    }

    [Fact]
    public async Task Contents_treats_a_different_corpus_as_a_different_cache_entry()
    {
        // Arrange
        var (client, handler) = MakeClient(
            """{"corpus":"bible","roots":[],"version":"a"}""",
            """{"corpus":"concord","roots":[],"version":"b"}""");
        // Act
        await client.Contents(Corpus.Bible);
        await client.Contents(Corpus.Concord);
        // Assert
        Assert.Equal(2, handler.RequestedUris.Count);
        Assert.Equal("/api/contents/concord", handler.RequestedUris[1].AbsolutePath);
    }
}
