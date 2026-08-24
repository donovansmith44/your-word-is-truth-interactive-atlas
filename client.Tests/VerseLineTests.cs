using BibleAtlas.Client.Components;

namespace BibleAtlas.Client.Tests;

// M-D4 fix round 2, P1b (owner, verbatim: "well there might be more than
// 26. if there are then we still need to be in mod26 land but have a
// system for new superscripts beyond a-z"): direct, isolated proof of
// VerseLine.BijectiveBase26's own boundary values -- fix round 1's bare
// `ordinal % 26` wraparound collided the 27th xref-bearing verse's own
// letter back onto the 1st's ("a" again); bijective base-26 numeration
// (the same scheme spreadsheet column names use) never collides, since
// there is no digit for zero -- "aa" (27) is genuinely distinct from "a"
// (1), not a wraparound back to an already-used, shorter letter. These
// six values are the coordinator's own named boundary set, verbatim:
// 26->"z" (the last single-letter value), 27->"aa" (the first
// two-letter value, the exact collision point fix round 1 got wrong),
// 52->"az" (last value starting with "a"), 53->"ba" (first starting
// with "b"), 702->"zz" (last two-letter value), 703->"aaa" (first
// three-letter value). Pure function, no chapter/verse fixture needed --
// ComputeXrefLetters's own per-chapter ordinal-tracking is exercised
// indirectly through the Playwright suite instead (reader-xref-
// superscripts.spec.ts's own real-data sweep and wraparound test).
public class VerseLineTests {
    [Theory]
    [InlineData(1, "a")] [InlineData(2, "b")] [InlineData(25, "y")]
    [InlineData(26, "z")]
    [InlineData(27, "aa")] [InlineData(28, "ab")]
    [InlineData(52, "az")]
    [InlineData(53, "ba")]
    [InlineData(702, "zz")]
    [InlineData(703, "aaa")]
    public void BijectiveBase26MatchesBoundaryValues(int n, string expected) =>
        Assert.Equal(expected, VerseLine.BijectiveBase26(n));

    // n <= 0 is never reached by ComputeXrefLetters (ordinal is always
    // incremented BEFORE use, so the smallest real call is n=1) -- still
    // asserted directly since it's the loop's own natural base case
    // ("while n > 0"), not merely assumed.
    [Fact] public void BijectiveBase26OfZeroIsEmpty() => Assert.Equal("", VerseLine.BijectiveBase26(0));
}
