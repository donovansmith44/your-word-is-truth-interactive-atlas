namespace BibleAtlas.Client.Exploring;

public sealed record PagePosition(int From, int To, int Total, bool Less, bool More)
{
    public bool Paged => Less || More;
}

public static class PageWindow
{
    public const int ShownEntries = 40;

    public static int BlocksShown(int step) => Math.Max(1, ShownEntries / step);
}

public sealed class PageWindow<T>
{
    private readonly PageRead<T> _read;
    private readonly Func<T, bool> _keep;
    private readonly int _step;
    private readonly int _blocksShown;
    private readonly CancellationTokenSource _stopped = new();
    private IReadOnlyList<int?> _earlier = [];
    private IReadOnlyList<Block> _blocks;
    private bool _settling;

    private PageWindow(PageRead<T> read, Func<T, bool> keep, int step, Block first)
    {
        (_read, _keep, _step, _blocksShown) = (read, keep, step, PageWindow.BlocksShown(step));
        _blocks = [first];
    }

    public static async Task<PageWindow<T>> Opened(PageRead<T> read, Func<T, bool> keep, int step) =>
        new(read, keep, step, await Block.At(read, keep, step, null));

    public int Wanted { get; private set; } = 1;

    public IEnumerable<T> Shown => _blocks.SelectMany(block => block.Read.Kept);

    public PagePosition Position(int total)
    {
        var from = _earlier.Count * _step;
        return new PagePosition(from + 1, from + _blocks.Sum(block => block.Read.Read), total, Revealed > 1, !_blocks[^1].Read.Ended);
    }

    private int Revealed => _earlier.Count + _blocks.Count;

    public Task<Outcome<Unit>> More()
    {
        Wanted++;
        return Settle();
    }

    public Task<Outcome<Unit>> Fewer()
    {
        Wanted = Math.Max(1, Wanted - 1);
        return Settle();
    }

    public Task<Outcome<Unit>> Resume() => Settle();

    public void Stop() => _stopped.Cancel();

    private async Task<Outcome<Unit>> Settle()
    {
        if (_settling)
        {
            return new Outcome<Unit>.Arrived(new Unit());
        }

        _settling = true;
        var settled = await Settling(new Request(_stopped.Token));
        _settling = false;
        return settled;
    }

    private async Task<Outcome<Unit>> Settling(Request request)
    {
        while (Revealed != Wanted)
        {
            if (Revealed < Wanted && _blocks[^1].Read.Ended)
            {
                Wanted = Revealed;
            }
            else if (Revealed > Wanted && _earlier is [])
            {
                _blocks = _blocks.SkipLast(1).ToList();
            }
            else if (await Step(request, Revealed < Wanted) is { } ended)
            {
                return ended;
            }
        }

        return new Outcome<Unit>.Arrived(new Unit());
    }

    private async Task<Outcome<Unit>?> Step(Request request, bool onward)
    {
        var start = onward ? _blocks[^1].Read.Next : _earlier[^1];
        var read = await request.Fetch(() => Block.At(_read, _keep, _step, start));
        var wanted = onward ? Revealed < Wanted : Revealed > Wanted;
        return read.Match<Outcome<Unit>?>(
            arrived: block => wanted ? Apply(block, onward) : null,
            failed: () => wanted ? new Outcome<Unit>.Failed() : null,
            superseded: () => new Outcome<Unit>.Superseded());
    }

    private Outcome<Unit>? Apply(Block block, bool onward)
    {
        if (onward)
        {
            _blocks = [.. _blocks, block];
            if (_blocks.Count > _blocksShown)
            {
                _earlier = [.. _earlier, _blocks[0].Start];
                _blocks = _blocks.Skip(1).ToList();
            }
        }
        else
        {
            _earlier = _earlier.SkipLast(1).ToList();
            _blocks = [block, .. _blocks.SkipLast(1)];
        }

        return null;
    }

    private sealed record Block(int? Start, Paging<T> Read)
    {
        public static async Task<Block> At(PageRead<T> read, Func<T, bool> keep, int step, int? start) =>
            new(start, await Paging.From(read, start, step, keep));
    }
}
