namespace BibleAtlas.Client.Contract;

public static class Neighbours
{
    public static IEnumerable<NodeRef> Nodes(this IEnumerable<EdgeEntry> entries) => entries.Select(e => e.Neighbour).Nodes();

    public static IEnumerable<NodeRef> Nodes(this IEnumerable<PositionRef> positions) => positions.OfType<NodePosition>().Select(p => p.Node);
}
