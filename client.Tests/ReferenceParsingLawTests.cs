using System.Text.RegularExpressions;

namespace BibleAtlas.Client.Tests;

public sealed class ReferenceParsingLawTests
{
    private static readonly Regex ParsesOrComposesAReference = new(@"\bCanonRef\.|\bLegacyNodeIds\.\w+[(<]", RegexOptions.Compiled);

    private static readonly IReadOnlyDictionary<string, Retirement> RetiredBy = new Dictionary<string, Retirement>
    {
        ["Components/ArrowNav.razor"] = new(Batch.Focus5, Sites: 3),
        ["Components/MentionScan.razor"] = new(Batch.Maps, Sites: 2),
        ["Components/PassageList.razor"] = new(Batch.Maps, Sites: 8),
        ["Exploring/EventAccounts.cs"] = new(Batch.Focus5, Sites: 9),
        ["Exploring/KretzmannCitationScan.cs"] = new(Batch.Focus7, Sites: 1),
        ["Exploring/PassageGrouping.cs"] = new(Batch.Maps, Sites: 4),
        ["Legacy/CatechismNode.cs"] = new(Batch.Focus7, Sites: 1),
        ["Legacy/CommentaryItemNode.cs"] = new(Batch.Focus7, Sites: 1),
        ["Legacy/EventNode.cs"] = new(Batch.Focus5, Sites: 1),
        ["Legacy/LegacyNodes.cs"] = new(Batch.Focus9, Sites: 6),
        ["Legacy/LegacyTextUnits.cs"] = new(Batch.Maps, Sites: 1),
        ["Legacy/LegacySaves.cs"] = new(Batch.Focus9, Sites: 9),
        ["Legacy/PassageBlock.cs"] = new(Batch.Maps, Sites: 5),
        ["Legacy/PassageNode.cs"] = new(Batch.Focus3, Sites: 6),
        ["Legacy/PolityDeltaNode.cs"] = new(Batch.Maps, Sites: 1),
        ["Legacy/PopoverSectionProviders.cs"] = new(Batch.Maps, Sites: 10),
        ["Pages/Concord.razor"] = new(Batch.Focus3, Sites: 1),
        ["Pages/Kretzmann.razor"] = new(Batch.Focus7, Sites: 2),
    };

    [Fact]
    public void No_client_source_parses_or_composes_a_reference_outside_its_listed_sites()
    {
        // Arrange
        var listed = RetiredBy;

        // Act
        var offences = SourceRatchet.OffencesOutside(ParsesOrComposesAReference, listed);

        // Assert
        Assert.Empty(offences);
    }

    [Fact]
    public void Every_listed_site_still_parses_or_composes_a_reference()
    {
        // Arrange
        var listed = RetiredBy;

        // Act
        var gone = SourceRatchet.ListedSitesGone(ParsesOrComposesAReference, listed);

        // Assert
        Assert.Empty(gone);
    }
}
