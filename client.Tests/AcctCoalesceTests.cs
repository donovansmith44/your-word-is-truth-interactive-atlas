using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

// ACCT-COALESCE-1 (owner bug report, verbatim: "in parallel accounts
// (sermon on the mount in particular), accounts from the same book +
// chapter are listed. makes no sense."): direct, isolated proof of the
// coalescing fix -- PassageSourceUnit.CoalesceAcrossChapters +
// PassageBlockBuilder.BuildCoalescedBlock + PassageGrouping.SpanRef's own
// cross-chapter branch. Fixtures below mirror the REAL curated data this
// bug was found against (data/curated/events-extra.toml /
// event-witnesses.toml): the Sermon on the Mount's own MAT witness (one
// witness row, 3 same-chapter VerseGroups -- MAT.5/6/7) and psa_014's own
// TWO separate PSA witnesses (Psalm 14 + Psalm 53, genuinely distinct
// accounts that must NOT coalesce). The real end-to-end wire-through-DOM
// proof (both fixtures, live compiled data) is
// tests/ux/popover-sections.spec.ts's own ACCT-COALESCE-1 tests.
public class AcctCoalesceTests
{
    private static PassageListVerse V(string vref, int? groupCount = null) => new(vref, $"text of {vref}", groupCount);

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
        // per witness, CoalesceAcrossChapters: true).
        var verses = new List<PassageListVerse>();
        verses.AddRange(Enumerable.Range(1, 48).Select(n => V($"MAT.5.{n}", 48)));
        verses.AddRange(Enumerable.Range(1, 34).Select(n => V($"MAT.6.{n}", 34)));
        verses.AddRange(Enumerable.Range(1, 29).Select(n => V($"MAT.7.{n}", 29)));
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks); // ONE account, not three -- the owner's own bug, fixed
        Assert.Equal("MAT.5.1-7.29", blocks[0].Span);
        Assert.Equal(48 + 34 + 29, blocks[0].Verses.Count);
        Assert.Equal(0, blocks[0].TruncatedBy);
    }

    [Fact]
    public void SermonOnTheMount_LukWitness_StaysItsOwnSeparateAccount()
    {
        // The event's OTHER witness (LUK.6.17-49, event-witnesses.toml:1498-1501)
        // is a SEPARATE unit (a separate witness row) -- proves the fix
        // coalesces WITHIN a witness, never ACROSS witnesses.
        var mat = new PassageSourceUnit(Enumerable.Range(1, 48).Select(n => V($"MAT.5.{n}")).ToList(), CoalesceAcrossChapters: true);
        var luk = new PassageSourceUnit(Enumerable.Range(17, 33).Select(n => V($"LUK.6.{n}")).ToList(), CoalesceAcrossChapters: true);

        var blocks = PassageBlockBuilder.Build(new[] { mat, luk });

        Assert.Equal(2, blocks.Count);
        Assert.Equal("MAT.5.1-48", blocks[0].Span);
        Assert.Equal("LUK.6.17-49", blocks[1].Span);
    }

    [Fact]
    public void Psalm14And53_TwoSeparateWitnessRows_NeverCoalesce_TheNonContiguousCounterexample()
    {
        // psa_014's own real fixture (event-witnesses.toml:2781-2791): TWO
        // separate [[witness]] rows, both book PSA, genuinely distinct
        // psalms ("a second, distinct psalm recounting the same substance,
        // not a copy of the same container") -- MUST stay two accounts,
        // never merge into one PSA.14.1-53.6 span. Proven WITHOUT any
        // gap-detection logic: they are two units (two witness rows), and
        // coalescing is strictly WITHIN one unit, never across units.
        var psa14 = new PassageSourceUnit(Enumerable.Range(1, 7).Select(n => V($"PSA.14.{n}")).ToList(), CoalesceAcrossChapters: true);
        var psa53 = new PassageSourceUnit(Enumerable.Range(1, 6).Select(n => V($"PSA.53.{n}")).ToList(), CoalesceAcrossChapters: true);

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
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true);

        var blocks = PassageBlockBuilder.Build(new[] { unit });

        Assert.Single(blocks);
        Assert.Equal("MAT.5.1-7.29", blocks[0].Span);
        Assert.Equal(9, blocks[0].TruncatedBy); // 29 - 20 = 9 missing from chapter 7
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
        var unit = new PassageSourceUnit(verses, CoalesceAcrossChapters: true);

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
