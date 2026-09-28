using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Components;

public sealed class ContentsTreeModel
{
    public sealed class Node
    {
        public required string Id { get; init; }
        public required string Title { get; init; }
        public required string Kind { get; init; }
        public string? Group { get; init; }
        public required string Ref { get; init; }
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
    public string? CurrentId { get; private set; }

    public static ContentsTreeModel From(Contract.Contents contents) =>
        new(
            contents.Corpus.WireName(),
            contents.Roots.Select(r => new Node
            {
                Id = r.Id,
                Title = r.Title,
                Kind = r.Kind.WireName(),
                Group = r.Group?.WireName(),
                Ref = r.Ref,
                Children = r.Children.Select(c => new Node { Id = c.Id, Title = c.Title, Kind = c.Kind.WireName(), Ref = c.Ref, Count = c.Count }).ToList(),
            }).ToList());

    public bool IsExpanded(string id) => _expanded.Contains(id);

    public IReadOnlyCollection<string> ExpandedIds => _expanded;

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
