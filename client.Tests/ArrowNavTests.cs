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

    // Fix round 2 (N-1): the real MAT/MRK chapter lengths
    // (data/compiled/canon.json) -- what SelectRefsFromWitnesses's own
    // versification-aware coalescing consults. Only the books these
    // fixtures touch.
    private static readonly Versification RealCanon = Versification.From(new[]
    {
        new BookTocEntry("MAT", "Matthew", new List<int> { 25, 23, 17, 25, 48, 34, 29, 34, 38, 42, 30, 50, 58, 36, 39, 28, 27, 35, 30, 34, 46, 46, 39, 51, 46, 75, 66, 20 }),
        new BookTocEntry("MRK", "Mark", new List<int> { 45, 28, 35, 41, 43, 56, 37, 38, 50, 52, 33, 44, 37, 72, 47, 20 }),
    });

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

    // Fix round 2 (review Critical N-1, the fallback's own share of it):
    // the REAL rob_peter_denies MRK group shape -- delivered [54, 66-72],
    // Count=8, a REAL gap (verses 55-61 belong to a DIFFERENT event).
    // The old first+Count-1 arithmetic rendered the fabricated envelope
    // "MRK.14.54-61" (claiming another event's verses, dropping the
    // delivered 62-72); the honest ref is the compound list.
    [Fact]
    public void SelectRefs_AGappedGroupRendersTheHonestCompoundRef_NeverAFabricatedEnvelope()
    {
        var delivered = new[] { "MRK.14.54" }.Concat(Enumerable.Range(66, 7).Select(n => $"MRK.14.{n}")).ToArray();
        var groups = new List<VerseGroup> { Group("MRK", 14, 8, delivered) };
        var refs = ArrowNav.SelectRefs(groups);
        Assert.Single(refs);
        Assert.Equal("MRK.14.54, 66-72", refs[0].Ref);
        Assert.Equal("MRK.14.54", Assert.IsType<VerseNode>(refs[0].Target).Title);
    }

    // ACCT-SET-MISMATCH-1 (fix round 2, owner bug report, verbatim: "when i
    // see Jesus selects the twelve apostles after a night of prayer,
    // MRK.3.13-19 shows below the button, but when i press it theres a
    // parallel from LUKE. So the mapping between the passages below the
    // button and what's actually there when you navigate doesnt work"):
    // direct, isolated proof of ArrowNav.SelectRefsFromWitnesses -- the
    // CORRECT, witness-aware derivation (fixes the mismatch: refs now come
    // from the SAME EventDetail.Witnesses the landed frontier's own
    // PARALLEL ACCOUNTS list reads, not the narrower Adjacent.VerseGroups).
    // Fixture mirrors the REAL rob_twelve_apostles data
    // (data/curated/event-witnesses.toml): a LUK witness (LUK.6.12-16) and
    // a MRK witness (MRK.3.13-19).
    [Fact]
    public void SelectRefsFromWitnesses_OneRefPerWitness_InServerOrder()
    {
        var witnesses = new List<EventWitnessDto>
        {
            new("LUK", new List<VerseGroup> { Group("LUK", 6, 5, "LUK.6.12", "LUK.6.13", "LUK.6.14", "LUK.6.15", "LUK.6.16") }),
            new("MRK", new List<VerseGroup> { Group("MRK", 3, 7, "MRK.3.13", "MRK.3.14", "MRK.3.15", "MRK.3.16", "MRK.3.17", "MRK.3.18", "MRK.3.19") }),
        };

        var refs = ArrowNav.SelectRefsFromWitnesses(witnesses, RealCanon);

        Assert.Equal(2, refs.Count);
        Assert.Equal("LUK.6.12-16", refs[0].Ref); // LUK first -- the SAME server order `witnesses` was given in, never re-sorted
        Assert.Equal("LUK.6.12", Assert.IsType<VerseNode>(refs[0].Target).Title);
        Assert.Equal("MRK.3.13-19", refs[1].Ref);
        Assert.Equal("MRK.3.13", Assert.IsType<VerseNode>(refs[1].Target).Title);
    }

    [Fact]
    public void SelectRefsFromWitnesses_CoalescesAMultiChapterWitness_LikeTheLandedFrontierDoes()
    {
        // Proves this derivation is the IDENTICAL coalescing
        // WitnessUnitsResolver uses (ACCT-COALESCE-1) -- a witness spanning
        // multiple chapters becomes ONE ref, never one per chapter.
        var witnesses = new List<EventWitnessDto>
        {
            new("MAT", new List<VerseGroup>
            {
                Group("MAT", 5, 48, Enumerable.Range(1, 48).Select(n => $"MAT.5.{n}").ToArray()),
                Group("MAT", 6, 34, Enumerable.Range(1, 34).Select(n => $"MAT.6.{n}").ToArray()),
                Group("MAT", 7, 29, Enumerable.Range(1, 29).Select(n => $"MAT.7.{n}").ToArray()),
            }),
        };

        var refs = ArrowNav.SelectRefsFromWitnesses(witnesses, RealCanon);

        Assert.Single(refs);
        Assert.Equal("MAT.5.1-7.29", refs[0].Ref);
    }

    [Fact]
    public void SelectRefsFromWitnesses_SkipsAWitnessWithNoVerses()
    {
        var witnesses = new List<EventWitnessDto>
        {
            new("MAT", new List<VerseGroup>()),
            new("LUK", new List<VerseGroup> { Group("LUK", 6, 1, "LUK.6.1") }),
        };

        var refs = ArrowNav.SelectRefsFromWitnesses(witnesses, RealCanon);

        Assert.Single(refs);
        Assert.Equal("LUK.6.1", refs[0].Ref);
    }
}
