using BibleAtlas.Client.Contract;
using BibleAtlas.Client;
using BibleAtlas.Client.Components;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public class ArrowNavTests
{
    private static VerseGroup Group(string book, int chapter, int count, params string[] verses) =>
        new(book: book, chapter: chapter, count: count, verses: verses);

    private static readonly Versification RealCanon = Versification.From(new[]
    {
        new CanonBook(chapters: new List<int> { 25, 23, 17, 25, 48, 34, 29, 34, 38, 42, 30, 50, 58, 36, 39, 28, 27, 35, 30, 34, 46, 46, 39, 51, 46, 75, 66, 20 }, code: "MAT", name: "Matthew"),
        new CanonBook(chapters: new List<int> { 45, 28, 35, 41, 43, 56, 37, 38, 50, 52, 33, 44, 37, 72, 47, 20 }, code: "MRK", name: "Mark"),
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

    [Fact]
    public void SkipsALeadingEmptyGroupInFavorOfTheNextOne()
    {
        var groups = new List<VerseGroup>
        {
            new(book: "MAT", chapter: 8, count: 0, verses: []),
            Group("LUK", 5, 2, "LUK.5.12", "LUK.5.13"),
        };
        var refs = ArrowNav.SelectRefs(groups);
        Assert.Single(refs);
        Assert.Equal("LUK.5.12-13", refs[0].Ref);
    }

    [Fact]
    public void EveryGroupEmptyReturnsNoRefs()
    {
        var groups = new List<VerseGroup> { new(book: "MAT", chapter: 8, count: 0, verses: []) };
        Assert.Empty(ArrowNav.SelectRefs(groups));
    }

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

    [Fact]
    public void SelectRefsFromWitnesses_OneRefPerWitness_InServerOrder()
    {
        var witnesses = new List<EventWitness>
        {
            new(book: "LUK", refNote: null, robertsonSection: null, verseGroups: new List<VerseGroup> { Group("LUK", 6, 5, "LUK.6.12", "LUK.6.13", "LUK.6.14", "LUK.6.15", "LUK.6.16") }),
            new(book: "MRK", refNote: null, robertsonSection: null, verseGroups: new List<VerseGroup> { Group("MRK", 3, 7, "MRK.3.13", "MRK.3.14", "MRK.3.15", "MRK.3.16", "MRK.3.17", "MRK.3.18", "MRK.3.19") }),
        };

        var refs = ArrowNav.SelectRefsFromWitnesses(witnesses, RealCanon);

        Assert.Equal(2, refs.Count);
        Assert.Equal("LUK.6.12-16", refs[0].Ref);
        Assert.Equal("LUK.6.12", Assert.IsType<VerseNode>(refs[0].Target).Title);
        Assert.Equal("MRK.3.13-19", refs[1].Ref);
        Assert.Equal("MRK.3.13", Assert.IsType<VerseNode>(refs[1].Target).Title);
    }

    [Fact]
    public void SelectRefsFromWitnesses_CoalescesAMultiChapterWitness_LikeTheLandedFrontierDoes()
    {
        var witnesses = new List<EventWitness>
        {
            new(book: "MAT", refNote: null, robertsonSection: null, verseGroups: new List<VerseGroup>
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
        var witnesses = new List<EventWitness>
        {
            new(book: "MAT", refNote: null, robertsonSection: null, verseGroups: new List<VerseGroup>()),
            new(book: "LUK", refNote: null, robertsonSection: null, verseGroups: new List<VerseGroup> { Group("LUK", 6, 1, "LUK.6.1") }),
        };

        var refs = ArrowNav.SelectRefsFromWitnesses(witnesses, RealCanon);

        Assert.Single(refs);
        Assert.Equal("LUK.6.1", refs[0].Ref);
    }
}
