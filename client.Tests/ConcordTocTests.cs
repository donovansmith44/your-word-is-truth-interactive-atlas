using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

// Batch UX-1 (BOC-CLICK-1, owner order verbatim: "Articles/blocks of text
// needn't be clickable in the BoC right now ... The exception should be
// the catechism and the creeds where we actually do have explorable
// stuff."): direct, isolated proof of ConcordToc.IsExplorablePart -- the
// ONE place that decides which Concord documents' own rows are explorable
// -- pinned here so a future batch cannot silently widen (or narrow) the
// explorable set without a failing test naming exactly what changed. See
// that method's own doc comment for the server-verified (not guessed)
// reasoning behind "Part 7 only."
public class ConcordTocTests
{
    [Fact]
    public void TheSmallCatechismPartIsExplorable()
    {
        Assert.True(ConcordToc.IsExplorablePart(7));
    }

    [Theory]
    [InlineData(1)] // Preface
    [InlineData(2)] // The Three Ecumenical Creeds -- no SC-overlap linkage exists for this document
    [InlineData(3)] // The Augsburg Confession
    [InlineData(4)] // Apology of the Augsburg Confession
    [InlineData(5)] // The Smalcald Articles
    [InlineData(6)] // Treatise on the Power and Primacy of the Pope
    [InlineData(8)] // The Large Catechism
    [InlineData(9)] // Formula of Concord: Epitome
    [InlineData(10)] // Formula of Concord: Solid Declaration
    public void EveryOtherDocumentIsNotExplorable(int part)
    {
        Assert.False(ConcordToc.IsExplorablePart(part));
    }

    [Fact]
    public void AnUnrecognizedPartNumberIsAlsoNotExplorable()
    {
        Assert.False(ConcordToc.IsExplorablePart(0));
        Assert.False(ConcordToc.IsExplorablePart(11));
    }

    // The predicate's own explorable set must stay a proper (non-empty,
    // non-total) subset of the real ten documents -- a defensive guard
    // against a future edit accidentally clearing or over-widening it.
    [Fact]
    public void ExactlyOneOfTheTenDocumentsIsExplorable()
    {
        var explorableCount = ConcordToc.Documents.Count(d => ConcordToc.IsExplorablePart(d.Part));
        Assert.Equal(1, explorableCount);
    }
}
