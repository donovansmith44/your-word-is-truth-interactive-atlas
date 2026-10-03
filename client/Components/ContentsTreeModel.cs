using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Components;

public sealed class ContentsTreeModel
{
    public abstract record Node(string Id, string Title, string Ref, TextRef Locus);

    public sealed record Root(string Id, string Title, string Ref, TextRef Locus, ContentsRootKind Kind, IReadOnlyList<Child> Children) : Node(Id, Title, Ref, Locus)
    {
        public bool Expandable => Children.Count > 0;
    }

    public sealed record Child(string Id, string Title, string Ref, TextRef Locus, ContentsChildKind Kind, int Count) : Node(Id, Title, Ref, Locus);

    public sealed record Row(Node Node, int Depth, bool Expandable, bool Expanded, bool Current);

    private readonly HashSet<string> _expanded = new(StringComparer.Ordinal);

    private ContentsTreeModel(IReadOnlyList<Root> roots) => Roots = roots;

    public IReadOnlyList<Root> Roots { get; }
    public string? CurrentId { get; private set; }

    public static ContentsTreeModel From(Contract.Contents contents) =>
        new(contents.Roots
            .Select(r => new Root(r.Id.ToString(), r.Title, r.Ref.ToString(), r.Locus, r.Kind, r.Children.Select(c => new Child(c.Id.ToString(), c.Title, c.Ref.ToString(), c.Locus, c.Kind, c.Count)).ToList()))
            .ToList());

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
