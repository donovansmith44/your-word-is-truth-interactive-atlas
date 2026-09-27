namespace BibleAtlas.Client.Views;

public static class ViewNames
{
    public const string Reader = "reader";
    public const string World = "world";
    public const string Sources = "sources";

    public const string Kretzmann = "kretzmann";

    public const string Concord = "concord";
}

[Flags]
public enum ViewCapabilities
{
    None = 0,
    BearsLocus = 1 << 0,
    BearsWindow = 1 << 1,
}

public static class HatchKinds
{
    public const string EnterSplit = "enter-split";

    public const string ToggleFollow = "toggle-follow";
}
