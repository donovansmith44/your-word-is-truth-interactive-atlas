using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public abstract record PopoverOpening
{
    private PopoverOpening()
    {
    }

    public abstract T Match<T>(Func<PositionRef, T> explore, Func<SavedExploration, T> resume, Func<IExplorable, T> legacy);

    public bool OpensLegacy(Func<IExplorable, bool> matching) => Match(explore: _ => false, resume: _ => false, legacy: matching);

    public sealed record Explore(PositionRef Target) : PopoverOpening
    {
        public override T Match<T>(Func<PositionRef, T> explore, Func<SavedExploration, T> resume, Func<IExplorable, T> legacy) => explore(Target);
    }

    public sealed record Resume(SavedExploration Saved) : PopoverOpening
    {
        public override T Match<T>(Func<PositionRef, T> explore, Func<SavedExploration, T> resume, Func<IExplorable, T> legacy) => resume(Saved);
    }

    public sealed record Legacy(IExplorable Node) : PopoverOpening
    {
        public override T Match<T>(Func<PositionRef, T> explore, Func<SavedExploration, T> resume, Func<IExplorable, T> legacy) => legacy(Node);
    }
}
