using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public abstract record Frame
{
    private Frame()
    {
    }

    public abstract T Match<T>(Func<T> current, Func<TimeRange, Link?, Link?, T> bounded);

    public sealed record Current : Frame
    {
        public override T Match<T>(Func<T> current, Func<TimeRange, Link?, Link?, T> bounded) => current();
    }

    public sealed record Bounded(TimeRange Window, Link? Previous, Link? Next) : Frame
    {
        public override T Match<T>(Func<T> current, Func<TimeRange, Link?, Link?, T> bounded) => bounded(Window, Previous, Next);
    }
}

public abstract record Emphasis
{
    private Emphasis()
    {
    }

    public abstract T Match<T>(Func<T> none, Func<NodeRef, double, double, T> site, Func<NodeRef, TimeRange, T> territory);

    public sealed record None : Emphasis
    {
        public override T Match<T>(Func<T> none, Func<NodeRef, double, double, T> site, Func<NodeRef, TimeRange, T> territory) => none();
    }

    public sealed record Site(NodeRef Place, double Lat, double Lon) : Emphasis
    {
        public bool Equals(Site? other) => other is not null && PositionIdentity.Comparer.Equals(Place, other.Place) && (Lat, Lon) == (other.Lat, other.Lon);

        public override int GetHashCode() => HashCode.Combine(PositionIdentity.Comparer.GetHashCode(Place), Lat, Lon);

        public override T Match<T>(Func<T> none, Func<NodeRef, double, double, T> site, Func<NodeRef, TimeRange, T> territory) => site(Place, Lat, Lon);
    }

    public sealed record Territory(NodeRef Polity, TimeRange Reign) : Emphasis
    {
        public bool Equals(Territory? other) => other is not null && PositionIdentity.Comparer.Equals(Polity, other.Polity) && Reign == other.Reign;

        public override int GetHashCode() => HashCode.Combine(PositionIdentity.Comparer.GetHashCode(Polity), Reign);

        public override T Match<T>(Func<T> none, Func<NodeRef, double, double, T> site, Func<NodeRef, TimeRange, T> territory) => territory(Polity, Reign);
    }
}

public static class Crossing
{
    public static ArrowDirection? Of(TimeRange bounds, int from, int to) =>
        to > bounds.To.Value ? ArrowDirection.Next
        : from < bounds.From.Value ? ArrowDirection.Previous
        : null;

    public static Explore<Explorable> Walk(EdgeKind kind) =>
        from page in Explore.Links(kind)
        from next in page.Items is [var first, ..] ? Explore.Follow(first) : Explore.Here
        select next;
}
