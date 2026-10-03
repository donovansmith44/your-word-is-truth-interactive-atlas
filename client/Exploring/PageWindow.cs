using BibleAtlas.Client.Contract;
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
    private readonly Func<bool> _moved;
    private readonly CancellationTokenSource _stopped = new();
    private IReadOnlyList<Block> _blocks;
    private int _blocksBefore;
    private int _entriesBefore;
    private bool _settling;

    private PageWindow(PageRead<T> read, Func<T, bool> keep, int step, Func<bool> moved, Block first)
    {
        (_read, _keep, _step, _blocksShown, _moved) = (read, keep, step, PageWindow.BlocksShown(step), moved);
        _blocks = [first];
    }

    internal static async Task<PageWindow<T>> Opened(PageRead<T> read, Func<T, bool> keep, int step, Func<bool> moved) =>
        new(read, keep, step, moved, await Block.At(read, keep, step, null));

    public int Wanted { get; private set; } = 1;

    public bool Moved => _moved();

    public IEnumerable<T> Shown => _blocks.SelectMany(block => block.Read.Kept);

    public IEnumerable<EdgePageCursor?> CursorsHeld => _blocks.SelectMany(block => new[] { block.Read.Previous, block.Read.Next });

    public PagePosition Position(int total) =>
        new(_entriesBefore + 1, _entriesBefore + _blocks.Sum(block => block.Read.Read), total, Revealed > 1, !_blocks[^1].Read.Ended);

    private int Revealed => _blocksBefore + _blocks.Count;

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
            else if (Revealed > Wanted && _blocks[0].Read.Previous is null)
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
        var start = onward ? _blocks[^1].Read.Next : _blocks[0].Read.Previous;
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
                (_blocksBefore, _entriesBefore) = (_blocksBefore + 1, _entriesBefore + _blocks[0].Read.Read);
                _blocks = _blocks.Skip(1).ToList();
            }
        }
        else
        {
            (_blocksBefore, _entriesBefore) = (_blocksBefore - 1, _entriesBefore - block.Read.Read);
            _blocks = [block, .. _blocks.SkipLast(1)];
        }

        return null;
    }

    private sealed record Block(Paging<T> Read)
    {
        public static async Task<Block> At(PageRead<T> read, Func<T, bool> keep, int step, EdgePageCursor? start) =>
            new(await Paging.From(read, start, step, keep));
    }
}
