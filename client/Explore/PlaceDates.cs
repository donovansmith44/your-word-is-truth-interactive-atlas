using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record PlaceDate(string Label, DateClaim Claim, IReadOnlyList<string> Verses);

public static class PlaceDates
{
    public const string Established = "Established";
    public const string Destroyed = "Destroyed";

    public static readonly IReadOnlyList<string> NoVerses = [];

    // The card serves each date labelled; the verses a claim rests on are read as the place page
    // names them, the form every verse chip and verse lookup here takes.
    public static IReadOnlyList<PlaceDate> Of(PlaceDetail? card, History? history) =>
        [.. DateOf(Established, card?.Established, history?.Established), .. DateOf(Destroyed, card?.Destroyed, history?.Destroyed)];

    private static IEnumerable<PlaceDate> DateOf(string label, DateClaim? claim, PlaceDateClaim? onThePage) =>
        claim is { } dated ? [new PlaceDate(label, dated, onThePage?.Verses ?? NoVerses)] : [];
}
