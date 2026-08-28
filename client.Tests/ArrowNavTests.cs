using BibleAtlas.Client;
using BibleAtlas.Client.Components;

namespace BibleAtlas.Client.Tests;

// EV-1 (owner, verbatim: "when traversing events, i should be able to see
// the actual event-mapped verse, rather than just the event title when
// going through the chronology."): direct, isolated proof of
// ArrowNav.SelectPrimaryVerse's own pure selection logic -- the piece that
// decides WHICH verse ref/text a Chronology traversal row's own
// always-visible verse line resolves and shows. The async fetch/render
// itself (VerseTextResolver.ResolveAsync, MentionText) is exercised by the
// Playwright suite instead (tests/ux/event-timeline.spec.ts), the same
// "pure logic here, real network/render there" split this app's other
// component-adjacent static helpers already follow (VerseLineTests.cs's
// own BijectiveBase26, PlaceMentionsTests.cs's own Scan).
public class ArrowNavTests
{
    private static VerseGroup Group(string book, int chapter, int count, params string[] verses) =>
        new(book, chapter, verses.ToList(), count);

    [Fact]
    public void NullGroupsReturnsNoRefAndNoVerse()
    {
        var (spanRef, firstVref) = ArrowNav.SelectPrimaryVerse(null);
        Assert.Null(spanRef);
        Assert.Null(firstVref);
    }

    [Fact]
    public void EmptyGroupListReturnsNoRefAndNoVerse()
    {
        var (spanRef, firstVref) = ArrowNav.SelectPrimaryVerse(new List<VerseGroup>());
        Assert.Null(spanRef);
        Assert.Null(firstVref);
    }

    // The charter case, verbatim from the leper pair: rob_leper_healed's
    // own top-level MAT.8.1-4 group (Batch CHRON-1 widened it from 8.2-4
    // to restore theo-286's own boundary-verse coverage -- see
    // batch-chron1-report.md) -- a RANGE group shows its own SPAN ref
    // ("MAT.8.1-4") but resolves only the FIRST verse (MAT.8.1), never
    // the whole range, per the brief's own words.
    [Fact]
    public void RangeGroupReturnsSpanRefAndFirstVrefOnly()
    {
        var groups = new List<VerseGroup> { Group("MAT", 8, 4, "MAT.8.1", "MAT.8.2", "MAT.8.3", "MAT.8.4") };
        var (spanRef, firstVref) = ArrowNav.SelectPrimaryVerse(groups);
        Assert.Equal("MAT.8.1-4", spanRef);
        Assert.Equal("MAT.8.1", firstVref);
    }

    [Fact]
    public void LoneVerseGroupReturnsTheBareRefItself()
    {
        var groups = new List<VerseGroup> { Group("JHN", 3, 1, "JHN.3.16") };
        var (spanRef, firstVref) = ArrowNav.SelectPrimaryVerse(groups);
        Assert.Equal("JHN.3.16", spanRef);
        Assert.Equal("JHN.3.16", firstVref);
    }

    // Multiple witness groups (e.g. an event with MAT + MRK witnesses) --
    // the FIRST group (server order) is this event's own primary
    // attestation; later groups are never consulted here at all (they
    // stay the dwell peek's own job, unchanged).
    [Fact]
    public void MultipleGroupsUsesOnlyTheFirst()
    {
        var groups = new List<VerseGroup>
        {
            Group("MAT", 8, 3, "MAT.8.2", "MAT.8.3", "MAT.8.4"),
            Group("MRK", 1, 6, "MRK.1.40", "MRK.1.41"),
        };
        var (spanRef, firstVref) = ArrowNav.SelectPrimaryVerse(groups);
        Assert.Equal("MAT.8.2-4", spanRef);
        Assert.Equal("MAT.8.2", firstVref);
    }

    // Defensive: a leading group with no verses at all (never real, but
    // not trusted blindly either -- see this method's own doc comment)
    // is skipped in favor of the next non-empty one.
    [Fact]
    public void SkipsALeadingEmptyGroupInFavorOfTheNextOne()
    {
        var groups = new List<VerseGroup>
        {
            new("MAT", 8, new List<string>(), 0),
            Group("LUK", 5, 5, "LUK.5.12", "LUK.5.13"),
        };
        var (spanRef, firstVref) = ArrowNav.SelectPrimaryVerse(groups);
        Assert.Equal("LUK.5.12-13", spanRef);
        Assert.Equal("LUK.5.12", firstVref);
    }

    [Fact]
    public void EveryGroupEmptyReturnsNoRefAndNoVerse()
    {
        var groups = new List<VerseGroup> { new("MAT", 8, new List<string>(), 0) };
        var (spanRef, firstVref) = ArrowNav.SelectPrimaryVerse(groups);
        Assert.Null(spanRef);
        Assert.Null(firstVref);
    }
}
