using System.Text.Json;
using BibleAtlas.Client.Tests.State;
using ReadBack = (string Label, (int From, int To)? Years);

namespace BibleAtlas.Client.Tests;

public sealed class YearInputTests
{
    [Fact]
    public void Every_served_year_label_reads_back_to_its_year()
    {
        // Arrange
        var served = ServedLabels("years", SingleYearOf);
        // Act
        var read = ReadEach(served);
        // Assert
        Assert.Equal(served, read);
    }

    [Fact]
    public void Every_served_range_label_reads_back_to_its_years()
    {
        // Arrange
        var served = ServedLabels("ranges", SpanOf);
        // Act
        var read = ReadEach(served);
        // Assert
        Assert.Equal(served, read);
    }

    [Fact]
    public void Every_served_date_claim_label_reads_back_to_its_years()
    {
        // Arrange
        var served = ServedLabels("claims", SpanOf);
        // Act
        var read = ReadEach(served);
        // Assert
        Assert.Equal(served, read);
    }

    [Fact]
    public void A_range_reads_the_same_in_the_long_spaced_and_unspaced_forms()
    {
        // Arrange
        ReadBack[] expected =
        [
            ("1450 BC – 1400 BC", (-1450, -1400)),
            ("1450 - 1400 BC", (-1450, -1400)),
            ("1450-1400 BC", (-1450, -1400)),
            ("1450–1400 BC", (-1450, -1400)),
            ("AD 30 – AD 70", (30, 70)),
            ("AD 30-70", (30, 70)),
        ];
        // Act
        var read = ReadEach(expected);
        // Assert
        Assert.Equal(expected, read);
    }

    [Fact]
    public void Either_end_of_a_range_may_name_the_era_for_both()
    {
        // Arrange
        ReadBack[] expected =
        [
            ("1450 BC – 1400", (-1450, -1400)),
            ("30 – AD 70", (30, 70)),
        ];
        // Act
        var read = ReadEach(expected);
        // Assert
        Assert.Equal(expected, read);
    }

    [Fact]
    public void A_range_that_starts_and_ends_in_one_year_reads_as_that_year()
    {
        // Arrange
        ReadBack[] expected = [("1447 BC – 1447 BC", (-1447, -1447))];
        // Act
        var read = ReadEach(expected);
        // Assert
        Assert.Equal(expected, read);
    }

    [Fact]
    public void Whitespace_around_the_text_is_ignored()
    {
        // Arrange
        ReadBack[] expected = [("  AD 33 ", (33, 33))];
        // Act
        var read = ReadEach(expected);
        // Assert
        Assert.Equal(expected, read);
    }

    [Fact]
    public void Year_zero_is_refused_because_the_scale_has_none()
    {
        // Arrange
        ReadBack[] expected =
        [
            ("AD 0", null),
            ("0 BC", null),
            ("5 BC – AD 0", null),
        ];
        // Act
        var read = ReadEach(expected);
        // Assert
        Assert.Equal(expected, read);
    }

    [Fact]
    public void A_range_running_backwards_in_time_is_refused()
    {
        // Arrange
        ReadBack[] expected =
        [
            ("1400 – 1450 BC", null),
            ("AD 70 – 30", null),
            ("AD 30 – 5 BC", null),
        ];
        // Act
        var read = ReadEach(expected);
        // Assert
        Assert.Equal(expected, read);
    }

    [Fact]
    public void A_year_must_name_exactly_one_era()
    {
        // Arrange
        ReadBack[] expected =
        [
            ("1447", null),
            ("1450 – 1400", null),
            ("AD 30 BC", null),
            ("AD 1 BC – 100", null),
        ];
        // Act
        var read = ReadEach(expected);
        // Assert
        Assert.Equal(expected, read);
    }

    [Fact]
    public void Text_that_is_not_a_year_is_refused()
    {
        // Arrange
        ReadBack[] expected =
        [
            ("", null),
            ("banana", null),
            ("BC 1447", null),
            ("1447 BC –", null),
            ("99999999999 BC", null),
        ];
        // Act
        var read = ReadEach(expected);
        // Assert
        Assert.Equal(expected, read);
    }

    private static ReadBack[] ServedLabels(string section, Func<JsonElement, (int From, int To)> yearsOf)
    {
        using var vectors = JsonDocument.Parse(File.ReadAllText(Path.Combine(ConformanceTests.RepoRoot(), "contracts", "atlas-query-contract", "vectors", "year-labels.json")));
        return vectors.RootElement.GetProperty(section).EnumerateArray()
            .Select(served => (served.GetProperty("label").GetString()!, ((int, int)?)yearsOf(served)))
            .ToArray();
    }

    private static (int From, int To) SingleYearOf(JsonElement served)
    {
        var year = served.GetProperty("value").GetInt32();
        return (year, year);
    }

    private static (int From, int To) SpanOf(JsonElement served) =>
        (served.GetProperty("from").GetInt32(), served.GetProperty("to").GetInt32());

    private static ReadBack[] ReadEach(IEnumerable<ReadBack> labels) =>
        labels.Select(label => (label.Label, YearInput.Read(label.Label))).ToArray();
}
