using BibleAtlas.Client;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

// ACCT-COALESCE-1 (owner bug report, verbatim: "in parallel accounts
// (sermon on the mount in particular), accounts from the same book +
// chapter are listed. makes no sense."): direct, isolated proof of the
// coalescing rule -- PassageSourceUnit.CoalesceAcrossChapters/.Canon +
// PassageBlockBuilder.BuildCoalescedBlock + PassageGrouping.SpanRef's own
// cross-chapter branch.
//
// FIX ROUND 2 (review Critical N-1): the round-1 suite proved only the
// CONTIGUOUS half of the rule -- every fixture was gapless within its
// unit, which is exactly why 340/340 green proved nothing about the false
// spans the real corpus was rendering (15 of 35 multi-range curated
// witness rows are genuinely NON-contiguous -- corrected from "14" in fix
// round 3, review NEW-4: the recount adds rob_paschal_meal_begins's own
// LUK row). The fixtures below now
// include the review's own worked failure cases, mirroring the REAL
// curated rows byte-for-byte (data/curated/event-witnesses.toml):
// rob_peter_denies (gap INSIDE one chapter -- the worst case, where the
// old math both claimed verses belonging to a DIFFERENT event and dropped
// delivered ones), theo-188 (gap ACROSS chapters), 1ch_davids_hymn_of_praise
// (eleven psalms that must never weld into one), and the Sermon on the
// Mount (genuinely contiguous across chapters -- must STILL coalesce to
// the one honest MAT.5.1-7.29). RealCanon below carries the REAL
// per-chapter verse counts (data/compiled/canon.json, the same numbers
// GET /api/books serves) for every book these fixtures touch.
// The real end-to-end wire-through-DOM proof (live compiled data) is
// tests/ux/popover-sections.spec.ts's own ACCT-COALESCE-1 tests.
public class AcctCoalesceTests
{
    private static PassageListVerse V(string vref, int? groupCount = null) => new(vref, $"text of {vref}", groupCount);

    private static BookTocEntry Toc(string code, string name, params int[] chapters) =>
        new(code, name, chapters.ToList());

