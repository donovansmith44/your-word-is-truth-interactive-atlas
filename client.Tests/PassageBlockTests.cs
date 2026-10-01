using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using static BibleAtlas.Client.Tests.EventAccountsTests;

namespace BibleAtlas.Client.Tests;

public sealed class PassageBlockTests
{
    private const int SermonsFirstChapterVerses = 48;

    [Fact]
    public void An_account_with_a_gap_reads_as_its_served_runs_and_explores_by_its_first()
    {
        // Arrange
        var verses = Verses("MRK", 14, [54, .. Enumerable.Range(66, 7)]);
        var peterDenies = new AccountSourceUnit(verses, new EventAccount([Span(BookId.MRK, (14, 54), (14, 54)), Span(BookId.MRK, (14, 66), (14, 72))], null));

        // Act
        var blocks = PassageBlockBuilder.Build([peterDenies]);

        // Assert
        Assert.Equal(
            WholeValue.Of(new[] { new PassageBlockData("MRK.14.54, 66-72", verses, null, FirstRangeEndVref: "MRK.14.54", ExploreSref: "MRK.14.54", ExploreVerseCount: 1) }),
            WholeValue.Of(blocks));
    }

    [Fact]
    public void An_account_read_on_across_chapters_is_one_ref_and_explores_by_its_first_chapter()
    {
        // Arrange
        var verses = Verses("MAT", 5, Enumerable.Range(1, SermonsFirstChapterVerses)).Concat(Verses("MAT", 6, Enumerable.Range(1, 34))).Concat(Verses("MAT", 7, Enumerable.Range(1, 29))).ToList();
        var sermon = new AccountSourceUnit(verses, new EventAccount([Span(BookId.MAT, (5, 1), (7, 29))], null));

        // Act
        var blocks = PassageBlockBuilder.Build([sermon]);

        // Assert
        Assert.Equal(
            WholeValue.Of(new[] { new PassageBlockData("MAT.5.1-7.29", verses, null, FirstRangeEndVref: "MAT.7.29", ExploreSref: "MAT.5.1-48", ExploreVerseCount: SermonsFirstChapterVerses) }),
            WholeValue.Of(blocks));
    }

    [Fact]
    public void A_passage_that_is_not_one_account_splits_into_its_runs_chapter_by_chapter()
    {
        // Arrange
        var fifth = Verses("MAT", 5, Enumerable.Range(1, SermonsFirstChapterVerses));
        var sixth = Verses("MAT", 6, Enumerable.Range(1, 34));

        // Act
        var blocks = PassageBlockBuilder.Build([new PassageSourceUnit([.. fifth, .. sixth])]);

        // Assert
        Assert.Equal(
            WholeValue.Of(new[] { new PassageBlockData("MAT.5.1-48", fifth, null), new PassageBlockData("MAT.6.1-34", sixth, null) }),
            WholeValue.Of(blocks));
    }

    private static List<PassageListVerse> Verses(string book, int chapter, IEnumerable<int> verses) =>
        verses.Select(verse => new PassageListVerse($"{book}.{chapter}.{verse}", $"text of {book}.{chapter}.{verse}")).ToList();
}
