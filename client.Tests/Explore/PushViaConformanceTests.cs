using System.Text.RegularExpressions;
using BibleAtlas.Client.Tests.State;

namespace BibleAtlas.Client.Tests;

public sealed class PushViaConformanceTests
{
    [Fact]
    public void Every_legacy_push_names_the_kind_of_link_it_follows()
    {
        // Arrange
        var sources = new[] { Path.Combine("client", "Explore", "PopoverSectionProviders.cs"), Path.Combine("client", "Explore", "YearNode.cs") };
        // Act
        var pushes = sources.SelectMany(source => PushesIn(File.ReadAllText(Path.Combine(ConformanceTests.RepoRoot(), source)))).ToList();
        // Assert
        Assert.Equal(
            [
                "ChapterCardSection: EdgeKind.Attests",
                "ChapterCardSection: EdgeKind.Mentions",
                "VerseTextSectionProvider: EdgeKind.Mentions",
                "CrossRefsSection: EdgeKind.Cites",
                "CatechismSeamSection: EdgeKind.CatechismLink",
                "CatechismScripturesSection: EdgeKind.CatechismLink",
                "PlaceDatesSection: EdgeKind.MentionedIn",
                "PlaceEventsSection: EdgeKind.SiteOf",
                "VerseEventMembershipSection: EdgeKind.Attests",
                "EventDateAndPlacesSection: EdgeKind.DatedBy",
                "EventDateAndPlacesSection: direction.Via",
                "EventWitnessesSection: EdgeKind.AttestedIn",
                "EventMentionsSection: EdgeKind.MentionedIn",
                "EventAnaloguesSection: EdgeKind.AnalogousTo",
                "VerseParallelsSection: EdgeKind.Parallel",
                "PolityDeltaScripturesSection: EdgeKind.JustifiedBy",
                "VersePersonsSection: EdgeKind.Mentions",
                "PersonCardAndMentionsSection: EdgeKind.MentionedIn",
                "CatechismInConcordSection: EdgeKind.CatechismLink",
                "ConcordSmallCatechismSection: EdgeKind.CatechismLink",
                "PersonSectionRendering: via",
                "PersonLifeSection: EdgeKind.MentionedIn",
                "PersonEventsSection: EdgeKind.ParticipatesIn",
                "PersonFamilySection: group.Via",
                "YearNode: EdgeKind.TemporalAdjacency",
            ],
            pushes);
    }

    private static readonly Regex ClassOrCall = new(@"class (?<class>\w+)|(?<call>\bPushAsync|\bChips)\(", RegexOptions.Compiled);

    private static IEnumerable<string> PushesIn(string source)
    {
        var owner = "";
        foreach (Match match in ClassOrCall.Matches(source))
        {
            if (match.Groups["class"].Success)
            {
                owner = match.Groups["class"].Value;
                continue;
            }

            if (owner != "PersonSectionRendering" || match.Groups["call"].Value != "Chips")
            {
                yield return $"{owner}: {LastArgument(source, match.Index + match.Length)}";
            }
        }
    }

    private static string LastArgument(string source, int openParenEnd)
    {
        var depth = 1;
        var argumentStart = openParenEnd;
        for (var i = openParenEnd; i < source.Length; i++)
        {
            switch (source[i])
            {
                case '(':
                    depth++;
                    break;
                case ')' when --depth == 0:
                    return source[argumentStart..i].Trim();
                case ',' when depth == 1:
                    argumentStart = i + 1;
                    break;
            }
        }

        throw new InvalidOperationException("an unclosed call");
    }
}
