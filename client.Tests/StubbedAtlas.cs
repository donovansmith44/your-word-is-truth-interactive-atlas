using System.Net;
using System.Text;

namespace BibleAtlas.Client.Tests;

internal sealed class StubbedAtlas(string body) : HttpMessageHandler
{
    public List<string> Asked { get; } = [];

    public AtlasClient Client() => new(new HttpClient(this) { BaseAddress = new Uri("http://localhost/") });

    protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
    {
        Asked.Add(request.RequestUri!.PathAndQuery);
        return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK) { Content = new StringContent(body, Encoding.UTF8, "application/json") });
    }
}
