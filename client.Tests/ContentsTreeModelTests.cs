using BibleAtlas.Client.Components;
using BibleAtlas.Client.Contract;
using static BibleAtlas.Client.Components.ContentsTreeModel;

namespace BibleAtlas.Client.Tests;

public class ContentsTreeModelTests
{
    private const string Genesis = "Container:bible-book-GEN";
    private const string Exodus = "Container:bible-book-EXO";
    private const string GenesisOne = "Container:bible-chapter-GEN-1";
    private const string GenesisTwo = "Container:bible-chapter-GEN-2";
    private const string ExodusOne = "Container:bible-chapter-EXO-1";
    private const int RootDepth = 0;
    private const int ChildDepth = 1;
    private static readonly BibleRef GenesisOneOne = new(BookId.GEN, 1, 1);
    private static readonly BibleRef ExodusOneOne = new(BookId.EXO, 1, 1);

    private static ContentsTreeModel Sample() => ContentsTreeModel.From(new Contents(corpus: Corpus.Bible, version: Wire.Root("v"), roots:
    [
        new(id: Wire.Node(Genesis), title: "Genesis", kind: ContentsRootKind.Book, group: Testament.OT, @ref: Wire.Read<ContentsReference>("GEN.1"), locus: GenesisOneOne, children:
        [
            new(id: Wire.Node(GenesisOne), title: "1", kind: ContentsChildKind.Chapter, @ref: Wire.Read<ContentsReference>("GEN.1"), locus: GenesisOneOne, count: 31),
            new(id: Wire.Node(GenesisTwo), title: "2", kind: ContentsChildKind.Chapter, @ref: Wire.Read<ContentsReference>("GEN.2"), locus: new BibleRef(BookId.GEN, 2, 1), count: 25),
        ]),
        new(id: Wire.Node(Exodus), title: "Exodus", kind: ContentsRootKind.Book, group: Testament.OT, @ref: Wire.Read<ContentsReference>("EXO.1"), locus: ExodusOneOne, children:
        [
            new(id: Wire.Node(ExodusOne), title: "1", kind: ContentsChildKind.Chapter, @ref: Wire.Read<ContentsReference>("EXO.1"), locus: ExodusOneOne, count: 22),
        ]),
    ]));

    [Fact]
    public void From_carries_the_contract_kinds_of_roots_and_children()
    {
        // Arrange
        var expected = new[]
        {
            new Root(Genesis, "Genesis", "GEN.1", GenesisOneOne, ContentsRootKind.Book, [
                new Child(GenesisOne, "1", "GEN.1", GenesisOneOne, ContentsChildKind.Chapter, 31),
                new Child(GenesisTwo, "2", "GEN.2", new BibleRef(BookId.GEN, 2, 1), ContentsChildKind.Chapter, 25),
            ]),
            new Root(Exodus, "Exodus", "EXO.1", ExodusOneOne, ContentsRootKind.Book, [
                new Child(ExodusOne, "1", "EXO.1", ExodusOneOne, ContentsChildKind.Chapter, 22),
            ]),
        };
        // Act
        var roots = Sample().Roots;
        // Assert
        Assert.Equivalent(expected, roots, strict: true);
    }

    [Fact]
    public void Collapsed_by_default_shows_only_roots()
    {
        // Arrange
        var model = Sample();
        var (genesis, exodus) = (model.Roots[0], model.Roots[1]);
        // Act
        var rows = model.Flatten();
        // Assert
        Assert.Equal(
            [
                new Row(genesis, RootDepth, Expandable: true, Expanded: false, Current: false),
                new Row(exodus, RootDepth, Expandable: true, Expanded: false, Current: false),
            ],
            rows);
    }

    [Fact]
    public void Toggle_expands_a_root_to_show_its_children()
    {
        // Arrange
        var model = Sample();
        var (genesis, exodus) = (model.Roots[0], model.Roots[1]);
        // Act
        model.Toggle(Genesis);
        // Assert
        Assert.Equal(
            [
                new Row(genesis, RootDepth, Expandable: true, Expanded: true, Current: false),
                new Row(genesis.Children[0], ChildDepth, Expandable: false, Expanded: false, Current: false),
                new Row(genesis.Children[1], ChildDepth, Expandable: false, Expanded: false, Current: false),
                new Row(exodus, RootDepth, Expandable: true, Expanded: false, Current: false),
            ],
            model.Flatten());
    }

    [Fact]
    public void Toggle_is_an_involution_for_any_id_sequence()
    {
        // Arrange
        var model = Sample();
        var before = model.Flatten();
        // Act
        foreach (var id in new[] { Exodus, Genesis, Exodus })
        {
            model.Toggle(id);
            model.Toggle(id);
        }
        // Assert
        Assert.Equal(before, model.Flatten());
    }

