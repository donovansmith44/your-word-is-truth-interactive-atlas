using BibleAtlas.Client;
using BibleAtlas.Client.Components;

namespace BibleAtlas.Client.Tests;

/// <summary>
/// D4 (owner, 2026-09-15): the contents tree's pure model -- collapsed by
/// default, toggling is an involution, leaves never expand, the current
/// ref's ancestors open.
/// </summary>
public class ContentsTreeModelTests
{
    private static ContentsTreeModel Sample() => ContentsTreeModel.From(new ContentsOut("bible", "v", new List<ContentsRootOut>
    {
        new("Container:bible-book-GEN", "Genesis", "book", "OT", "GEN.1", new List<ContentsChildOut>
        {
            new("Container:bible-chapter-GEN-1", "1", "chapter", "GEN.1", 31),
            new("Container:bible-chapter-GEN-2", "2", "chapter", "GEN.2", 25),
        }),
        new("Container:bible-book-EXO", "Exodus", "book", "OT", "EXO.1", new List<ContentsChildOut>
        {
            new("Container:bible-chapter-EXO-1", "1", "chapter", "EXO.1", 22),
        }),
    }));

    [Fact]
    public void Collapsed_by_default_shows_only_roots()
        => Assert.Equal(new[] { "Genesis", "Exodus" }, Sample().Flatten().Select(r => r.Node.Title));

    [Fact]
    public void Toggle_expands_then_collapses_children()
    {
        var m = Sample();
        m.Toggle("Container:bible-book-GEN");
        Assert.Equal(new[] { "Genesis", "1", "2", "Exodus" }, m.Flatten().Select(r => r.Node.Title));
        Assert.Equal(new[] { 0, 1, 1, 0 }, m.Flatten().Select(r => r.Depth));
        m.Toggle("Container:bible-book-GEN");
        Assert.Equal(new[] { "Genesis", "Exodus" }, m.Flatten().Select(r => r.Node.Title));
    }

    [Fact]
    public void Toggle_is_an_involution_for_any_id_sequence()
    {
        var m = Sample();
        var before = m.Flatten().Select(r => r.Node.Id).ToList();
        foreach (var id in new[] { "Container:bible-book-EXO", "Container:bible-book-GEN", "Container:bible-book-EXO" })
        {
            m.Toggle(id);
            m.Toggle(id);
        }

        Assert.Equal(before, m.Flatten().Select(r => r.Node.Id));
    }

    [Fact]
    public void ExpandPathTo_opens_exactly_the_ancestors_of_the_current_ref()
    {
        var m = Sample();
        m.ExpandPathTo("EXO.1");
        Assert.Equal(new[] { "Genesis", "Exodus", "1" }, m.Flatten().Select(r => r.Node.Title));
        Assert.Equal("Container:bible-chapter-EXO-1", m.CurrentId);
        Assert.Single(m.Flatten(), r => r.Current);
    }

    [Fact]
    public void ExpandPathTo_prefers_a_child_over_a_root_with_the_same_ref()
    {
        var m = Sample();
        m.ExpandPathTo("GEN.1"); // both the Genesis root and its chapter 1 carry GEN.1
        Assert.Equal("Container:bible-chapter-GEN-1", m.CurrentId);
    }

    [Fact]
    public void Leaves_never_expand()
    {
        var m = Sample();
        m.Toggle("Container:bible-chapter-GEN-1");
        Assert.Equal(new[] { "Genesis", "Exodus" }, m.Flatten().Select(r => r.Node.Title));
        Assert.Empty(m.ExpandedIds);
    }

    [Fact]
    public void SetExpanded_keeps_only_real_expandable_roots()
    {
        var m = Sample();
        m.SetExpanded(new[] { "Container:bible-book-EXO", "Container:bible-chapter-GEN-1", "nope" });
        Assert.Equal(new[] { "Container:bible-book-EXO" }, m.ExpandedIds);
    }
}
