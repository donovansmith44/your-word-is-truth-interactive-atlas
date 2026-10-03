using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

public static class LegacyTextUnits
{
    public static NodeRef Node(string servedRef) => new(id: NodeIds.Of(NodeKind.TextUnit, servedRef), kind: NodeKind.TextUnit, label: servedRef);

    public static PopoverOpening Opening(string servedRef) => new PopoverOpening.Explore(new NodePosition(Node(servedRef)));
}
