using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

// Batch UX-1 (FRONTIER-ORDER-1, owner order verbatim: "when clicking on a
// verse, things should be ordered as such (visually): Verse, Event,
// Catechism, Parallels, then cross references"): direct, isolated proof
// that PopoverSectionRegistry.Providers -- the ORDER-KEYED registry
// (Explore/PopoverSections.cs) ExplorerPopover.razor renders straight
// through, unchanged -- actually resolves the VERSE/PASSAGE frontier in
// the owner's own ruled sequence. Pinned against the REAL registry output
// (not a copy of the Order values) so a future edit that reorders a
// registration line without meaning to breaks this test immediately,
// naming exactly which two providers swapped.
public class PopoverSectionRegistryTests
{
    private static int IndexOfProvider<T>() where T : IPopoverSectionProvider =>
        PopoverSectionRegistry.Providers.ToList().FindIndex(p => p is T);

    [Fact]
    public void VerseComesBeforeEvent()
    {
        Assert.True(IndexOfProvider<VerseTextSectionProvider>() < IndexOfProvider<VerseEventMembershipSection>());
    }

    [Fact]
    public void EventComesBeforeCatechism()
    {
        Assert.True(IndexOfProvider<VerseEventMembershipSection>() < IndexOfProvider<CatechismSeamSection>());
    }

    [Fact]
    public void CatechismComesBeforeParallels()
    {
        Assert.True(IndexOfProvider<CatechismSeamSection>() < IndexOfProvider<VerseParallelsSection>());
    }

    [Fact]
    public void ParallelsComesBeforeCrossReferences()
    {
        Assert.True(IndexOfProvider<VerseParallelsSection>() < IndexOfProvider<CrossRefsSection>());
    }

    // The general-kind Passage membership row is the sibling of EVENT
    // membership (PERI-1's own "dated first, general second" rule) --
    // still ahead of Catechism/Parallels/CrossRefs in the owner's own
    // ruled sequence.
    [Fact]
    public void GeneralPassageMembershipComesAfterEventAndBeforeCatechism()
    {
        var eventIdx = IndexOfProvider<VerseEventMembershipSection>();
        var passageIdx = IndexOfProvider<VersePassageMembershipSection>();
        var catechismIdx = IndexOfProvider<CatechismSeamSection>();
        Assert.True(eventIdx < passageIdx);
        Assert.True(passageIdx < catechismIdx);
    }

    // Every provider actually appears exactly once -- a sanity guard that
    // the OrderBy projection never silently drops or duplicates an entry.
    [Fact]
    public void EveryVersePassageFrontierProviderAppearsExactlyOnce()
    {
        var providers = PopoverSectionRegistry.Providers;
        Assert.Single(providers, p => p is VerseTextSectionProvider);
        Assert.Single(providers, p => p is VerseEventMembershipSection);
        Assert.Single(providers, p => p is VersePassageMembershipSection);
        Assert.Single(providers, p => p is CatechismSeamSection);
        Assert.Single(providers, p => p is VerseParallelsSection);
        Assert.Single(providers, p => p is CrossRefsSection);
    }

    // Batch ATTEST-1, THE OWNER'S PLACEMENT AMENDMENT (verbatim): "let's
    // have a 'Similar Accounts' or something similar added to the frontier
    // part of the UI where it was getting pulled in as a parallel account.
    // Have that section be right below the 'Parallel ..' section."
    //
    // ADJACENCY, not a number. Asserting `Order == 161` would pass while a
    // future provider registered at 160.5-equivalent (any value between the
    // two, or a tie resolved by array position) silently slid in between
    // and pushed Similar Accounts away from the parallels it is supposed to
    // sit under. What the owner asked for is that a row which was WRONGLY
    // appearing as a parallel account moves down EXACTLY ONE section -- so
    // that is what is pinned: consecutive indices in the real resolved
    // registry, nothing permitted between them.
    [Fact]
    public void SimilarAccountsRendersImmediatelyBelowParallelAccounts()
    {
        var parallels = IndexOfProvider<EventWitnessesSection>();
        var similar = IndexOfProvider<EventAnaloguesSection>();
        Assert.True(parallels >= 0, "PARALLEL ACCOUNTS (EventWitnessesSection) must be registered");
        Assert.True(similar >= 0, "SIMILAR ACCOUNTS (EventAnaloguesSection) must be registered");
        Assert.Equal(parallels + 1, similar);
    }

    // ... and MENTIONED IN follows both, never between them (it is a
    // different claim again -- scriptural basis, not a relation to another
    // event -- and the owner's amendment named the analogue specifically as
    // the section that sits under the parallels).
    [Fact]
    public void MentionedInFollowsSimilarAccounts()
    {
        Assert.True(IndexOfProvider<EventAnaloguesSection>() < IndexOfProvider<EventMentionsSection>());
    }

    [Fact]
    public void EveryEventFrontierProviderAddedThisBatchAppearsExactlyOnce()
    {
        var providers = PopoverSectionRegistry.Providers;
        Assert.Single(providers, p => p is EventAnaloguesSection);
        Assert.Single(providers, p => p is EventMentionsSection);
    }
}
