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

    private static ContentsTreeModel Sample() => ContentsTreeModel.From(new Contents(corpus: Corpus.Bible, version: "v", roots:
    [
        new(id: Genesis, title: "Genesis", kind: ContentsRootKind.Book, group: Testament.OT, @ref: "GEN.1", children:
        [
            new(id: GenesisOne, title: "1", kind: ContentsChildKind.Chapter, @ref: "GEN.1", count: 31),
            new(id: GenesisTwo, title: "2", kind: ContentsChildKind.Chapter, @ref: "GEN.2", count: 25),
        ]),
        new(id: Exodus, title: "Exodus", kind: ContentsRootKind.Book, group: Testament.OT, @ref: "EXO.1", children:
        [
            new(id: ExodusOne, title: "1", kind: ContentsChildKind.Chapter, @ref: "EXO.1", count: 22),
        ]),
    ]));

    [Fact]
    public void From_carries_the_contract_kinds_of_roots_and_children()
    {
        // Arrange
        var expected = new[]
        {
            new Root(Genesis, "Genesis", "GEN.1", ContentsRootKind.Book, [
                new Child(GenesisOne, "1", "GEN.1", ContentsChildKind.Chapter, 31),
                new Child(GenesisTwo, "2", "GEN.2", ContentsChildKind.Chapter, 25),
            ]),
            new Root(Exodus, "Exodus", "EXO.1", ContentsRootKind.Book, [
                new Child(ExodusOne, "1", "EXO.1", ContentsChildKind.Chapter, 22),
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
}
