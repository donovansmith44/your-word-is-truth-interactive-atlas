using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

public sealed class ExplorationOwnershipHandoffTests
{
    private static readonly Explorable Genesis1 = Resolved.Node(NodeKind.TextUnit, "text-unit:GEN.1.1", "GEN.1.1");
    private static readonly Explorable Genesis2 = Resolved.Node(NodeKind.TextUnit, "text-unit:GEN.1.2", "GEN.1.2");
    private static readonly Explorable Exodus3 = Resolved.Node(NodeKind.TextUnit, "text-unit:EXO.3.14", "EXO.3.14");
    private static readonly Explorable Psalm23 = Resolved.Node(NodeKind.TextUnit, "text-unit:PSA.23.1", "PSA.23.1");
    private static readonly Step ToGenesis2 = new(EdgeKind.FollowsIn, Genesis2);

    private static ExplorationState Closed => new ExplorationState.Closed();

    private static ExplorationState At(Explorable node) => new ExplorationState.Open(new Exploration(node, []));

    private static StateAtom<ExplorationState> Atom() => new(AtomNames.Exploration, Closed);

    private sealed class FakePopover
    {
        private readonly StateAtom<ExplorationState> _atom;
        private readonly OwnershipRegistry _ownership;
        private ExplorationState _frozen = Closed;

        public FakePopover(StateAtom<ExplorationState> atom, OwnershipRegistry ownership)
        {
            _atom = atom;
            _ownership = ownership;
            _atom.Changed += OnChanged;
        }

        public OwnershipClaim? Claim { get; private set; }

        public ExplorationState Value => _frozen;

        public void Open(Explorable root)
        {
            Claim = _ownership.Claim(AtomNames.Exploration);
            ApplyLocally(new ExplorationIntent.Open(root));
        }

        public void Close()
        {
            _atom.Changed -= OnChanged;
            ApplyLocally(new ExplorationIntent.Reset());
            Claim?.Dispose();
        }

        private void ApplyLocally(ExplorationIntent intent)
        {
            if (Claim?.IsCurrent == true)
            {
                _atom.Dispatch(intent);
                SyncSnapshot();
            }
            else
            {
                _frozen = intent.Apply(_frozen);
            }
        }

        private void SyncSnapshot()
        {
            if (Claim?.IsCurrent == true)
            {
                _frozen = _atom.Value;
            }
        }

        private void OnChanged()
        {
            if (Claim?.IsCurrent != true && _atom.Value is ExplorationState.Closed && _frozen is ExplorationState.Open own)
            {
                Claim?.Dispose();
                Claim = _ownership.Claim(AtomNames.Exploration);
                _atom.Dispatch(new ExplorationIntent.Reseed(own.Exploration));
            }

            SyncSnapshot();
        }
    }

    [Fact]
    public void An_exploration_a_popover_has_claimed_is_held()
    {
        // Arrange
        var ownership = new OwnershipRegistry();

        // Act
        ownership.Claim(AtomNames.Exploration);

        // Assert
        Assert.True(ownership.IsHeld(AtomNames.Exploration));
    }

    [Fact]
    public void An_exploration_whose_claim_was_released_is_held_by_nobody()
    {
        // Arrange
        var ownership = new OwnershipRegistry();
        var claim = ownership.Claim(AtomNames.Exploration);

        // Act
        claim.Dispose();

        // Assert
        Assert.False(ownership.IsHeld(AtomNames.Exploration));
    }

    [Fact]
    public void The_owning_popover_renders_the_atom_itself_not_a_copy_taken_when_it_opened()
    {
        // Arrange
        var atom = Atom();
        var popover = new FakePopover(atom, new OwnershipRegistry());
        popover.Open(Genesis1);

        // Act
        atom.Dispatch(new ExplorationIntent.Reseed(new Exploration(Genesis1, [ToGenesis2])));

        // Assert
        Assert.Equal(new ExplorationState.Open(new Exploration(Genesis1, [ToGenesis2])), popover.Value);
    }

    [Fact]
    public void Opening_a_second_popover_supersedes_the_first_without_touching_its_own_exploration()
    {
        // Arrange
        var atom = Atom();
        var ownership = new OwnershipRegistry();
        var first = new FakePopover(atom, ownership);
        first.Open(Genesis1);
        var second = new FakePopover(atom, ownership);

        // Act
        second.Open(Exodus3);

        // Assert
        Assert.Equal(
            (false, true, At(Exodus3), At(Genesis1)),
            (first.Claim!.IsCurrent, second.Claim!.IsCurrent, atom.Value, first.Value));
    }

    [Fact]
    public void Closing_the_active_popover_lets_a_live_superseded_one_reclaim_the_atom_and_reseed_its_own_exploration()
    {
        // Arrange
        var atom = Atom();
        var ownership = new OwnershipRegistry();
        var first = new FakePopover(atom, ownership);
        first.Open(Genesis1);
        var second = new FakePopover(atom, ownership);
        second.Open(Exodus3);

        // Act
        second.Close();

        // Assert
        Assert.Equal((true, At(Genesis1), At(Genesis1)), (first.Claim!.IsCurrent, atom.Value, first.Value));
    }

    [Fact]
    public void Closing_the_active_popover_with_no_other_live_one_leaves_the_atom_closed()
    {
        // Arrange
        var atom = Atom();
        var only = new FakePopover(atom, new OwnershipRegistry());
        only.Open(Genesis1);

        // Act
        only.Close();

        // Assert
        Assert.Equal(Closed, atom.Value);
    }

    [Fact]
    public void When_three_popovers_are_live_closing_the_active_one_hands_the_atom_to_the_first_subscriber_and_the_other_stays_inert()
    {
        // Arrange
        var atom = Atom();
        var ownership = new OwnershipRegistry();
        var first = new FakePopover(atom, ownership);
        first.Open(Genesis1);
        var second = new FakePopover(atom, ownership);
        second.Open(Exodus3);
        var third = new FakePopover(atom, ownership);
        third.Open(Psalm23);

        // Act
        third.Close();

        // Assert
        Assert.Equal((true, false, At(Genesis1)), (first.Claim!.IsCurrent, second.Claim!.IsCurrent, atom.Value));
    }
}
