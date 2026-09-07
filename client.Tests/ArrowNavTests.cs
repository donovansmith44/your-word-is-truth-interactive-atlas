using BibleAtlas.Client;
using BibleAtlas.Client.Components;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

// EVT-3 Ticket 2 (owner ruling, supersedes EV-1 -- ArrowNav.razor's own
// header comment has the full retirement story: "not showing the verse
// contents, just the list of refs"; EVENT-ACCOUNTS-1's own governing law,
// "events are keys that map to sets of Biblical accounts"): direct,
// isolated proof of ArrowNav.SelectRefs's own pure selection logic -- the
// piece that decides which refs (and which VerseNode targets) a Chronology
// traversal row's own RefsList shows. RETARGETED from the retired
// SelectPrimaryVerse's own single-(ref,vref)-tuple shape onto the new
// List<RefsList.RefDescriptor> shape -- same "pure logic here, real
// render/interaction there (tests/ux/event-timeline.spec.ts)" split this
// app's other component-adjacent static helpers already follow
// (VerseLineTests.cs's own BijectiveBase26, PlaceMentionsTests.cs's own
// Scan).
public class ArrowNavTests
{
    private static VerseGroup Group(string book, int chapter, int count, params string[] verses) =>
        new(book, chapter, verses.ToList(), count);

    [Fact]
    public void NullGroupsReturnsNoRefs()
    {
        Assert.Empty(ArrowNav.SelectRefs(null));
    }

    [Fact]
    public void EmptyGroupListReturnsNoRefs()
    {
        Assert.Empty(ArrowNav.SelectRefs(new List<VerseGroup>()));
    }

    // The charter case, verbatim from the leper pair: rob_leper_healed's
    // own top-level MAT.8.1-4 group -- a RANGE group's own ref is its FULL
    // span, and its target is a VerseNode at the group's own FIRST vref
    // ("where it points," never a fabricated PassageNode with no real
    // text -- this file's own header comment has the "why" story).
    [Fact]
    public void RangeGroupReturnsItsFullSpanAndAVerseNodeAtItsFirstVref()
    {
        var groups = new List<VerseGroup> { Group("MAT", 8, 4, "MAT.8.1", "MAT.8.2", "MAT.8.3", "MAT.8.4") };
        var refs = ArrowNav.SelectRefs(groups);
        Assert.Single(refs);
        Assert.Equal("MAT.8.1-4", refs[0].Ref);
        var target = Assert.IsType<VerseNode>(refs[0].Target);
        Assert.Equal("MAT.8.1", target.Title);
    }

    [Fact]
    public void LoneVerseGroupReturnsTheBareRefItself()
    {
        var groups = new List<VerseGroup> { Group("JHN", 3, 1, "JHN.3.16") };
        var refs = ArrowNav.SelectRefs(groups);
        Assert.Single(refs);
        Assert.Equal("JHN.3.16", refs[0].Ref);
        Assert.Equal("JHN.3.16", Assert.IsType<VerseNode>(refs[0].Target).Title);
    }

    // EVENT-ACCOUNTS-1's own core reversal from the retired SelectPrimaryVerse:
    // MULTIPLE witness groups (e.g. an event with MAT + MRK witnesses) each
    // contribute their OWN ref -- the SET, not just the first ("primary")
    // one SelectPrimaryVerse used to keep alone.
    [Fact]
    public void MultipleGroupsReturnsARefForEachOne()
    {
        var groups = new List<VerseGroup>
        {
            Group("MAT", 8, 3, "MAT.8.2", "MAT.8.3", "MAT.8.4"),
            Group("MRK", 1, 2, "MRK.1.40", "MRK.1.41"),
        };
        var refs = ArrowNav.SelectRefs(groups);
        Assert.Equal(2, refs.Count);
        Assert.Equal("MAT.8.2-4", refs[0].Ref);
        Assert.Equal("MAT.8.2", Assert.IsType<VerseNode>(refs[0].Target).Title);
        Assert.Equal("MRK.1.40-41", refs[1].Ref);
        Assert.Equal("MRK.1.40", Assert.IsType<VerseNode>(refs[1].Target).Title);
    }

    // PERF-3's own "identity never narrows" ruling, extended here: a
    // server-capped group (HOTFIX-4's own 20-verse-per-chapter cap --
    // Count=66 but only 20 real Verses delivered) must still report its
    // HONEST full span, computed from Count, never truncated to what
    // merely arrived on the wire.
    [Fact]
    public void ACappedGroupReportsItsHonestFullSpanFromCount_NeverTheDeliveredTail()
    {
        var deliveredVerses = Enumerable.Range(1, 20).Select(n => $"1KI.8.{n}").ToArray();
        var groups = new List<VerseGroup> { Group("1KI", 8, 66, deliveredVerses) };
        var refs = ArrowNav.SelectRefs(groups);
        Assert.Single(refs);
        Assert.Equal("1KI.8.1-66", refs[0].Ref);
        Assert.Equal("1KI.8.1", Assert.IsType<VerseNode>(refs[0].Target).Title);
    }

    // Defensive: a leading group with no verses at all (never real, but
    // not trusted blindly either) is skipped in favor of the next
    // non-empty one.
    [Fact]
    public void SkipsALeadingEmptyGroupInFavorOfTheNextOne()
    {
        var groups = new List<VerseGroup>
        {
            new("MAT", 8, new List<string>(), 0),
            Group("LUK", 5, 2, "LUK.5.12", "LUK.5.13"),
        };
        var refs = ArrowNav.SelectRefs(groups);
        Assert.Single(refs);
        Assert.Equal("LUK.5.12-13", refs[0].Ref);
    }

    [Fact]
    public void EveryGroupEmptyReturnsNoRefs()
    {
        var groups = new List<VerseGroup> { new("MAT", 8, new List<string>(), 0) };
        Assert.Empty(ArrowNav.SelectRefs(groups));
    }
}
