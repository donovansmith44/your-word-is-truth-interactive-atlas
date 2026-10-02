using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public enum Surface
{
    World,
    Reader,
    Popover,
}

public static class HomeSurfaces
{
    public static Surface Of(NodeKind kind) =>
        Enum.GetValues<Surface>().SingleOrDefault(surface => surface != Surface.Popover && Presentation.Of(kind, surface) is not null, Surface.Popover);
}
