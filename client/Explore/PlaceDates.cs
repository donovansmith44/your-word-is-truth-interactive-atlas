using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record PlaceDate(string Label, DateClaim Claim);

public static class PlaceDates
{
    public const string Established = "Established";
    public const string Destroyed = "Destroyed";

    public static IReadOnlyList<PlaceDate> Of(PlaceDetail? card) =>
        [.. DateOf(Established, card?.Established), .. DateOf(Destroyed, card?.Destroyed)];

    private static IEnumerable<PlaceDate> DateOf(string label, DateClaim? claim) =>
        claim is { } dated ? [new PlaceDate(label, dated)] : [];
}
