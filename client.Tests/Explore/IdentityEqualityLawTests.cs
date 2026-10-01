using System.Text.RegularExpressions;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.State;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public sealed class IdentityEqualityLawTests
{
    private static readonly NodeRef Moses = ServedGraph.Ref(NodeKind.Person, "Person:moses_2108", "Moses");
    private static readonly NodeRef MosesRelabelled = Moses with { Label = "Moses, the man of God" };
    private static readonly NodeRef Aaron = ServedGraph.Ref(NodeKind.Person, "Person:aaron_1", "Aaron");
    private static readonly NodeRef MosesAsPlace = Moses with { Kind = NodeKind.Place };
    private static readonly EdgeRef Mention = ServedGraph.EdgeRef(EdgeKind.Mentions, "Mentions:00ee", "EXO.2.10 · Mentions · Moses");
    private static readonly EdgeRef MentionRelabelled = Mention with { Label = "a relabelled mention" };
    private static readonly Regex EqualityMember = new(@"(bool Equals\(|int GetHashCode\()[^;]*;", RegexOptions.Compiled);
    private static readonly Regex KindAndId = new(@"(?=[\s\S]*\bKind\b)(?=[\s\S]*\bId\b)", RegexOptions.Compiled);

    [Fact]
    public void Every_wrapper_of_an_element_is_equal_across_labels_and_unequal_across_ids_and_kinds()
    {
        // Arrange
        var relabelled = Explorables(Moses, MosesRelabelled);
        var other = Explorables(Moses, Aaron);
        var rekinded = Explorables(Moses, MosesAsPlace);

        // Act
        var wrappers = new (string Wrapper, bool Relabelled, bool OtherId, bool OtherKind)[]
        {
            ("NodeRef", Same(Moses, MosesRelabelled), Same(Moses, Aaron), Same(Moses, MosesAsPlace)),
            ("node PositionRef", Same(At(Moses), At(MosesRelabelled)), Same(At(Moses), At(Aaron)), Same(At(Moses), At(MosesAsPlace))),
            ("edge PositionRef", Same(ServedGraph.AtEdge(Mention), ServedGraph.AtEdge(MentionRelabelled)), Same(ServedGraph.AtEdge(Mention), ServedGraph.AtEdge(Mention with { Id = "Mentions:00ff" })), Same(ServedGraph.AtEdge(Mention), ServedGraph.AtEdge(Mention with { Kind = EdgeKind.Cites }))),
            ("Explorable", Same(relabelled.First, relabelled.Second), Same(other.First, other.Second), Same(rekinded.First, rekinded.Second)),
            ("Link", Same(new Link(EdgeKind.Mentions, At(Moses)), new Link(EdgeKind.Mentions, At(MosesRelabelled))), Same(new Link(EdgeKind.Mentions, At(Moses)), new Link(EdgeKind.Mentions, At(Aaron))), Same(new Link(EdgeKind.Mentions, At(Moses)), new Link(EdgeKind.Mentions, At(MosesAsPlace)))),
            ("Step", Same(new Step(EdgeKind.Mentions, relabelled.First), new Step(EdgeKind.Mentions, relabelled.Second)), Same(new Step(EdgeKind.Mentions, other.First), new Step(EdgeKind.Mentions, other.Second)), Same(new Step(EdgeKind.Mentions, rekinded.First), new Step(EdgeKind.Mentions, rekinded.Second))),
            ("Exploration", Same(new Exploration(relabelled.First, []), new Exploration(relabelled.Second, [])), Same(new Exploration(other.First, []), new Exploration(other.Second, [])), Same(new Exploration(rekinded.First, []), new Exploration(rekinded.Second, []))),
        };

        // Assert
        Assert.Equal(
            new[] { "NodeRef", "node PositionRef", "edge PositionRef", "Explorable", "Link", "Step", "Exploration" }.Select(wrapper => (wrapper, true, false, false)),
            wrappers);
    }

    [Fact]
    public void Selecting_a_node_again_under_another_label_deselects_it()
    {
        // Arrange
        var selected = new ToggleSelection(Moses).Apply(Selection.Empty);

        // Act
        var toggled = new ToggleSelection(MosesRelabelled).Apply(selected);

        // Assert
        Assert.Empty(toggled);
    }

    [Fact]
    public void No_equality_member_restates_an_elements_identity_as_its_kind_and_id()
    {
        // Arrange
        var sources = ConformanceTests.ClientSourceFiles();

        // Act
        var declaring = sources
            .Where(source => EqualityMember.Matches(File.ReadAllText(source)).Any(member => KindAndId.IsMatch(member.Value)))
            .Select(Path.GetFileName);

        // Assert
        Assert.Empty(declaring);
    }

    private static PositionRef At(NodeRef node) => ServedGraph.At(node);

    private static (Explorable First, Explorable Second) Explorables(NodeRef first, NodeRef second) =>
        (Resolved.Node(first), Resolved.Node(second));

    private static bool Same(NodeRef first, NodeRef second) =>
        PositionIdentity.Comparer.Equals(first, second) && PositionIdentity.Comparer.GetHashCode(first) == PositionIdentity.Comparer.GetHashCode(second);

    private static bool Same(PositionRef first, PositionRef second) =>
        PositionIdentity.Comparer.Equals(first, second) && PositionIdentity.Comparer.GetHashCode(first) == PositionIdentity.Comparer.GetHashCode(second);

    private static bool Same<T>(T first, T second) where T : notnull =>
        first.Equals(second) && first.GetHashCode() == second.GetHashCode();
}