    // The REAL canon chapter lengths (data/compiled/canon.json -- the same
    // per-chapter verse counts AtlasClient.Books() serves the live app)
    // for every book a fixture below touches. Real numbers, full books --
    // a placeholder count here would make the contiguity verdicts below
    // meaningless.
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
        Assert.Null(RealCanon.VersesIn("MAT", 29)); // Matthew has 28 chapters
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
        // The real shape: ONE witness row, three same-chapter VerseGroups
        // (event-witnesses.toml:592-596) -- flattened here exactly as
        // WitnessUnitsResolver.ResolveAsync builds it (one PassageSourceUnit
        // per witness, CoalesceAcrossChapters: true, the real canon). The
        // chapter boundaries are GENUINELY contiguous (Matthew 5 really
        // has 48 verses, 6 really has 34 -- RealCanon), so the whole
        // witness is one range and MUST still coalesce to one span.
        var verses = new List<PassageListVerse>();
        verses.AddRange(Enumerable.Range(1, 48).Select(n => V($"MAT.5.{n}", 48)));
        verses.AddRange(Enumerable.Range(1, 34).Select(n => V($"MAT.6.{n}", 34)));
        verses.AddRange(Enumerable.Range(1, 29).Select(n => V($"MAT.7.{n}", 29)));
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks); // ONE account, not three -- the owner's own bug, fixed
        Assert.Equal("MAT.5.1-7.29", blocks[0].Span);
        Assert.Equal(48 + 34 + 29, blocks[0].Verses.Count);
        Assert.Equal(0, blocks[0].TruncatedBy);
    }

    [Fact]
    public void RobPeterDenies_GapInsideOneChapter_RendersTheHonestCompoundList_NeverAFabricatedEnvelope()
    {
        // Fix round 2 (review N-1's own sharpest worked case, the worst
        // class of bug under this project's inerrancy law): the REAL
        // rob_peter_denies MRK witness (event-witnesses.toml:1171-1173,
        // ["MRK.14.54", "MRK.14.66-72"]) is ONE account with a REAL gap --
        // verses 55-61 belong to a DIFFERENT event (rob_tried_by_caiaphas,
        // Jesus before the council, happening INSIDE the house while Peter
        // is in the courtyard below). On the wire: one (MRK,14) VerseGroup,
        // 8 delivered verses, Count=8. The round-1 first+Count-1 math
        // rendered "MRK.14.54-61" -- claiming another event's verses while
        // dropping the delivered 62-72 (Span and LastVref disagreeing on
        // the same object). The honest shape: still ONE account, compound
        // ref.
        var verses = new List<PassageListVerse> { V("MRK.14.54", 8) };
        verses.AddRange(Enumerable.Range(66, 7).Select(n => V($"MRK.14.{n}", 8)));
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks); // ONE account -- never fake separate accounts
        Assert.Equal("MRK.14.54, 66-72", blocks[0].Span);
        Assert.Equal("MRK.14.72", blocks[0].LastVref); // Span and LastVref agree by construction now
        Assert.Equal(0, blocks[0].TruncatedBy); // all 8 delivered -- nothing capped
        // The expand highlight bound is the FIRST range's own end (verse
        // 54 alone) -- never straight through the gap into 55-61.
        Assert.Equal("MRK.14.54", blocks[0].FirstRangeEndVref);
    }

    [Fact]
    public void Theo188_GapAcrossChapters_NeverWeldsIntoASevenChapterEnvelope()
    {
        // The REAL theo-188 2KI witness (event-witnesses.toml:2501-2503,
        // ["2KI.1.17", "2KI.8.16-24"]): a synchronism marker plus the
        // reign account seven chapters later. Round 1 rendered the
        // fictional continuous envelope "2KI.1.17-8.24". Wire shape: two
        // VerseGroups, (2KI,1) Count=1 and (2KI,8) Count=9.
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
        // The REAL 1ch_davids_hymn_of_praise PSA witness
        // (event-witnesses.toml:2300-2302, ["PSA.96.1-13", "PSA.105.1-15",
        // "PSA.106.1", "PSA.106.47-48"]): the exact psalm fragments
        // 1 Chronicles 16 quotes, an honest curated subset. Round 1
        // rendered "PSA.96.1-106.3" (corrected in fix round 3, review
        // NEW-4: the round-1 arithmetic was lastGroupFirstVerse + Count -
        // 1 = 106.3, not the previously-quoted "106.48"; either way a
        // fabricated envelope welding eleven psalms into one "account").
        // Wire shape: three VerseGroups -- (PSA,96) 13
        // verses, (PSA,105) 15 verses, (PSA,106) the 3 verses 1/47/48
        // (Count=3 -- the WITNESS's own total in that chapter, never the
        // chapter's).
        var verses = new List<PassageListVerse>();
        verses.AddRange(Enumerable.Range(1, 13).Select(n => V($"PSA.96.{n}", 13)));
        verses.AddRange(Enumerable.Range(1, 15).Select(n => V($"PSA.105.{n}", 15)));
        verses.Add(V("PSA.106.1", 3));
        verses.Add(V("PSA.106.47", 3));
        verses.Add(V("PSA.106.48", 3));
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks); // still ONE account (one witness row)...
        Assert.Equal("PSA.96.1-13, 105.1-15, 106.1, 47-48", blocks[0].Span); // ...with its four real ranges
        Assert.Equal(0, blocks[0].TruncatedBy);
    }

    [Fact]
    public void HezekiahEmbassy_SameChapterGap_NeverNarrowsTheSpan_NorClaimsTheGapVerses()
    {
        // The REAL 2ki_hezekiah_babylonian_embassy 2CH witness
        // (event-witnesses.toml:2623-2626, ["2CH.32.25-26", "2CH.32.31"]):
        // round 1 rendered "2CH.32.25-27" -- naming verse 27 (which the
        // account does NOT include) while omitting verse 31 (which it
        // does). Same shape as ab_ur's own GEN.11.28+31, asserted below.
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
        Assert.Equal("GEN.11.28, 31", abUrBlocks[0].Span); // never "GEN.11.28-29"
        Assert.Equal("GEN.11.28", abUrBlocks[0].FirstRangeEndVref); // expand highlights 28 alone, never 28-31 through the gap
    }

    [Fact]
    public void ChapterBoundary_JoinsOnlyWhenThePriorChapterIsGenuinelyComplete()
    {
        // The boundary rule's own negative: 2KI.1.17 then 2KI.2.1 -- next
        // chapter, verse 1, but 2 Kings 1 really has 18 verses (RealCanon),
        // so verse 18 is missing and the crossing is NOT contiguous.
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
        // Canon null (the Books() fetch failed): whether MAT.5.48 -> 6.1
        // is contiguous is now UNKNOWABLE client-side, and under the
        // inerrancy law the only honest degrade is the compound list --
        // still ONE account, never a join the data at hand cannot prove.
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
        // The event's OTHER witness (LUK.6.17-49, event-witnesses.toml:1498-1501)
        // is a SEPARATE unit (a separate witness row) -- proves the fix
        // coalesces WITHIN a witness, never ACROSS witnesses.
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
        // psa_014's own real fixture (event-witnesses.toml:2781-2791): TWO
        // separate [[witness]] rows, both book PSA, genuinely distinct
        // psalms ("a second, distinct psalm recounting the same substance,
        // not a copy of the same container") -- MUST stay two accounts,
        // never merge into one PSA.14.1-53.6 span. Guards the ACROSS-UNIT
        // boundary (coalescing is strictly within one witness/unit); the
        // WITHIN-unit gap boundary -- the one fix round 1 left unguarded
        // -- is what the rob_peter_denies/theo-188/Davids-hymn fixtures
        // above now pin.
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
        // Fix round 1 (real, live-caught bug, PERF-3's own "identity never
        // narrows" law): the REAL Sermon-on-the-Mount fixture -- Matthew 7
        // has 29 real verses, but the server's own 20-verse-per-chapter cap
        // only delivers the first 20. The block's own Span must still read
        // "MAT.5.1-7.29" (the true end), never "MAT.5.1-7.20" (the
        // delivered tail silently mistaken for the true one).
        var verses = new List<PassageListVerse>();
        verses.AddRange(Enumerable.Range(1, 48).Select(n => V($"MAT.5.{n}", 48)));
        verses.AddRange(Enumerable.Range(1, 34).Select(n => V($"MAT.6.{n}", 34)));
        verses.AddRange(Enumerable.Range(1, 20).Select(n => V($"MAT.7.{n}", 29))); // ch7: capped, 20 of 29 delivered
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal("MAT.5.1-7.29", blocks[0].Span);
        Assert.Equal(9, blocks[0].TruncatedBy); // 29 - 20 = 9 missing from chapter 7
    }

    [Fact]
    public void WireCapRemainder_ExtendsTheLastRunAfterTheGap_NeverTheFirst()
    {
        // Cap + gap together: a group delivering [1, 30..42] (14 verses)
        // whose true Count is 20 -- the server's take(20)-ascending cap
        // always keeps the LOWEST-numbered verses, so the 6 undelivered
        // verses continue after the LAST delivered one (43-48), never
        // after the first run's own verse 1. CORRECTED in fix round 3
        // (review NEW-2): the round-2 version of this fixture extended
        // past Mark 14's own real end (66-83 in a 72-verse chapter) and
        // PINNED that invented-verse output as correct -- worse than no
        // fixture. This one stays within the chapter (48 <= 72); the
        // clamp itself is asserted by the next test.
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
        // Fix round 3 (review NEW-2, Low -- the ONE remaining step that
        // could invent verses): a Count that arithmetically overruns the
        // chapter (only possible when the delivered portion is gapped AND
        // capped -- zero live cases in the corpus today, per the round-2
        // review's own 1212-derivation scan) must stop at the chapter's
        // own real end. Delivered [54, 60..71] (13 verses), Count=19 --
        // unclamped math says the last run ends at 71 + 6 = 77, but Mark
        // 14 has 72 verses (RealCanon): the span must end at 72, never
        // name 73-77. The honest cap disclosure (TruncatedBy) is
        // unaffected -- the +N-more signal still fires.
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
        // Fix round 3 (review NEW-1, Moderate): a coalesced account's own
        // DISPLAY span can be compound/cross-chapter, but its exploration
        // sref must be something the server's ScriptureRef::parse accepts
        // (BOOK.CH.V / BOOK.CH.V1-V2) -- pushing the display span verbatim
        // 400'd /api/xrefs + /api/catechism and silently deleted the
        // Cross References / Small Catechism sections for 167 of 1212
        // real accounts. The exploration sref is the account's own FIRST
        // contiguous same-chapter run, with ExploreVerseCount bounding
        // the pushed node's text to that run's delivered verses.
        var parserSafe = new System.Text.RegularExpressions.Regex(@"^[A-Z0-9]{3}\.\d+\.\d+(-\d+)?$");

        // rob_peter_denies: compound span, one-verse first run.
        var peter = new List<PassageListVerse> { V("MRK.14.54", 8) };
        peter.AddRange(Enumerable.Range(66, 7).Select(n => V($"MRK.14.{n}", 8)));
        var peterBlock = PassageBlockBuilder.BuildCoalescedBlock(new PassageSourceUnit(peter, CoalesceAcrossChapters: true, Canon: RealCanon));
        Assert.Equal("MRK.14.54", peterBlock.ExploreSref);
        Assert.Equal(1, peterBlock.ExploreVerseCount); // -> VerseNode, the run's own cardinality rule
        Assert.Matches(parserSafe, peterBlock.ExploreSref!);

        // The Sermon: cross-chapter CONTIGUOUS span (also unparseable!) --
        // explores by its first chapter's own run, true-cap-corrected.
        var sermon = new List<PassageListVerse>();
        sermon.AddRange(Enumerable.Range(1, 20).Select(n => V($"MAT.5.{n}", 48))); // capped: 20 of 48 delivered
        sermon.AddRange(Enumerable.Range(1, 20).Select(n => V($"MAT.6.{n}", 34)));
        sermon.AddRange(Enumerable.Range(1, 20).Select(n => V($"MAT.7.{n}", 29)));
        var sermonBlock = PassageBlockBuilder.BuildCoalescedBlock(new PassageSourceUnit(sermon, CoalesceAcrossChapters: true, Canon: RealCanon));
        Assert.Equal("MAT.5.1-7.29", sermonBlock.Span);
        Assert.Equal("MAT.5.1-48", sermonBlock.ExploreSref);
        Assert.Equal(20, sermonBlock.ExploreVerseCount); // text bounded to chapter 5's own DELIVERED verses
        Assert.Matches(parserSafe, sermonBlock.ExploreSref!);

        // A single-chapter contiguous account: ExploreSref == Span and the
        // count covers every verse -- click behavior byte-identical to
        // before this round.
        var luk = Enumerable.Range(17, 33).Select(n => V($"LUK.6.{n}", 33)).ToList();
        var lukBlock = PassageBlockBuilder.BuildCoalescedBlock(new PassageSourceUnit(luk, CoalesceAcrossChapters: true, Canon: RealCanon));
        Assert.Equal(lukBlock.Span, lukBlock.ExploreSref);
        Assert.Equal(lukBlock.Verses.Count, lukBlock.ExploreVerseCount);
        Assert.Matches(parserSafe, lukBlock.ExploreSref!);

        // theo-188: compound with a one-verse first run in ANOTHER chapter
        // than the bulk -- still the first run, still parseable.
        var theo = new List<PassageListVerse> { V("2KI.1.17", 1) };
        theo.AddRange(Enumerable.Range(16, 9).Select(n => V($"2KI.8.{n}", 9)));
        var theoBlock = PassageBlockBuilder.BuildCoalescedBlock(new PassageSourceUnit(theo, CoalesceAcrossChapters: true, Canon: RealCanon));
        Assert.Equal("2KI.1.17", theoBlock.ExploreSref);
        Assert.Matches(parserSafe, theoBlock.ExploreSref!);
    }

    [Fact]
    public void CoalescedBlock_SumsTruncationAcrossEveryDistinctChapterGroup_NotJustTheLast()
    {
        // A capped MIDDLE chapter (MAT.6: 34 real, only 10 delivered) must
        // still be honestly disclosed even though it's not the block's own
        // LAST chapter -- the per-single-chapter check the non-coalesced
        // path uses (which only ever examines the block's own LAST vref)
        // would silently miss this; BuildCoalescedBlock's own flat,
        // every-group sum (mirroring ArrowNav.ComputeTruncatedBy) must not.
        var verses = new List<PassageListVerse>();
        verses.AddRange(Enumerable.Range(1, 48).Select(n => V($"MAT.5.{n}", 48))); // ch5: fully delivered
        verses.AddRange(Enumerable.Range(1, 10).Select(n => V($"MAT.6.{n}", 34))); // ch6: capped, 24 missing
        verses.AddRange(Enumerable.Range(1, 29).Select(n => V($"MAT.7.{n}", 29))); // ch7: fully delivered
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true, Canon: RealCanon);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal(24, blocks[0].TruncatedBy);
    }

    [Fact]
    public void NonCoalescedUnit_UnchangedBehavior_StillSplitsPerChapter()
    {
        // Regression guard: every OTHER PassageSourceUnit consumer
        // (cross-references, THE SCRIPTURES, place est/dest) defaults
        // CoalesceAcrossChapters to false and must keep the PRE-existing
        // per-chapter-run split unchanged.
        var verses = Enumerable.Range(1, 48).Select(n => V($"MAT.5.{n}"))
            .Concat(Enumerable.Range(1, 34).Select(n => V($"MAT.6.{n}")))
            .ToList();
        var unit = new PassageSourceUnit(verses); // CoalesceAcrossChapters defaults false

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Equal(2, blocks.Count);
        Assert.Equal("MAT.5.1-48", blocks[0].Span);
        Assert.Equal("MAT.6.1-34", blocks[1].Span);
    }
}
