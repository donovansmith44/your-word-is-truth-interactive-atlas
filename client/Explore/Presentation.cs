using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public abstract record Presentation
{
    private Presentation()
    {
    }

    public static Form? Of(NodeKind kind, Surface surface) => surface switch
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
}
