using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

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
        public override T Match<T>(Func<T> none, Func<NodeRef, double, double, T> site, Func<NodeRef, TimeRange, T> territory) => site(Place, Lat, Lon);
    }

    public sealed record Territory(NodeRef Polity, TimeRange Reign) : Emphasis
    {
        public override T Match<T>(Func<T> none, Func<NodeRef, double, double, T> site, Func<NodeRef, TimeRange, T> territory) => territory(Polity, Reign);
    }
}

public static class Crossing
{
    public static ArrowDirection? Of(TimeRange bounds, int from, int to) =>
        to > bounds.To.Value ? ArrowDirection.Next
        : from < bounds.From.Value ? ArrowDirection.Previous
        : null;
}
