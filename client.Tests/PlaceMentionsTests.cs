using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public class PlaceMentionsTests {
    private static PlaceRef Place(string id, string name) => new(id: id, name: name);

    [Fact]
    public void NoPlacesReturnsWholeTextAsOnePlainSegment() {
        var segments = PlaceMentions.Scan("In the beginning God created the heaven.", Array.Empty<PlaceRef>(), Array.Empty<PersonRef>());
        var seg = Assert.Single(segments);
        Assert.Equal("In the beginning God created the heaven.", seg.Text);
        Assert.Null(seg.PlaceId);
    }

    [Fact]
    public void EmptyTextReturnsOnePlainEmptySegment() {
        var segments = PlaceMentions.Scan("", new[] { Place("jerusalem", "Jerusalem") }, Array.Empty<PersonRef>());
        var seg = Assert.Single(segments);
        Assert.Equal("", seg.Text);
        Assert.Null(seg.PlaceId);
    }

    [Fact]
    public void SingleMentionSplitsIntoThreeSegments() {
        var segments = PlaceMentions.Scan("Abram dwelt in Hebron by the plain of Mamre.", new[] { Place("hebron", "Hebron") }, Array.Empty<PersonRef>());
        Assert.Equal(3, segments.Count);
        Assert.Equal("Abram dwelt in ", segments[0].Text);
        Assert.Null(segments[0].PlaceId);
        Assert.Equal("Hebron", segments[1].Text);
        Assert.Equal("hebron", segments[1].PlaceId);
        Assert.Equal("Hebron", segments[1].PlaceName);
        Assert.Equal(" by the plain of Mamre.", segments[2].Text);
        Assert.Null(segments[2].PlaceId);
    }

    [Fact]
    public void MentionAtTheVeryStartHasNoLeadingPlainSegment() {
        var segments = PlaceMentions.Scan("Jerusalem was besieged.", new[] { Place("jerusalem", "Jerusalem") }, Array.Empty<PersonRef>());
        Assert.Equal(2, segments.Count);
        Assert.Equal("Jerusalem", segments[0].Text);
        Assert.Equal("jerusalem", segments[0].PlaceId);
        Assert.Equal(" was besieged.", segments[1].Text);
    }

    [Fact]
    public void MentionAtTheVeryEndHasNoTrailingPlainSegment() {
        var segments = PlaceMentions.Scan("They came to Jericho", new[] { Place("jericho", "Jericho") }, Array.Empty<PersonRef>());
        Assert.Equal(2, segments.Count);
        Assert.Equal("They came to ", segments[0].Text);
        Assert.Equal("Jericho", segments[1].Text);
        Assert.Equal("jericho", segments[1].PlaceId);
    }

    [Fact]
    public void DifferentlyCasedTextNeverMatches() {
        var segments = PlaceMentions.Scan("go up to JERUSALEM now.", new[] { Place("jerusalem", "Jerusalem") }, Array.Empty<PersonRef>());
        var seg = Assert.Single(segments);
        Assert.Equal("go up to JERUSALEM now.", seg.Text);
        Assert.Null(seg.PlaceId);
    }

    [Fact]
    public void ExactlyCasedTextStillMatches() {
        var segments = PlaceMentions.Scan("go up to Jerusalem now.", new[] { Place("jerusalem", "Jerusalem") }, Array.Empty<PersonRef>());
        Assert.Equal(3, segments.Count);
        Assert.Equal("Jerusalem", segments[1].Text);
        Assert.Equal("jerusalem", segments[1].PlaceId);
    }

    [Fact]
    public void TwoDistinctNonOverlappingMentionsBothWrap() {
        var places = new[] { Place("jerusalem", "Jerusalem"), Place("bethlehem", "Bethlehem") };
        var segments = PlaceMentions.Scan("From Bethlehem to Jerusalem is a short journey.", places, Array.Empty<PersonRef>());
        var mentionIds = segments.Where(s => s.PlaceId != null).Select(s => s.PlaceId).ToList();
        Assert.Equal(new[] { "bethlehem", "jerusalem" }, mentionIds);
    }

    [Fact]
    public void LongerContainingNameWinsOverAShorterSubstringName() {
        var places = new[] { Place("beersheba", "Beersheba"), Place("sheba", "Sheba") };
        var segments = PlaceMentions.Scan("They journeyed to Beersheba and rested.", places, Array.Empty<PersonRef>());
        var mentions = segments.Where(s => s.PlaceId != null).ToList();
        var mention = Assert.Single(mentions);
        Assert.Equal("beersheba", mention.PlaceId);
        Assert.Equal("Beersheba", mention.Text);
    }

    [Fact]
    public void UnmatchedPlaceIsSimplyAbsentNotAnError() {
        var segments = PlaceMentions.Scan("The LORD spake unto Moses.", new[] { Place("egypt", "Egypt") }, Array.Empty<PersonRef>());
        var seg = Assert.Single(segments);
        Assert.Equal("The LORD spake unto Moses.", seg.Text);
    }

    [Fact]
    public void PlaceNameLongerThanTextNeverThrows() {
        var segments = PlaceMentions.Scan("Ur", new[] { Place("mesopotamia", "Mesopotamia") }, Array.Empty<PersonRef>());
        var seg = Assert.Single(segments);
        Assert.Equal("Ur", seg.Text);
    }
}