    [Fact]
    public void ExpandPathTo_opens_exactly_the_ancestors_of_the_current_ref()
    {
        // Arrange
        var model = Sample();
        var (genesis, exodus) = (model.Roots[0], model.Roots[1]);
        // Act
        model.ExpandPathTo("EXO.1");
        // Assert
        Assert.Equal(
            [
                new Row(genesis, RootDepth, Expandable: true, Expanded: false, Current: false),
                new Row(exodus, RootDepth, Expandable: true, Expanded: true, Current: false),
                new Row(exodus.Children[0], ChildDepth, Expandable: false, Expanded: false, Current: true),
            ],
            model.Flatten());
    }

    [Fact]
    public void ExpandPathTo_prefers_a_child_over_a_root_with_the_same_ref()
    {
        // Arrange
        var model = Sample();
        // Act
        model.ExpandPathTo("GEN.1");
        // Assert
        Assert.Equal(GenesisOne, model.CurrentId);
    }

    [Fact]
    public void Leaves_never_expand()
    {
        // Arrange
        var model = Sample();
        var (genesis, exodus) = (model.Roots[0], model.Roots[1]);
        // Act
        model.Toggle(GenesisOne);
        // Assert
        Assert.Equal(
            [
                new Row(genesis, RootDepth, Expandable: true, Expanded: false, Current: false),
                new Row(exodus, RootDepth, Expandable: true, Expanded: false, Current: false),
            ],
            model.Flatten());
    }

    [Fact]
    public void SetExpanded_keeps_only_real_expandable_roots()
    {
        // Arrange
        var model = Sample();
        // Act
        model.SetExpanded([Exodus, GenesisOne, "nope"]);
        // Assert
        Assert.Equal([Exodus], model.ExpandedIds);
    }

    [Fact]
    public void SetExpanded_replaces_a_prior_expansion_rather_than_adding_to_it()
    {
        // Arrange
        var model = Sample();
        model.SetExpanded([Genesis]);
        // Act
        model.SetExpanded([Exodus]);
        // Assert
        Assert.Equal([Exodus], model.ExpandedIds);
    }

    private const string Leaf = "Container:leaf";
    private const string Grove = "Container:grove";
    private const string GroveOne = "Container:grove-1";

    private static ContentsTreeModel WithALeafRoot() => ContentsTreeModel.From(new Contents(corpus: Corpus.Bible, version: Wire.Root("v"), roots:
    [
        new(id: Wire.Node(Leaf), title: "Leaf", kind: ContentsRootKind.Book, group: Testament.OT, @ref: Wire.Read<ContentsReference>("LEAF.1"), locus: GenesisOneOne, children: []),
        new(id: Wire.Node(Grove), title: "Grove", kind: ContentsRootKind.Book, group: Testament.OT, @ref: Wire.Read<ContentsReference>("GROVE.1"), locus: ExodusOneOne, children:
        [
            new(id: Wire.Node(GroveOne), title: "1", kind: ContentsChildKind.Chapter, @ref: Wire.Read<ContentsReference>("GROVE.1"), locus: ExodusOneOne, count: 1),
        ]),
    ]));

    [Fact]
    public void A_root_with_no_children_is_not_expandable()
    {
        // Arrange
        var model = WithALeafRoot();
        // Act
        var leaf = model.Roots[0];
        // Assert
        Assert.False(leaf.Expandable);
    }

    [Fact]
    public void Toggle_on_a_root_with_no_children_leaves_nothing_expanded_even_though_another_root_is_expandable()
    {
        // Arrange
        var model = WithALeafRoot();
        // Act
        model.Toggle(Leaf);
        // Assert
        Assert.Empty(model.ExpandedIds);
    }

    [Fact]
    public void Expand_adds_an_id_whose_root_is_expandable()
    {
        // Arrange
        var model = Sample();
        // Act
        model.Expand(Genesis);
        // Assert
        Assert.Equal([Genesis], model.ExpandedIds);
    }

    [Fact]
    public void Expand_ignores_an_id_no_root_carries_even_though_another_root_is_expandable()
    {
        // Arrange
        var model = Sample();
        // Act
        model.Expand("nonexistent-id");
        // Assert
        Assert.Empty(model.ExpandedIds);
    }

    [Fact]
    public void ExpandPathTo_leaves_the_current_id_unset_when_nothing_matches_the_ref()
    {
        // Arrange
        var model = WithALeafRoot();
        // Act
        model.ExpandPathTo("nothing-matches-this");
        // Assert
        Assert.Null(model.CurrentId);
    }

    [Fact]
    public void ExpandPathTo_sets_the_current_id_to_a_root_whose_own_ref_matches_when_no_child_does()
    {
        // Arrange
        var model = WithALeafRoot();
        // Act
        model.ExpandPathTo("LEAF.1");
        // Assert
        Assert.Equal(Leaf, model.CurrentId);
    }
}
