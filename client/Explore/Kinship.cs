using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record KinGroup(string Heading, string TestId, IReadOnlyList<NodeRef> People);

public static class Kinship
{
    private static readonly IReadOnlyDictionary<Parentage, (string AsParent, string AsChild)> Labels = new Dictionary<Parentage, (string, string)>
    {
        [Parentage.Natural] = ("Parents", "Children"),
        [Parentage.Eternal] = ("Father (eternal Son of God)", "Only begotten Son"),
        [Parentage.Virgin] = ("Mother (born of the Virgin)", "Son (born of the Virgin)"),
        [Parentage.Legal] = ("Legal father (as was supposed)", "Son (as was supposed)"),
        [Parentage.Created] = ("Created by God", "Created"),
    };

    public static string AsParent(Parentage parentage) => Labels[parentage].AsParent;

    public static string AsChild(Parentage parentage) => Labels[parentage].AsChild;

    public static bool MakesSiblings(Parentage parentage) => parentage is Parentage.Natural or Parentage.Virgin or Parentage.Legal;

    public static Parentage Of(EdgeEntry entry) => entry.Parentage!.Value;

    public static IReadOnlyList<KinGroup> Groups(IReadOnlyList<EdgeEntry> parents, IReadOnlyList<NodeRef> spouses, IReadOnlyList<EdgeEntry> children, IReadOnlyList<NodeRef> siblings, IReadOnlyList<NodeRef> brethren)
    {
        IEnumerable<KinGroup> all =
        [
            .. ByParentage(parents, "parents", AsParent),
            Counted("Spouses", "spouses", spouses),
            .. ByParentage(children, "children", AsChild),
            Counted("Siblings", "siblings", siblings),
            Counted("Brethren", "brethren", brethren),
        ];
        return all.Where(g => g.People.Count > 0).ToList();
    }

    private static KinGroup Counted(string label, string testId, IReadOnlyList<NodeRef> people) => new($"{label} ({people.Count})", testId, people);

    private static IEnumerable<KinGroup> ByParentage(IReadOnlyList<EdgeEntry> entries, string testId, Func<Parentage, string> label) =>
        Enum.GetValues<Parentage>().Select(parentage =>
        {
            var people = entries.Where(e => Of(e) == parentage).Nodes().ToList();
            return parentage == Parentage.Natural
                ? Counted(label(parentage), testId, people)
                : new KinGroup(label(parentage), $"{testId}-{parentage.ToString().ToLowerInvariant()}", people);
        });
}
