using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Views;

namespace BibleAtlas.Client.State;

public sealed record ViewArrangement(IReadOnlyList<string> Members, string LayoutKind, double? DividerFraction, bool Follow)
{
    public static readonly ViewArrangement Default = new(new[] { ViewNames.Reader }, LayoutKinds.Single, null, false);

    public const double InitialDividerFraction = 0.5;

    public bool Equals(ViewArrangement? other) =>
        other is not null
        && Members.SequenceEqual(other.Members)
        && LayoutKind == other.LayoutKind
        && DividerFraction == other.DividerFraction
        && Follow == other.Follow;

    public override int GetHashCode()
    {
        var hash = new HashCode();
        foreach (var m in Members)
        {
            hash.Add(m);
        }

        hash.Add(LayoutKind);
        hash.Add(DividerFraction);
        hash.Add(Follow);
        return hash.ToHashCode();
    }
}

public static class LayoutKinds
{
    public const string Single = "single";
    public const string SplitH = "split-h";

    public static readonly IReadOnlyList<string> All = new[] { Single, SplitH };

    public static bool IsKnown(string kind) => All.Contains(kind);
}

public sealed record EnterSingle(string ViewName, string? Origin = null) : IIntent<ViewArrangement>
{
    string IIntent<ViewArrangement>.Name => "enter-single";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is { LayoutKind: LayoutKinds.Single } a && a.Members.Count == 1 && a.Members[0] == ViewName
            ? current
            : new ViewArrangement(new[] { ViewName }, LayoutKinds.Single, null, false);
}

public sealed record EnterSplit(string Host, string Guest, bool DefaultFollow = false, double? DefaultDividerFraction = null, string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "enter-split";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is { LayoutKind: LayoutKinds.SplitH } a && a.Members.Count == 2 && a.Members[0] == Host && a.Members[1] == Guest
            ? current
            : new ViewArrangement(new[] { Host, Guest }, LayoutKinds.SplitH, DefaultDividerFraction, DefaultFollow);
}

public sealed record CloseGuest(string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "close-guest";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is { LayoutKind: LayoutKinds.SplitH } a
            ? new ViewArrangement(new[] { a.Members[0] }, LayoutKinds.Single, null, false)
            : current;
}

public sealed record SetDivider(double Fraction, string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "set-divider";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is { LayoutKind: LayoutKinds.SplitH } a ? a with { DividerFraction = Fraction } : current;
}

// Deliberately unconditional: capability checks (whether the pair can actually follow) happen at
// read sites, not here -- dispatching this against a pairing that can't follow is still a valid,
// inert write, not a caller error.
public sealed record ToggleFollow(bool Follow, string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "toggle-follow";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is { LayoutKind: LayoutKinds.SplitH } a ? a with { Follow = Follow } : current;
}
