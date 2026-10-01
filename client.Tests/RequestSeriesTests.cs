namespace BibleAtlas.Client.Tests;

public sealed class RequestSeriesTests
{
    [Fact]
    public async Task The_newest_requests_answer_is_kept()
    {
        // Arrange
        var series = new RequestSeries();

        // Act
        var answer = await series.Next().Fetch(() => Task.FromResult("the scene"));

        // Assert
        Assert.Equal("the scene", answer);
    }

    [Fact]
    public async Task An_answer_arriving_after_a_newer_request_went_out_is_dropped()
    {
        // Arrange
        var series = new RequestSeries();
        var slow = new TaskCompletionSource<string>();
        var first = series.Next();
        var pending = first.Fetch(() => slow.Task);

        // Act
        var second = series.Next();
        slow.SetResult("the old scene");
        var answer = await pending;

        // Assert
        Assert.Equal((null, true, false), (answer, first.Superseded, second.Superseded));
    }

    [Fact]
    public async Task A_newer_request_cancels_the_delay_the_earlier_one_was_waiting_out()
    {
        // Arrange
        var series = new RequestSeries();
        var first = series.Next();

        // Act
        series.Next();
        var delay = await Record.ExceptionAsync(() => Task.Delay(TimeSpan.FromSeconds(30), first.Token));

        // Assert
        Assert.IsType<TaskCanceledException>(delay);
    }

    [Fact]
    public void Before_any_request_the_current_one_is_never_superseded_and_stopping_supersedes_the_latest()
    {
        // Arrange
        var series = new RequestSeries();
        var beforeAny = series.Current;
        var issued = series.Next();

        // Act
        series.Stop();

        // Assert
        Assert.Equal((false, true, true), (beforeAny.Superseded, issued.Superseded, series.Current.Superseded));
    }

    [Fact]
    public async Task Two_answers_fetched_together_are_kept_together_or_dropped_together()
    {
        // Arrange
        var series = new RequestSeries();
        var slow = new TaskCompletionSource<string>();
        var pending = series.Next().Fetch(() => Task.FromResult("a chapter"), () => slow.Task);

        // Act
        var together = await series.Next().Fetch(() => Task.FromResult("a chapter"), () => Task.FromResult("its text"));
        slow.SetResult("its text");
        var dropped = await pending;

        // Assert
        Assert.Equal((("a chapter", "its text"), null), (together, dropped));
    }
}
