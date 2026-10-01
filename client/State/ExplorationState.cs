using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.State;

public abstract record ExplorationState
{
    private ExplorationState()
    {
    }

    public sealed record Closed : ExplorationState;

    public sealed record Open(Exploration Exploration) : ExplorationState;
}

public abstract record ExplorationIntent : IIntent<ExplorationState>
{
    private ExplorationIntent(string? origin)
    {
        Origin = origin;
    }

    public abstract string Name { get; }

    public string? Origin { get; }

    public abstract ExplorationState Apply(ExplorationState current);

    public sealed record Open(Explorable Node, string? Origin = null) : ExplorationIntent(Origin)
    {
        public override string Name => "exploration-open";

        public override ExplorationState Apply(ExplorationState current) =>
            current is ExplorationState.Open { Exploration.Current: var here } && here == Node
                ? current
                : new ExplorationState.Open(new Exploration(Node, []));
    }

    public sealed record Arrive(Exploration From, Exploration Trail, string? Origin = null) : ExplorationIntent(Origin)
    {
        public override string Name => "exploration-arrive";

        public override ExplorationState Apply(ExplorationState current) =>
            current is ExplorationState.Open { Exploration: var here } && here == From ? new ExplorationState.Open(Trail) : current;
    }

    public sealed record Reset(string? Origin = null) : ExplorationIntent(Origin)
    {
        public override string Name => "exploration-reset";

        public override ExplorationState Apply(ExplorationState current) => new ExplorationState.Closed();
    }

    public sealed record Reseed(Exploration Snapshot, string? Origin = null) : ExplorationIntent(Origin)
    {
        public override string Name => "exploration-reseed";

        public override ExplorationState Apply(ExplorationState current) => new ExplorationState.Open(Snapshot);
    }
}
