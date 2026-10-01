using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

public abstract record PopoverOpening
{
    private PopoverOpening()
    {
    }

    public abstract T Match<T>(Func<PositionRef, T> explore, Func<SavedExploration, T> resume, Func<IExplorable, T> legacy);

    public bool OpensLegacy(Func<IExplorable, bool> matching) => Match(explore: _ => false, resume: _ => false, legacy: matching);

    public sealed record Explore(PositionRef Target) : PopoverOpening
    {
        public bool Equals(Explore? other) => other is not null && PositionIdentity.Comparer.Equals(Target, other.Target);

        public override int GetHashCode() => PositionIdentity.Comparer.GetHashCode(Target);

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
