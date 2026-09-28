using BibleAtlas.Client.Contract;
using BibleAtlas.Client;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public class AcctCoalesceTests
{
    private const int WireCappedVerses = 20;
    private const int MatthewChapters = 28;
    private const int MatthewSevenVerses = 29;

    private static PassageListVerse V(string vref, int? groupCount = null) => new(vref, $"text of {vref}", groupCount);

    private static CanonBook Toc(string code, string name, params int[] chapters) =>
        new(chapters: chapters, code: code, name: name);

    private static readonly Versification RealCanon = Versification.From(new[]
    {
        Toc("GEN", "Genesis", 31, 25, 24, 26, 32, 22, 24, 22, 29, 32, 32, 20, 18, 24, 21, 16, 27, 33, 38, 18, 34, 24, 20, 67, 34, 35, 46, 22, 35, 43, 55, 32, 20, 31, 29, 43, 36, 30, 23, 23, 57, 38, 34, 34, 28, 34, 31, 22, 33, 26),
        Toc("2KI", "2 Kings", 18, 25, 27, 44, 27, 33, 20, 29, 37, 36, 21, 21, 25, 29, 38, 20, 41, 37, 37, 21, 26, 20, 37, 20, 30),
        Toc("2CH", "2 Chronicles", 17, 18, 17, 22, 14, 42, 22, 18, 31, 19, 23, 16, 22, 15, 19, 14, 19, 34, 11, 37, 20, 12, 21, 27, 28, 23, 9, 27, 36, 27, 21, 33, 25, 33, 27, 23),
        Toc("PSA", "Psalms", 6, 12, 8, 8, 12, 10, 17, 9, 20, 18, 7, 8, 6, 7, 5, 11, 15, 50, 14, 9, 13, 31, 6, 10, 22, 12, 14, 9, 11, 12, 24, 11, 22, 22, 28, 12, 40, 22, 13, 17, 13, 11, 5, 26, 17, 11, 9, 14, 20, 23, 19, 9, 6, 7, 23, 13, 11, 11, 17, 12, 8, 12, 11, 10, 13, 20, 7, 35, 36, 5, 24, 20, 28, 23, 10, 12, 20, 72, 13, 19, 16, 8, 18, 12, 13, 17, 7, 18, 52, 17, 16, 15, 5, 23, 11, 13, 12, 9, 9, 5, 8, 28, 22, 35, 45, 48, 43, 13, 31, 7, 10, 10, 9, 8, 18, 19, 2, 29, 176, 7, 8, 9, 4, 8, 5, 6, 5, 6, 8, 8, 3, 18, 3, 3, 21, 26, 9, 8, 24, 13, 10, 7, 12, 15, 21, 10, 20, 14, 9, 6),
        Toc("MAT", "Matthew", 25, 23, 17, 25, 48, 34, 29, 34, 38, 42, 30, 50, 58, 36, 39, 28, 27, 35, 30, 34, 46, 46, 39, 51, 46, 75, 66, 20),
        Toc("MRK", "Mark", 45, 28, 35, 41, 43, 56, 37, 38, 50, 52, 33, 44, 37, 72, 47, 20),
    });

    [Fact]
    public void Versification_AnswersRealChapterLengths_AndNullForTheUnknown()
    {
        Assert.Equal(48, RealCanon.VersesIn("MAT", 5));
        Assert.Equal(72, RealCanon.VersesIn("MRK", 14));
        Assert.Null(RealCanon.VersesIn("MAT", 0));
        Assert.Null(RealCanon.VersesIn("MAT", MatthewChapters + 1));
        Assert.Null(RealCanon.VersesIn("ZZZ", 1));
    }

    [Fact]
    public void SpanRef_SameChapter_UnchangedShape()
    {
        Assert.Equal("MAT.5.1-48", PassageGrouping.SpanRef("MAT.5.1", "MAT.5.48"));
    }

    [Fact]
    public void SpanRef_CrossChapter_NamesBothChapters()
    {
        Assert.Equal("MAT.5.1-7.29", PassageGrouping.SpanRef("MAT.5.1", "MAT.7.29"));
    }

    [Fact]
    public void SermonOnTheMount_MatWitness_CoalescesIntoOneAccount_NotThree()
    {
        var verses = new List<PassageListVerse>();
        verses.AddRange(Enumerable.Range(1, 48).Select(n => V($"MAT.5.{n}", 48)));
        verses.AddRange(Enumerable.Range(1, 34).Select(n => V($"MAT.6.{n}", 34)));
        verses.AddRange(Enumerable.Range(1, 29).Select(n => V($"MAT.7.{n}", 29)));
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal("MAT.5.1-7.29", blocks[0].Span);
        Assert.Equal(48 + 34 + 29, blocks[0].Verses.Count);
        Assert.Equal(0, blocks[0].TruncatedBy);
    }

    [Fact]
    public void RobPeterDenies_GapInsideOneChapter_RendersTheHonestCompoundList_NeverAFabricatedEnvelope()
    {
        var verses = new List<PassageListVerse> { V("MRK.14.54", 8) };
        verses.AddRange(Enumerable.Range(66, 7).Select(n => V($"MRK.14.{n}", 8)));
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal("MRK.14.54, 66-72", blocks[0].Span);
        Assert.Equal("MRK.14.72", blocks[0].LastVref);
        Assert.Equal(0, blocks[0].TruncatedBy);
        Assert.Equal("MRK.14.54", blocks[0].FirstRangeEndVref);
    }

    [Fact]
    public void Theo188_GapAcrossChapters_NeverWeldsIntoASevenChapterEnvelope()
    {
        var verses = new List<PassageListVerse> { V("2KI.1.17", 1) };
        verses.AddRange(Enumerable.Range(16, 9).Select(n => V($"2KI.8.{n}", 9)));
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal("2KI.1.17, 8.16-24", blocks[0].Span);
        Assert.Equal("2KI.1.17", blocks[0].FirstRangeEndVref);
    }

    [Fact]
    public void DavidsHymnOfPraise_FourPsalmFragments_StayFourHonestRanges_OfOneAccount()
    {
        var verses = new List<PassageListVerse>();
        verses.AddRange(Enumerable.Range(1, 13).Select(n => V($"PSA.96.{n}", 13)));
        verses.AddRange(Enumerable.Range(1, 15).Select(n => V($"PSA.105.{n}", 15)));
        verses.Add(V("PSA.106.1", 3));
        verses.Add(V("PSA.106.47", 3));
        verses.Add(V("PSA.106.48", 3));
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal("PSA.96.1-13, 105.1-15, 106.1, 47-48", blocks[0].Span);
        Assert.Equal(0, blocks[0].TruncatedBy);
    }

    [Fact]
    public void HezekiahEmbassy_SameChapterGap_NeverNarrowsTheSpan_NorClaimsTheGapVerses()
    {
        var unit = new PassageSourceUnit(
            new List<PassageListVerse> { V("2CH.32.25", 3), V("2CH.32.26", 3), V("2CH.32.31", 3) },
            CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal("2CH.32.25-26, 31", blocks[0].Span);

        var abUr = new PassageSourceUnit(
            new List<PassageListVerse> { V("GEN.11.28", 2), V("GEN.11.31", 2) },
            CoalesceAcrossChapters: true, Canon: RealCanon);
        var abUrBlocks = PassageBlockBuilder.Build(new[] { abUr });
        Assert.Equal("GEN.11.28, 31", abUrBlocks[0].Span);
        Assert.Equal("GEN.11.28", abUrBlocks[0].FirstRangeEndVref);
    }

    [Fact]
    public void ChapterBoundary_JoinsOnlyWhenThePriorChapterIsGenuinelyComplete()
    {
        var unit = new PassageSourceUnit(
            new List<PassageListVerse> { V("2KI.1.17", 1), V("2KI.2.1", 1) },
            CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal("2KI.1.17, 2.1", blocks[0].Span);
    }

    [Fact]
    public void MissingCanon_DegradesConservatively_HonestCompoundList_NeverAGuessedJoin()
    {
        var verses = new List<PassageListVerse>();
        verses.AddRange(Enumerable.Range(1, 48).Select(n => V($"MAT.5.{n}", 48)));
        verses.AddRange(Enumerable.Range(1, 34).Select(n => V($"MAT.6.{n}", 34)));
        verses.AddRange(Enumerable.Range(1, 29).Select(n => V($"MAT.7.{n}", 29)));
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: null);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal("MAT.5.1-48, 6.1-34, 7.1-29", blocks[0].Span);
    }

    [Fact]
    public void SermonOnTheMount_LukWitness_StaysItsOwnSeparateAccount()
    {
        var mat = new PassageSourceUnit(Enumerable.Range(1, 48).Select(n => V($"MAT.5.{n}")).ToList(), CoalesceAcrossChapters: true, Canon: RealCanon);
        var luk = new PassageSourceUnit(Enumerable.Range(17, 33).Select(n => V($"LUK.6.{n}")).ToList(), CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { mat, luk });

        Assert.Equal(2, blocks.Count);
        Assert.Equal("MAT.5.1-48", blocks[0].Span);
        Assert.Equal("LUK.6.17-49", blocks[1].Span);
    }

    [Fact]
    public void Psalm14And53_TwoSeparateWitnessRows_NeverCoalesce_TheAcrossUnitCounterexample()
    {
        var psa14 = new PassageSourceUnit(Enumerable.Range(1, 7).Select(n => V($"PSA.14.{n}")).ToList(), CoalesceAcrossChapters: true, Canon: RealCanon);
        var psa53 = new PassageSourceUnit(Enumerable.Range(1, 6).Select(n => V($"PSA.53.{n}")).ToList(), CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { psa14, psa53 });

        Assert.Equal(2, blocks.Count);
        Assert.Equal("PSA.14.1-7", blocks[0].Span);
        Assert.Equal("PSA.53.1-6", blocks[1].Span);
    }

    [Fact]
    public void CoalescedBlock_SpanHonorsTheTrueLastVerse_EvenWhenTheFinalChapterIsWireCapped()
    {
        var verses = new List<PassageListVerse>();
        verses.AddRange(Enumerable.Range(1, 48).Select(n => V($"MAT.5.{n}", 48)));
        verses.AddRange(Enumerable.Range(1, 34).Select(n => V($"MAT.6.{n}", 34)));
        verses.AddRange(Enumerable.Range(1, WireCappedVerses).Select(n => V($"MAT.7.{n}", MatthewSevenVerses)));
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal("MAT.5.1-7.29", blocks[0].Span);
        Assert.Equal(MatthewSevenVerses - WireCappedVerses, blocks[0].TruncatedBy);
    }

    [Fact]
    public void WireCapRemainder_ExtendsTheLastRunAfterTheGap_NeverTheFirst()
    {
        var verses = new List<PassageListVerse> { V("MRK.14.1", 20) };
        verses.AddRange(Enumerable.Range(30, 13).Select(n => V($"MRK.14.{n}", 20)));
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal("MRK.14.1, 30-48", blocks[0].Span);
        Assert.Equal(6, blocks[0].TruncatedBy);
    }

    [Fact]
    public void WireCapRemainder_IsClampedToTheChaptersRealEnd_NeverInventingVerses()
    {
        var verses = new List<PassageListVerse> { V("MRK.14.54", 19) };
        verses.AddRange(Enumerable.Range(60, 12).Select(n => V($"MRK.14.{n}", 19)));
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal("MRK.14.54, 60-72", blocks[0].Span);
        Assert.Equal(6, blocks[0].TruncatedBy);
    }

    [Fact]
    public void ExploreSref_IsTheFirstContiguousRun_AlwaysAParserSafeSingleChapterShape()
    {
        var parserSafe = new System.Text.RegularExpressions.Regex(@"^[A-Z0-9]{3}\.\d+\.\d+(-\d+)?$");

        var peter = new List<PassageListVerse> { V("MRK.14.54", 8) };
        peter.AddRange(Enumerable.Range(66, 7).Select(n => V($"MRK.14.{n}", 8)));
        var peterBlock = PassageBlockBuilder.BuildCoalescedBlock(new PassageSourceUnit(peter, CoalesceAcrossChapters: true, Canon: RealCanon));
        Assert.Equal("MRK.14.54", peterBlock.ExploreSref);
        Assert.Equal(1, peterBlock.ExploreVerseCount);
        Assert.Matches(parserSafe, peterBlock.ExploreSref!);

        var sermon = new List<PassageListVerse>();
        sermon.AddRange(Enumerable.Range(1, WireCappedVerses).Select(n => V($"MAT.5.{n}", 48)));
        sermon.AddRange(Enumerable.Range(1, WireCappedVerses).Select(n => V($"MAT.6.{n}", 34)));
        sermon.AddRange(Enumerable.Range(1, WireCappedVerses).Select(n => V($"MAT.7.{n}", MatthewSevenVerses)));
        var sermonBlock = PassageBlockBuilder.BuildCoalescedBlock(new PassageSourceUnit(sermon, CoalesceAcrossChapters: true, Canon: RealCanon));
        Assert.Equal("MAT.5.1-7.29", sermonBlock.Span);
        Assert.Equal("MAT.5.1-48", sermonBlock.ExploreSref);
        Assert.Equal(WireCappedVerses, sermonBlock.ExploreVerseCount);
        Assert.Matches(parserSafe, sermonBlock.ExploreSref!);

        var luk = Enumerable.Range(17, 33).Select(n => V($"LUK.6.{n}", 33)).ToList();
        var lukBlock = PassageBlockBuilder.BuildCoalescedBlock(new PassageSourceUnit(luk, CoalesceAcrossChapters: true, Canon: RealCanon));
        Assert.Equal(lukBlock.Span, lukBlock.ExploreSref);
        Assert.Equal(lukBlock.Verses.Count, lukBlock.ExploreVerseCount);
        Assert.Matches(parserSafe, lukBlock.ExploreSref!);

        var theo = new List<PassageListVerse> { V("2KI.1.17", 1) };
        theo.AddRange(Enumerable.Range(16, 9).Select(n => V($"2KI.8.{n}", 9)));
        var theoBlock = PassageBlockBuilder.BuildCoalescedBlock(new PassageSourceUnit(theo, CoalesceAcrossChapters: true, Canon: RealCanon));
        Assert.Equal("2KI.1.17", theoBlock.ExploreSref);
        Assert.Matches(parserSafe, theoBlock.ExploreSref!);
    }

    [Fact]
    public void CoalescedBlock_SumsTruncationAcrossEveryDistinctChapterGroup_NotJustTheLast()
    {
        var verses = new List<PassageListVerse>();
        verses.AddRange(Enumerable.Range(1, 48).Select(n => V($"MAT.5.{n}", 48)));
        verses.AddRange(Enumerable.Range(1, 10).Select(n => V($"MAT.6.{n}", 34)));
        verses.AddRange(Enumerable.Range(1, 29).Select(n => V($"MAT.7.{n}", 29)));
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal(24, blocks[0].TruncatedBy);
    }

    [Fact]
    public void NonCoalescedUnit_UnchangedBehavior_StillSplitsPerChapter()
    {
        var verses = Enumerable.Range(1, 48).Select(n => V($"MAT.5.{n}"))
            .Concat(Enumerable.Range(1, 34).Select(n => V($"MAT.6.{n}")))
            .ToList();
        var unit = new PassageSourceUnit(verses);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Equal(2, blocks.Count);
        Assert.Equal("MAT.5.1-48", blocks[0].Span);
        Assert.Equal("MAT.6.1-34", blocks[1].Span);
    }
}
