using System.Net;
using System.Net.Http;
using System.Text;
using System.Threading;

namespace BibleAtlas.Client.Tests;

public sealed class RequiredResponsesTests
{
    private const string BaseAddress = "http://localhost:8000/";

    private const string Url = "api/text?ref=GEN.1.1";

    [Fact]
    public async Task GetRequired_reads_the_body_as_the_asked_for_type()
    {
        // Arrange
        var http = ClientAnswering("""["GEN.1.1","GEN.1.2"]""");
        // Act
        var refs = await http.GetRequired<List<string>>(Url);
        // Assert
        Assert.Equal(["GEN.1.1", "GEN.1.2"], refs);
    }

    [Fact]
    public async Task GetRequired_of_a_null_body_fails_naming_the_url()
    {
        // Arrange
        var http = ClientAnswering("null");
        // Act
        Func<Task> act = () => http.GetRequired<List<string>>(Url);
        // Assert
        var thrown = await Assert.ThrowsAsync<InvalidOperationException>(act);
        Assert.Equal("empty response body from api/text?ref=GEN.1.1", thrown.Message);
    }

    private static HttpClient ClientAnswering(string body) =>
        new(new FixedAnswer(body)) { BaseAddress = new Uri(BaseAddress) };

    private sealed class FixedAnswer(string body) : HttpMessageHandler
    {
        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken) =>
            Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK) { Content = new StringContent(body, Encoding.UTF8, "application/json") });
    }
}
