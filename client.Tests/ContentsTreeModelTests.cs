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

    private static ContentsTreeModel Sample() => ContentsTreeModel.From(new Contents(Corpus.Bible, new List<ContentsRoot>
    {
        new(new List<ContentsChild>
        {
            new(31, GenesisOne, ContentsChildKind.Chapter, "GEN.1", "1"),
            new(25, GenesisTwo, ContentsChildKind.Chapter, "GEN.2", "2"),
        }, Testament.OT, Genesis, ContentsRootKind.Book, "GEN.1", "Genesis"),
        new(new List<ContentsChild>
        {
            new(22, ExodusOne, ContentsChildKind.Chapter, "EXO.1", "1"),
        }, Testament.OT, Exodus, ContentsRootKind.Book, "EXO.1", "Exodus"),
    }, "v"));

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
        // Act
        var rows = Rows(model);
        // Assert
        Assert.Equal([(Genesis, 0, true, false, false), (Exodus, 0, true, false, false)], rows);
    }

    [Fact]
    public void Toggle_expands_a_root_to_show_its_children()
    {
        // Arrange
        var model = Sample();
        // Act
        model.Toggle(Genesis);
        // Assert
        Assert.Equal(
            [(Genesis, 0, true, true, false), (GenesisOne, 1, false, false, false), (GenesisTwo, 1, false, false, false), (Exodus, 0, true, false, false)],
            Rows(model));
    }

    [Fact]
    public void Toggle_is_an_involution_for_any_id_sequence()
    {
        // Arrange
        var model = Sample();
        var before = Rows(model);
        // Act
        foreach (var id in new[] { Exodus, Genesis, Exodus })
        {
            model.Toggle(id);
            model.Toggle(id);
        }
        // Assert
        Assert.Equal(before, Rows(model));
    }

    [Fact]
    public void ExpandPathTo_opens_exactly_the_ancestors_of_the_current_ref()
    {
        // Arrange
        var model = Sample();
        // Act
        model.ExpandPathTo("EXO.1");
        // Assert
        Assert.Equal([(Genesis, 0, true, false, false), (Exodus, 0, true, true, false), (ExodusOne, 1, false, false, true)], Rows(model));
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
        // Act
        model.Toggle(GenesisOne);
        // Assert
        Assert.Empty(model.ExpandedIds);
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

    private static (string Id, int Depth, bool Expandable, bool Expanded, bool Current)[] Rows(ContentsTreeModel model) =>
        model.Flatten().Select(r => (r.Node.Id, r.Depth, r.Expandable, r.Expanded, r.Current)).ToArray();
}
