namespace BibleAtlas.Client.Explore;

public sealed class RenderedLegacyNodes
{
    private readonly Dictionary<Explorable, IExplorable> _rendered = [];

    public IExplorable? For(Explorable node)
    {
        if (_rendered.TryGetValue(node, out var rendered))
        {
            return rendered;
        }

        if (LegacyNodes.For(node) is { } fresh)
        {
            Remember(node, fresh);
            return fresh;
        }

        return null;
    }

    public void Remember(Explorable node, IExplorable legacy) => _rendered[node] = legacy;
}
