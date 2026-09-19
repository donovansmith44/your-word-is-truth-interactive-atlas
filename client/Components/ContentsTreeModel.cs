namespace BibleAtlas.Client.Components;

/// <summary>
/// D4 (owner, 2026-09-15, verbatim: "Table of contents = a tree. ...
/// Contents is a tree; clicking a node toggles visibility of its children.
/// Stop at the level of ARTICLE (BoC) or TOPIC (Small Catechism). Pages are
/// not a meaningful way of thinking about things."): the PURE, testable model
/// behind <c>ContentsTree.razor</c> -- the two-level containment forest
/// <c>GET /api/contents/{corpus}</c> serves (books ⊃ chapters; documents ⊃
/// articles), plus which roots are expanded and which node is current.
/// <see cref="Flatten"/> is the one projection the component renders: the
/// visible rows, in order, with their depth. Toggling an id twice always
/// restores the rows (an involution -- <c>ContentsTreeModelTests</c> pins
/// it); leaves never expand.
/// </summary>
public sealed class ContentsTreeModel
{
    public sealed class Node
    {
        public required string Id { get; init; }
        public required string Title { get; init; }
        /// <summary><c>book</c> | <c>chapter</c> | <c>document</c> | <c>article</c>.</summary>
        public required string Kind { get; init; }
        /// <summary><c>OT</c> | <c>NT</c> for a Bible book; null otherwise.</summary>
        public string? Group { get; init; }
        /// <summary>The navigation target (<c>GEN.2</c>, <c>BoC 7.2.1</c>).</summary>
        public required string Ref { get; init; }
        /// <summary>A leaf's own member count (verses, paragraphs); null for a root.</summary>
        public int? Count { get; init; }
        public IReadOnlyList<Node> Children { get; init; } = Array.Empty<Node>();
        public bool Expandable => Children.Count > 0;
    }

    public sealed record Row(Node Node, int Depth, bool Expandable, bool Expanded, bool Current);

    private readonly HashSet<string> _expanded = new(StringComparer.Ordinal);

    private ContentsTreeModel(string corpus, IReadOnlyList<Node> roots)
    {
        Corpus = corpus;
        Roots = roots;
    }

    public string Corpus { get; }
    public IReadOnlyList<Node> Roots { get; }
    /// <summary>The id of the node <see cref="ExpandPathTo"/> last landed on, or null.</summary>
    public string? CurrentId { get; private set; }

    public static ContentsTreeModel From(ContentsOut contents) =>
        new(
            contents.Corpus,
            contents.Roots.Select(r => new Node
            {
                Id = r.Id,
                Title = r.Title,
                Kind = r.Kind,
                Group = r.Group,
                Ref = r.Ref,
                Children = r.Children.Select(c => new Node { Id = c.Id, Title = c.Title, Kind = c.Kind, Ref = c.Ref, Count = c.Count }).ToList(),
            }).ToList());

    public bool IsExpanded(string id) => _expanded.Contains(id);

    /// <summary>The ids currently expanded, for a per-viewer persistence convenience.</summary>
    public IReadOnlyCollection<string> ExpandedIds => _expanded;

    /// <summary>Expands or collapses one root; a leaf (nothing to show) is a no-op.</summary>
    public void Toggle(string id)
    {
        var node = Roots.FirstOrDefault(r => r.Id == id);
        if (node is null || !node.Expandable)
        {
            return;
        }

        if (!_expanded.Remove(id))
        {
            _expanded.Add(id);
        }
    }

    /// <summary>Opens one root (a leaf or unknown id is a no-op).</summary>
    public void Expand(string id)
    {
        if (Roots.Any(r => r.Id == id && r.Expandable))
        {
            _expanded.Add(id);
        }
    }

    public void SetExpanded(IEnumerable<string> ids)
    {
        _expanded.Clear();
        foreach (var id in ids)
        {
            if (Roots.Any(r => r.Id == id && r.Expandable))
            {
                _expanded.Add(id);
            }
        }
    }

    /// <summary>
    /// Opens exactly the ancestors of the node whose <c>Ref</c> is
    /// <paramref name="sref"/> (a child first; a root when only a root
    /// matches) and makes it current. Unknown refs change nothing.
    /// </summary>
    public void ExpandPathTo(string sref)
    {
        foreach (var root in Roots)
        {
            var child = root.Children.FirstOrDefault(c => c.Ref == sref);
            if (child is not null)
            {
                if (root.Expandable)
                {
                    _expanded.Add(root.Id);
                }

                CurrentId = child.Id;
                return;
            }
        }

        var rootMatch = Roots.FirstOrDefault(r => r.Ref == sref);
        if (rootMatch is not null)
        {
            CurrentId = rootMatch.Id;
        }
    }

    /// <summary>The visible rows, in order: every root; each expanded root's children right after it.</summary>
    public IReadOnlyList<Row> Flatten()
    {
        var rows = new List<Row>();
        foreach (var root in Roots)
        {
            var expanded = root.Expandable && _expanded.Contains(root.Id);
            rows.Add(new Row(root, 0, root.Expandable, expanded, root.Id == CurrentId));
            if (expanded)
            {
                foreach (var child in root.Children)
                {
                    rows.Add(new Row(child, 1, false, false, child.Id == CurrentId));
                }
            }
        }

        return rows;
    }
}
