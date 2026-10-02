using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class ChipTests
{
    [Fact]
    public async Task A_chip_that_pushes_names_the_link_it_follows()
    {
        // Arrange
        var authors = new IExplorable[]
        {
            new VerseNode("GEN.1.1"),
            new ChapterNode("GEN", 1),
            new PassageNode("GEN.1.1-5", "In the beginning"),
            new BookNode("GEN"),
        };
        var api = new StubbedAtlas("").Client();
        // Act
        var pushes = new List<(string Author, string Label, string Next, EdgeKind Via)>();
        foreach (var author in authors)
        {
            pushes.AddRange((await author.ExploreAsync(api))
                .Select(chip => chip.Target)
                .OfType<ChipTarget.Push>()
                .Select(push => (author.GetType().Name, push.Next.Title, push.Next.Identity.Id, push.Via)));
        }
        // Assert
        Assert.Equal(
            [
                ("VerseNode", "GEN", "Container:bible-book-GEN", EdgeKind.MemberOf),
                ("ChapterNode", "GEN", "Container:bible-book-GEN", EdgeKind.MemberOf),
                ("PassageNode", "GEN", "Container:bible-book-GEN", EdgeKind.MemberOf),
                ("BookNode", "GEN", "Container:bible-book-GEN", EdgeKind.AuthoredBy),
            ],
            pushes);
    }
}
