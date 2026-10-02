using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class PageWindowTests
{
    private static readonly int Step = Affordances.PageSize;
    private const int Unending = 1_000;
    private const int LongestSequence = 5;
    private const int PastTheEnd = 2;
    private const int MostRowsShown = 40;

    private static readonly int[] Cardinalities = [300, 3_000, 30_000];

    private static readonly Op[] Ops = Enum.GetValues<Op>();

    private enum Op
    {
        More,
        Fewer,
        Answer,
        Fail,
    }

    [Fact]
    public async Task Demand_made_while_a_page_is_still_arriving_is_read_after_it_one_page_at_a_time()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var collection = new Collection(Unending) { Held = true };
            var window = await Opened(collection, Step);

            // Act
            var first = window.More();
            var second = window.More();
            var inFlight = collection.Pending.Count;
            collection.Answer();
            collection.Answer();
            await Task.WhenAll(first, second);

            // Assert
            Assert.Equal(
                (1, WholeValue.Of(Range(Step, 3 * Step)), WholeValue.Of(new int?[] { null, Step, 2 * Step })),
                (inFlight, WholeValue.Of(window.Shown), WholeValue.Of(collection.Asked)));
        });
    }

    [Fact]
    public async Task Every_interleaving_of_demand_answers_and_failures_settles_on_the_window_last_wanted()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var sequences = Enumerable.Range(1, LongestSequence).SelectMany(length => Sequences(length)).ToList();

            // Act
            var offenders = new List<string>();
            foreach (var sequence in sequences)
            {
                if (await Offence(sequence) is { } offence)
                {
                    offenders.Add($"{string.Join(", ", sequence)}: {offence}");
                }
            }

            // Assert
            Assert.Empty(offenders);
        });
    }

    [Fact]
    public async Task A_failure_of_a_page_still_wanted_is_reported_and_the_window_keeps_what_it_shows()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var collection = new Collection(Unending) { Held = true };
            var window = await Opened(collection, Step);
            var more = window.More();

            // Act
            collection.Fail();
            var outcome = await more;

            // Assert
            Assert.Equal((new Outcome<Unit>.Failed() as Outcome<Unit>, WholeValue.Of(Range(0, Step))), (outcome, WholeValue.Of(window.Shown)));
        });
    }

    [Fact]
    public async Task A_failure_of_a_page_no_longer_wanted_is_dropped()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var collection = new Collection(Unending) { Held = true };
            var window = await Opened(collection, Step);
            var more = window.More();
            await window.Fewer();

            // Act
            collection.Fail();
            var outcome = await more;

            // Assert
            Assert.Equal((new Outcome<Unit>.Arrived(new Unit()) as Outcome<Unit>, WholeValue.Of(Range(0, Step))), (outcome, WholeValue.Of(window.Shown)));
        });
    }

    [Fact]
    public async Task A_page_arriving_after_the_window_was_stopped_changes_nothing()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var collection = new Collection(Unending) { Held = true };
            var window = await Opened(collection, Step);
            var more = window.More();
            window.Stop();

            // Act
            collection.Answer();
            var outcome = await more;

            // Assert
            Assert.Equal((new Outcome<Unit>.Superseded() as Outcome<Unit>, WholeValue.Of(Range(0, Step))), (outcome, WholeValue.Of(window.Shown)));
        });
    }

    [Fact]
    public async Task Resuming_after_a_failure_reads_the_page_still_wanted()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var collection = new Collection(Unending) { Held = true };
            var window = await Opened(collection, Step);
            var more = window.More();
            collection.Fail();
            await more;

            // Act
            var resumed = window.Resume();
            collection.Answer();
            await resumed;

            // Assert
            Assert.Equal(Range(0, 2 * Step), window.Shown);
        });
    }

    [Fact]
    public async Task Fewer_after_the_window_slid_reads_back_the_page_it_let_go()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var collection = new Collection(Unending);
            var window = await Opened(collection, Step);
            var slid = PageWindow.BlocksShown(Step) + 1;
            for (var more = 1; more < slid; more++)
            {
                await window.More();
            }

            // Act
            await window.Fewer();

            // Assert
            Assert.Equal((WholeValue.Of(Range(0, PageWindow.ShownEntries)), slid - 1), (WholeValue.Of(window.Shown), window.Wanted));
        });
    }

    [Fact]
    public async Task More_past_the_end_stops_at_the_end()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var size = 2 * Step;
            var collection = new Collection(size);
            var window = await Opened(collection, Step);

            // Act
            for (var more = 0; more < size / Step + PastTheEnd; more++)
            {
                await window.More();
            }

            // Assert
            Assert.Equal((WholeValue.Of(Range(0, size)), size / Step, false), (WholeValue.Of(window.Shown), window.Wanted, window.Position(size).More));
        });
    }

    [Fact]
    public void A_step_wider_than_the_window_still_shows_one_whole_step()
    {
        // Arrange
        var wide = 2 * PageWindow.ShownEntries;

        // Act
        var blocks = PageWindow.BlocksShown(wide);

        // Assert
        Assert.Equal(1, blocks);
    }

    [Fact]
    public async Task Every_affordance_shows_the_same_bounded_window_however_large_the_collection()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var steps = Enum.GetValues<EdgeKind>().Select(kind => Affordances.Of(kind).InitialClamp).Distinct().ToList();

            // Act
            var resident = new List<(int Step, int Cardinality, int MostShown, int Read)>();
            foreach (var step in steps)
            {
                foreach (var cardinality in Cardinalities)
                {
                    var collection = new Collection(cardinality);
                    var window = await Opened(collection, step);
                    var mostShown = window.Shown.Count();
                    for (var more = 1; more < Cardinalities[0] / step; more++)
                    {
                        await window.More();
                        mostShown = Math.Max(mostShown, window.Shown.Count());
                    }

                    for (var fewer = 1; fewer < Cardinalities[0] / step; fewer++)
                    {
                        await window.Fewer();
                        mostShown = Math.Max(mostShown, window.Shown.Count());
                    }

                    resident.Add((step, cardinality, mostShown, collection.Asked.Count));
                }
            }

            // Assert
            Assert.All(resident, read => Assert.InRange(read.MostShown, 1, MostRowsShown));
            Assert.All(resident.GroupBy(read => read.Step), byStep => Assert.Single(byStep.Select(read => (read.MostShown, read.Read)).Distinct()));
        });
    }

    [Fact]
    public async Task Every_position_of_a_long_reveal_shows_a_bounded_window_states_its_rows_and_offers_only_the_ways_that_lead_somewhere()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var turns = Cardinalities[0] / Step + PastTheEnd;

            // Act
            var offenders = new List<string>();
            foreach (var cardinality in Cardinalities)
            {
                var window = await Opened(new Collection(cardinality), Step);
                var positions = new List<string?> { Misplaced(window, cardinality) };
                for (var more = 0; more < turns; more++)
                {
                    await window.More();
                    positions.Add(Misplaced(window, cardinality));
                }

                for (var less = 0; less < turns; less++)
                {
                    await window.Fewer();
                    positions.Add(Misplaced(window, cardinality));
                }

                offenders.AddRange(positions.OfType<string>().Select(offence => $"{cardinality}: {offence}"));
            }

            // Assert
            Assert.Empty(offenders);
        });
    }

    [Fact]
    public async Task Less_retraces_every_step_more_took()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var window = await Opened(new Collection(Cardinalities[0]), Step);
            var onward = new List<(string Shown, PagePosition Position)> { Seen(window) };
            for (var more = 1; more < LongestSequence; more++)
            {
                await window.More();
                onward.Add(Seen(window));
            }

            // Act
            var back = new List<(string Shown, PagePosition Position)> { Seen(window) };
            for (var less = 1; less < LongestSequence; less++)
            {
                await window.Fewer();
                back.Add(Seen(window));
            }

            // Assert
            Assert.Equal(Enumerable.Reverse(onward), back);
        });
    }

    [Fact]
    public async Task Less_after_more_reads_back_from_the_page_store_without_asking_again()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var graph = new EdgesGraph(Cardinalities[0]);
            var window = await Paging.Window(new PresentationRequest(Resolved.At(graph, Subject), Surface.Popover), EdgeKind.MentionedIn);
            for (var more = 0; more <= PageWindow.BlocksShown(Step); more++)
            {
                await window.More();
            }

            var asked = graph.Asked;

            // Act
            await window.Fewer();

            // Assert
            Assert.Equal((asked, Step), (graph.Asked, window.Position(Cardinalities[0]).From - 1));
        });
    }

    [Fact]
    public async Task Less_back_to_the_first_block_reads_it_from_the_page_store_without_asking_again()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var graph = new EdgesGraph(Cardinalities[0]);
            var window = await Paging.Window(new PresentationRequest(Resolved.At(graph, Subject), Surface.Popover), EdgeKind.MentionedIn);
            for (var more = 0; more <= PageWindow.BlocksShown(Step); more++)
            {
                await window.More();
            }

            while (window.Position(Cardinalities[0]).Less && window.Position(Cardinalities[0]).From - 1 > Step)
            {
                await window.Fewer();
            }

            var asked = graph.Asked;

            // Act
            await window.Fewer();

            // Assert
            Assert.Equal((asked, 0), (graph.Asked, window.Position(Cardinalities[0]).From - 1));
        });
    }

    [Fact]
    public async Task The_first_page_asked_with_no_cursor_and_at_the_previous_of_the_second_page_is_one_page_store_entry()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var graph = new EdgesGraph(Cardinalities[0]);
            var element = Resolved.At(graph, Subject);
            var first = await element.Entries(EdgeKind.MentionedIn, null, Step);
            var second = await element.Entries(EdgeKind.MentionedIn, first.Next, Step);
            var asked = graph.Asked;

            // Act
            var back = await element.Entries(EdgeKind.MentionedIn, second.Previous, Step);

            // Assert
            Assert.Equal((true, asked, first), (second.Previous is not null, graph.Asked, back));
        });
    }

    [Fact]
    public async Task A_whole_walk_holds_the_same_cursors_and_does_the_same_work_each_turn_however_large_the_collection()
    {
        await Task.Run(async () =>
        {
            // Arrange
            var walks = new List<(int MostCursorsHeld, int MostReadsInATurn, long MostAllocatedInATurn)>();

            // Act
            foreach (var cardinality in Cardinalities)
            {
                var reads = 0;
                Task<Page<int>> Read(int? cursor, int limit)
                {
                    reads++;
                    var from = cursor ?? 0;
                    var to = Math.Min(from + limit, cardinality);
                    return Task.FromResult(new Page<int>(Enumerable.Range(from, to - from).ToList(), ServedGraph.PageBefore(from, limit), to < cardinality ? to : null));
                }

                var window = await PageWindow<int>.Opened(Read, Paging.Everything, Step, () => false);
                var turns = new List<(int CursorsHeld, int Reads, long Allocated)>();
                foreach (var turn in Enumerable.Repeat<Func<Task<Outcome<Unit>>>>(window.More, cardinality / Step).Concat(Enumerable.Repeat<Func<Task<Outcome<Unit>>>>(window.Fewer, cardinality / Step)))
                {
                    var (readBefore, allocatedBefore) = (reads, GC.GetAllocatedBytesForCurrentThread());
                    await turn();
                    var allocated = GC.GetAllocatedBytesForCurrentThread() - allocatedBefore;
                    turns.Add((window.CursorsHeld.Count(), reads - readBefore, allocated));
                }

                walks.Add((turns.Max(turn => turn.CursorsHeld), turns.Max(turn => turn.Reads), turns.Skip(WarmTurns).Max(turn => turn.Allocated)));
            }

            // Assert
            Assert.Single(walks.Distinct());
        });
    }

    private static (string Shown, PagePosition Position) Seen(PageWindow<int> window) =>
        (WholeValue.Of(window.Shown), window.Position(Cardinalities[0]));

    private static string? Misplaced(PageWindow<int> window, int cardinality)
    {
        var shown = window.Shown.ToList();
        var position = window.Position(cardinality);
        var stated = new PagePosition(shown[0] + 1, shown[^1] + 1, cardinality, window.Wanted > 1, shown[^1] + 1 < cardinality);
        return shown.Count <= MostRowsShown && position == stated ? null : $"showed {Describe(shown)} as {position}";
    }

    private static async Task<string?> Offence(IReadOnlyList<Op> sequence)
    {
        var collection = new Collection(Unending) { Held = true };
        var window = await Opened(collection, Step);
        var wanted = 1;
        foreach (var op in sequence)
        {
            switch (op)
            {
                case Op.More:
                    _ = window.More();
                    wanted++;
                    break;
                case Op.Fewer:
                    _ = window.Fewer();
                    wanted = Math.Max(1, wanted - 1);
                    break;
                case Op.Answer:
                    collection.Answer();
                    break;
                case Op.Fail:
                    collection.Fail();
                    break;
            }
            if (collection.Pending.Count > 1)
            {
                return "more than one page in flight";
            }

            if (Contiguity(window.Shown.ToList()) is { } broken)
            {
                return broken;
            }
        }

        var resumed = window.Resume();
        while (collection.Pending.Count > 0)
        {
            collection.Answer();
        }

        await resumed;
        var expected = Range(Math.Max(0, wanted - PageWindow.BlocksShown(Step)) * Step, wanted * Step);
        return window.Shown.SequenceEqual(expected) && window.Wanted == wanted ? null : $"settled on {Describe(window.Shown.ToList())}, wanted {Describe(expected)}";
    }

    private static string? Contiguity(IReadOnlyList<int> shown) =>
        shown is [var first, ..] && first % Step == 0 && shown.SequenceEqual(Range(first, first + shown.Count)) && shown.Count <= PageWindow.ShownEntries
            ? null
            : $"showed {Describe(shown)}";

    private static string Describe(IReadOnlyList<int> shown) => shown is [var first, .., var last] ? $"{first}..{last}" : $"{shown.Count} entries";

    private static IEnumerable<IReadOnlyList<Op>> Sequences(int length) =>
        length == 0 ? [[]] : Sequences(length - 1).SelectMany(prefix => Ops.Select(op => (IReadOnlyList<Op>)[.. prefix, op]));

    private static List<int> Range(int from, int to) => Enumerable.Range(from, to - from).ToList();

    private static Task<PageWindow<int>> Opened(Collection collection, int step) =>
        PageWindow<int>.Opened(collection.Read, Paging.Everything, step, () => false);

    private const int WarmTurns = 4;

    private static readonly PositionRef Subject = ServedGraph.At(NodeKind.Person, "Person:abraham", "Abraham");

    private sealed class EdgesGraph(int size) : IExplorableClient
    {
        public int Asked { get; private set; }

        public Task<NodeRecord> Card(string id) => throw new NotSupportedException();

        public Task<ElementPage> Elements(IReadOnlyList<string> ids) =>
            Task.FromResult(new ElementPage(
                elements: [new NodeElement(ServedGraph.Card(NodeKind.Person, Positions.Of(Subject).Id, Positions.Of(Subject).Label, new FrontierGroup(EdgeKind.MentionedIn, size)))],
                next: null, previous: null, version: ServedGraph.Version));

        public Task<EdgePage> Edges(string positionId, EdgeKind kind, int? cursor = null, int limit = IExplorableClient.DefaultPageSize)
        {
            Asked++;
            var from = cursor ?? 0;
            var to = Math.Min(from + limit, size);
            var verses = Enumerable.Range(from, to - from).Select(n => ServedGraph.Ref(NodeKind.TextUnit, $"text-unit:GEN.1.{n}", $"GEN.1.{n}")).ToArray();
            return Task.FromResult(ServedGraph.Page(kind, to < size ? to : null, verses) with { Previous = ServedGraph.PageBefore(from, limit) });
        }

        public Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
            throw new NotSupportedException();
    }

    private sealed class Collection(int size)
    {
        public bool Held { get; init; }

        public List<int?> Asked { get; } = [];

        public List<(Page<int> Page, TaskCompletionSource<Page<int>> Answer)> Pending { get; } = [];

        public Task<Page<int>> Read(int? cursor, int limit)
        {
            Asked.Add(cursor);
            var from = cursor ?? 0;
            var to = Math.Min(from + limit, size);
            var page = new Page<int>(Enumerable.Range(from, to - from).ToList(), ServedGraph.PageBefore(from, limit), to < size ? to : null);
            if (!Held || cursor is null)
            {
                return Task.FromResult(page);
            }

            var answer = new TaskCompletionSource<Page<int>>();
            Pending.Add((page, answer));
            return answer.Task;
        }

        public void Answer() => Settle(pending => pending.Answer.SetResult(pending.Page));

        public void Fail() => Settle(pending => pending.Answer.SetException(new HttpRequestException("offline")));

        private void Settle(Action<(Page<int> Page, TaskCompletionSource<Page<int>> Answer)> settle)
        {
            if (Pending is [var pending, ..])
            {
                Pending.RemoveAt(0);
                settle(pending);
            }
        }
    }
}
