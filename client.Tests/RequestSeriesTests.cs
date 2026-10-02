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
        Assert.Equal(new Outcome<string>.Arrived("the scene"), answer);
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
        Assert.Equal(((Outcome<string>)new Outcome<string>.Superseded(), true, false), (answer, first.Superseded, second.Superseded));
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
        Assert.Equal(((Outcome<(string, string)>)new Outcome<(string, string)>.Arrived(("a chapter", "its text")), (Outcome<(string, string)>)new Outcome<(string, string)>.Superseded()), (together, dropped));
    }

    [Fact]
    public async Task A_current_request_that_fails_is_a_failure_and_never_an_exception()
    {
        // Arrange
        var series = new RequestSeries();

        // Act
        var outcome = await series.Next().Fetch<string>(() => throw new HttpRequestException(Offline));

        // Assert
        Assert.Equal(new Outcome<string>.Failed(), outcome);
    }

    [Fact]
    public async Task A_failure_arriving_after_a_newer_request_went_out_is_superseded_not_a_failure()
    {
        // Arrange
        var series = new RequestSeries();
        var slow = new TaskCompletionSource<string>();
        var pending = series.Next().Fetch(() => slow.Task);

        // Act
        series.Next();
        slow.SetException(new HttpRequestException(Offline));

        // Assert
        Assert.Equal(new Outcome<string>.Superseded(), await pending);
    }

    [Fact]
    public async Task Stopping_the_series_supersedes_every_answer_still_out_whether_it_arrives_or_fails()
    {
        // Arrange
        var series = new RequestSeries();
        var arriving = new TaskCompletionSource<string>();
        var failing = new TaskCompletionSource<string>();
        var request = series.Next();
        var pending = (request.Fetch(() => arriving.Task), request.Fetch(() => failing.Task));

        // Act
        series.Stop();
        arriving.SetResult("the scene");
        failing.SetException(new HttpRequestException(Offline));

        // Assert
        Assert.Equal(
            ((Outcome<string>)new Outcome<string>.Superseded(), (Outcome<string>)new Outcome<string>.Superseded()),
            (await pending.Item1, await pending.Item2));
    }

    [Fact]
    public async Task Asking_again_after_a_failure_asks_the_server_again()
    {
        // Arrange
        var series = new RequestSeries();
        var answers = new Queue<Func<Task<string>>>([() => throw new HttpRequestException(Offline), () => Task.FromResult("the scene")]);
        var failed = await series.Next().Fetch(answers.Dequeue());

        // Act
        var retried = await series.Next().Fetch(answers.Dequeue());

        // Assert
        Assert.Equal<Outcome<string>>([new Outcome<string>.Failed(), new Outcome<string>.Arrived("the scene")], [failed, retried]);
    }

    [Fact]
    public void Every_outcome_is_matched_by_its_own_case()
    {
        // Arrange
        var outcomes = new Outcome<string>[] { new Outcome<string>.Arrived("the scene"), new Outcome<string>.Failed(), new Outcome<string>.Superseded() };

        // Act
        var matched = outcomes.Select(outcome => outcome.Match(arrived: value => $"arrived {value}", failed: () => "failed", superseded: () => "superseded")).ToList();

        // Assert
        Assert.Equal(["arrived the scene", "failed", "superseded"], matched);
    }

    [Fact]
    public void Every_outcome_acts_on_its_own_case()
    {
        // Arrange
        var outcomes = new Outcome<string>[] { new Outcome<string>.Arrived("the scene"), new Outcome<string>.Failed(), new Outcome<string>.Superseded() };
        var acted = new List<string>();

        // Act
        foreach (var outcome in outcomes)
        {
            outcome.Match(arrived: value => acted.Add($"arrived {value}"), failed: () => acted.Add("failed"), superseded: () => acted.Add("superseded"));
        }

        // Assert
        Assert.Equal(["arrived the scene", "failed", "superseded"], acted);
    }

    [Fact]
    public async Task Every_outcome_awaits_the_work_of_its_own_case()
    {
        // Arrange
        var outcomes = new Outcome<string>[] { new Outcome<string>.Arrived("the scene"), new Outcome<string>.Failed(), new Outcome<string>.Superseded() };
        var acted = new List<string>();

        // Act
        foreach (var outcome in outcomes)
        {
            await outcome.Match(
                arrived: async value =>
                {
                    await Task.Yield();
                    acted.Add($"arrived {value}");
                },
                failed: () => acted.Add("failed"),
                superseded: () => acted.Add("superseded"));
        }

        // Assert
        Assert.Equal(["arrived the scene", "failed", "superseded"], acted);
    }

    [Fact]
    public void An_outcome_maps_its_arrival_and_keeps_every_other_case()
    {
        // Arrange
        var outcomes = new Outcome<string>[] { new Outcome<string>.Arrived("the scene"), new Outcome<string>.Failed(), new Outcome<string>.Superseded() };

        // Act
        var mapped = outcomes.Select(outcome => outcome.Select(value => value.Length)).ToList();

        // Assert
        Assert.Equal<Outcome<int>>([new Outcome<int>.Arrived("the scene".Length), new Outcome<int>.Failed(), new Outcome<int>.Superseded()], mapped);
    }

    private const string Offline = "offline";

    [Fact]
    public async Task A_walk_answers_with_its_own_outcome_not_one_wrapped_in_an_arrival()
    {
        // Arrange
        var request = new RequestSeries().Next();

        // Act
        var answers = (
            await request.Walk(() => Task.FromResult<Outcome<string>>(new Outcome<string>.Arrived("the trail"))),
            await request.Walk(() => Task.FromResult<Outcome<string>>(new Outcome<string>.Failed())));

        // Assert
        Assert.Equal(((Outcome<string>)new Outcome<string>.Arrived("the trail"), (Outcome<string>)new Outcome<string>.Failed()), answers);
    }

    [Fact]
    public async Task A_walk_ending_after_a_newer_request_went_out_is_dropped()
    {
        // Arrange
        var series = new RequestSeries();
        var slow = new TaskCompletionSource<Outcome<string>>();
        var pending = series.Next().Walk(() => slow.Task);

        // Act
        series.Next();
        slow.SetResult(new Outcome<string>.Arrived("the old trail"));
        var answer = await pending;

        // Assert
        Assert.Equal<Outcome<string>>(new Outcome<string>.Superseded(), answer);
    }
}
