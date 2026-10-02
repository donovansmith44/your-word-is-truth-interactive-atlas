using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public abstract record Presentation
{
    private Presentation()
    {
    }

    public static Form? Of(ElementKind kind, Surface surface) =>
        kind.Match(node: node => OfNode(node, surface), edge: _ => OfEdge(surface));

    public static bool Offers(Link link, Surface surface) => Of(ElementKind.Of(link.Target), surface) is not null;

    private static Form? OfNode(NodeKind kind, Surface surface) => surface switch
    {
        Surface.World => kind switch
        {
            NodeKind.Map or NodeKind.Place or NodeKind.Polity or NodeKind.Era => Form.Geography,
            NodeKind.TextUnit or NodeKind.Container or NodeKind.Event or NodeKind.Narrative or NodeKind.Person
                or NodeKind.Anchor or NodeKind.CatechismItem or NodeKind.Source or NodeKind.Translation
                or NodeKind.PeopleGroup or NodeKind.CommentaryItem or NodeKind.LexiconEntry => null,
        },
        Surface.Reader => kind switch
        {
            NodeKind.Container => Form.Sequence,
            NodeKind.TextUnit => Form.Text,
            NodeKind.Event or NodeKind.Narrative or NodeKind.Place or NodeKind.Person or NodeKind.Anchor
                or NodeKind.Era or NodeKind.Polity or NodeKind.CatechismItem or NodeKind.Source or NodeKind.Translation
                or NodeKind.PeopleGroup or NodeKind.CommentaryItem or NodeKind.LexiconEntry or NodeKind.Map => null,
        },
        Surface.Popover => kind switch
        {
            NodeKind.TextUnit or NodeKind.Container or NodeKind.Event or NodeKind.Narrative or NodeKind.Place
                or NodeKind.Person or NodeKind.Anchor or NodeKind.Era or NodeKind.Polity or NodeKind.CatechismItem
                or NodeKind.Source or NodeKind.Translation or NodeKind.PeopleGroup or NodeKind.CommentaryItem
                or NodeKind.LexiconEntry or NodeKind.Map => Form.Card,
        },
    };

    private static Form? OfEdge(Surface surface) => surface switch
    {
        Surface.World or Surface.Reader => null,
        Surface.Popover => Form.Card,
    };

    public enum Form
    {
        Card,
        Sequence,
        Text,
        Geography,
    }

    public sealed record Card(string Title, IReadOnlyList<Field> Fields) : Presentation
    {
        public bool Equals(Card? other) => other is not null && Title == other.Title && Fields.SequenceEqual(other.Fields);

        public override int GetHashCode() => Fields.Aggregate(Title.GetHashCode(), HashCode.Combine);
    }

    public sealed record Field(string Name, string Value);

    public sealed record Geography(Frame Frame, Emphasis Emphasis) : Presentation;
}
