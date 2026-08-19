namespace BibleAtlas.Client.Tests;

public class YearTextTests {
    [Theory]
    [InlineData(-1447, "1447 BC")] [InlineData(30, "AD 30")]
    public void Formats(int y, string s) => Assert.Equal(s, YearText.Format(y));
    [Fact] public void RangeUsesEnDash() => Assert.Equal("1447 BC – 1400 BC", YearText.FormatRange(-1447, -1400));
    [Fact] public void SingleYearRange() => Assert.Equal("1447 BC", YearText.FormatRange(-1447, -1447));
    [Theory]
    [InlineData("1447 BC", -1447, -1447)] [InlineData("1447 BC - 1400 BC", -1447, -1400)]
    [InlineData("AD 30 – AD 70", 30, 70)]
    public void Parses(string t, int f, int to) {
        Assert.True(YearText.TryParse(t, out var a, out var b));
        Assert.Equal((f, to), (a, b));
    }
    [Theory] [InlineData("0 BC")] [InlineData("banana")] [InlineData("1400 BC - 1447 BC")]
    public void Rejects(string t) => Assert.False(YearText.TryParse(t, out _, out _));

    // Batch E: FormatClaim -- "c." prefix appears exactly when a note is
    // curated, matching the brief's own worked example verbatim
    // ("Established c. 1003 BC · Destroyed 586 BC": the first is
    // curated "traditional", the second is not).
    [Fact] public void ClaimWithNotePrefixesCirca() => Assert.Equal("c. 1003 BC", YearText.FormatClaim(-1003, -1003, "traditional"));
    [Fact] public void ClaimWithoutNoteHasNoCirca() => Assert.Equal("586 BC", YearText.FormatClaim(-586, -586, null));
    [Fact] public void ClaimRangeStillCollapsesLikeFormatRange() => Assert.Equal("c. 1447 BC – 1400 BC", YearText.FormatClaim(-1447, -1400, "traditional"));
}
