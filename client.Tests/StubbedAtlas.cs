using System.Net;
using System.Text;

namespace BibleAtlas.Client.Tests;

internal sealed class StubbedAtlas(Func<string, string> bodyFor) : HttpMessageHandler
{
    public StubbedAtlas(string body) : this(_ => body)
    {
    }

    public StubbedAtlas(IReadOnlyDictionary<string, string> bodies) : this(asked => bodies[asked])
    {
    }

    public List<string> Asked { get; } = [];

    public AtlasClient Client() => new(Http());

    public GraphExplorableClient Graph() => new(Http());

    private HttpClient Http() => new(this) { BaseAddress = new Uri("http://localhost/") };

    protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
    {
        var asked = request.RequestUri!.PathAndQuery;
        Asked.Add(asked);
        return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK) { Content = new StringContent(bodyFor(asked), Encoding.UTF8, "application/json") });
    }
}
