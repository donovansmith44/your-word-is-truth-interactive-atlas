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
}
