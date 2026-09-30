using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class KinshipTests
{
    private static NodeRef Person(string id, string label) => new(id: $"Person:{id}", kind: PositionKind.Person, label: label);

    private static EdgeEntry Kin(NodeRef node, Parentage parentage) =>
        new(edge: $"ParentOf:{node.Id}", loci: null, narrative: null, node: node, note: null, parentage: parentage, votes: null);

    private static List<string> Shape(IEnumerable<KinGroup> groups) =>
        groups.Select(g => $"{g.Heading} | {g.TestId} | {string.Join(", ", g.People.Select(p => p.Id))} | {g.Via}").ToList();

    private static readonly NodeRef God = Person("god_1324", "God");
    private static readonly NodeRef Mary = Person("mary_1938", "Mary (Mother of Jesus)");
    private static readonly NodeRef Joseph = Person("joseph_1715", "Joseph (Mary's Husband)");
    private static readonly NodeRef Jesus = Person("jesus_905", "Jesus");
    private static readonly NodeRef James = Person("james_719", "James");
    private static readonly NodeRef Adam = Person("adam_78", "Adam");
    private static readonly NodeRef Eve = Person("eve_1231", "Eve");

    [Fact]
    public void Every_parentage_is_labelled_from_both_ends()
    {
        // Act
        var labels = Enum.GetValues<Parentage>().Select(p => (p, Kinship.AsParent(p), Kinship.AsChild(p))).ToList();
        // Assert
        Assert.Equal(
            [
                (Parentage.Natural, "Parents", "Children"),
                (Parentage.Eternal, "Father (eternal Son of God)", "Only begotten Son"),
                (Parentage.Virgin, "Mother (born of the Virgin)", "Son (born of the Virgin)"),
                (Parentage.Legal, "Legal father (as was supposed)", "Son (as was supposed)"),
                (Parentage.Created, "Created by God", "Created"),
            ],
            labels);
    }

    [Fact]
    public void Only_an_eternal_or_created_parentage_makes_no_siblings()
    {
        // Act
        var making = Enum.GetValues<Parentage>().Where(Kinship.MakesSiblings).ToList();
        // Assert
        Assert.Equal([Parentage.Natural, Parentage.Virgin, Parentage.Legal], making);
    }

    [Fact]
    public void Jesus_reads_the_eternal_father_the_virgin_mother_and_the_supposed_father_each_under_its_own_label()
    {
        // Act
        var groups = Kinship.Groups(parents: [Kin(God, Parentage.Eternal), Kin(Joseph, Parentage.Legal), Kin(Mary, Parentage.Virgin)], spouses: [], children: [], siblings: [], brethren: [James]);
        // Assert
        Assert.Equal(
            [
                "Father (eternal Son of God) | parents-eternal | Person:god_1324 | ChildOf",
                "Mother (born of the Virgin) | parents-virgin | Person:mary_1938 | ChildOf",
                "Legal father (as was supposed) | parents-legal | Person:joseph_1715 | ChildOf",
                "Brethren (1) | brethren | Person:james_719 | BrethrenOf",
            ],
            Shape(groups));
    }

    [Fact]
    public void God_reads_his_only_begotten_son_apart_from_the_man_and_woman_he_created()
    {
        // Act
        var groups = Kinship.Groups(parents: [], spouses: [], children: [Kin(Adam, Parentage.Created), Kin(Eve, Parentage.Created), Kin(Jesus, Parentage.Eternal)], siblings: [], brethren: []);
        // Assert
        Assert.Equal(
            [
                "Only begotten Son | children-eternal | Person:jesus_905 | ParentOf",
                "Created | children-created | Person:adam_78, Person:eve_1231 | ParentOf",
            ],
            Shape(groups));
    }

    [Fact]
    public void A_family_reads_parents_spouses_children_siblings_and_brethren_with_their_counts()
    {
        // Act
        var groups = Kinship.Groups(parents: [Kin(Joseph, Parentage.Natural), Kin(Mary, Parentage.Natural)], spouses: [Eve], children: [Kin(Adam, Parentage.Natural)], siblings: [Jesus], brethren: [James]);
        // Assert
        Assert.Equal(
            [
                "Parents (2) | parents | Person:joseph_1715, Person:mary_1938 | ChildOf",
                "Spouses (1) | spouses | Person:eve_1231 | SpouseOf",
                "Children (1) | children | Person:adam_78 | ParentOf",
                "Siblings (1) | siblings | Person:jesus_905 | BrethrenOf",
                "Brethren (1) | brethren | Person:james_719 | BrethrenOf",
            ],
            Shape(groups));
    }
}
