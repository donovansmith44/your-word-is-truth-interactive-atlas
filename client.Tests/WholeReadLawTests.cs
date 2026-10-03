using System.Text.RegularExpressions;

namespace BibleAtlas.Client.Tests;

public sealed class WholeReadLawTests
{
    private static readonly Regex ReadsAWholeCollection = new(@"\bPaging\.Whole\(|\bOffersAll=""true""", RegexOptions.Compiled);

    private static readonly IReadOnlyDictionary<string, Retirement> RetiredBy = new Dictionary<string, Retirement>
    {
        ["Components/ArrowNav.razor"] = new(Batch.Focus5, Sites: 1),
        ["Components/CatechismList.razor"] = new(Batch.Focus7, Sites: 1),
        ["Components/PassageList.razor"] = new(Batch.Maps, Sites: 1),
        ["Exploring/EventAccounts.cs"] = new(Batch.Focus5, Sites: 1),
        ["Legacy/PopoverSectionProviders.cs"] = new(Batch.Maps, Sites: 7),
    };

    [Fact]
    public void No_client_source_reads_a_whole_collection_outside_its_listed_sites()
    {
        // Arrange
        var listed = RetiredBy;

        // Act
        var offences = SourceRatchet.OffencesOutside(ReadsAWholeCollection, listed);

        // Assert
        Assert.Empty(offences);
    }

    [Fact]
    public void Every_listed_site_still_reads_a_whole_collection()
    {
        // Arrange
        var listed = RetiredBy;

        // Act
        var gone = SourceRatchet.ListedSitesGone(ReadsAWholeCollection, listed);

        // Assert
        Assert.Empty(gone);
    }
}
