using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class CrossingTests
{
    private const int Start = -1450;
    private const int End = -1399;
    private const int Before = -1500;
    private const int Inside = -1420;
    private const int After = -1350;

    private static readonly TimeRange Bounds = ServedGraph.Range(new Year(label: "1451 BC", value: Start), new Year(label: "1400 BC", value: End), "1451 BC – 1400 BC");

    [Fact]
    public void A_window_inside_the_bounds_crosses_nothing()
    {
        // Act
        var crossed = Crossing.Of(Bounds, Inside, Inside);

        // Assert
        Assert.Null(crossed);
    }

    [Fact]
    public void A_window_reaching_past_the_end_crosses_to_the_next()
    {
        // Act
        var crossed = Crossing.Of(Bounds, Inside, After);

        // Assert
        Assert.Equal(ArrowDirection.Next, crossed);
    }

    [Fact]
    public void A_window_reaching_before_the_start_crosses_to_the_previous()
    {
        // Act
        var crossed = Crossing.Of(Bounds, Before, Inside);

        // Assert
        Assert.Equal(ArrowDirection.Previous, crossed);
    }

    [Fact]
    public void Touching_a_bound_is_not_crossing_it()
    {
        // Act
        var crossed = (Crossing.Of(Bounds, Start, Inside), Crossing.Of(Bounds, Inside, End), Crossing.Of(Bounds, Start, End));

        // Assert
        Assert.Equal(((ArrowDirection?)null, (ArrowDirection?)null, (ArrowDirection?)null), crossed);
    }
}
