namespace BibleAtlas.Client.Tests;

public sealed class WholeValueTests
{
    [Fact]
    public void Values_differing_only_inside_a_tuple_a_record_or_a_field_are_different_whole_values()
    {
        // Arrange
        var pairs = new (object First, object Second)[]
        {
            ((new[] { "a" }, "root-a"), (new[] { "a" }, "root-b")),
            (new Fielded(1), new Fielded(2)),
            (new Recorded("a"), new Recorded("b")),
            (new[] { (1, "a") }, new[] { (1, "b") }),
        };

        // Act
        var same = pairs.Where(pair => WholeValue.Of(pair.First) == WholeValue.Of(pair.Second));

        // Assert
        Assert.Empty(same);
    }

    [Fact]
    public void A_value_with_nothing_visible_to_compare_is_refused()
    {
        // Arrange
        var blind = new[] { (object)new Blind(1), new[] { new Blind(2) } };

        // Act
        var refused = blind.Select(value => Record.Exception(() => WholeValue.Of(value))?.GetType());

        // Assert
        Assert.Equal([typeof(ArgumentException), typeof(ArgumentException)], refused);
    }

    private sealed class Fielded(int value)
    {
        public readonly int Value = value;
    }

    private sealed record Recorded(string Value);

    private sealed class Blind(int value)
    {
        private readonly int _value = value;

        public override int GetHashCode() => _value;
    }
}
