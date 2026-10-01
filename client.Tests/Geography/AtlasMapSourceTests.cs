using System.Net;
using System.Net.Http;
using System.Text;
using BibleAtlas.Client.Geography;

namespace BibleAtlas.Client.Tests.Geography;

public sealed class AtlasMapSourceTests
{
    private const string SceneJson = """{"places":[],"events":[],"narratives":[],"arrows":[],"polities":[],"landmarks":[],"version":"s"}""";
    private const string PolitiesJson = """{"eras":[],"version":"p"}""";

    private sealed class RoutingHandler : HttpMessageHandler
    {
        public readonly List<string> Requested = [];

        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
        {
            var path = request.RequestUri!.AbsolutePath;
            Requested.Add(path);
            var body = path == "/api/polities" ? PolitiesJson : SceneJson;
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK) { Content = new StringContent(body, Encoding.UTF8, "application/json") });
        }
    }

    private static (AtlasMapSource Source, AtlasClient Atlas, RoutingHandler Handler) Make()
    {
        var handler = new RoutingHandler();
        var atlas = new AtlasClient(new HttpClient(handler) { BaseAddress = new Uri("http://localhost:8000/") });
        return (new AtlasMapSource(atlas), atlas, handler);
    }

    [Fact]
    public async Task The_layers_during_a_window_are_that_windows_scene_and_its_polities()
    {
        // Arrange
        var (source, atlas, _) = Make();
        var expected = new MapLayers(await atlas.SceneTime(-5, 33), (await atlas.Polities(-5, 33)).All);
        // Act
        var layers = await source.During(-5, 33);
        // Assert
        Assert.Equal(expected, layers);
    }

    [Fact]
    public async Task The_layers_for_a_scripture_are_its_scene_with_no_polities()
    {
        // Arrange
        var (source, atlas, handler) = Make();
        var expected = new MapLayers(await atlas.SceneScripture("JHN.3.16"), []);
        handler.Requested.Clear();
        // Act
        var layers = await source.For("JHN.3.16");
        // Assert
        Assert.Equal((expected.Scene, 0, 0), (layers.Scene, layers.Polities.Count, handler.Requested.Count));
    }

    [Fact]
    public async Task The_roster_is_the_polities_of_the_whole_timeline_without_its_scene()
    {
        // Arrange
        var (source, atlas, handler) = Make();
        var expected = (await atlas.Polities(-4004, 100)).All;
        handler.Requested.Clear();
        // Act
        var roster = await source.Roster();
        // Assert
        Assert.Equal((expected, 0), (roster, handler.Requested.Count));
    }
}
