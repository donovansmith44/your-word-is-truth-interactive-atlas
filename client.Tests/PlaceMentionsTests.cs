using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

// Batch R requirement 5 (place-in-verse hover -> marker blink): PlaceMentions.Scan
// is the mini-reader's own plain-text substring detector -- given a verse's
// text and the places GET /api/chapter/{cref} says are linked to it (server-
// side reverse index, no character-offset data), find which substrings to
// wrap as hoverable mentions. Pure, so tested directly rather than through
// the Razor component that consumes it.
//
// M-D3 fix round (R-D1, review C1): widened to a required third `persons`
// argument when U5 generalized Scan to a second entity kind -- this
// pre-existing file was left uncompiling by that batch (10x CS7036), caught
// by review, not by this session's own original verification. Every call
// site below passes `Array.Empty<PersonRefDto>()` (this file is exercising
// the place-only half of the contract deliberately, one concern per test,
// matching its own established convention -- Person-kind coverage rides
// the Playwright layer instead, MENTION-2/MENTION-4, `tests/ux/popover-
// sections.spec.ts`). `PersonRefDto` resolves with no new `using` needed --
// this file's own `namespace BibleAtlas.Client.Tests` already encloses
// `BibleAtlas.Client` (where both `PlaceRefDto` and `PersonRefDto` live),
// the same reason the pre-existing `PlaceRefDto` usage below never needed
// one either.
public class PlaceMentionsTests {
    private static PlaceRefDto Place(string id, string name) => new(id, name);

    [Fact]
    public void NoPlacesReturnsWholeTextAsOnePlainSegment() {
        var segments = PlaceMentions.Scan("In the beginning God created the heaven.", Array.Empty<PlaceRefDto>(), Array.Empty<PersonRefDto>());
        var seg = Assert.Single(segments);
        Assert.Equal("In the beginning God created the heaven.", seg.Text);
        Assert.Null(seg.PlaceId);
    }

    [Fact]
    public void EmptyTextReturnsOnePlainEmptySegment() {
        var segments = PlaceMentions.Scan("", new[] { Place("jerusalem", "Jerusalem") }, Array.Empty<PersonRefDto>());
        var seg = Assert.Single(segments);
        Assert.Equal("", seg.Text);
        Assert.Null(seg.PlaceId);
    }

    [Fact]
    public void SingleMentionSplitsIntoThreeSegments() {
        var segments = PlaceMentions.Scan("Abram dwelt in Hebron by the plain of Mamre.", new[] { Place("hebron", "Hebron") }, Array.Empty<PersonRefDto>());
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
        var segments = PlaceMentions.Scan("Jerusalem was besieged.", new[] { Place("jerusalem", "Jerusalem") }, Array.Empty<PersonRefDto>());
        Assert.Equal(2, segments.Count);
        Assert.Equal("Jerusalem", segments[0].Text);
        Assert.Equal("jerusalem", segments[0].PlaceId);
        Assert.Equal(" was besieged.", segments[1].Text);
    }

    [Fact]
    public void MentionAtTheVeryEndHasNoTrailingPlainSegment() {
        var segments = PlaceMentions.Scan("They came to Jericho", new[] { Place("jericho", "Jericho") }, Array.Empty<PersonRefDto>());
        Assert.Equal(2, segments.Count);
        Assert.Equal("They came to ", segments[0].Text);
        Assert.Equal("Jericho", segments[1].Text);
        Assert.Equal("jericho", segments[1].PlaceId);
    }

    // M-D3/U5 (Explore/PlaceMentions.cs's own doc comment): matching is now
    // case-SENSITIVE, corrected FROM case-insensitive -- a real, deliberate,
    // tested production fix, not a regression. Case-insensitive matching
    // made the real place "Sin" (a wilderness/city name, EXO.16.1/EZE.30.15)
    // indistinguishable from the ordinary English word "sin". Replaces this
    // file's own former MatchIsCaseInsensitive, which asserted the OPPOSITE,
    // now-retired contract; split into two, one behavior per test, matching
    // this file's own established convention.
    [Fact]
    public void DifferentlyCasedTextNeverMatches() {
        var segments = PlaceMentions.Scan("go up to JERUSALEM now.", new[] { Place("jerusalem", "Jerusalem") }, Array.Empty<PersonRefDto>());
        var seg = Assert.Single(segments);
        Assert.Equal("go up to JERUSALEM now.", seg.Text);
        Assert.Null(seg.PlaceId);
    }

    [Fact]
    public void ExactlyCasedTextStillMatches() {
        var segments = PlaceMentions.Scan("go up to Jerusalem now.", new[] { Place("jerusalem", "Jerusalem") }, Array.Empty<PersonRefDto>());
        Assert.Equal(3, segments.Count);
        Assert.Equal("Jerusalem", segments[1].Text);
        Assert.Equal("jerusalem", segments[1].PlaceId);
    }

    [Fact]
    public void TwoDistinctNonOverlappingMentionsBothWrap() {
        var places = new[] { Place("jerusalem", "Jerusalem"), Place("bethlehem", "Bethlehem") };
        var segments = PlaceMentions.Scan("From Bethlehem to Jerusalem is a short journey.", places, Array.Empty<PersonRefDto>());
        var mentionIds = segments.Where(s => s.PlaceId != null).Select(s => s.PlaceId).ToList();
        Assert.Equal(new[] { "bethlehem", "jerusalem" }, mentionIds);
    }

    // Overlap resolution: a longer place name that CONTAINS a shorter one as
    // a real substring claims the span; the shorter one never also matches
    // inside it (a real risk once two curated names are anywhere near each
    // other alphabetically -- "beersheba" genuinely contains "sheba").
    // M-D3/U5 note: since the Ordinal fix above, this now ALSO exercises the
    // case-sensitivity guard incidentally ("Beersheba"'s own embedded
    // "sheba" is lowercase, so the capitalized "Sheba" candidate no longer
    // even reaches the overlap-resolution step to begin with) -- the
    // OBSERVABLE outcome (exactly one mention, Beersheba) is unchanged
    // either way, so this test still stands as written; not expanded
    // further here (R-D1's own "mechanical" scope), disclosed in the batch
    // report instead.
    [Fact]
    public void LongerContainingNameWinsOverAShorterSubstringName() {
        var places = new[] { Place("beersheba", "Beersheba"), Place("sheba", "Sheba") };
        var segments = PlaceMentions.Scan("They journeyed to Beersheba and rested.", places, Array.Empty<PersonRefDto>());
        var mentions = segments.Where(s => s.PlaceId != null).ToList();
        var mention = Assert.Single(mentions);
        Assert.Equal("beersheba", mention.PlaceId);
        Assert.Equal("Beersheba", mention.Text);
    }

    [Fact]
    public void UnmatchedPlaceIsSimplyAbsentNotAnError() {
        // "Egypt" is in the caller-supplied place list but does not actually
        // appear in this particular verse's text -- no exception, no
        // spurious segment, the text just comes back whole.
        var segments = PlaceMentions.Scan("The LORD spake unto Moses.", new[] { Place("egypt", "Egypt") }, Array.Empty<PersonRefDto>());
        var seg = Assert.Single(segments);
        Assert.Equal("The LORD spake unto Moses.", seg.Text);
    }

    [Fact]
    public void PlaceNameLongerThanTextNeverThrows() {
        var segments = PlaceMentions.Scan("Ur", new[] { Place("mesopotamia", "Mesopotamia") }, Array.Empty<PersonRefDto>());
        var seg = Assert.Single(segments);
        Assert.Equal("Ur", seg.Text);
    }
}
