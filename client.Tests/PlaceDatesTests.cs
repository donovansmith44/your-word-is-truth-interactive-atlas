using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class PlaceDatesTests
{
    private const string Traditional = "traditional";
    private static readonly Year Year1003Bc = new(label: "1003 BC", value: -1003);
    private static readonly Year Year586Bc = new(label: "586 BC", value: -586);
    private static readonly DateClaim DavidTakesZion = Claim(Year1003Bc, "c. 1003 BC", Traditional, Verse(BookId._2SA, 5, 7));
    private static readonly DateClaim BabylonBurnsIt = Claim(Year586Bc, "586 BC", note: null, Verse(BookId._2KI, 25, 9));

    [Fact]
    public void A_places_dates_are_the_cards_labelled_claims()
    {
        // Arrange
        var card = Jerusalem(established: DavidTakesZion, destroyed: BabylonBurnsIt);

        // Act
        var dates = PlaceDates.Of(card);

        // Assert
        Assert.Equal([new PlaceDate("Established", DavidTakesZion), new PlaceDate("Destroyed", BabylonBurnsIt)], dates);
    }

    [Fact]
    public void A_place_with_only_a_fall_has_only_that_date()
    {
        // Arrange
        var card = Jerusalem(established: null, destroyed: BabylonBurnsIt);

        // Act
        var dates = PlaceDates.Of(card);

        // Assert
        Assert.Equal([new PlaceDate("Destroyed", BabylonBurnsIt)], dates);
    }

    [Fact]
    public void A_place_the_card_dates_nowhere_has_no_dates()
    {
        // Arrange
        var card = Jerusalem(established: null, destroyed: null);

        // Act
        var dates = new[] { PlaceDates.Of(card), PlaceDates.Of(null) };

        // Assert
        Assert.Equal([[], []], dates);
    }

    private static PlaceDetail Jerusalem(DateClaim? established, DateClaim? destroyed) =>
        new(blurb: null, canonicalName: null, destroyed: destroyed, displayName: "Jerusalem", established: established, lat: 31.78, lon: 35.23);

    private static TextSpan Verse(BookId book, int chapter, int verse) =>
        new(from: new TextPoint(unit: new BibleRef(book, chapter, verse), word: null), to: new TextPoint(unit: new BibleRef(book, chapter, verse), word: null));

    private static DateClaim Claim(Year year, string label, string? note, TextSpan verse) =>
        new(@event: null, label: label, note: note, verses: [verse], when: new TimeRange(from: year, label: year.Label, to: year));
}
